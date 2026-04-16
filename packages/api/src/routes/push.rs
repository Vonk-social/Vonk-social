//! Web Push subscription management (Phase 3).
//!
//! * `GET    /api/push/vapid-public-key` — public key for service worker
//! * `POST   /api/push/subscriptions`    — register a subscription
//! * `DELETE /api/push/subscriptions/:uuid` — revoke it
//! * `PATCH  /api/push/preferences`      — toggle notify_dm / _mention / _reply
//!
//! Dispatch (actually pushing a payload to a subscription) is wired in a
//! later phase — see `crate::push`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/push/vapid-public-key", get(vapid_public_key))
        .route("/api/push/subscriptions", post(subscribe))
        .route("/api/push/subscriptions/{uuid}", delete(unsubscribe))
        .route("/api/push/preferences", patch(update_prefs))
}

#[derive(Debug, Serialize)]
struct VapidKeyResponse {
    public_key: String,
}

async fn vapid_public_key(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    if !state.config.vapid_configured() {
        return Err(ApiError::bad_request(
            "vapid_not_configured",
            "VAPID keys are not configured on the server",
        ));
    }
    Ok(Json(serde_json::json!({
        "data": VapidKeyResponse {
            public_key: state.config.vapid_public_key.clone(),
        }
    })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct SubscribeRequest {
    #[validate(length(max = 2048))]
    pub endpoint: String,
    #[validate(length(max = 256))]
    pub p256dh: String,
    #[validate(length(max = 256))]
    pub auth: String,
    #[validate(length(max = 255))]
    pub user_agent: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubscribeResponse {
    pub uuid: Uuid,
}

async fn subscribe(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<SubscribeRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    req.validate()?;

    // Upsert on (user_id, md5(endpoint)) — already covered by the
    // partial unique index in migration 005. We use an MD5-equivalent
    // match in SQL via the `uq_push_user_endpoint` uniqueness, and
    // ON CONFLICT by constraint to refresh the keys.
    let uuid: Uuid = sqlx::query_scalar(
        "INSERT INTO push_subscriptions \
            (user_id, endpoint, p256dh, auth, user_agent) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT ON CONSTRAINT uq_push_user_endpoint \
         DO UPDATE SET p256dh = EXCLUDED.p256dh, \
                       auth = EXCLUDED.auth, \
                       user_agent = EXCLUDED.user_agent \
         RETURNING uuid",
    )
    .bind(user.id)
    .bind(&req.endpoint)
    .bind(&req.p256dh)
    .bind(&req.auth)
    .bind(&req.user_agent)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "data": SubscribeResponse { uuid } })))
}

async fn unsubscribe(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(uuid): Path<Uuid>,
) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM push_subscriptions WHERE uuid = $1 AND user_id = $2")
        .bind(uuid)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct UpdatePrefsRequest {
    pub notify_dm: Option<bool>,
    pub notify_mention: Option<bool>,
    pub notify_follow: Option<bool>,
    pub notify_reply: Option<bool>,
}

async fn update_prefs(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<UpdatePrefsRequest>,
) -> ApiResult<StatusCode> {
    // Apply the requested toggle across all of this user's subscriptions.
    // Preferences are per-user in Phase 3; per-device granularity can come
    // later if/when it's asked for.
    sqlx::query(
        "UPDATE push_subscriptions SET \
            notify_dm      = COALESCE($1, notify_dm), \
            notify_mention = COALESCE($2, notify_mention), \
            notify_follow  = COALESCE($3, notify_follow), \
            notify_reply   = COALESCE($4, notify_reply) \
         WHERE user_id = $5",
    )
    .bind(req.notify_dm)
    .bind(req.notify_mention)
    .bind(req.notify_follow)
    .bind(req.notify_reply)
    .bind(user.id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
