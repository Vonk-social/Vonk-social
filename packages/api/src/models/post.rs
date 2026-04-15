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

/// Public post representation.
///
/// Per CLAUDE.md §7, `like_count` is **only populated when the requester is
/// the post author** — and serialised-away entirely otherwise (via the
/// `skip_serializing_if` below) so the field literally does not appear in
/// non-author API responses. Impossible for the UI to display a stray count
/// it never received.
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
    /// Author-only. Number of likes on this post. Absent in the JSON when
    /// the requester isn't the author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like_count: Option<i32>,
}

// `MyPost` used to be a superset of `PublicPost` carrying `like_count`.
// That responsibility has moved onto `PublicPost` itself (with an Option +
// skip_serializing_if), so this helper is gone. Keeping the trail via git
// history rather than a deprecated stub.
