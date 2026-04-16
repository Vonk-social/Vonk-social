//! DM (Direct Messages) — persistent 1-to-1 text chat.
//!
//! Backed by the existing `messages` + `conversations` + `conversation_members`
//! tables. Text content is stored as UTF-8 bytes in the `ciphertext BYTEA`
//! column with `protocol_version = 'plaintext-phase2'`.
//!
//! * `GET    /api/dm/conversations`                — list user's conversations
//! * `POST   /api/dm/conversations`                — start or find existing 1:1
//! * `GET    /api/dm/conversations/{uuid}/messages` — paginated messages
//! * `POST   /api/dm/conversations/{uuid}/messages` — send a text message

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::models::post::PostAuthor;
use crate::state::AppState;
use crate::ws::{WsEvent, WsMessageData};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/dm/conversations", get(list_conversations).post(start_conversation))
        .route(
            "/api/dm/conversations/{uuid}/messages",
            get(list_messages).post(send_message),
        )
}

#[derive(Serialize)]
struct DataEnvelope<T: Serialize> {
    data: T,
}

// ── list conversations ──────────────────────────────────────

#[derive(Serialize)]
struct ConversationListItem {
    uuid: Uuid,
    other_user: PostAuthor,
    last_message: Option<String>,
    last_message_at: Option<DateTime<Utc>>,
    unread_count: i64,
}

async fn list_conversations(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
) -> ApiResult<Json<DataEnvelope<Vec<ConversationListItem>>>> {
    let rows = sqlx::query(
        r#"
        SELECT
            c.uuid AS conversation_uuid,
            other.uuid AS other_uuid,
            other.username AS other_username,
            other.display_name AS other_display_name,
            other.avatar_url AS other_avatar_url,
            last_msg.preview AS last_message,
            last_msg.created_at AS last_message_at,
            COALESCE(unread.cnt, 0)::bigint AS unread_count
        FROM conversations c
        JOIN conversation_members my_cm
            ON my_cm.conversation_id = c.id AND my_cm.user_id = $1
        JOIN conversation_members other_cm
            ON other_cm.conversation_id = c.id AND other_cm.user_id != $1
        JOIN users other
            ON other.id = other_cm.user_id
        LEFT JOIN LATERAL (
            SELECT
                LEFT(convert_from(m.ciphertext, 'UTF8'), 100) AS preview,
                m.created_at
            FROM messages m
            WHERE m.conversation_id = c.id
              AND m.deleted_at IS NULL
              AND m.content_type = 'text'
            ORDER BY m.created_at DESC
            LIMIT 1
        ) last_msg ON true
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::bigint AS cnt
            FROM messages m
            WHERE m.conversation_id = c.id
              AND m.sender_id != $1
              AND m.deleted_at IS NULL
              AND m.content_type = 'text'
              AND m.created_at > COALESCE(my_cm.last_read_at, '1970-01-01'::timestamptz)
        ) unread ON true
        WHERE c.conversation_type = 'direct'
          AND (
              SELECT COUNT(*) FROM conversation_members
               WHERE conversation_id = c.id
          ) = 2
        ORDER BY last_msg.created_at DESC NULLS LAST
        LIMIT 100
        "#,
    )
    .bind(me.id)
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .iter()
        .map(|r| ConversationListItem {
            uuid: r.try_get("conversation_uuid").unwrap_or_else(|_| Uuid::nil()),
            other_user: PostAuthor {
                uuid: r.try_get("other_uuid").unwrap_or_else(|_| Uuid::nil()),
                username: r.try_get("other_username").unwrap_or_default(),
                display_name: r.try_get("other_display_name").unwrap_or_default(),
                avatar_url: r.try_get("other_avatar_url").ok(),
            },
            last_message: r.try_get("last_message").ok().flatten(),
            last_message_at: r.try_get("last_message_at").ok().flatten(),
            unread_count: r.try_get("unread_count").unwrap_or(0),
        })
        .collect();

    Ok(Json(DataEnvelope { data }))
}

// ── start / find conversation ───────────────────────────────

#[derive(Debug, Deserialize)]
struct StartConversationRequest {
    to_username: String,
}

#[derive(Serialize)]
struct StartConversationResponse {
    uuid: Uuid,
}

async fn start_conversation(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Json(req): Json<StartConversationRequest>,
) -> ApiResult<(StatusCode, Json<DataEnvelope<StartConversationResponse>>)> {
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
            "cannot_dm_self",
            "You cannot start a conversation with yourself",
        ));
    }

    // Find existing direct conversation (same logic as snaps.rs).
    let existing_conv: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT c.uuid FROM conversations c
         WHERE c.conversation_type = 'direct'
           AND EXISTS (SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = $1)
           AND EXISTS (SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = $2)
           AND (SELECT COUNT(*) FROM conversation_members WHERE conversation_id = c.id) = 2
         LIMIT 1
        "#,
    )
    .bind(me.id)
    .bind(recipient_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some((uuid,)) = existing_conv {
        return Ok((
            StatusCode::OK,
            Json(DataEnvelope {
                data: StartConversationResponse { uuid },
            }),
        ));
    }

    // Create a new conversation.
    let mut tx = state.db.begin().await?;
    let conv_uuid = Uuid::new_v4();
    let conv_id: i64 = sqlx::query_scalar(
        "INSERT INTO conversations (uuid, conversation_type) VALUES ($1, 'direct') RETURNING id",
    )
    .bind(conv_uuid)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO conversation_members (conversation_id, user_id) \
         VALUES ($1, $2), ($1, $3)",
    )
    .bind(conv_id)
    .bind(me.id)
    .bind(recipient_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(DataEnvelope {
            data: StartConversationResponse { uuid: conv_uuid },
        }),
    ))
}

// ── list messages ───────────────────────────────────────────

#[derive(Deserialize)]
struct ListMessagesQuery {
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct DmMessage {
    uuid: Uuid,
    sender: PostAuthor,
    content: String,
    created_at: DateTime<Utc>,
    is_mine: bool,
}

#[derive(Serialize)]
struct MessageListResponse {
    data: Vec<DmMessage>,
    cursor: Option<String>,
    has_more: bool,
}

async fn list_messages(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(conv_uuid): Path<Uuid>,
    Query(q): Query<ListMessagesQuery>,
) -> ApiResult<Json<MessageListResponse>> {
    // Resolve conversation and verify membership.
    let conv: Option<(i64,)> = sqlx::query_as(
        "SELECT c.id FROM conversations c \
         JOIN conversation_members cm ON cm.conversation_id = c.id AND cm.user_id = $2 \
         WHERE c.uuid = $1 AND c.conversation_type = 'direct'",
    )
    .bind(conv_uuid)
    .bind(me.id)
    .fetch_optional(&state.db)
    .await?;
    let conv_id = conv.ok_or(ApiError::NotFound)?.0;

    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let fetch_limit = limit + 1;

    // Decode cursor (created_at,id).
    let cursor = q.cursor.as_deref().and_then(|s| {
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(s)
            .ok()?;
        let text = String::from_utf8(bytes).ok()?;
        let (ts_str, id_str) = text.split_once(',')?;
        let ts = DateTime::parse_from_rfc3339(ts_str).ok()?.with_timezone(&Utc);
        let id: i64 = id_str.parse().ok()?;
        Some((ts, id))
    });

    let rows = if let Some((cursor_ts, cursor_id)) = cursor {
        sqlx::query(
            r#"
            SELECT m.id, m.uuid, m.sender_id, m.ciphertext, m.created_at,
                   u.uuid AS sender_uuid, u.username, u.display_name, u.avatar_url
              FROM messages m
              JOIN users u ON u.id = m.sender_id
             WHERE m.conversation_id = $1
               AND m.deleted_at IS NULL
               AND m.content_type = 'text'
               AND (m.created_at, m.id) < ($3, $4)
             ORDER BY m.created_at DESC, m.id DESC
             LIMIT $2
            "#,
        )
        .bind(conv_id)
        .bind(fetch_limit)
        .bind(cursor_ts)
        .bind(cursor_id)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT m.id, m.uuid, m.sender_id, m.ciphertext, m.created_at,
                   u.uuid AS sender_uuid, u.username, u.display_name, u.avatar_url
              FROM messages m
              JOIN users u ON u.id = m.sender_id
             WHERE m.conversation_id = $1
               AND m.deleted_at IS NULL
               AND m.content_type = 'text'
             ORDER BY m.created_at DESC, m.id DESC
             LIMIT $2
            "#,
        )
        .bind(conv_id)
        .bind(fetch_limit)
        .fetch_all(&state.db)
        .await?
    };

    let has_more = rows.len() as i64 > limit;
    let shown = if has_more {
        &rows[..limit as usize]
    } else {
        &rows[..]
    };

    let mut last_ts: Option<DateTime<Utc>> = None;
    let mut last_id: Option<i64> = None;

    let data: Vec<DmMessage> = shown
        .iter()
        .map(|r| {
            let id: i64 = r.try_get("id").unwrap_or(0);
            let created_at: DateTime<Utc> =
                r.try_get("created_at").unwrap_or_else(|_| Utc::now());
            last_ts = Some(created_at);
            last_id = Some(id);

            let sender_id: i64 = r.try_get("sender_id").unwrap_or(0);
            let ciphertext: Vec<u8> = r.try_get("ciphertext").unwrap_or_default();
            let content = String::from_utf8_lossy(&ciphertext).to_string();

            DmMessage {
                uuid: r.try_get("uuid").unwrap_or_else(|_| Uuid::nil()),
                sender: PostAuthor {
                    uuid: r.try_get("sender_uuid").unwrap_or_else(|_| Uuid::nil()),
                    username: r.try_get("username").unwrap_or_default(),
                    display_name: r.try_get("display_name").unwrap_or_default(),
                    avatar_url: r.try_get("avatar_url").ok(),
                },
                content,
                created_at,
                is_mine: sender_id == me.id,
            }
        })
        .collect();

    let next_cursor = if has_more {
        use base64::Engine as _;
        last_ts.zip(last_id).map(|(ts, id)| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(format!("{},{}", ts.to_rfc3339(), id))
        })
    } else {
        None
    };

    // Mark messages as read — update last_read_at on the membership row.
    sqlx::query(
        "UPDATE conversation_members SET last_read_at = now() \
         WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conv_id)
    .bind(me.id)
    .execute(&state.db)
    .await?;

    Ok(Json(MessageListResponse {
        data,
        cursor: next_cursor,
        has_more,
    }))
}

// ── send message ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SendMessageRequest {
    content: String,
}

#[derive(Serialize)]
struct SendMessageResponse {
    uuid: Uuid,
    sender: PostAuthor,
    content: String,
    created_at: DateTime<Utc>,
    is_mine: bool,
}

async fn send_message(
    State(state): State<AppState>,
    AuthUser(me): AuthUser,
    Path(conv_uuid): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> ApiResult<(StatusCode, Json<DataEnvelope<SendMessageResponse>>)> {
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Err(ApiError::bad_request(
            "empty_message",
            "Message content cannot be empty",
        ));
    }
    if content.len() > 5000 {
        return Err(ApiError::bad_request(
            "message_too_long",
            "Message content must not exceed 5000 characters",
        ));
    }

    // Resolve conversation and verify membership.
    let conv: Option<(i64,)> = sqlx::query_as(
        "SELECT c.id FROM conversations c \
         JOIN conversation_members cm ON cm.conversation_id = c.id AND cm.user_id = $2 \
         WHERE c.uuid = $1 AND c.conversation_type = 'direct'",
    )
    .bind(conv_uuid)
    .bind(me.id)
    .fetch_optional(&state.db)
    .await?;
    let conv_id = conv.ok_or(ApiError::NotFound)?.0;

    let msg_uuid = Uuid::new_v4();
    let ciphertext_bytes = content.as_bytes().to_vec();

    sqlx::query(
        "INSERT INTO messages \
            (uuid, conversation_id, sender_id, ciphertext, protocol_version, \
             content_type, encryption_version) \
         VALUES ($1, $2, $3, $4, 'plaintext-phase2', 'text', 0)",
    )
    .bind(msg_uuid)
    .bind(conv_id)
    .bind(me.id)
    .bind(&ciphertext_bytes)
    .execute(&state.db)
    .await?;

    // Update sender's last_read_at so their own message doesn't count as
    // unread.
    sqlx::query(
        "UPDATE conversation_members SET last_read_at = now() \
         WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind(conv_id)
    .bind(me.id)
    .execute(&state.db)
    .await?;

    let now = Utc::now();

    // Broadcast to all WebSocket subscribers in this conversation.
    state
        .ws_hub
        .broadcast(
            conv_uuid,
            WsEvent::Message {
                data: WsMessageData {
                    uuid: msg_uuid,
                    sender: PostAuthor {
                        uuid: me.uuid,
                        username: me.username.clone(),
                        display_name: me.display_name.clone(),
                        avatar_url: me.avatar_url.clone(),
                    },
                    content: content.clone(),
                    created_at: now,
                    is_mine: false,
                },
            },
        )
        .await;

    Ok((
        StatusCode::CREATED,
        Json(DataEnvelope {
            data: SendMessageResponse {
                uuid: msg_uuid,
                sender: PostAuthor {
                    uuid: me.uuid,
                    username: me.username.clone(),
                    display_name: me.display_name.clone(),
                    avatar_url: me.avatar_url.clone(),
                },
                content,
                created_at: now,
                is_mine: true,
            },
        }),
    ))
}
