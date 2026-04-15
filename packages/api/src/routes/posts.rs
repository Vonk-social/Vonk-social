//! Post CRUD + like / unlike + replies listing.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use validator::Validate;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::feed::{
    cursor::Cursor,
    query::{self, fetch_media_for_posts, fetch_replies, Page},
};
use crate::models::post::{PostAuthor, PublicPost};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/posts", post(create_post))
        .route(
            "/api/posts/{uuid}",
            get(get_post).patch(patch_post).delete(delete_post),
        )
        .route("/api/posts/{uuid}/replies", get(get_replies))
        .route("/api/posts/{uuid}/like", post(like_post).delete(unlike_post))
        .route("/api/posts/{uuid}/viewed", post(mark_story_viewed))
}

#[derive(Serialize)]
struct DataEnvelope<T: Serialize> {
    data: T,
}

#[derive(Serialize)]
struct PageResponse<T: Serialize> {
    data: Vec<T>,
    cursor: Option<String>,
    has_more: bool,
}

#[derive(Debug, Deserialize, Validate)]
struct CreatePostRequest {
    #[validate(length(max = 5000))]
    content: Option<String>,
    #[serde(default)]
    media_uuids: Vec<Uuid>,
    #[serde(default = "default_post_type")]
    post_type: String,
    #[serde(default = "default_visibility")]
    visibility: String,
    #[serde(default)]
    reply_to_uuid: Option<Uuid>,
    #[serde(default)]
    mentions: Vec<String>,
}

fn default_post_type() -> String {
    "post".to_string()
}
fn default_visibility() -> String {
    "public".to_string()
}

#[derive(Debug, Deserialize, Validate)]
struct PatchPostRequest {
    #[validate(length(max = 5000))]
    content: Option<String>,
}

#[derive(Deserialize)]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

// ── create ───────────────────────────────────────────────────

async fn create_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreatePostRequest>,
) -> ApiResult<(StatusCode, Json<DataEnvelope<PublicPost>>)> {
    req.validate()?;

    if !matches!(req.post_type.as_str(), "post" | "story") {
        return Err(ApiError::bad_request(
            "invalid_post_type",
            "post_type must be 'post' or 'story'",
        ));
    }
    if !matches!(req.visibility.as_str(), "public" | "followers" | "mentioned") {
        return Err(ApiError::bad_request(
            "invalid_visibility",
            "visibility must be 'public', 'followers' or 'mentioned'",
        ));
    }
    if req.media_uuids.len() > 4 {
        return Err(ApiError::bad_request(
            "too_many_media",
            "max 4 media items per post",
        ));
    }
    let has_content = req
        .content
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !has_content && req.media_uuids.is_empty() {
        return Err(ApiError::bad_request(
            "empty_post",
            "A post needs either content or media",
        ));
    }

    // Stories: server-set 24h expiry, media required.
    let expires_at: Option<DateTime<Utc>> = if req.post_type == "story" {
        if req.media_uuids.is_empty() {
            return Err(ApiError::bad_request(
                "story_needs_media",
                "A story needs at least one image",
            ));
        }
        Some(Utc::now() + ChronoDuration::hours(24))
    } else {
        None
    };

    // Resolve reply_to_uuid → id.
    let reply_to_id = if let Some(rt) = req.reply_to_uuid {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM posts WHERE uuid = $1 AND deleted_at IS NULL",
        )
        .bind(rt)
        .fetch_optional(&state.db)
        .await?;
        match row {
            Some((id,)) => Some(id),
            None => {
                return Err(ApiError::bad_request(
                    "reply_target_not_found",
                    "parent post not found",
                ))
            }
        }
    } else {
        None
    };

    // Resolve mentions.
    let mentioned_ids: Vec<i64> = if req.mentions.is_empty() {
        Vec::new()
    } else {
        sqlx::query("SELECT id FROM users WHERE username = ANY($1) AND deleted_at IS NULL")
            .bind(&req.mentions)
            .fetch_all(&state.db)
            .await?
            .into_iter()
            .filter_map(|r| r.try_get::<i64, _>("id").ok())
            .collect()
    };

    let mut tx = state.db.begin().await?;
    let post_uuid = Uuid::new_v4();
    let post_id: i64 = sqlx::query_scalar(
        "INSERT INTO posts (uuid, user_id, content, post_type, visibility, \
                            reply_to_id, expires_at, mentioned_user_ids) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING id",
    )
    .bind(post_uuid)
    .bind(user.id)
    .bind(req.content.as_deref())
    .bind(&req.post_type)
    .bind(&req.visibility)
    .bind(reply_to_id)
    .bind(expires_at)
    .bind(&mentioned_ids)
    .fetch_one(&mut *tx)
    .await?;

    // Link media rows (must belong to the author and not be attached yet).
    if !req.media_uuids.is_empty() {
        let linked = sqlx::query(
            "UPDATE media SET post_id = $1 \
              WHERE uuid = ANY($2) AND user_id = $3 AND post_id IS NULL \
             RETURNING id",
        )
        .bind(post_id)
        .bind(&req.media_uuids)
        .bind(user.id)
        .fetch_all(&mut *tx)
        .await?;
        if linked.len() != req.media_uuids.len() {
            return Err(ApiError::bad_request(
                "media_not_yours_or_used",
                "one or more media items are unknown or already attached",
            ));
        }
    }

    if let Some(rt_id) = reply_to_id {
        sqlx::query("UPDATE posts SET reply_count = reply_count + 1 WHERE id = $1")
            .bind(rt_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    let post = load_public_post(&state, user.id, post_id).await?;
    Ok((StatusCode::CREATED, Json(DataEnvelope { data: post })))
}

// ── get ──────────────────────────────────────────────────────

async fn get_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(post_uuid): Path<Uuid>,
) -> ApiResult<Json<DataEnvelope<PublicPost>>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM posts WHERE uuid = $1 AND deleted_at IS NULL",
    )
    .bind(post_uuid)
    .fetch_optional(&state.db)
    .await?;
    let post_id = row.ok_or(ApiError::NotFound)?.0;

    if !is_visible_to(&state, user.id, post_id).await? {
        return Err(ApiError::NotFound);
    }
    let post = load_public_post(&state, user.id, post_id).await?;
    Ok(Json(DataEnvelope { data: post }))
}

// ── patch / delete ───────────────────────────────────────────

async fn patch_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(post_uuid): Path<Uuid>,
    Json(req): Json<PatchPostRequest>,
) -> ApiResult<Json<DataEnvelope<PublicPost>>> {
    req.validate()?;

    let row = sqlx::query("SELECT id, user_id FROM posts WHERE uuid = $1 AND deleted_at IS NULL")
        .bind(post_uuid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(ApiError::NotFound)?;
    let post_id: i64 = row.try_get("id").unwrap_or(0);
    let author_id: i64 = row.try_get("user_id").unwrap_or(0);
    if author_id != user.id {
        return Err(ApiError::Forbidden);
    }

    sqlx::query(
        "UPDATE posts SET \
            content = COALESCE($2, content), \
            is_edited = true, \
            updated_at = now() \
          WHERE id = $1",
    )
    .bind(post_id)
    .bind(req.content.as_deref())
    .execute(&state.db)
    .await?;

    let post = load_public_post(&state, user.id, post_id).await?;
    Ok(Json(DataEnvelope { data: post }))
}

async fn delete_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(post_uuid): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let row = sqlx::query(
        "SELECT id, user_id, reply_to_id FROM posts \
          WHERE uuid = $1 AND deleted_at IS NULL",
    )
    .bind(post_uuid)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound)?;
    let post_id: i64 = row.try_get("id").unwrap_or(0);
    let author_id: i64 = row.try_get("user_id").unwrap_or(0);
    let reply_to_id: Option<i64> = row.try_get("reply_to_id").ok();
    if author_id != user.id {
        return Err(ApiError::Forbidden);
    }

    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE posts SET deleted_at = now() WHERE id = $1")
        .bind(post_id)
        .execute(&mut *tx)
        .await?;
    if let Some(rt_id) = reply_to_id {
        sqlx::query("UPDATE posts SET reply_count = GREATEST(reply_count - 1, 0) WHERE id = $1")
            .bind(rt_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── replies ──────────────────────────────────────────────────

async fn get_replies(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(post_uuid): Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<PageResponse<PublicPost>>> {
    let parent: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM posts WHERE uuid = $1 AND deleted_at IS NULL",
    )
    .bind(post_uuid)
    .fetch_optional(&state.db)
    .await?;
    let parent_id = parent.ok_or(ApiError::NotFound)?.0;

    let cursor = q.cursor.as_deref().and_then(Cursor::decode);
    let limit = query::clamp_limit(q.limit);
    let Page { items, next_cursor } =
        fetch_replies(&state.db, user.id, parent_id, cursor, limit).await?;
    let has_more = next_cursor.is_some();

    Ok(Json(PageResponse {
        data: items,
        cursor: next_cursor,
        has_more,
    }))
}

// ── like / unlike ────────────────────────────────────────────

async fn like_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(post_uuid): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let post: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM posts WHERE uuid = $1 AND deleted_at IS NULL",
    )
    .bind(post_uuid)
    .fetch_optional(&state.db)
    .await?;
    let post_id = post.ok_or(ApiError::NotFound)?.0;

    if !is_visible_to(&state, user.id, post_id).await? {
        return Err(ApiError::NotFound);
    }

    let mut tx = state.db.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO likes (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(user.id)
    .bind(post_id)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() > 0 {
        sqlx::query("UPDATE posts SET like_count = like_count + 1 WHERE id = $1")
            .bind(post_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unlike_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(post_uuid): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let post: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM posts WHERE uuid = $1 AND deleted_at IS NULL",
    )
    .bind(post_uuid)
    .fetch_optional(&state.db)
    .await?;
    let post_id = post.ok_or(ApiError::NotFound)?.0;

    let mut tx = state.db.begin().await?;
    let removed = sqlx::query("DELETE FROM likes WHERE user_id = $1 AND post_id = $2")
        .bind(user.id)
        .bind(post_id)
        .execute(&mut *tx)
        .await?;
    if removed.rows_affected() > 0 {
        sqlx::query("UPDATE posts SET like_count = GREATEST(like_count - 1, 0) WHERE id = $1")
            .bind(post_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── story viewed ─────────────────────────────────────────────

async fn mark_story_viewed(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(post_uuid): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, post_type FROM posts WHERE uuid = $1 AND deleted_at IS NULL",
    )
    .bind(post_uuid)
    .fetch_optional(&state.db)
    .await?;
    let (id, post_type) = row.ok_or(ApiError::NotFound)?;
    if post_type != "story" {
        return Ok(StatusCode::NO_CONTENT);
    }
    sqlx::query(
        "INSERT INTO story_views (story_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(user.id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── helpers ──────────────────────────────────────────────────

async fn is_visible_to(state: &AppState, viewer_id: i64, post_id: i64) -> ApiResult<bool> {
    let row: Option<(bool,)> = sqlx::query_as(
        r#"
        SELECT (
               p.visibility = 'public'
            OR (p.visibility = 'followers' AND (
                    p.user_id = $1 OR EXISTS (
                        SELECT 1 FROM follows f
                         WHERE f.follower_id = $1
                           AND f.following_id = p.user_id
                           AND f.status = 'active'
                    )
                ))
            OR (p.visibility = 'mentioned' AND ($1 = ANY(p.mentioned_user_ids) OR p.user_id = $1))
        )
        FROM posts p WHERE p.id = $2 AND p.deleted_at IS NULL
        "#,
    )
    .bind(viewer_id)
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?;
    Ok(row.map(|(v,)| v).unwrap_or(false))
}

async fn load_public_post(state: &AppState, viewer_id: i64, post_id: i64) -> ApiResult<PublicPost> {
    let row = sqlx::query(
        r#"
        SELECT
            p.uuid, p.user_id AS author_id, p.content, p.post_type, p.visibility,
            p.reply_count, p.like_count, p.is_edited, p.expires_at, p.created_at,
            u.uuid AS author_uuid, u.username AS author_username,
            u.display_name AS author_display_name, u.avatar_url AS author_avatar_url,
            (SELECT uuid FROM posts r WHERE r.id = p.reply_to_id) AS reply_to_uuid,
            EXISTS(SELECT 1 FROM likes l WHERE l.post_id = p.id AND l.user_id = $1) AS liked_by_me
        FROM posts p JOIN users u ON u.id = p.user_id
        WHERE p.id = $2
        "#,
    )
    .bind(viewer_id)
    .bind(post_id)
    .fetch_one(&state.db)
    .await?;

    let media_by_post = fetch_media_for_posts(&state.db, &[post_id]).await?;
    let media = media_by_post
        .into_iter()
        .next()
        .map(|(_, v)| v)
        .unwrap_or_default();

    let author_id: i64 = row.try_get("author_id").unwrap_or(0);
    let like_count = if author_id == viewer_id {
        row.try_get::<i32, _>("like_count").ok()
    } else {
        None
    };

    Ok(PublicPost {
        uuid: row.try_get("uuid").unwrap_or_else(|_| Uuid::nil()),
        author: PostAuthor {
            uuid: row.try_get("author_uuid").unwrap_or_else(|_| Uuid::nil()),
            username: row.try_get("author_username").unwrap_or_default(),
            display_name: row.try_get("author_display_name").unwrap_or_default(),
            avatar_url: row.try_get("author_avatar_url").ok(),
        },
        content: row.try_get("content").ok(),
        media,
        post_type: row.try_get::<String, _>("post_type").unwrap_or_default(),
        visibility: row.try_get::<String, _>("visibility").unwrap_or_default(),
        reply_to_uuid: row.try_get("reply_to_uuid").ok(),
        reply_count: row.try_get("reply_count").unwrap_or(0),
        is_edited: row
            .try_get::<Option<bool>, _>("is_edited")
            .unwrap_or(Some(false))
            .unwrap_or(false),
        expires_at: row.try_get("expires_at").ok(),
        created_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
        liked_by_me: row.try_get::<bool, _>("liked_by_me").unwrap_or(false),
        like_count,
    })
}
