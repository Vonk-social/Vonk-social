//! Feed + stories-tray + profile-posts endpoints.

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::feed::{
    cursor::Cursor,
    query::{self, fetch_feed, fetch_media_for_posts, fetch_user_posts, Page},
};
use crate::models::{
    media::PublicMedia,
    post::{PostAuthor, PublicPost},
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/feed", get(get_feed))
        .route("/api/feed/stories", get(get_stories_tray))
        .route("/api/users/{username}/posts", get(get_user_posts))
}

#[derive(Deserialize)]
struct ListQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct PageResponse<T: Serialize> {
    data: Vec<T>,
    cursor: Option<String>,
    has_more: bool,
}

async fn get_feed(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<PageResponse<PublicPost>>> {
    let cursor = q.cursor.as_deref().and_then(Cursor::decode);
    let limit = query::clamp_limit(q.limit);
    let Page { items, next_cursor } = fetch_feed(&state.db, user.id, cursor, limit).await?;
    let has_more = next_cursor.is_some();
    Ok(Json(PageResponse {
        data: items,
        cursor: next_cursor,
        has_more,
    }))
}

async fn get_user_posts(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(username): Path<String>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<PageResponse<PublicPost>>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM users WHERE username = $1 \
           AND deleted_at IS NULL \
           AND COALESCE(is_suspended, false) = false",
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await?;
    let author_id = row.ok_or(ApiError::NotFound)?.0;

    let cursor = q.cursor.as_deref().and_then(Cursor::decode);
    let limit = query::clamp_limit(q.limit);
    let Page { items, next_cursor } =
        fetch_user_posts(&state.db, user.id, author_id, cursor, limit).await?;
    let has_more = next_cursor.is_some();

    Ok(Json(PageResponse {
        data: items,
        cursor: next_cursor,
        has_more,
    }))
}

// ── Stories tray ─────────────────────────────────────────────

#[derive(Serialize)]
struct StoryItem {
    uuid: Uuid,
    media: Vec<PublicMedia>,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    viewed_by_me: bool,
}

#[derive(Serialize)]
struct StoryGroup {
    author: PostAuthor,
    items: Vec<StoryItem>,
    total_count: i32,
    unseen_count: i32,
}

#[derive(Serialize)]
struct StoryTrayResponse {
    data: Vec<StoryGroup>,
}

/// One row per active story, grouped in the handler by author.
/// `viewed_by_me` comes from `story_views`.
async fn get_stories_tray(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<StoryTrayResponse>> {
    let rows = sqlx::query(
        r#"
        SELECT
            p.id, p.uuid, p.created_at, p.expires_at,
            u.id AS author_id, u.uuid AS author_uuid, u.username, u.display_name, u.avatar_url,
            EXISTS(SELECT 1 FROM story_views sv WHERE sv.story_id = p.id AND sv.user_id = $1) AS viewed_by_me
        FROM posts p
        JOIN users u ON u.id = p.user_id
        WHERE p.post_type = 'story'
          AND p.deleted_at IS NULL
          AND p.expires_at > now()
          AND (
                p.user_id = $1
             OR p.user_id IN (SELECT following_id FROM follows
                               WHERE follower_id = $1 AND status = 'active')
              )
          AND (
                p.visibility = 'public'
             OR (p.visibility = 'followers' AND (
                    p.user_id = $1 OR EXISTS (
                        SELECT 1 FROM follows f
                         WHERE f.follower_id = $1 AND f.following_id = p.user_id
                           AND f.status = 'active'
                    )
                ))
             OR (p.visibility = 'mentioned' AND ($1 = ANY(p.mentioned_user_ids) OR p.user_id = $1))
              )
        ORDER BY u.id, p.created_at ASC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let ids: Vec<i64> = rows.iter().filter_map(|r| r.try_get("id").ok()).collect();
    let media_by_post = fetch_media_for_posts(&state.db, &ids).await?;

    let mut groups: Vec<StoryGroup> = Vec::new();
    let mut current_author: Option<i64> = None;
    let mut media_map = media_by_post;
    for r in &rows {
        let author_id: i64 = r.try_get("author_id").unwrap_or(0);
        let id: i64 = r.try_get("id").unwrap_or(0);
        let viewed: bool = r.try_get("viewed_by_me").unwrap_or(false);

        let item = StoryItem {
            uuid: r.try_get("uuid").unwrap_or_else(|_| Uuid::nil()),
            media: media_map.remove(&id).unwrap_or_default(),
            created_at: r.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            expires_at: r.try_get("expires_at").ok(),
            viewed_by_me: viewed,
        };

        if current_author != Some(author_id) {
            groups.push(StoryGroup {
                author: PostAuthor {
                    uuid: r.try_get("author_uuid").unwrap_or_else(|_| Uuid::nil()),
                    username: r.try_get("username").unwrap_or_default(),
                    display_name: r.try_get("display_name").unwrap_or_default(),
                    avatar_url: r.try_get("avatar_url").ok(),
                },
                items: Vec::new(),
                total_count: 0,
                unseen_count: 0,
            });
            current_author = Some(author_id);
        }

        let g = groups.last_mut().expect("we just pushed");
        g.total_count += 1;
        if !viewed {
            g.unseen_count += 1;
        }
        g.items.push(item);
    }

    Ok(Json(StoryTrayResponse { data: groups }))
}
