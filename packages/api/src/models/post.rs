//! `posts` row mapping + response shapes.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use super::media::PublicMedia;

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct Post {
    pub id: i64,
    pub uuid: Uuid,
    pub user_id: i64,
    pub content: Option<String>,
    pub post_type: String,
    pub visibility: String,
    pub reply_to_id: Option<i64>,
    pub thread_root_id: Option<i64>,
    pub reply_count: i32,
    pub like_count: i32,
    pub is_edited: Option<bool>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub group_id: Option<i64>,
    pub mentioned_user_ids: Vec<i64>,
}

impl Post {
    /// All columns in the stable order used by every SELECT on `posts`.
    /// Reserved for the profile/my-posts endpoint landing later in Phase 2.
    #[allow(dead_code)]
    pub const COLUMNS: &'static str = "id, uuid, user_id, content, post_type, visibility, \
        reply_to_id, thread_root_id, reply_count, like_count, is_edited, expires_at, \
        created_at, updated_at, deleted_at, group_id, mentioned_user_ids";
}

/// Author preview embedded in every post response.
#[derive(Debug, Serialize, Clone)]
pub struct PostAuthor {
    pub uuid: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Public post representation. **Never includes `like_count`** — per
/// CLAUDE.md §7, only the author may see the like count (see `MyPost`).
#[derive(Debug, Serialize)]
pub struct PublicPost {
    pub uuid: Uuid,
    pub author: PostAuthor,
    pub content: Option<String>,
    pub media: Vec<PublicMedia>,
    pub post_type: String,
    pub visibility: String,
    pub reply_to_uuid: Option<Uuid>,
    pub reply_count: i32,
    pub is_edited: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    /// Did the requesting user like this post? Present on every list
    /// response to avoid N+1 client-side lookups.
    pub liked_by_me: bool,
}

/// Author-only post representation. Superset of `PublicPost` with
/// `like_count` exposed. Used by the upcoming "my posts" endpoint.
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct MyPost {
    #[serde(flatten)]
    pub public: PublicPost,
    pub like_count: i32,
}
