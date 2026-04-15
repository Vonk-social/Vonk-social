//! Feed + profile-post query helpers.
//!
//! One privacy contract shared across three endpoints (`/api/feed`,
//! `/api/users/:username/posts`, `/api/posts/:uuid/replies`):
//!
//! - `deleted_at IS NULL`
//! - stories hidden from the main feed (use `/api/feed/stories`)
//! - visibility predicate:
//!   - `public` always, OR
//!   - `followers` AND requester follows the author (or is the author), OR
//!   - `mentioned` AND requester is in `mentioned_user_ids` (or is the author)
//! - `PublicPost` response NEVER carries `like_count` (CLAUDE.md §7)

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::cursor::Cursor;
use crate::error::ApiResult;
use crate::models::{media::PublicMedia, post::PostAuthor, post::PublicPost};

const PAGE_SIZE_DEFAULT: i64 = 20;
const PAGE_SIZE_MAX: i64 = 50;

pub fn clamp_limit(requested: Option<i64>) -> i64 {
    requested.unwrap_or(PAGE_SIZE_DEFAULT).clamp(1, PAGE_SIZE_MAX)
}

pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

/// Shared SELECT projection and visibility predicate. `{author}` is the
/// audience-specific predicate placeholder filled in by each caller.
/// `{cursor}` is replaced with either an empty string or the cursor clause
/// using the given placeholder numbers.
fn base_sql(author_predicate: &str, cursor_clause: &str, hide_stories: bool) -> String {
    let story_filter = if hide_stories {
        "AND p.post_type != 'story'"
    } else {
        ""
    };
    format!(
        r#"
        SELECT
            p.id, p.uuid, p.user_id AS author_id, p.content, p.post_type, p.visibility,
            p.reply_count, p.like_count, p.is_edited, p.expires_at, p.created_at,
            u.uuid AS author_uuid, u.username AS author_username,
            u.display_name AS author_display_name, u.avatar_url AS author_avatar_url,
            (SELECT uuid FROM posts r WHERE r.id = p.reply_to_id) AS reply_to_uuid,
            EXISTS(SELECT 1 FROM likes l WHERE l.post_id = p.id AND l.user_id = $1) AS liked_by_me
        FROM posts p
        JOIN users u ON u.id = p.user_id
        WHERE p.deleted_at IS NULL
          AND ({author_predicate})
          {story_filter}
          AND (
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
          {cursor_clause}
        ORDER BY p.created_at DESC, p.id DESC
    "#
    )
}

/// Main chronological feed: requester's own posts + posts of users they
/// actively follow.
pub async fn fetch_feed(
    db: &PgPool,
    requester_id: i64,
    cursor: Option<Cursor>,
    limit: i64,
) -> ApiResult<Page<PublicPost>> {
    let author = "p.user_id = $1 \
                  OR p.user_id IN (SELECT following_id FROM follows \
                                    WHERE follower_id = $1 AND status = 'active')";
    match cursor {
        Some(c) => {
            let sql = format!(
                "{body} LIMIT {lim}",
                body = base_sql(author, "AND (p.created_at, p.id) < ($2, $3)", true),
                lim = limit + 1,
            );
            let rows = sqlx::query(&sql)
                .bind(requester_id)
                .bind(c.ts)
                .bind(c.id)
                .fetch_all(db)
                .await?;
            build_page(db, rows, requester_id, limit).await
        }
        None => {
            let sql = format!("{body} LIMIT {lim}", body = base_sql(author, "", true), lim = limit + 1);
            let rows = sqlx::query(&sql).bind(requester_id).fetch_all(db).await?;
            build_page(db, rows, requester_id, limit).await
        }
    }
}

/// Posts authored by a specific user (public profile view).
pub async fn fetch_user_posts(
    db: &PgPool,
    requester_id: i64,
    author_id: i64,
    cursor: Option<Cursor>,
    limit: i64,
) -> ApiResult<Page<PublicPost>> {
    let author = "p.user_id = $2";
    match cursor {
        Some(c) => {
            let sql = format!(
                "{body} LIMIT {lim}",
                body = base_sql(author, "AND (p.created_at, p.id) < ($3, $4)", true),
                lim = limit + 1,
            );
            let rows = sqlx::query(&sql)
                .bind(requester_id)
                .bind(author_id)
                .bind(c.ts)
                .bind(c.id)
                .fetch_all(db)
                .await?;
            build_page(db, rows, requester_id, limit).await
        }
        None => {
            let sql = format!(
                "{body} LIMIT {lim}",
                body = base_sql(author, "", true),
                lim = limit + 1,
            );
            let rows = sqlx::query(&sql)
                .bind(requester_id)
                .bind(author_id)
                .fetch_all(db)
                .await?;
            build_page(db, rows, requester_id, limit).await
        }
    }
}

/// Replies to a specific parent post.
pub async fn fetch_replies(
    db: &PgPool,
    requester_id: i64,
    parent_id: i64,
    cursor: Option<Cursor>,
    limit: i64,
) -> ApiResult<Page<PublicPost>> {
    let author = "p.reply_to_id = $2";
    // Replies may themselves be stories (rare), but we hide them from /feed.
    // In a reply thread we keep them, so `hide_stories=false`.
    match cursor {
        Some(c) => {
            let sql = format!(
                "{body} LIMIT {lim}",
                body = base_sql(author, "AND (p.created_at, p.id) < ($3, $4)", false),
                lim = limit + 1,
            );
            let rows = sqlx::query(&sql)
                .bind(requester_id)
                .bind(parent_id)
                .bind(c.ts)
                .bind(c.id)
                .fetch_all(db)
                .await?;
            build_page(db, rows, requester_id, limit).await
        }
        None => {
            let sql = format!(
                "{body} LIMIT {lim}",
                body = base_sql(author, "", false),
                lim = limit + 1,
            );
            let rows = sqlx::query(&sql)
                .bind(requester_id)
                .bind(parent_id)
                .fetch_all(db)
                .await?;
            build_page(db, rows, requester_id, limit).await
        }
    }
}

async fn build_page(
    db: &PgPool,
    rows: Vec<sqlx::postgres::PgRow>,
    requester_id: i64,
    limit: i64,
) -> ApiResult<Page<PublicPost>> {
    let has_more = rows.len() as i64 > limit;
    let shown = if has_more {
        &rows[..limit as usize]
    } else {
        &rows[..]
    };

    let ids: Vec<i64> = shown.iter().map(|r| r.try_get::<i64, _>("id").unwrap_or(0)).collect();
    let media_by_post = fetch_media_for_posts(db, &ids).await?;

    let mut items: Vec<PublicPost> = Vec::with_capacity(shown.len());
    let mut last_row_opt: Option<(DateTime<Utc>, i64)> = None;

    let mut media_map = media_by_post;
    for r in shown {
        let id: i64 = r.try_get("id").unwrap_or(0);
        let author_id: i64 = r.try_get("author_id").unwrap_or(0);
        let created_at: DateTime<Utc> = r.try_get("created_at").unwrap_or_else(|_| Utc::now());
        last_row_opt = Some((created_at, id));

        // Author-only: like_count surfaced in the JSON when the requester
        // is the post's author; None otherwise, which serde skips entirely.
        let like_count = if author_id == requester_id {
            r.try_get::<i32, _>("like_count").ok()
        } else {
            None
        };

        items.push(PublicPost {
            uuid: r.try_get("uuid").unwrap_or_else(|_| Uuid::nil()),
            author: PostAuthor {
                uuid: r.try_get("author_uuid").unwrap_or_else(|_| Uuid::nil()),
                username: r.try_get("author_username").unwrap_or_default(),
                display_name: r.try_get("author_display_name").unwrap_or_default(),
                avatar_url: r.try_get("author_avatar_url").ok(),
            },
            content: r.try_get("content").ok(),
            media: media_map.remove(&id).unwrap_or_default(),
            post_type: r.try_get::<String, _>("post_type").unwrap_or_default(),
            visibility: r.try_get::<String, _>("visibility").unwrap_or_default(),
            reply_to_uuid: r.try_get("reply_to_uuid").ok(),
            reply_count: r.try_get("reply_count").unwrap_or(0),
            is_edited: r
                .try_get::<Option<bool>, _>("is_edited")
                .unwrap_or(Some(false))
                .unwrap_or(false),
            expires_at: r.try_get("expires_at").ok(),
            created_at,
            liked_by_me: r.try_get::<bool, _>("liked_by_me").unwrap_or(false),
            like_count,
        });
    }

    let next_cursor = if has_more {
        last_row_opt.map(|(ts, id)| Cursor { ts, id }.encode())
    } else {
        None
    };

    Ok(Page { items, next_cursor })
}

pub async fn fetch_media_for_posts(
    db: &PgPool,
    post_ids: &[i64],
) -> ApiResult<std::collections::HashMap<i64, Vec<PublicMedia>>> {
    let mut map: std::collections::HashMap<i64, Vec<PublicMedia>> = Default::default();
    if post_ids.is_empty() {
        return Ok(map);
    }
    let rows = sqlx::query(
        "SELECT post_id, uuid, media_type, width, height, blurhash, alt_text, variants \
         FROM media WHERE post_id = ANY($1) AND processing_status = 'completed' \
         ORDER BY id ASC",
    )
    .bind(post_ids)
    .fetch_all(db)
    .await?;

    for r in rows {
        let post_id: i64 = r.try_get("post_id").unwrap_or(0);
        let pm = PublicMedia {
            uuid: r.try_get("uuid").unwrap_or_else(|_| Uuid::nil()),
            media_type: r.try_get::<String, _>("media_type").unwrap_or_default(),
            width: r.try_get("width").ok(),
            height: r.try_get("height").ok(),
            blurhash: r.try_get("blurhash").ok(),
            alt_text: r.try_get("alt_text").ok(),
            variants: r
                .try_get::<serde_json::Value, _>("variants")
                .unwrap_or(serde_json::json!({})),
        };
        map.entry(post_id).or_default().push(pm);
    }
    Ok(map)
}
