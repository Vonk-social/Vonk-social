//! Authentication HTTP routes.
//!
//! * `GET  /api/auth/login/google`      — redirect to Google consent screen
//! * `GET  /api/auth/callback/google`   — finish OAuth flow, issue cookies
//! * `POST /api/auth/refresh`           — rotate access JWT
//! * `POST /api/auth/logout`            — invalidate session

use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{Duration as ChronoDuration, Utc};
use redis::AsyncCommands;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{cookies, jwt, oauth_apple, oauth_github, oauth_google};
use crate::error::{ApiError, ApiResult};
use crate::models::User;
use crate::state::AppState;

/// Build the `/api/auth/...` router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login/google", get(login_google))
        .route("/api/auth/callback/google", get(callback_google))
        .route("/api/auth/login/github", get(login_github))
        .route("/api/auth/callback/github", get(callback_github))
        .route("/api/auth/login/apple", get(login_apple))
        .route("/api/auth/callback/apple", post(callback_apple))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
}

// ── /login/google ────────────────────────────────────────────

async fn login_google(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Redirect> {
    if !state.config.google_configured() {
        return Err(ApiError::bad_request(
            "google_not_configured",
            "Google OAuth credentials are not configured on the server",
        ));
    }

    let csrf = oauth_google::random_state();
    let pkce = oauth_google::PkcePair::new_sha256();

    // Store the verifier + the origin the user was on when they started the
    // flow, so the callback — which may not see the same Host — can rebuild
    // the same redirect_uri. Google enforces byte-for-byte equality of
    // redirect_uri between authorize and token endpoints.
    let origin = origin_from_headers(&headers);
    let mut conn = state.redis.clone();
    let key = redis_key(&csrf);
    let payload = format!(
        "{}\n{}",
        pkce.verifier,
        origin.as_deref().unwrap_or("")
    );
    let _: () = conn.set_ex(&key, &payload, 600).await?;

    let url =
        oauth_google::build_authorize_url(&state.config, origin.as_deref(), &csrf, &pkce.challenge);
    Ok(Redirect::to(&url))
}

// ── /callback/google ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn callback_google(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
) -> ApiResult<Response> {
    if let Some(err) = q.error {
        return Err(ApiError::bad_request(
            "google_oauth_error",
            format!("Google returned error: {err}"),
        ));
    }
    let code = q
        .code
        .ok_or_else(|| ApiError::bad_request("missing_code", "Missing ?code"))?;
    let csrf = q
        .state
        .ok_or_else(|| ApiError::bad_request("missing_state", "Missing ?state"))?;

    // 1. Recover the PKCE verifier + originating host (and validate state in
    //    one step — if state is unknown / already consumed, Redis returns
    //    None).
    let mut conn = state.redis.clone();
    let stored: Option<String> = redis::cmd("GETDEL")
        .arg(redis_key(&csrf))
        .query_async(&mut conn)
        .await?;
    let stored = stored.ok_or_else(|| {
        ApiError::bad_request("invalid_state", "OAuth state expired or unknown")
    })?;
    let (verifier, stored_origin) = match stored.split_once('\n') {
        Some((v, o)) if !o.is_empty() => (v.to_string(), Some(o.to_string())),
        Some((v, _)) => (v.to_string(), None),
        None => (stored, None),
    };

    // 2. Exchange the code — use the origin we captured at login so the
    //    redirect_uri matches exactly.
    let tokens = oauth_google::exchange_code(
        &state.http,
        &state.config,
        stored_origin.as_deref(),
        &code,
        &verifier,
    )
    .await
    .map_err(ApiError::Upstream)?;

    // 3. Fetch userinfo.
    let info = oauth_google::fetch_userinfo(&state.http, &tokens.access_token)
        .await
        .map_err(ApiError::Upstream)?;

    // 4. Find or create user.
    let (user, _is_new) = upsert_google_user(&state, &info).await?;

    // 5. Create session row.
    let device_name = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| truncate(s, 120).to_string());
    let ip_hash = hash_client_ip(&headers, &addr, &state.config.ip_hash_salt);
    let expires_at = Utc::now()
        + ChronoDuration::from_std(state.config.refresh_ttl)
            .unwrap_or(ChronoDuration::days(30));

    let session_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, device_name, ip_hash, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(session_id)
    .bind(user.id)
    .bind(device_name)
    .bind(ip_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    // 6. Mint access JWT.
    let access = jwt::mint(&user, session_id, &state.config).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("failed to mint JWT: {e}"))
    })?;

    // 7. Set cookies and redirect.
    let jar = jar
        .add(cookies::access(access, &state.config))
        .add(cookies::refresh(session_id.to_string(), &state.config));

    // Send the user back to whatever host they started on — falls back to
    // the configured web_url if the login began before this host-capture
    // mechanism existed.
    let dest_base = stored_origin
        .as_deref()
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| state.config.web_url.trim_end_matches('/').to_string());
    let dest = if user.needs_onboarding() {
        format!("{dest_base}/onboarding/username")
    } else {
        format!("{dest_base}/home")
    };

    Ok((jar, Redirect::to(&dest)).into_response())
}

/// Find or create a user for this Google identity.
async fn upsert_google_user(
    state: &AppState,
    info: &oauth_google::GoogleUserInfo,
) -> ApiResult<(User, bool)> {
    // 1. Provider lookup.
    let existing = sqlx::query_as::<_, User>(&format!(
        "SELECT {cols} FROM users u \
           JOIN user_auth_providers p ON p.user_id = u.id \
          WHERE p.provider = 'google' AND p.provider_uid = $1",
        cols = prefixed("u", User::COLUMNS),
    ))
    .bind(&info.sub)
    .fetch_optional(&state.db)
    .await?;

    if let Some(u) = existing {
        return Ok((u, false));
    }

    // 2. Email lookup — link provider to existing account.
    if info.email_verified {
        if let Some(email) = info.email.as_deref() {
            let by_email = sqlx::query_as::<_, User>(&format!(
                "SELECT {cols} FROM users WHERE email = $1 AND deleted_at IS NULL",
                cols = User::COLUMNS,
            ))
            .bind(email)
            .fetch_optional(&state.db)
            .await?;
            if let Some(u) = by_email {
                sqlx::query(
                    "INSERT INTO user_auth_providers \
                        (user_id, provider, provider_uid, provider_email, provider_name) \
                     VALUES ($1, 'google', $2, $3, $4) \
                     ON CONFLICT (provider, provider_uid) DO NOTHING",
                )
                .bind(u.id)
                .bind(&info.sub)
                .bind(&info.email)
                .bind(&info.name)
                .execute(&state.db)
                .await?;
                return Ok((u, false));
            }
        }
    }

    // 3. New user. Auto-username of the form `user_xxxxxxxx` (hex slice of uuid)
    //    guaranteed to match the `username_format` CHECK constraint.
    let user_uuid = Uuid::new_v4();
    let auto_username = format!(
        "user_{}",
        &user_uuid.as_simple().to_string()[..8]
    );
    let display_name = info
        .given_name
        .clone()
        .or_else(|| info.name.clone())
        .unwrap_or_else(|| "Nieuwe Vonk".to_string());
    let locale = info
        .locale
        .as_deref()
        .map(|l| if l.starts_with("nl") { "nl" } else { "en" })
        .unwrap_or("nl");

    let mut tx = state.db.begin().await?;
    let new_user = sqlx::query_as::<_, User>(&format!(
        "INSERT INTO users (uuid, username, display_name, email, email_verified, locale) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING {cols}",
        cols = User::COLUMNS,
    ))
    .bind(user_uuid)
    .bind(&auto_username)
    .bind(&display_name)
    .bind(info.email.as_deref())
    .bind(info.email_verified)
    .bind(locale)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO user_auth_providers \
            (user_id, provider, provider_uid, provider_email, provider_name) \
         VALUES ($1, 'google', $2, $3, $4)",
    )
    .bind(new_user.id)
    .bind(&info.sub)
    .bind(&info.email)
    .bind(&info.name)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((new_user, true))
}

// ── /login/github ────────────────────────────────────────────

async fn login_github(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Redirect> {
    if !state.config.github_configured() {
        return Err(ApiError::bad_request(
            "github_not_configured",
            "GitHub OAuth credentials are not configured on the server",
        ));
    }

    let csrf = oauth_github::random_state();
    let pkce = oauth_github::PkcePair::new_sha256();

    let origin = origin_from_headers(&headers);
    let mut conn = state.redis.clone();
    let key = redis_key_github(&csrf);
    let payload = format!("{}\n{}", pkce.verifier, origin.as_deref().unwrap_or(""));
    let _: () = conn.set_ex(&key, &payload, 600).await?;

    let url =
        oauth_github::build_authorize_url(&state.config, origin.as_deref(), &csrf, &pkce.challenge);
    Ok(Redirect::to(&url))
}

// ── /callback/github ─────────────────────────────────────────

async fn callback_github(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
) -> ApiResult<Response> {
    if let Some(err) = q.error {
        return Err(ApiError::bad_request(
            "github_oauth_error",
            format!("GitHub returned error: {err}"),
        ));
    }
    let code = q
        .code
        .ok_or_else(|| ApiError::bad_request("missing_code", "Missing ?code"))?;
    let csrf = q
        .state
        .ok_or_else(|| ApiError::bad_request("missing_state", "Missing ?state"))?;

    let mut conn = state.redis.clone();
    let stored: Option<String> = redis::cmd("GETDEL")
        .arg(redis_key_github(&csrf))
        .query_async(&mut conn)
        .await?;
    let stored = stored.ok_or_else(|| {
        ApiError::bad_request("invalid_state", "OAuth state expired or unknown")
    })?;
    let (verifier, stored_origin) = match stored.split_once('\n') {
        Some((v, o)) if !o.is_empty() => (v.to_string(), Some(o.to_string())),
        Some((v, _)) => (v.to_string(), None),
        None => (stored, None),
    };

    let tokens = oauth_github::exchange_code(
        &state.http,
        &state.config,
        stored_origin.as_deref(),
        &code,
        &verifier,
    )
    .await
    .map_err(ApiError::Upstream)?;

    let gh_user = oauth_github::fetch_user(&state.http, &tokens.access_token)
        .await
        .map_err(ApiError::Upstream)?;

    let (user, _is_new) = upsert_github_user(&state, &gh_user).await?;

    let device_name = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| truncate(s, 120).to_string());
    let ip_hash = hash_client_ip(&headers, &addr, &state.config.ip_hash_salt);
    let expires_at = Utc::now()
        + ChronoDuration::from_std(state.config.refresh_ttl)
            .unwrap_or(ChronoDuration::days(30));

    let session_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, device_name, ip_hash, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(session_id)
    .bind(user.id)
    .bind(device_name)
    .bind(ip_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    let access = jwt::mint(&user, session_id, &state.config)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("failed to mint JWT: {e}")))?;

    let jar = jar
        .add(cookies::access(access, &state.config))
        .add(cookies::refresh(session_id.to_string(), &state.config));

    let dest_base = stored_origin
        .as_deref()
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| state.config.web_url.trim_end_matches('/').to_string());
    let dest = if user.needs_onboarding() {
        format!("{dest_base}/onboarding/username")
    } else {
        format!("{dest_base}/home")
    };

    Ok((jar, Redirect::to(&dest)).into_response())
}

/// Find or create a user for this GitHub identity.
async fn upsert_github_user(
    state: &AppState,
    info: &oauth_github::GitHubUser,
) -> ApiResult<(User, bool)> {
    let provider_uid = info.id.to_string();

    // 1. Provider lookup.
    let existing = sqlx::query_as::<_, User>(&format!(
        "SELECT {cols} FROM users u \
           JOIN user_auth_providers p ON p.user_id = u.id \
          WHERE p.provider = 'github' AND p.provider_uid = $1",
        cols = prefixed("u", User::COLUMNS),
    ))
    .bind(&provider_uid)
    .fetch_optional(&state.db)
    .await?;

    if let Some(u) = existing {
        return Ok((u, false));
    }

    // 2. Email lookup — GitHub emails on `/user/emails` are already marked
    //    verified+primary by the fetch_user helper.
    if let Some(email) = info.email.as_deref() {
        let by_email = sqlx::query_as::<_, User>(&format!(
            "SELECT {cols} FROM users WHERE email = $1 AND deleted_at IS NULL",
            cols = User::COLUMNS,
        ))
        .bind(email)
        .fetch_optional(&state.db)
        .await?;
        if let Some(u) = by_email {
            sqlx::query(
                "INSERT INTO user_auth_providers \
                    (user_id, provider, provider_uid, provider_email, provider_name) \
                 VALUES ($1, 'github', $2, $3, $4) \
                 ON CONFLICT (provider, provider_uid) DO NOTHING",
            )
            .bind(u.id)
            .bind(&provider_uid)
            .bind(&info.email)
            .bind(&info.name)
            .execute(&state.db)
            .await?;
            return Ok((u, false));
        }
    }

    // 3. New user.
    let user_uuid = Uuid::new_v4();
    let auto_username = format!("user_{}", &user_uuid.as_simple().to_string()[..8]);
    let display_name = info
        .name
        .clone()
        .unwrap_or_else(|| info.login.clone());
    let locale = "nl";

    let mut tx = state.db.begin().await?;
    let new_user = sqlx::query_as::<_, User>(&format!(
        "INSERT INTO users (uuid, username, display_name, email, email_verified, locale) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING {cols}",
        cols = User::COLUMNS,
    ))
    .bind(user_uuid)
    .bind(&auto_username)
    .bind(&display_name)
    .bind(info.email.as_deref())
    .bind(info.email.is_some())
    .bind(locale)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO user_auth_providers \
            (user_id, provider, provider_uid, provider_email, provider_name) \
         VALUES ($1, 'github', $2, $3, $4)",
    )
    .bind(new_user.id)
    .bind(&provider_uid)
    .bind(&info.email)
    .bind(&info.name)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((new_user, true))
}

// ── /login/apple ─────────────────────────────────────────────
//
// Apple Sign-in is wired as far as the authorize redirect. The token
// exchange requires an ES256-signed JWT minted from a .p8 private key
// which we don't ship yet — `apple_configured()` gates the endpoint so
// we never hit the unfinished path in production.

async fn login_apple(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Redirect> {
    if !state.config.apple_configured() {
        return Err(ApiError::bad_request(
            "apple_not_configured",
            "Apple Sign-in is not configured on the server",
        ));
    }

    let csrf = oauth_apple::random_state();
    let origin = origin_from_headers(&headers);

    let mut conn = state.redis.clone();
    let key = format!("oauth:apple:state:{csrf}");
    let payload = origin.as_deref().unwrap_or("").to_string();
    let _: () = conn.set_ex(&key, &payload, 600).await?;

    let url = oauth_apple::build_authorize_url(&state.config, origin.as_deref(), &csrf);
    Ok(Redirect::to(&url))
}

// ── /callback/apple ──────────────────────────────────────────
//
// Apple uses `response_mode=form_post` — the callback is a POST with
// form-encoded body, not a GET with query params. The body contains:
//   code, state, id_token (always), user (JSON, only on first consent)

#[derive(Debug, Deserialize)]
pub struct AppleCallbackForm {
    code: Option<String>,
    state: Option<String>,
    #[allow(dead_code)]
    id_token: Option<String>,
    /// JSON string with { name: { firstName, lastName } } — only on first consent.
    user: Option<String>,
    error: Option<String>,
}

async fn callback_apple(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    axum::extract::Form(form): axum::extract::Form<AppleCallbackForm>,
) -> ApiResult<Response> {
    if let Some(err) = form.error {
        return Err(ApiError::bad_request(
            "apple_oauth_error",
            format!("Apple returned error: {err}"),
        ));
    }
    let code = form
        .code
        .ok_or_else(|| ApiError::bad_request("missing_code", "Missing code"))?;
    let csrf = form
        .state
        .ok_or_else(|| ApiError::bad_request("missing_state", "Missing state"))?;

    // Recover the stored origin.
    let mut conn = state.redis.clone();
    let stored_origin: Option<String> = redis::cmd("GETDEL")
        .arg(format!("oauth:apple:state:{csrf}"))
        .query_async(&mut conn)
        .await?;
    let stored_origin = stored_origin.ok_or_else(|| {
        ApiError::bad_request("invalid_state", "OAuth state expired or unknown")
    })?;
    let stored_origin = if stored_origin.is_empty() {
        None
    } else {
        Some(stored_origin)
    };

    // Exchange code for tokens (mints ES256 client_secret on the fly).
    let tokens = oauth_apple::exchange_code(
        &state.http,
        &state.config,
        stored_origin.as_deref(),
        &code,
    )
    .await
    .map_err(ApiError::Upstream)?;

    // Decode the id_token to get sub + email.
    let claims = oauth_apple::decode_id_token(&tokens.id_token)
        .map_err(|e| ApiError::Upstream(anyhow::anyhow!("id_token: {e}")))?;

    // Parse user info (name) — only present on first consent.
    let user_info: Option<oauth_apple::AppleUserInfo> = form
        .user
        .as_deref()
        .and_then(|u| serde_json::from_str(u).ok());

    let display_name = user_info
        .as_ref()
        .and_then(|u| u.display_name());

    let (user, _is_new) =
        upsert_apple_user(&state, &claims, display_name.as_deref()).await?;

    // Create session.
    let device_name = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| truncate(s, 120).to_string());
    let ip_hash = hash_client_ip(&headers, &addr, &state.config.ip_hash_salt);
    let expires_at = Utc::now()
        + ChronoDuration::from_std(state.config.refresh_ttl)
            .unwrap_or(ChronoDuration::days(30));

    let session_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, device_name, ip_hash, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(session_id)
    .bind(user.id)
    .bind(device_name)
    .bind(ip_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    let access = jwt::mint(&user, session_id, &state.config)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("failed to mint JWT: {e}")))?;

    let jar = jar
        .add(cookies::access(access, &state.config))
        .add(cookies::refresh(session_id.to_string(), &state.config));

    let dest_base = stored_origin
        .as_deref()
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| state.config.web_url.trim_end_matches('/').to_string());
    let dest = if user.needs_onboarding() {
        format!("{dest_base}/onboarding/username")
    } else {
        format!("{dest_base}/home")
    };

    Ok((jar, Redirect::to(&dest)).into_response())
}

/// Find or create a user for this Apple identity.
async fn upsert_apple_user(
    state: &AppState,
    claims: &oauth_apple::AppleIdTokenClaims,
    display_name: Option<&str>,
) -> ApiResult<(User, bool)> {
    let provider_uid = &claims.sub;

    // 1. Provider lookup.
    let existing = sqlx::query_as::<_, User>(&format!(
        "SELECT {cols} FROM users u \
           JOIN user_auth_providers p ON p.user_id = u.id \
          WHERE p.provider = 'apple' AND p.provider_uid = $1",
        cols = prefixed("u", User::COLUMNS),
    ))
    .bind(provider_uid)
    .fetch_optional(&state.db)
    .await?;

    if let Some(u) = existing {
        return Ok((u, false));
    }

    // 2. Email lookup — Apple emails are verified when email_verified claim is true.
    if claims.is_email_verified() {
        if let Some(email) = claims.email.as_deref() {
            let by_email = sqlx::query_as::<_, User>(&format!(
                "SELECT {cols} FROM users WHERE email = $1 AND deleted_at IS NULL",
                cols = User::COLUMNS,
            ))
            .bind(email)
            .fetch_optional(&state.db)
            .await?;
            if let Some(u) = by_email {
                sqlx::query(
                    "INSERT INTO user_auth_providers \
                        (user_id, provider, provider_uid, provider_email, provider_name) \
                     VALUES ($1, 'apple', $2, $3, $4) \
                     ON CONFLICT (provider, provider_uid) DO NOTHING",
                )
                .bind(u.id)
                .bind(provider_uid)
                .bind(&claims.email)
                .bind(display_name)
                .execute(&state.db)
                .await?;
                return Ok((u, false));
            }
        }
    }

    // 3. New user.
    let user_uuid = Uuid::new_v4();
    let auto_username = format!("user_{}", &user_uuid.as_simple().to_string()[..8]);
    let name = display_name
        .unwrap_or("Nieuwe Vonk")
        .to_string();
    let locale = "nl";

    let mut tx = state.db.begin().await?;
    let new_user = sqlx::query_as::<_, User>(&format!(
        "INSERT INTO users (uuid, username, display_name, email, email_verified, locale) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING {cols}",
        cols = User::COLUMNS,
    ))
    .bind(user_uuid)
    .bind(&auto_username)
    .bind(&name)
    .bind(claims.email.as_deref())
    .bind(claims.is_email_verified())
    .bind(locale)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO user_auth_providers \
            (user_id, provider, provider_uid, provider_email, provider_name) \
         VALUES ($1, 'apple', $2, $3, $4)",
    )
    .bind(new_user.id)
    .bind(provider_uid)
    .bind(&claims.email)
    .bind(display_name)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((new_user, true))
}

// ── /refresh ─────────────────────────────────────────────────

async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> ApiResult<(CookieJar, StatusCode)> {
    let refresh_cookie = jar
        .get(cookies::REFRESH_COOKIE)
        .ok_or(ApiError::Unauthenticated)?;
    let session_id: Uuid = refresh_cookie
        .value()
        .parse()
        .map_err(|_| ApiError::Unauthenticated)?;

    // Look up the session, including its rotation state.
    let row: Option<(i64, Option<Uuid>)> = sqlx::query_as(
        "SELECT user_id, rotated_to FROM sessions \
         WHERE id = $1 AND expires_at > now()",
    )
    .bind(session_id)
    .fetch_optional(&state.db)
    .await?;
    let (user_id, rotated_to) = row.ok_or(ApiError::Unauthenticated)?;

    // Reuse detection: this refresh cookie was already redeemed. Kill
    // the lineage and tell the caller to log in from scratch.
    if rotated_to.is_some() {
        let _ = invalidate_session_chain(&state, session_id).await;
        tracing::warn!(?session_id, "refresh-token reuse detected — chain revoked");
        return Err(ApiError::Unauthenticated);
    }

    let user = sqlx::query_as::<_, User>(&format!(
        "SELECT {cols} FROM users WHERE id = $1 AND deleted_at IS NULL",
        cols = User::COLUMNS,
    ))
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthenticated)?;

    if user.is_suspended.unwrap_or(false) {
        return Err(ApiError::Forbidden);
    }

    // Mint a new session row and mark the old one rotated. Both updates
    // happen in a transaction so an error doesn't leave a dangling
    // rotated-but-no-successor state.
    let new_session_id = Uuid::new_v4();
    let new_expires_at = Utc::now()
        + ChronoDuration::from_std(state.config.refresh_ttl)
            .unwrap_or(ChronoDuration::days(30));

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO sessions (id, user_id, device_name, ip_hash, expires_at) \
         SELECT $1, user_id, device_name, ip_hash, $2 \
           FROM sessions WHERE id = $3",
    )
    .bind(new_session_id)
    .bind(new_expires_at)
    .bind(session_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE sessions SET rotated_to = $1, rotated_at = now(), last_active = now() \
         WHERE id = $2",
    )
    .bind(new_session_id)
    .bind(session_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Issue the new cookie pair against the new session id.
    let access = jwt::mint(&user, new_session_id, &state.config)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("mint jwt: {e}")))?;

    let jar = jar
        .add(cookies::access(access, &state.config))
        .add(cookies::refresh(new_session_id.to_string(), &state.config));
    Ok((jar, StatusCode::NO_CONTENT))
}

/// Walk the rotation chain (forward + backward) and delete every
/// session in it. Used when a rotated-already token is presented a second
/// time — classic reuse-detection response.
async fn invalidate_session_chain(state: &AppState, seed: Uuid) -> ApiResult<()> {
    sqlx::query(
        r#"
        WITH RECURSIVE chain AS (
            SELECT id, rotated_to FROM sessions WHERE id = $1
          UNION
            SELECT s.id, s.rotated_to
              FROM sessions s JOIN chain c ON s.id = c.rotated_to
          UNION
            SELECT s.id, s.rotated_to
              FROM sessions s JOIN chain c ON s.rotated_to = c.id
        )
        DELETE FROM sessions WHERE id IN (SELECT id FROM chain)
        "#,
    )
    .bind(seed)
    .execute(&state.db)
    .await?;
    Ok(())
}

// ── /logout ──────────────────────────────────────────────────

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> ApiResult<(CookieJar, StatusCode)> {
    if let Some(c) = jar.get(cookies::REFRESH_COOKIE) {
        if let Ok(sid) = Uuid::parse_str(c.value()) {
            sqlx::query("DELETE FROM sessions WHERE id = $1")
                .bind(sid)
                .execute(&state.db)
                .await?;
        }
    }

    let jar = jar
        .add(cookies::clear_access(&state.config))
        .add(cookies::clear_refresh(&state.config));
    Ok((jar, StatusCode::NO_CONTENT))
}

// ── helpers ──────────────────────────────────────────────────

fn redis_key(state: &str) -> String {
    format!("oauth:google:state:{state}")
}

fn redis_key_github(state: &str) -> String {
    format!("oauth:github:state:{state}")
}

fn hash_client_ip(headers: &HeaderMap, fallback: &SocketAddr, salt: &str) -> String {
    let ip = extract_client_ip(headers).unwrap_or(fallback.ip());
    crate::auth::ip::hash_ip(ip, salt, Utc::now().date_naive())
}

fn extract_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    // Honour X-Forwarded-For (first hop) behind nginx.
    if let Some(h) = headers.get("x-forwarded-for") {
        if let Ok(s) = h.to_str() {
            if let Some(first) = s.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
    }
    if let Some(h) = headers.get("x-real-ip") {
        if let Ok(s) = h.to_str() {
            if let Ok(ip) = s.parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }
    None
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        // Find a valid char boundary at or below max.
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &s[..end]
    }
}

/// Prefix every column name with `table.` for joined queries.
fn prefixed(prefix: &str, cols: &str) -> String {
    cols.split(',')
        .map(|c| format!("{prefix}.{}", c.trim()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build `scheme://host` from forwarded headers. nginx sets
/// `X-Forwarded-Proto` + forwards `Host`, so we can reconstruct the exact
/// origin the user is on. Returns `None` if we can't — callers fall back to
/// the configured api_url in that case.
fn origin_from_headers(headers: &HeaderMap) -> Option<String> {
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(axum::http::header::HOST))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())?;
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("https")
        .to_string();
    Some(format!("{scheme}://{host}"))
}
