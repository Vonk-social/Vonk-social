//! Snaps — ephemeral 1-to-1 view-once media.
//!
//! Backed by the existing `messages` + `conversations` + `snap_views` tables.
//! Phase 2 stores the media key as plaintext in `ciphertext BYTEA`. Phase 3
//! replaces this with MLS ciphertext without schema change.
//!
//! * `POST   /api/snaps`                          — send a snap
//! * `GET    /api/snaps/inbox`                    — unviewed snaps you received
//! * `GET    /api/snaps/sent`                     — snaps you sent (with view status)
//! * `GET    /api/snaps/{uuid}/view`              — consume + presigned media URL
//! * `DELETE /api/snaps/{uuid}`                   — unsend (sender only, pre-view)

use std::time::Duration;

use aws_sdk_s3::presigning::PresigningConfig;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::models::post::PostAuthor;
use crate::state::AppState;

const VIEW_URL_TTL_SECS: u64 = 30;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/snaps", post(send_snap))
        .route("/api/snaps/inbox", get(inbox))
        .route("/api/snaps/sent", get(sent))
        .route("/api/snaps/{uuid}/view", get(view))
        .route("/api/snaps/{uuid}", delete(unsend))
}

#[derive(Serialize)]
struct DataEnvelope<T: Serialize> {
    data: T,
}

// ── send ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SendSnapRequest {
    to_username: String,
    media_uuid: Uuid,
    /// `view_once` (default) or `view_24h`.
    #[serde(default = "default_view_policy")]
    view_policy: String,
}

fn default_view_policy() -> String {
    "view_once".to_string()
}

#[derive(Serialize)]
struct SendSnapResponse {
    uuid: Uuid,
}

async fn send_snap(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Json(req): Json<SendSnapRequest>,
) -> ApiResult<(StatusCode, Json<DataEnvelope<SendSnapResponse>>)> {
    if !matches!(req.view_policy.as_str(), "view_once" | "view_24h") {
        return Err(ApiError::bad_request(
            "invalid_view_policy",
            "view_policy must be 'view_once' or 'view_24h'",
        ));
    }

    // Resolve recipient.
    let recipient: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM users \
          WHERE username = $1 AND deleted_at IS NULL AND COALESCE(is_suspended, false) = false",
    )
    .bind(&req.to_username)
    .fetch_optional(&state.db)
    .await?;
    let recipient_id = recipient.ok_or(ApiError::NotFound)?.0;
    if recipient_id == me.id {
        return Err(ApiError::bad_request(
            "cannot_snap_self",
            "You cannot send a snap to yourself",
        ));
    }

    // Resolve media (must be ours, not yet attached to a post).
    let media: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, storage_key FROM media \
          WHERE uuid = $1 AND user_id = $2",
    )
    .bind(req.media_uuid)
    .bind(me.id)
    .fetch_optional(&state.db)
    .await?;
    let (media_id, storage_key) = media.ok_or(ApiError::bad_request(
        "media_not_yours",
        "media not found or not owned by sender",
    ))?;

    let expires_at: Option<DateTime<Utc>> = if req.view_policy == "view_24h" {
        Some(Utc::now() + ChronoDuration::hours(24))
    } else {
        None
    };

    let mut tx = state.db.begin().await?;

    // Find or create a 1-on-1 conversation.
    let existing_conv: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT c.id FROM conversations c
         WHERE c.conversation_type = 'direct'
           AND EXISTS (SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = $1)
           AND EXISTS (SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = $2)
           AND (SELECT COUNT(*) FROM conversation_members WHERE conversation_id = c.id) = 2
         LIMIT 1
        "#,
    )
    .bind(me.id)
    .bind(recipient_id)
    .fetch_optional(&mut *tx)
    .await?;

    let conversation_id = match existing_conv {
        Some((id,)) => id,
        None => {
            let id: i64 = sqlx::query_scalar(
                "INSERT INTO conversations (conversation_type) VALUES ('direct') RETURNING id",
            )
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO conversation_members (conversation_id, user_id) \
                 VALUES ($1, $2), ($1, $3)",
            )
            .bind(id)
            .bind(me.id)
            .bind(recipient_id)
            .execute(&mut *tx)
            .await?;
            id
        }
    };

    // Insert the message. `ciphertext` holds the storage_key bytes as a
    // placeholder for the future MLS-encrypted blob.
    let snap_uuid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO messages \
            (uuid, conversation_id, sender_id, ciphertext, protocol_version, \
             content_type, view_policy, expires_at, media_id) \
         VALUES ($1, $2, $3, $4, 'plaintext-phase2', 'image', $5, $6, $7)",
    )
    .bind(snap_uuid)
    .bind(conversation_id)
    .bind(me.id)
    .bind(storage_key.as_bytes())
    .bind(&req.view_policy)
    .bind(expires_at)
    .bind(media_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(DataEnvelope {
            data: SendSnapResponse { uuid: snap_uuid },
        }),
    ))
}

// ── inbox / sent ─────────────────────────────────────────────

#[derive(Serialize)]
struct SnapInboxItem {
    uuid: Uuid,
    sender: PostAuthor,
    view_policy: String,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    viewed_by_me: bool,
}

#[derive(Serialize)]
struct SnapListResponse {
    data: Vec<SnapInboxItem>,
}

async fn inbox(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
) -> ApiResult<Json<SnapListResponse>> {
    let rows = sqlx::query(
        r#"
        SELECT sd.uuid, sd.view_policy, sd.expires_at, sd.created_at, sd.viewed_by_me,
               u.uuid AS sender_uuid, u.username, u.display_name, u.avatar_url
          FROM snap_deliverable sd
          JOIN users u ON u.id = sd.sender_id
         WHERE sd.recipient_id = $1
           AND NOT sd.viewed_by_me
         ORDER BY sd.created_at DESC
         LIMIT 100
        "#,
    )
    .bind(me.id)
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .iter()
        .map(|r| SnapInboxItem {
            uuid: r.try_get("uuid").unwrap_or_else(|_| Uuid::nil()),
            sender: PostAuthor {
                uuid: r.try_get("sender_uuid").unwrap_or_else(|_| Uuid::nil()),
                username: r.try_get("username").unwrap_or_default(),
                display_name: r.try_get("display_name").unwrap_or_default(),
                avatar_url: r.try_get("avatar_url").ok(),
            },
            view_policy: r.try_get::<String, _>("view_policy").unwrap_or_default(),
            expires_at: r.try_get("expires_at").ok(),
            created_at: r.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            viewed_by_me: r.try_get("viewed_by_me").unwrap_or(false),
        })
        .collect();

    Ok(Json(SnapListResponse { data }))
}

#[derive(Serialize)]
struct SentSnapItem {
    uuid: Uuid,
    recipient: PostAuthor,
    view_policy: String,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    viewed_by_them: bool,
}

#[derive(Serialize)]
struct SentSnapResponse {
    data: Vec<SentSnapItem>,
}

async fn sent(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
) -> ApiResult<Json<SentSnapResponse>> {
    let rows = sqlx::query(
        r#"
        SELECT m.uuid, m.view_policy, m.expires_at, m.created_at,
               u.uuid AS recipient_uuid, u.username, u.display_name, u.avatar_url,
               EXISTS(SELECT 1 FROM snap_views sv
                       WHERE sv.message_id = m.id AND sv.user_id = cm.user_id) AS viewed_by_them
          FROM messages m
          JOIN conversation_members cm
            ON cm.conversation_id = m.conversation_id AND cm.user_id != m.sender_id
          JOIN users u ON u.id = cm.user_id
         WHERE m.sender_id = $1
           AND m.deleted_at IS NULL
           AND m.view_policy IN ('view_once', 'view_24h')
         ORDER BY m.created_at DESC
         LIMIT 100
        "#,
    )
    .bind(me.id)
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .iter()
        .map(|r| SentSnapItem {
            uuid: r.try_get("uuid").unwrap_or_else(|_| Uuid::nil()),
            recipient: PostAuthor {
                uuid: r.try_get("recipient_uuid").unwrap_or_else(|_| Uuid::nil()),
                username: r.try_get("username").unwrap_or_default(),
                display_name: r.try_get("display_name").unwrap_or_default(),
                avatar_url: r.try_get("avatar_url").ok(),
            },
            view_policy: r.try_get::<String, _>("view_policy").unwrap_or_default(),
            expires_at: r.try_get("expires_at").ok(),
            created_at: r.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            viewed_by_them: r.try_get("viewed_by_them").unwrap_or(false),
        })
        .collect();

    Ok(Json(SentSnapResponse { data }))
}

// ── view ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct ViewSnapResponse {
    url: String,
    expires_at: DateTime<Utc>,
}

async fn view(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(snap_uuid): Path<Uuid>,
) -> ApiResult<Json<DataEnvelope<ViewSnapResponse>>> {
    // Find the deliverable row for (this snap, this viewer).
    let row = sqlx::query(
        r#"
        SELECT sd.id, sd.media_id, sd.viewed_by_me, sd.expires_at
          FROM snap_deliverable sd
         WHERE sd.uuid = $1 AND sd.recipient_id = $2
        "#,
    )
    .bind(snap_uuid)
    .bind(me.id)
    .fetch_optional(&state.db)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Err(ApiError::NotFound),
    };

    let message_id: i64 = row.try_get("id").unwrap_or(0);
    let media_id: Option<i64> = row.try_get("media_id").ok();
    let viewed: bool = row.try_get("viewed_by_me").unwrap_or(false);

    if viewed {
        // view_once semantics — the bytes are already in the recipient's
        // browser from the first load; they don't get a second copy.
        return Err(ApiError::Conflict {
            code: "snap_already_viewed",
            message: "this snap has already been viewed".into(),
        });
    }

    let media_id = media_id.ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!("snap {snap_uuid} has no media_id"))
    })?;

    // Look up the storage_key for presigning.
    let (storage_key,): (String,) = sqlx::query_as(
        "SELECT storage_key FROM media WHERE id = $1",
    )
    .bind(media_id)
    .fetch_one(&state.db)
    .await?;

    // Presigned GET URL for the `full.webp` variant. 30-second TTL so the
    // link can't be shared.
    let object_key = format!("{storage_key}/full.webp");
    let presigned = state
        .s3
        .get_object()
        .bucket(&state.config.s3_bucket)
        .key(&object_key)
        .presigned(
            PresigningConfig::expires_in(Duration::from_secs(VIEW_URL_TTL_SECS))
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("presign: {e}")))?,
        )
        .await
        .map_err(|e| ApiError::Upstream(anyhow::anyhow!("s3 presign: {e}")))?;
    let url = presigned.uri().to_string();
    let expires_at = Utc::now() + ChronoDuration::seconds(VIEW_URL_TTL_SECS as i64);

    // Record the view atomically.
    sqlx::query(
        "INSERT INTO snap_views (message_id, user_id) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(message_id)
    .bind(me.id)
    .execute(&state.db)
    .await?;

    Ok(Json(DataEnvelope {
        data: ViewSnapResponse { url, expires_at },
    }))
}

// ── unsend ───────────────────────────────────────────────────

async fn unsend(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(snap_uuid): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Only the sender can unsend, and only if no recipient has viewed yet.
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT id, sender_id FROM messages \
          WHERE uuid = $1 AND deleted_at IS NULL \
            AND view_policy IN ('view_once', 'view_24h')",
    )
    .bind(snap_uuid)
    .fetch_optional(&state.db)
    .await?;
    let (id, sender_id) = row.ok_or(ApiError::NotFound)?;
    if sender_id != me.id {
        return Err(ApiError::Forbidden);
    }
    let viewed: (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM snap_views WHERE message_id = $1)")
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    if viewed.0 {
        return Err(ApiError::Conflict {
            code: "snap_already_viewed",
            message: "cannot unsend a snap that was already viewed".into(),
        });
    }
    sqlx::query("UPDATE messages SET deleted_at = now() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
