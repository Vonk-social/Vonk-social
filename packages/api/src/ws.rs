//! Real-time WebSocket hub for DM chat.
//!
//! Provides live message delivery, typing indicators, and online status.
//!
//! ## Endpoint
//!
//! `GET /api/ws` — upgrades to WebSocket. Auth via `vonk_access` cookie.
//!
//! ## Client → Server messages (JSON)
//!
//! * `{"type": "join", "conversation": "<uuid>"}` — subscribe to a conversation
//! * `{"type": "typing", "conversation": "<uuid>"}` — I'm typing
//! * `{"type": "stop_typing", "conversation": "<uuid>"}` — I stopped typing
//!
//! ## Server → Client messages (JSON)
//!
//! * `{"type": "message", "data": {uuid, sender, content, created_at, is_mine}}`
//! * `{"type": "typing", "user": "<username>"}`
//! * `{"type": "stop_typing", "user": "<username>"}`
//! * `{"type": "online", "users": ["username1", ...]}`
//!
//! ## Nginx note
//!
//! The `/api/ws` path must be proxied with WebSocket upgrade headers:
//! ```nginx
//! location /api/ws {
//!     proxy_pass http://127.0.0.1:8080;
//!     proxy_http_version 1.1;
//!     proxy_set_header Upgrade $http_upgrade;
//!     proxy_set_header Connection "upgrade";
//!     proxy_set_header Host $host;
//!     proxy_read_timeout 3600s;
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::HeaderMap,
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::auth::cookies::ACCESS_COOKIE;
use crate::auth::jwt;
use crate::models::post::PostAuthor;
use crate::state::AppState;

// ── Hub types ───────────────────────────────────────────────

/// A message broadcast through a conversation channel.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    /// New chat message.
    Message { data: WsMessageData },
    /// Someone is typing.
    Typing { user: String },
    /// Someone stopped typing.
    StopTyping { user: String },
    /// Online users in the conversation.
    Online { users: Vec<String> },
}

#[derive(Debug, Clone, Serialize)]
pub struct WsMessageData {
    pub uuid: Uuid,
    pub sender: PostAuthor,
    pub content: String,
    pub created_at: DateTime<Utc>,
    /// Always `false` for broadcast — the client checks sender UUID against
    /// its own UUID to determine `is_mine`.
    pub is_mine: bool,
}

/// Inbound JSON from clients.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Join { conversation: Uuid },
    Typing { conversation: Uuid },
    StopTyping { conversation: Uuid },
}

/// Per-conversation broadcast channel.
struct ConversationChannel {
    tx: broadcast::Sender<WsEvent>,
}

/// Info about a connected user.
struct ConnectedUser {
    username: String,
    /// The conversation they're currently viewing (if any).
    active_conversation: Option<Uuid>,
}

/// Central hub managing conversation channels and connected users.
pub struct WsHub {
    /// Per-conversation broadcast channels. Lazy-created on first join.
    conversations: RwLock<HashMap<Uuid, ConversationChannel>>,
    /// Connected users keyed by internal user id.
    connected: RwLock<HashMap<i64, ConnectedUser>>,
}

impl WsHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            conversations: RwLock::new(HashMap::new()),
            connected: RwLock::new(HashMap::new()),
        })
    }

    /// Get or create the broadcast sender for a conversation.
    async fn get_or_create_tx(
        &self,
        conv_uuid: Uuid,
    ) -> broadcast::Sender<WsEvent> {
        // Fast path: read lock.
        {
            let map = self.conversations.read().await;
            if let Some(ch) = map.get(&conv_uuid) {
                return ch.tx.clone();
            }
        }
        // Slow path: write lock to insert.
        let mut map = self.conversations.write().await;
        // Double-check after acquiring write lock.
        if let Some(ch) = map.get(&conv_uuid) {
            return ch.tx.clone();
        }
        let (tx, _rx) = broadcast::channel::<WsEvent>(256);
        map.insert(conv_uuid, ConversationChannel { tx: tx.clone() });
        tx
    }

    /// Broadcast a `WsEvent` to all subscribers of a conversation.
    pub async fn broadcast(&self, conv_uuid: Uuid, event: WsEvent) {
        let map = self.conversations.read().await;
        if let Some(ch) = map.get(&conv_uuid) {
            // Ignore send errors — just means no active receivers.
            let _ = ch.tx.send(event);
        }
    }

    /// Register a user as connected.
    async fn user_connected(&self, user_id: i64, username: String) {
        let mut map = self.connected.write().await;
        map.insert(
            user_id,
            ConnectedUser {
                username,
                active_conversation: None,
            },
        );
    }

    /// Mark a user as viewing a specific conversation and return list of
    /// online usernames in that conversation.
    async fn user_joined_conversation(
        &self,
        user_id: i64,
        conv_uuid: Uuid,
    ) -> Vec<String> {
        let mut map = self.connected.write().await;
        if let Some(cu) = map.get_mut(&user_id) {
            cu.active_conversation = Some(conv_uuid);
        }
        // Collect online usernames for this conversation.
        map.values()
            .filter(|cu| cu.active_conversation == Some(conv_uuid))
            .map(|cu| cu.username.clone())
            .collect()
    }

    /// Remove a user from the connected set.
    async fn user_disconnected(&self, user_id: i64) -> Option<Uuid> {
        let mut map = self.connected.write().await;
        let cu = map.remove(&user_id)?;
        cu.active_conversation
    }

    /// Get online usernames for a conversation.
    async fn online_users(&self, conv_uuid: Uuid) -> Vec<String> {
        let map = self.connected.read().await;
        map.values()
            .filter(|cu| cu.active_conversation == Some(conv_uuid))
            .map(|cu| cu.username.clone())
            .collect()
    }
}

// ── Router ──────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new().route("/api/ws", get(ws_upgrade))
}

/// Upgrade handler — authenticates via the `vonk_access` cookie, then
/// switches to the WebSocket protocol.
async fn ws_upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Extract JWT from cookie header.
    let jar = axum_extra::extract::cookie::CookieJar::from_headers(&headers);
    let token = match jar.get(ACCESS_COOKIE) {
        Some(c) => c.value().to_string(),
        None => {
            return axum::http::Response::builder()
                .status(401)
                .body(axum::body::Body::from("missing auth cookie"))
                .unwrap()
                .into_response();
        }
    };

    let claims = match jwt::verify(&token, &state.config) {
        Ok(c) => c,
        Err(_) => {
            return axum::http::Response::builder()
                .status(401)
                .body(axum::body::Body::from("invalid token"))
                .unwrap()
                .into_response();
        }
    };

    let user_id = match claims.user_id() {
        Some(id) => id,
        None => {
            return axum::http::Response::builder()
                .status(401)
                .body(axum::body::Body::from("invalid token claims"))
                .unwrap()
                .into_response();
        }
    };

    let username = claims.username.clone();

    ws.on_upgrade(move |socket| handle_ws(socket, state, user_id, username))
        .into_response()
}

/// Handle a single WebSocket connection.
async fn handle_ws(
    mut socket: WebSocket,
    state: AppState,
    user_id: i64,
    username: String,
) {
    let hub = &state.ws_hub;
    hub.user_connected(user_id, username.clone()).await;

    // Current subscription receiver.
    let mut current_conv: Option<Uuid> = None;
    let mut rx: Option<broadcast::Receiver<WsEvent>> = None;

    // Typing auto-clear: track the last typing timestamp and auto-send
    // stop_typing after 5 seconds of silence.
    let typing_timeout = tokio::time::Duration::from_secs(5);
    let mut typing_deadline: Option<tokio::time::Instant> = None;

    loop {
        let sleep_fut = async {
            match typing_deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending::<()>().await,
            }
        };

        let rx_fut = async {
            match rx.as_mut() {
                Some(r) => r.recv().await,
                None => std::future::pending().await,
            }
        };

        tokio::select! {
            // Inbound message from client.
            msg = socket.recv() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    _ => break, // Connection closed or errored.
                };

                match msg {
                    Message::Text(text) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) {
                            match client_msg {
                                ClientMsg::Join { conversation } => {
                                    // Subscribe to conversation channel.
                                    let tx = hub.get_or_create_tx(conversation).await;
                                    rx = Some(tx.subscribe());
                                    current_conv = Some(conversation);

                                    let online = hub.user_joined_conversation(user_id, conversation).await;
                                    // Notify the conversation about online users.
                                    hub.broadcast(conversation, WsEvent::Online { users: online }).await;
                                }
                                ClientMsg::Typing { conversation } => {
                                    hub.broadcast(conversation, WsEvent::Typing { user: username.clone() }).await;
                                    typing_deadline = Some(tokio::time::Instant::now() + typing_timeout);
                                }
                                ClientMsg::StopTyping { conversation } => {
                                    hub.broadcast(conversation, WsEvent::StopTyping { user: username.clone() }).await;
                                    typing_deadline = None;
                                }
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {} // Ignore binary, ping, pong.
                }
            }

            // Broadcast message from the hub.
            event = rx_fut => {
                match event {
                    Ok(ev) => {
                        if let Ok(json) = serde_json::to_string(&ev) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Missed messages — continue listening.
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Channel was dropped.
                        rx = None;
                    }
                }
            }

            // Typing auto-clear timeout.
            _ = sleep_fut => {
                if let Some(conv) = current_conv {
                    hub.broadcast(conv, WsEvent::StopTyping { user: username.clone() }).await;
                }
                typing_deadline = None;
            }
        }
    }

    // Cleanup: remove from connected and notify remaining users.
    let left_conv = hub.user_disconnected(user_id).await;
    if let Some(conv) = left_conv {
        let online = hub.online_users(conv).await;
        hub.broadcast(conv, WsEvent::Online { users: online }).await;
    }
}
