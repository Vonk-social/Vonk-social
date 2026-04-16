//! Outbox endpoint: serves a user's public posts as an ActivityPub
//! OrderedCollection.
//!
//! `GET /ap/users/{username}/outbox`

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use sqlx::Row;

use crate::activitypub::types::{ApOrderedCollection, AP_CONTENT_TYPE, AP_CONTEXT};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/ap/users/{username}/outbox", get(outbox))
}

async fn outbox(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> impl IntoResponse {
    // Load the user.
    let user: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, username FROM users WHERE username = $1 AND deleted_at IS NULL",
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let (user_id, username) = match user {
        Some(u) => u,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": {"code": "not_found", "message": "user not found"}})),
            )
                .into_response();
        }
    };

    let base_url = state.config.api_url.trim_end_matches('/');

    // Fetch public posts (non-deleted, non-expired, public visibility).
    let rows = sqlx::query(
        "SELECT p.uuid, p.content, p.created_at, p.reply_to_id \
         FROM posts p \
         WHERE p.user_id = $1 \
           AND p.deleted_at IS NULL \
           AND p.visibility = 'public' \
           AND p.post_type = 'post' \
           AND (p.expires_at IS NULL OR p.expires_at > now()) \
         ORDER BY p.created_at DESC \
         LIMIT 20",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let total_items = rows.len() as i64;

    let ordered_items: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let post_uuid: uuid::Uuid = row.try_get("uuid").unwrap_or_else(|_| uuid::Uuid::nil());
            let content: Option<String> = row.try_get("content").ok();
            let created_at: chrono::DateTime<chrono::Utc> =
                row.try_get("created_at").unwrap_or_else(|_| chrono::Utc::now());

            let post_url = format!("{base_url}/ap/posts/{post_uuid}");
            let actor_url = format!("{base_url}/ap/users/{username}");

            serde_json::json!({
                "type": "Create",
                "actor": actor_url,
                "published": created_at.to_rfc3339(),
                "object": {
                    "type": "Note",
                    "id": post_url,
                    "attributedTo": actor_url,
                    "content": content.unwrap_or_default(),
                    "published": created_at.to_rfc3339(),
                    "to": ["https://www.w3.org/ns/activitystreams#Public"],
                    "cc": [format!("{actor_url}/followers")],
                    "url": post_url,
                }
            })
        })
        .collect();

    let collection = ApOrderedCollection {
        context: AP_CONTEXT,
        id: format!("{base_url}/ap/users/{username}/outbox"),
        kind: "OrderedCollection",
        total_items,
        ordered_items,
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, AP_CONTENT_TYPE)],
        Json(collection),
    )
        .into_response()
}
