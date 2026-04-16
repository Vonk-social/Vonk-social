//! NodeInfo 2.0 endpoints for fediverse platform discovery.
//!
//! `GET /.well-known/nodeinfo` — returns a link to the NodeInfo document
//! `GET /nodeinfo/2.0`         — returns the actual NodeInfo document

use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::activitypub::types::{
    NodeInfo, NodeInfoSoftware, NodeInfoUsage, NodeInfoUsers, NodeInfoWellKnown,
    NodeInfoWellKnownLink,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/.well-known/nodeinfo", get(well_known_nodeinfo))
        .route("/nodeinfo/2.0", get(nodeinfo))
}

async fn well_known_nodeinfo(State(state): State<AppState>) -> Json<NodeInfoWellKnown> {
    let base_url = state.config.api_url.trim_end_matches('/');
    Json(NodeInfoWellKnown {
        links: vec![NodeInfoWellKnownLink {
            rel: "http://nodeinfo.diaspora.software/ns/schema/2.0",
            href: format!("{base_url}/nodeinfo/2.0"),
        }],
    })
}

async fn nodeinfo(State(state): State<AppState>) -> (StatusCode, Json<NodeInfo>) {
    // User statistics.
    let total_users: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let active_month: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_id) FROM sessions \
         WHERE last_active > now() - interval '30 days'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let active_halfyear: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_id) FROM sessions \
         WHERE last_active > now() - interval '180 days'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let local_posts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    (
        StatusCode::OK,
        Json(NodeInfo {
            version: "2.0",
            software: NodeInfoSoftware {
                name: "vonk",
                version: env!("CARGO_PKG_VERSION"),
            },
            protocols: vec!["activitypub"],
            usage: NodeInfoUsage {
                users: NodeInfoUsers {
                    total: total_users,
                    active_month,
                    active_halfyear,
                },
                local_posts,
            },
            open_registrations: false,
        }),
    )
}
