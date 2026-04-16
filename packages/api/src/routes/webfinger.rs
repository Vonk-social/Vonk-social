//! WebFinger endpoint for fediverse user discovery.
//!
//! `GET /.well-known/webfinger?resource=acct:username@vonk.social`
//!
//! This is the standard entry point for Mastodon / Pixelfed / etc. to
//! discover a Vonk user's ActivityPub actor URL.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;

use crate::activitypub::types::{WebFingerLink, WebFingerResponse, AP_CONTENT_TYPE};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/.well-known/webfinger", get(webfinger))
}

#[derive(Deserialize)]
struct WebFingerQuery {
    resource: String,
}

async fn webfinger(
    State(state): State<AppState>,
    Query(q): Query<WebFingerQuery>,
) -> impl IntoResponse {
    // Parse "acct:username@domain".
    let acct = match q.resource.strip_prefix("acct:") {
        Some(a) => a,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": {"code": "invalid_resource", "message": "resource must start with acct:"}})),
            ).into_response();
        }
    };

    let (username, _domain) = match acct.split_once('@') {
        Some(parts) => parts,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": {"code": "invalid_resource", "message": "resource must be acct:user@domain"}})),
            ).into_response();
        }
    };

    // Look up the user.
    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM users WHERE username = $1 AND deleted_at IS NULL",
    )
    .bind(username)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    if exists.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": {"code": "not_found", "message": "user not found"}})),
        ).into_response();
    }

    let base_url = state.config.api_url.trim_end_matches('/');

    let response = WebFingerResponse {
        subject: q.resource.clone(),
        links: vec![WebFingerLink {
            rel: "self",
            kind: AP_CONTENT_TYPE,
            href: format!("{base_url}/ap/users/{username}"),
        }],
    };

    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/jrd+json; charset=utf-8",
        )],
        Json(response),
    )
        .into_response()
}
