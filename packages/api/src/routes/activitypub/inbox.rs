//! Inbox endpoint: receives activities from remote fediverse servers.
//!
//! `POST /ap/users/{username}/inbox`
//!
//! Handles:
//! - Follow      → auto-accept, store remote actor + follower
//! - Undo(Follow)→ remove from followers
//! - Create(Note)→ store in remote_posts for future feed integration
//! - Like, Announce → acknowledge (200 OK) but don't process yet

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};

use crate::activitypub::signatures;
use crate::activitypub::types::IncomingActivity;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/ap/users/{username}/inbox", post(inbox))
}

async fn inbox(
    State(state): State<AppState>,
    Path(username): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Load the target user.
    let user: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM users WHERE username = $1 AND deleted_at IS NULL",
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let (user_id,) = match user {
        Some(u) => u,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": {"code": "not_found", "message": "user not found"}})),
            )
                .into_response();
        }
    };

    // Reconstruct the request path for signature verification.
    let request_path = format!("/ap/users/{username}/inbox");

    // Verify HTTP Signature.
    let actor_uri = match signatures::verify_incoming(
        &state.db,
        &state.http,
        "post",
        &request_path,
        &headers,
    )
    .await
    {
        Ok(uri) => uri,
        Err(e) => {
            tracing::warn!(error = %e, "AP inbox signature verification failed");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": {"code": "signature_invalid", "message": "HTTP Signature verification failed"}})),
            )
                .into_response();
        }
    };

    // Parse the activity.
    let activity: IncomingActivity = match serde_json::from_slice(&body) {
        Ok(a) => a,
        Err(e) => {
            tracing::warn!(error = %e, "AP inbox: invalid JSON");
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": {"code": "invalid_json", "message": "could not parse activity"}})),
            )
                .into_response();
        }
    };

    // Ensure the actor in the activity matches the signature.
    if activity.actor != actor_uri {
        tracing::warn!(
            signed_as = %actor_uri,
            claimed = %activity.actor,
            "AP inbox: actor mismatch"
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": {"code": "actor_mismatch", "message": "activity actor does not match signature"}})),
        )
            .into_response();
    }

    // Dispatch by activity type.
    match activity.kind.as_str() {
        "Follow" => {
            handle_follow(&state, user_id, &activity).await;
        }
        "Undo" => {
            handle_undo(&state, user_id, &activity).await;
        }
        "Create" => {
            handle_create(&state, &activity).await;
        }
        "Like" | "Announce" | "Delete" | "Update" => {
            // Acknowledged but not processed yet.
            tracing::debug!(kind = %activity.kind, "AP inbox: acknowledged activity");
        }
        other => {
            tracing::debug!(kind = %other, "AP inbox: unknown activity type, ignored");
        }
    }

    StatusCode::ACCEPTED.into_response()
}

/// Handle a Follow activity: auto-accept, store the follower, and send
/// an Accept activity back to the remote server.
async fn handle_follow(state: &AppState, user_id: i64, activity: &IncomingActivity) {
    let actor_uri = &activity.actor;

    // Fetch inbox URL from cached remote actor.
    let remote: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT inbox_url, shared_inbox_url FROM ap_remote_actors WHERE actor_uri = $1",
    )
    .bind(actor_uri)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let (inbox_url, shared_inbox_url) = match remote {
        Some(r) => r,
        None => {
            tracing::warn!(actor = %actor_uri, "AP Follow from unknown actor (not cached)");
            return;
        }
    };

    // Store the follower.
    let result = sqlx::query(
        "INSERT INTO ap_followers (user_id, actor_uri, inbox_url, shared_inbox_url) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (user_id, actor_uri) DO NOTHING",
    )
    .bind(user_id)
    .bind(actor_uri)
    .bind(&inbox_url)
    .bind(&shared_inbox_url)
    .execute(&state.db)
    .await;

    if let Err(e) = result {
        tracing::error!(error = %e, "failed to store AP follower");
        return;
    }

    tracing::info!(actor = %actor_uri, user_id, "AP Follow accepted");

    // Queue an Accept activity back to the remote server.
    let base_url = state.config.api_url.trim_end_matches('/');

    // Load username for actor URL.
    let username: Option<(String,)> =
        sqlx::query_as("SELECT username FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    let username = match username {
        Some((u,)) => u,
        None => return,
    };

    let actor_id = format!("{base_url}/ap/users/{username}");

    let accept_payload = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "type": "Accept",
        "actor": actor_id,
        "object": {
            "type": "Follow",
            "actor": actor_uri,
            "object": actor_id,
        }
    });

    // Queue delivery.
    let _ = sqlx::query(
        "INSERT INTO ap_delivery_queue (user_id, inbox_url, payload) \
         VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(&inbox_url)
    .bind(&accept_payload)
    .execute(&state.db)
    .await;
}

/// Handle an Undo activity. Currently only processes Undo(Follow).
async fn handle_undo(state: &AppState, user_id: i64, activity: &IncomingActivity) {
    // Check if the inner object is a Follow.
    let inner_type = activity
        .object
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if inner_type != "Follow" {
        tracing::debug!(inner = %inner_type, "AP Undo: non-Follow undo, ignored");
        return;
    }

    let actor_uri = &activity.actor;

    let result = sqlx::query(
        "DELETE FROM ap_followers WHERE user_id = $1 AND actor_uri = $2",
    )
    .bind(user_id)
    .bind(actor_uri)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            tracing::info!(actor = %actor_uri, user_id, "AP Undo(Follow) processed");
        }
        Ok(_) => {
            tracing::debug!(actor = %actor_uri, "AP Undo(Follow): no matching follower");
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to process AP Undo(Follow)");
        }
    }
}

/// Handle a Create(Note) activity: store the remote post for future
/// feed integration.
async fn handle_create(state: &AppState, activity: &IncomingActivity) {
    let object = &activity.object;

    let note_type = object.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if note_type != "Note" {
        tracing::debug!(object_type = %note_type, "AP Create: non-Note object, ignored");
        return;
    }

    let uri = match object.get("id").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return,
    };

    let content = object.get("content").and_then(|v| v.as_str());
    let in_reply_to = object.get("inReplyTo").and_then(|v| v.as_str());
    let published_at = object
        .get("published")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let result = sqlx::query(
        "INSERT INTO ap_remote_posts (uri, actor_uri, content, in_reply_to, published_at, raw_json) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (uri) DO NOTHING",
    )
    .bind(uri)
    .bind(&activity.actor)
    .bind(content)
    .bind(in_reply_to)
    .bind(published_at)
    .bind(object)
    .execute(&state.db)
    .await;

    if let Err(e) = result {
        tracing::warn!(error = %e, uri = %uri, "failed to store remote post");
    }
}
