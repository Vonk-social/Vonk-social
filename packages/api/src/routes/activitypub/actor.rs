//! Actor endpoint: serves the ActivityPub Person document.
//!
//! `GET /ap/users/{username}` — returns a JSON-LD Person object when
//! the client sends `Accept: application/activity+json`. Regular
//! browser requests fall through to a redirect to the web profile.

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::get,
    Json, Router,
};

use crate::activitypub::keys;
use crate::activitypub::types::{
    ApImage, ApPerson, ApPublicKey, AP_CONTENT_TYPE, AP_CONTEXT, SECURITY_CONTEXT,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/ap/users/{username}", get(actor))
}

async fn actor(
    State(state): State<AppState>,
    Path(username): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Content negotiation: only serve AP JSON when explicitly requested.
    let wants_ap = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|accept| {
            accept.contains("application/activity+json")
                || accept.contains("application/ld+json")
        })
        .unwrap_or(false);

    if !wants_ap {
        // Redirect browsers to the web profile.
        let web_url = state.config.web_url.trim_end_matches('/');
        return Redirect::temporary(&format!("{web_url}/u/{username}")).into_response();
    }

    // Load the user.
    let user: Option<UserRow> = sqlx::query_as(
        "SELECT id, username, display_name, bio, avatar_url \
         FROM users WHERE username = $1 AND deleted_at IS NULL",
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let user = match user {
        Some(u) => u,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": {"code": "not_found", "message": "user not found"}})),
            )
                .into_response();
        }
    };

    // Ensure RSA keypair exists (generates on first access).
    let public_key_pem = match keys::ensure_keypair(&state.db, user.id).await {
        Ok(pem) => pem,
        Err(e) => {
            tracing::error!(error = %e, chain = ?e.source(), "failed to ensure AP keypair");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": {"code": "internal_error", "message": "internal server error"}})),
            )
                .into_response();
        }
    };

    let base_url = state.config.api_url.trim_end_matches('/');
    let actor_id = format!("{base_url}/ap/users/{}", user.username);

    let icon = user.avatar_url.map(|url| ApImage {
        kind: "Image",
        url,
        media_type: "image/webp",
    });

    let person = ApPerson {
        context: vec![AP_CONTEXT, SECURITY_CONTEXT],
        id: actor_id.clone(),
        kind: "Person",
        preferred_username: user.username.clone(),
        name: user.display_name,
        summary: user.bio.unwrap_or_default(),
        icon,
        inbox: format!("{actor_id}/inbox"),
        outbox: format!("{actor_id}/outbox"),
        followers: format!("{actor_id}/followers"),
        following: format!("{actor_id}/following"),
        public_key: ApPublicKey {
            id: format!("{actor_id}#main-key"),
            owner: actor_id.clone(),
            public_key_pem,
        },
        url: format!("{}/u/{}", state.config.web_url.trim_end_matches('/'), user.username),
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, AP_CONTENT_TYPE)],
        Json(person),
    )
        .into_response()
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: i64,
    username: String,
    display_name: String,
    bio: Option<String>,
    avatar_url: Option<String>,
}
