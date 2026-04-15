//! Snap row mapping.
//!
//! Snaps layer on the existing `messages` table via `view_policy` + per-recipient
//! tracking in `snap_views`. See migration 003. Phase 2 stores plaintext media
//! keys in the `ciphertext` BYTEA column; Phase 3 will swap that for real MLS
//! ciphertext without schema change.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct DeliverableSnap {
    pub id: i64,
    pub uuid: Uuid,
    pub conversation_id: i64,
    pub sender_id: i64,
    pub recipient_id: i64,
    pub view_policy: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub media_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub viewed_by_me: bool,
}

/// Public snap envelope — sent to the recipient's inbox list. The actual
/// media URL is only issued on `GET /api/snaps/:uuid/view`.
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct SnapEnvelope {
    pub uuid: Uuid,
    pub sender: crate::models::post::PostAuthor,
    pub view_policy: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub viewed_by_me: bool,
}
