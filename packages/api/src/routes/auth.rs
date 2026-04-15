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

use crate::auth::{cookies, jwt, oauth_google};
use crate::error::{ApiError, ApiResult};
use crate::models::User;
use crate::state::AppState;

/// Build the `/api/auth/...` router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login/google", get(login_google))
        .route("/api/auth/callback/google", get(callback_google))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
}

// ── /login/google ────────────────────────────────────────────

async fn login_google(State(state): State<AppState>) -> ApiResult<Redirect> {
    if !state.config.google_configured() {
        return Err(ApiError::bad_request(
            "google_not_configured",
            "Google OAuth credentials are not configured on the server",
        ));
    }

    let csrf = oauth_google::random_state();
    let pkce = oauth_google::PkcePair::new_sha256();

    // Store the verifier under the state key so we can recover it in the
    // callback. 10 minute TTL is plenty for a user clicking through consent.
    let mut conn = state.redis.clone();
    let key = redis_key(&csrf);
    let _: () = conn.set_ex(&key, &pkce.verifier, 600).await?;

    let url = oauth_google::build_authorize_url(&state.config, &csrf, &pkce.challenge);
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

    // 1. Recover the PKCE verifier (and validate state in one step — if
    //    state is unknown / already consumed, Redis returns None).
    let mut conn = state.redis.clone();
    let verifier: Option<String> = redis::cmd("GETDEL")
        .arg(redis_key(&csrf))
        .query_async(&mut conn)
        .await?;
    let verifier = verifier.ok_or_else(|| {
        ApiError::bad_request("invalid_state", "OAuth state expired or unknown")
    })?;

    // 2. Exchange the code.
    let tokens = oauth_google::exchange_code(&state.http, &state.config, &code, &verifier)
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

    let dest = if user.needs_onboarding() {
        format!("{}/onboarding/username", state.config.web_url.trim_end_matches('/'))
    } else {
        format!("{}/home", state.config.web_url.trim_end_matches('/'))
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

    // Validate session and load the user.
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT user_id FROM sessions WHERE id = $1 AND expires_at > now()",
    )
    .bind(session_id)
    .fetch_optional(&state.db)
    .await?;
    let user_id = row.ok_or(ApiError::Unauthenticated)?.0;

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

    // Bump last_active.
    sqlx::query("UPDATE sessions SET last_active = now() WHERE id = $1")
        .bind(session_id)
        .execute(&state.db)
        .await?;

    let access = jwt::mint(&user, session_id, &state.config)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("mint jwt: {e}")))?;

    let jar = jar.add(cookies::access(access, &state.config));
    Ok((jar, StatusCode::NO_CONTENT))
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
