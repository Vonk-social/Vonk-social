//! `follows` row mapping + relationship state enum.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct Follow {
    pub follower_id: i64,
    pub following_id: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Summary of the (requester → target) relationship, returned by
/// `GET /api/users/:username` and used by the web `FollowButton`.
#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum FollowState {
    /// Not following and no pending request.
    None,
    /// We requested; awaiting approval from a private account.
    Pending,
    /// Active follow.
    Active,
    /// Target is the requester themself.
    Self_,
}

impl FollowState {
    pub fn as_str(self) -> &'static str {
        match self {
            FollowState::None => "none",
            FollowState::Pending => "pending",
            FollowState::Active => "active",
            FollowState::Self_ => "self",
        }
    }
}
