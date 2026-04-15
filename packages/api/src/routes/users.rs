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
