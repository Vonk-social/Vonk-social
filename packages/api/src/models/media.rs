//! `media` row mapping.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct Media {
    pub id: i64,
    pub uuid: Uuid,
    pub user_id: i64,
    pub post_id: Option<i64>,
    pub media_type: String,
    pub storage_key: String,
    pub mime_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_ms: Option<i32>,
    pub blurhash: Option<String>,
    pub alt_text: Option<String>,
    pub processing_status: Option<String>,
    pub variants: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl Media {
    /// Stable column projection for `SELECT`s on `media`. First consumer
    /// lands in Phase 3 (background media-reprocess worker); keeping the
    /// constant means queries share one source of truth.
    #[allow(dead_code)]
    pub const COLUMNS: &'static str = "id, uuid, user_id, post_id, media_type, storage_key, \
         mime_type, file_size, width, height, duration_ms, blurhash, alt_text, \
         processing_status, variants, created_at";
}

/// Public representation — omits internal id, user_id, storage_key.
#[derive(Debug, Serialize, Clone)]
pub struct PublicMedia {
    pub uuid: Uuid,
    pub media_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub blurhash: Option<String>,
    pub alt_text: Option<String>,
    pub variants: serde_json::Value,
}

impl From<&Media> for PublicMedia {
    fn from(m: &Media) -> Self {
        PublicMedia {
            uuid: m.uuid,
            media_type: m.media_type.clone(),
            width: m.width,
            height: m.height,
            blurhash: m.blurhash.clone(),
            alt_text: m.alt_text.clone(),
            variants: m.variants.clone().unwrap_or(serde_json::json!({})),
        }
    }
}
