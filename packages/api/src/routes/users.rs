//! User profile routes.
//!
//! * `GET  /api/users/me`                 — private profile (requires auth)
//! * `PATCH /api/users/me`                — update own profile
//! * `POST /api/users/me/avatar`          — upload avatar (multipart, ≤5 MiB)
//! * `DELETE /api/users/me/avatar`        — remove avatar
//! * `GET  /api/users/check-username`     — username availability check
//! * `GET  /api/users/:username`          — public profile

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use aws_sdk_s3::primitives::ByteStream;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use validator::Validate;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::media::pipeline;
use crate::models::{MeProfile, PublicProfile, User};
use crate::routes::follows::{follow_counts, follow_state};
use crate::state::AppState;

const AVATAR_MAX_BYTES: usize = 5 * 1024 * 1024;

/// Username format constraint mirrors the SQL CHECK in `001_initial.sql`:
/// `^[a-z0-9_]{3,30}$`.
static USERNAME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z0-9_]{3,30}$").unwrap());

/// Build the `/api/users/...` router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/users/me", get(me).patch(update_me))
        .route(
            "/api/users/me/avatar",
            post(upload_avatar).delete(delete_avatar),
        )
        // Larger body limit for the avatar endpoint.
        .layer(DefaultBodyLimit::max(AVATAR_MAX_BYTES + 128 * 1024))
        .route("/api/users/check-username", get(check_username))
        .route("/api/users/search", get(search_users))
        .route("/api/users/suggestions", get(suggest_users))
        .route("/api/users/{username}", get(public_profile))
}

// ── /me ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct DataEnvelope<T: Serialize> {
    data: T,
}

async fn me(AuthUser(user): AuthUser) -> ApiResult<Json<DataEnvelope<MeProfile>>> {
    Ok(Json(DataEnvelope {
        data: (&user).into(),
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMeRequest {
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    #[validate(length(min = 1, max = 60))]
    display_name: Option<String>,
    #[serde(default)]
    #[validate(length(max = 500))]
    bio: Option<String>,
    #[serde(default)]
    #[validate(length(max = 80))]
    location_city: Option<String>,
    #[serde(default)]
    #[validate(length(max = 60))]
    location_country: Option<String>,
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    is_private: Option<bool>,
    /// When true, mark onboarding as complete (idempotent).
    #[serde(default)]
    finish_onboarding: Option<bool>,
}

async fn update_me(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<UpdateMeRequest>,
) -> ApiResult<Json<DataEnvelope<MeProfile>>> {
    req.validate()?;

    // ── Username change rules ───────────────────────────────
    // Username may only be set once — i.e. while the current one matches the
    // auto-generated `user_xxxxxxxx` pattern. Anything else → 403.
    let new_username = if let Some(requested) = req.username.as_deref() {
        if !is_auto_generated(&user.username) {
            return Err(ApiError::Forbidden);
        }
        if !USERNAME_RE.is_match(requested) {
            return Err(ApiError::bad_request(
                "invalid_username",
                "Username must be 3–30 chars of a–z, 0–9 or underscore",
            ));
        }
        // Reserved prefix to avoid impersonation of the auto-namespace.
        if requested.starts_with("user_") {
            return Err(ApiError::bad_request(
                "reserved_username",
                "Usernames starting with 'user_' are reserved",
            ));
        }
        Some(requested.to_string())
    } else {
        None
    };

    if let Some(l) = req.locale.as_deref() {
        if !matches!(l, "nl" | "en") {
            return Err(ApiError::bad_request(
                "invalid_locale",
                "Locale must be 'nl' or 'en'",
            ));
        }
    }

    // ── Build dynamic UPDATE ────────────────────────────────
    // We keep this simple: conditionally COALESCE each column in the SQL so
    // NULL in the bind leaves the row untouched. This avoids dynamic query
    // construction while staying readable.
    let finish = req.finish_onboarding.unwrap_or(false) || new_username.is_some();

    let updated = sqlx::query_as::<_, User>(&format!(
        r#"
        UPDATE users SET
            username         = COALESCE($2, username),
            display_name     = COALESCE($3, display_name),
            bio              = COALESCE($4, bio),
            location_city    = COALESCE($5, location_city),
            location_country = COALESCE($6, location_country),
            locale           = COALESCE($7, locale),
            is_private       = COALESCE($8, is_private),
            onboarding_completed_at = CASE
                WHEN $9::bool AND onboarding_completed_at IS NULL THEN now()
                ELSE onboarding_completed_at
            END,
            updated_at       = now()
        WHERE id = $1
        RETURNING {cols}
        "#,
        cols = User::COLUMNS,
    ))
    .bind(user.id)
    .bind(new_username.as_deref())
    .bind(req.display_name.as_deref())
    .bind(req.bio.as_deref())
    .bind(req.location_city.as_deref())
    .bind(req.location_country.as_deref())
    .bind(req.locale.as_deref())
    .bind(req.is_private)
    .bind(finish)
    .fetch_one(&state.db)
    .await
    .map_err(map_unique_violation)?;

    Ok(Json(DataEnvelope {
        data: (&updated).into(),
    }))
}

fn is_auto_generated(username: &str) -> bool {
    username.len() == 13 // "user_" + 8 hex chars
        && username.starts_with("user_")
        && username[5..].chars().all(|c| c.is_ascii_hexdigit())
}

fn map_unique_violation(err: sqlx::Error) -> ApiError {
    if let Some(db_err) = err.as_database_error() {
        if db_err.is_unique_violation() {
            return ApiError::conflict("username_taken", "That username is already taken");
        }
    }
    ApiError::from(err)
}

// ── /users/check-username ────────────────────────────────────

#[derive(Deserialize)]
pub struct CheckUsernameQuery {
    q: String,
}

#[derive(Serialize)]
struct CheckUsernameResponse {
    available: bool,
    reason: Option<&'static str>,
}

async fn check_username(
    State(state): State<AppState>,
    Query(q): Query<CheckUsernameQuery>,
) -> ApiResult<Json<DataEnvelope<CheckUsernameResponse>>> {
    let candidate = q.q.trim();
    if !USERNAME_RE.is_match(candidate) {
        return Ok(Json(DataEnvelope {
            data: CheckUsernameResponse {
                available: false,
                reason: Some("invalid_format"),
            },
        }));
    }
    if candidate.starts_with("user_") {
        return Ok(Json(DataEnvelope {
            data: CheckUsernameResponse {
                available: false,
                reason: Some("reserved"),
            },
        }));
    }
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM users WHERE username = $1")
        .bind(candidate)
        .fetch_optional(&state.db)
        .await?;
    Ok(Json(DataEnvelope {
        data: CheckUsernameResponse {
            available: row.is_none(),
            reason: None,
        },
    }))
}

// ── /users/{username} ────────────────────────────────────────

#[derive(Serialize)]
struct ProfilePage {
    #[serde(flatten)]
    profile: PublicProfile,
    is_private: bool,
    followers_count: i64,
    following_count: i64,
    follow_state: &'static str,
}

async fn public_profile(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(username): Path<String>,
) -> ApiResult<Json<DataEnvelope<ProfilePage>>> {
    let user = sqlx::query_as::<_, User>(&format!(
        "SELECT {cols} FROM users WHERE username = $1 \
                                   AND deleted_at IS NULL \
                                   AND COALESCE(is_suspended, false) = false",
        cols = User::COLUMNS,
    ))
    .bind(&username)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound)?;

    let (followers_count, following_count) = follow_counts(&state, user.id).await?;
    let state_ = follow_state(&state, me.id, user.id).await?;

    Ok(Json(DataEnvelope {
        data: ProfilePage {
            profile: (&user).into(),
            is_private: user.is_private.unwrap_or(false),
            followers_count,
            following_count,
            follow_state: state_.as_str(),
        },
    }))
}

// ── /users/me/avatar ─────────────────────────────────────────

#[derive(Serialize)]
struct AvatarResponse {
    avatar_url: String,
}

async fn upload_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    mut multipart: Multipart,
) -> ApiResult<Json<DataEnvelope<AvatarResponse>>> {
    // Grab the first file field.
    let field = loop {
        match multipart.next_field().await {
            Ok(Some(f)) => {
                if f.file_name().is_some()
                    || f.content_type()
                        .map(|c| c.starts_with("image/"))
                        .unwrap_or(false)
                {
                    break f;
                }
            }
            Ok(None) => {
                return Err(ApiError::bad_request(
                    "missing_file",
                    "Expected an image file in multipart body",
                ));
            }
            Err(e) => {
                return Err(ApiError::bad_request("bad_multipart", e.to_string()));
            }
        }
    };

    let bytes = field
        .bytes()
        .await
        .map_err(|e| ApiError::bad_request("bad_multipart", e.to_string()))?;

    if bytes.len() > AVATAR_MAX_BYTES {
        return Err(ApiError::PayloadTooLarge);
    }

    // Decode & re-encode via the shared pipeline. Re-encoding drops
    // EXIF/XMP/IPTC metadata entirely.
    let variants = {
        let bytes = bytes.clone();
        tokio::task::spawn_blocking(move || {
            pipeline::process_image(&bytes, pipeline::AVATAR_VARIANTS)
        })
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("avatar worker panicked: {e}")))??
    };

    // Upload all three variants.
    for v in &variants {
        let key = format!("avatars/{}/{}.webp", user.uuid, v.name);
        state
            .s3
            .put_object()
            .bucket(&state.config.s3_bucket)
            .key(&key)
            .body(ByteStream::from(v.bytes.clone()))
            .content_type("image/webp")
            .cache_control("public, max-age=31536000, immutable")
            .send()
            .await
            .map_err(|e| ApiError::Upstream(anyhow::anyhow!("s3 put {key}: {e}")))?;
    }

    // Medium variant is what we put on the profile.
    let avatar_url = format!("/media/avatars/{}/medium.webp", user.uuid);

    sqlx::query("UPDATE users SET avatar_url = $1, updated_at = now() WHERE id = $2")
        .bind(&avatar_url)
        .bind(user.id)
        .execute(&state.db)
        .await?;

    Ok(Json(DataEnvelope {
        data: AvatarResponse { avatar_url },
    }))
}

async fn delete_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<StatusCode> {
    sqlx::query("UPDATE users SET avatar_url = NULL, updated_at = now() WHERE id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;
    // We deliberately leave the S3 objects in place — cleanup is handled by a
    // future background sweep so we can recover on accidental delete.
    Ok(StatusCode::NO_CONTENT)
}

// ── search / suggestions ─────────────────────────────────────

#[derive(Serialize)]
struct UserCardRow {
    uuid: uuid::Uuid,
    username: String,
    display_name: String,
    bio: Option<String>,
    avatar_url: Option<String>,
    is_private: bool,
    follow_state: &'static str,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<i64>,
}

/// Fuzzy-ish search on username + display_name via ILIKE. Self + deleted +
/// suspended users are excluded. Ordered so that username-prefix matches rank
/// above substring matches. Future: swap to `pg_trgm` for proper fuzziness.
async fn search_users(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<DataEnvelope<Vec<UserCardRow>>>> {
    let term = q.q.trim();
    if term.len() < 2 {
        return Ok(Json(DataEnvelope { data: vec![] }));
    }
    let limit = q.limit.unwrap_or(20).clamp(1, 50);
    let pattern = format!("%{}%", term.replace('%', "\\%").replace('_', "\\_"));
    let prefix = format!("{}%", term.replace('%', "\\%").replace('_', "\\_"));

    let rows = sqlx::query(
        r#"
        SELECT u.id, u.uuid, u.username, u.display_name, u.bio, u.avatar_url,
               COALESCE(u.is_private, false) AS is_private,
               COALESCE(f.status, '') AS follow_status
          FROM users u
          LEFT JOIN follows f
            ON f.follower_id = $1 AND f.following_id = u.id
         WHERE u.id != $1
           AND u.deleted_at IS NULL
           AND COALESCE(u.is_suspended, false) = false
           AND (u.username ILIKE $2 OR u.display_name ILIKE $2)
         ORDER BY
           (u.username ILIKE $3) DESC,
           (u.display_name ILIKE $3) DESC,
           u.username ASC
         LIMIT $4
        "#,
    )
    .bind(me.id)
    .bind(&pattern)
    .bind(&prefix)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(DataEnvelope {
        data: rows
            .into_iter()
            .map(|r| UserCardRow {
                uuid: r.try_get("uuid").unwrap_or_else(|_| uuid::Uuid::nil()),
                username: r.try_get("username").unwrap_or_default(),
                display_name: r.try_get("display_name").unwrap_or_default(),
                bio: r.try_get("bio").ok(),
                avatar_url: r.try_get("avatar_url").ok(),
                is_private: r.try_get("is_private").unwrap_or(false),
                follow_state: match r.try_get::<String, _>("follow_status").as_deref() {
                    Ok("active") => "active",
                    Ok("pending") => "pending",
                    _ => "none",
                },
            })
            .collect(),
    }))
}

/// Simple "people you may know": users you don't already follow, ranked by
/// how many of the people YOU follow also follow them. Fallback: newest users
/// when you follow nobody yet.
///
/// This is a Phase-3 placeholder — the real implementation lives in a
/// `suggested_connections` materialized view refreshed nightly.
async fn suggest_users(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
) -> ApiResult<Json<DataEnvelope<Vec<UserCardRow>>>> {
    let rows = sqlx::query(
        r#"
        WITH my_follows AS (
            SELECT following_id FROM follows
             WHERE follower_id = $1 AND status = 'active'
        ),
        mutuals AS (
            SELECT f.following_id AS candidate_id, COUNT(*) AS score
              FROM follows f
             WHERE f.follower_id IN (SELECT following_id FROM my_follows)
               AND f.status = 'active'
               AND f.following_id != $1
               AND f.following_id NOT IN (SELECT following_id FROM my_follows)
             GROUP BY f.following_id
        ),
        fallback AS (
            SELECT u.id AS candidate_id, 0::bigint AS score
              FROM users u
             WHERE u.id != $1
               AND u.deleted_at IS NULL
               AND COALESCE(u.is_suspended, false) = false
               AND u.id NOT IN (SELECT following_id FROM my_follows)
             ORDER BY u.created_at DESC
             LIMIT 20
        ),
        candidates AS (
            SELECT candidate_id, score FROM mutuals
            UNION ALL
            SELECT candidate_id, score FROM fallback
             WHERE NOT EXISTS (SELECT 1 FROM mutuals)
        )
        SELECT u.uuid, u.username, u.display_name, u.bio, u.avatar_url,
               COALESCE(u.is_private, false) AS is_private,
               c.score
          FROM candidates c
          JOIN users u ON u.id = c.candidate_id
         WHERE u.deleted_at IS NULL
           AND COALESCE(u.is_suspended, false) = false
         ORDER BY c.score DESC, u.created_at DESC
         LIMIT 20
        "#,
    )
    .bind(me.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(DataEnvelope {
        data: rows
            .into_iter()
            .map(|r| UserCardRow {
                uuid: r.try_get("uuid").unwrap_or_else(|_| uuid::Uuid::nil()),
                username: r.try_get("username").unwrap_or_default(),
                display_name: r.try_get("display_name").unwrap_or_default(),
                bio: r.try_get("bio").ok(),
                avatar_url: r.try_get("avatar_url").ok(),
                is_private: r.try_get("is_private").unwrap_or(false),
                follow_state: "none",
            })
            .collect(),
    }))
}
