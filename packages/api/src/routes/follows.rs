//! Follow graph endpoints.
//!
//! * `POST   /api/users/{username}/follow`             — follow (or request if private)
//! * `DELETE /api/users/{username}/follow`             — unfollow / cancel request
//! * `POST   /api/users/{username}/follow/accept`      — accept an incoming pending request
//! * `DELETE /api/users/{username}/follow/accept`      — reject an incoming pending request
//! * `GET    /api/users/{username}/followers`
//! * `GET    /api/users/{username}/following`

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::models::FollowState;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/users/{username}/follow",
            post(follow).delete(unfollow),
        )
        .route(
            "/api/users/{username}/follow/accept",
            post(accept).delete(reject),
        )
        .route("/api/users/{username}/followers", get(followers))
        .route("/api/users/{username}/following", get(following))
}

#[derive(Deserialize)]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct FollowResponse {
    follow_state: &'static str,
}

#[derive(Serialize)]
struct FollowListItem {
    uuid: Uuid,
    username: String,
    display_name: String,
    avatar_url: Option<String>,
    is_private: bool,
}

#[derive(Serialize)]
struct FollowListResponse {
    data: Vec<FollowListItem>,
    cursor: Option<String>,
    has_more: bool,
}

// ── follow / unfollow ────────────────────────────────────────

async fn follow(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(username): Path<String>,
) -> ApiResult<Json<FollowResponse>> {
    let target = load_target(&state, &username).await?;
    if target.id == me.id {
        return Err(ApiError::bad_request(
            "cannot_follow_self",
            "You cannot follow yourself",
        ));
    }

    let status = if target.is_private { "pending" } else { "active" };
    sqlx::query(
        "INSERT INTO follows (follower_id, following_id, status) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (follower_id, following_id) DO NOTHING",
    )
    .bind(me.id)
    .bind(target.id)
    .bind(status)
    .execute(&state.db)
    .await?;

    Ok(Json(FollowResponse {
        follow_state: if status == "pending" { "pending" } else { "active" },
    }))
}

async fn unfollow(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(username): Path<String>,
) -> ApiResult<StatusCode> {
    let target = load_target(&state, &username).await?;
    sqlx::query("DELETE FROM follows WHERE follower_id = $1 AND following_id = $2")
        .bind(me.id)
        .bind(target.id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── accept / reject incoming pending ─────────────────────────

async fn accept(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(username): Path<String>,
) -> ApiResult<StatusCode> {
    let other = load_target(&state, &username).await?;
    let row = sqlx::query(
        "UPDATE follows SET status = 'active' \
           WHERE follower_id = $1 AND following_id = $2 AND status = 'pending' \
         RETURNING 1",
    )
    .bind(other.id)
    .bind(me.id)
    .fetch_optional(&state.db)
    .await?;
    if row.is_none() {
        return Err(ApiError::bad_request(
            "no_pending_request",
            "no pending follow request from that user",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn reject(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(username): Path<String>,
) -> ApiResult<StatusCode> {
    let other = load_target(&state, &username).await?;
    sqlx::query(
        "DELETE FROM follows \
           WHERE follower_id = $1 AND following_id = $2 AND status = 'pending'",
    )
    .bind(other.id)
    .bind(me.id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── listings ─────────────────────────────────────────────────

async fn followers(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(username): Path<String>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<FollowListResponse>> {
    let target = load_target(&state, &username).await?;
    ensure_list_visible(&state, me.id, &target).await?;

    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let cursor_created_at = q
        .cursor
        .as_deref()
        .and_then(decode_ts_cursor);

    let rows = sqlx::query(
        r#"
        SELECT u.uuid, u.username, u.display_name, u.avatar_url,
               COALESCE(u.is_private, false) AS is_private,
               f.created_at
          FROM follows f
          JOIN users u ON u.id = f.follower_id
         WHERE f.following_id = $1
           AND f.status = 'active'
           AND u.deleted_at IS NULL
           AND ($2::timestamptz IS NULL OR f.created_at < $2)
         ORDER BY f.created_at DESC
         LIMIT $3
        "#,
    )
    .bind(target.id)
    .bind(cursor_created_at)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(build_follow_list(rows, limit)))
}

async fn following(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(username): Path<String>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<FollowListResponse>> {
    let target = load_target(&state, &username).await?;
    ensure_list_visible(&state, me.id, &target).await?;

    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let cursor_created_at = q.cursor.as_deref().and_then(decode_ts_cursor);

    let rows = sqlx::query(
        r#"
        SELECT u.uuid, u.username, u.display_name, u.avatar_url,
               COALESCE(u.is_private, false) AS is_private,
               f.created_at
          FROM follows f
          JOIN users u ON u.id = f.following_id
         WHERE f.follower_id = $1
           AND f.status = 'active'
           AND u.deleted_at IS NULL
           AND ($2::timestamptz IS NULL OR f.created_at < $2)
         ORDER BY f.created_at DESC
         LIMIT $3
        "#,
    )
    .bind(target.id)
    .bind(cursor_created_at)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(build_follow_list(rows, limit)))
}

// ── helpers ──────────────────────────────────────────────────

struct Target {
    id: i64,
    is_private: bool,
}

async fn load_target(state: &AppState, username: &str) -> ApiResult<Target> {
    let row: Option<(i64, Option<bool>)> = sqlx::query_as(
        "SELECT id, is_private FROM users \
          WHERE username = $1 AND deleted_at IS NULL AND COALESCE(is_suspended, false) = false",
    )
    .bind(username)
    .fetch_optional(&state.db)
    .await?;
    let (id, is_private) = row.ok_or(ApiError::NotFound)?;
    Ok(Target {
        id,
        is_private: is_private.unwrap_or(false),
    })
}

/// Compute the `(viewer → target)` follow state. Public helper, also used by
/// `/api/users/:username` in `routes/users.rs`.
pub async fn follow_state(state: &AppState, viewer_id: i64, target_id: i64) -> ApiResult<FollowState> {
    if viewer_id == target_id {
        return Ok(FollowState::Self_);
    }
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT status FROM follows WHERE follower_id = $1 AND following_id = $2",
    )
    .bind(viewer_id)
    .bind(target_id)
    .fetch_optional(&state.db)
    .await?;
    Ok(match row.as_ref().map(|(s,)| s.as_str()) {
        Some("active") => FollowState::Active,
        Some("pending") => FollowState::Pending,
        _ => FollowState::None,
    })
}

/// Public helper: count followers / following for a profile.
pub async fn follow_counts(state: &AppState, target_id: i64) -> ApiResult<(i64, i64)> {
    let followers: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM follows \
         WHERE following_id = $1 AND status = 'active'",
    )
    .bind(target_id)
    .fetch_one(&state.db)
    .await?;
    let following: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM follows \
         WHERE follower_id = $1 AND status = 'active'",
    )
    .bind(target_id)
    .fetch_one(&state.db)
    .await?;
    Ok((followers.0, following.0))
}

/// Private profiles only expose their follow lists to active followers (+self).
async fn ensure_list_visible(
    state: &AppState,
    viewer_id: i64,
    target: &Target,
) -> ApiResult<()> {
    if !target.is_private || viewer_id == target.id {
        return Ok(());
    }
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT status FROM follows WHERE follower_id = $1 AND following_id = $2",
    )
    .bind(viewer_id)
    .bind(target.id)
    .fetch_optional(&state.db)
    .await?;
    match row.as_ref().map(|(s,)| s.as_str()) {
        Some("active") => Ok(()),
        _ => Err(ApiError::Forbidden),
    }
}

fn decode_ts_cursor(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Follow-list cursor is just a plain RFC3339 timestamp. Opaque enough
    // for this and trivially debuggable.
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

fn build_follow_list(rows: Vec<sqlx::postgres::PgRow>, limit: i64) -> FollowListResponse {
    let has_more = rows.len() as i64 > limit;
    let shown: &[sqlx::postgres::PgRow] = if has_more {
        &rows[..limit as usize]
    } else {
        &rows[..]
    };
    let next_cursor = if has_more {
        shown.last().and_then(|r| {
            r.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .ok()
                .map(|ts| ts.to_rfc3339())
        })
    } else {
        None
    };
    let items: Vec<FollowListItem> = shown
        .iter()
        .map(|r| FollowListItem {
            uuid: r.try_get("uuid").unwrap_or_else(|_| Uuid::nil()),
            username: r.try_get("username").unwrap_or_default(),
            display_name: r.try_get("display_name").unwrap_or_default(),
            avatar_url: r.try_get("avatar_url").ok(),
            is_private: r.try_get("is_private").unwrap_or(false),
        })
        .collect();
    FollowListResponse {
        data: items,
        cursor: next_cursor,
        has_more,
    }
}
