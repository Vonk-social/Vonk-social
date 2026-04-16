//! Public cluster endpoints.
//!
//! * `POST /api/cluster/join-request`     — volunteer submits hosting application
//! * `GET  /api/cluster/status`           — public cluster health summary
//! * `POST /api/cluster/heartbeat`        — node-to-node heartbeat (API-key auth)
//! * `POST /api/cluster/replicate`        — receive replication events from another node

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/cluster/join-request", post(submit_join_request))
        .route("/api/cluster/status", get(cluster_status))
        .route("/api/cluster/heartbeat", post(heartbeat))
        .route("/api/cluster/replicate", post(replicate))
}

// ── Join request (public, no auth) ──────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct JoinRequestBody {
    #[validate(length(min = 2, max = 100))]
    pub name: String,
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(max = 1000))]
    pub note: Option<String>,
    #[validate(length(max = 50))]
    pub proposed_region: Option<String>,
    #[validate(length(max = 500))]
    pub proposed_url: Option<String>,
    pub cpu_cores: Option<i32>,
    pub ram_gb: Option<i32>,
    pub disk_gb: Option<i32>,
}

async fn submit_join_request(
    State(state): State<AppState>,
    Json(body): Json<JoinRequestBody>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    body.validate()?;

    // Check if join requests are enabled.
    let allowed: Option<String> = sqlx::query_scalar(
        "SELECT value FROM cluster_config WHERE key = 'allow_join_requests'",
    )
    .fetch_optional(&state.db)
    .await?;
    if allowed.as_deref() != Some("true") {
        return Err(ApiError::bad_request(
            "join_requests_disabled",
            "Node join requests are currently disabled",
        ));
    }

    // Dedupe: don't allow same email to submit twice while pending.
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM node_join_requests \
         WHERE email = $1 AND status = 'pending'",
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await?;
    if existing.is_some() {
        return Err(ApiError::conflict(
            "already_pending",
            "Je hebt al een aanvraag ingediend. We nemen contact op.",
        ));
    }

    let uuid: Uuid = sqlx::query_scalar(
        "INSERT INTO node_join_requests \
            (name, email, note, proposed_region, proposed_url, \
             cpu_cores, ram_gb, disk_gb) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING uuid",
    )
    .bind(&body.name)
    .bind(&body.email)
    .bind(&body.note)
    .bind(&body.proposed_region)
    .bind(&body.proposed_url)
    .bind(body.cpu_cores)
    .bind(body.ram_gb)
    .bind(body.disk_gb)
    .fetch_one(&state.db)
    .await?;

    tracing::info!(email = %body.email, name = %body.name, "new node join request");

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "data": {
                "uuid": uuid,
                "message": "Bedankt! Je aanvraag wordt zo snel mogelijk beoordeeld."
            }
        })),
    ))
}

// ── Cluster status (public) ─────────────────────────────────

#[derive(Debug, Serialize)]
struct ClusterStatus {
    cluster_name: String,
    node_count: i64,
    active_nodes: i64,
    total_users: i64,
    accepting_volunteers: bool,
}

async fn cluster_status(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let cluster_name: String = sqlx::query_scalar(
        "SELECT value FROM cluster_config WHERE key = 'cluster_name'",
    )
    .fetch_optional(&state.db)
    .await?
    .unwrap_or_else(|| "vonk".to_string());

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cluster_nodes")
        .fetch_one(&state.db)
        .await?;
    let active: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM cluster_nodes WHERE status = 'active'")
            .fetch_one(&state.db)
            .await?;
    let users: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL")
            .fetch_one(&state.db)
            .await?;
    let accepting: Option<String> = sqlx::query_scalar(
        "SELECT value FROM cluster_config WHERE key = 'allow_join_requests'",
    )
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "data": ClusterStatus {
            cluster_name,
            node_count: total.0,
            active_nodes: active.0,
            total_users: users.0,
            accepting_volunteers: accepting.as_deref() == Some("true"),
        }
    })))
}

// ── Heartbeat (node-to-node, API-key auth) ──────────────────

#[derive(Debug, Deserialize)]
struct HeartbeatBody {
    cpu_usage: Option<f32>,
    memory_usage: Option<f32>,
    disk_usage: Option<f32>,
    user_count: Option<i32>,
}

async fn heartbeat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<HeartbeatBody>,
) -> ApiResult<StatusCode> {
    // Authenticate via X-Cluster-Key header.
    let api_key = headers
        .get("X-Cluster-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthenticated)?;

    let key_hash = hash_api_key(api_key);

    let result = sqlx::query(
        "UPDATE cluster_nodes SET \
            last_heartbeat = now(), \
            cpu_usage = COALESCE($1, cpu_usage), \
            memory_usage = COALESCE($2, memory_usage), \
            disk_usage = COALESCE($3, disk_usage), \
            user_count = COALESCE($4, user_count), \
            status = CASE WHEN status = 'joining' THEN 'active' ELSE status END \
         WHERE api_key_hash = $5 AND status IN ('active', 'joining', 'syncing')",
    )
    .bind(body.cpu_usage)
    .bind(body.memory_usage)
    .bind(body.disk_usage)
    .bind(body.user_count)
    .bind(&key_hash)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::Unauthenticated);
    }

    Ok(StatusCode::NO_CONTENT)
}

fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(key.as_bytes());
    hex::encode(hash)
}

// ── Replicate (node-to-node, cluster-secret auth) ───────────

#[derive(Debug, Deserialize)]
struct ReplicateBody {
    source_node: Uuid,
    events: Vec<ReplicateEvent>,
}

#[derive(Debug, Deserialize)]
struct ReplicateEvent {
    #[allow(dead_code)]
    id: i64,
    table_name: String,
    row_id: i64,
    operation: String,
    payload: serde_json::Value,
}

async fn replicate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ReplicateBody>,
) -> ApiResult<Json<serde_json::Value>> {
    // Authenticate via cluster secret.
    let provided_key = headers
        .get("X-Cluster-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthenticated)?;

    let cluster_secret: Option<String> = sqlx::query_scalar(
        "SELECT value FROM cluster_config WHERE key = 'cluster_secret'",
    )
    .fetch_optional(&state.db)
    .await?;

    let secret = cluster_secret.unwrap_or_default();
    if secret.is_empty() || provided_key != secret {
        return Err(ApiError::Unauthenticated);
    }

    // Convert to the replication engine's format and apply.
    let events: Vec<crate::cluster::replication::QueuedEvent> = body
        .events
        .into_iter()
        .map(|e| crate::cluster::replication::QueuedEvent {
            id: e.id,
            table_name: e.table_name,
            row_id: e.row_id,
            operation: e.operation,
            payload: e.payload,
        })
        .collect();

    let count = crate::cluster::replication::apply_events(&state.db, events)
        .await
        .map_err(ApiError::Internal)?;

    tracing::info!(
        source_node = %body.source_node,
        applied = count,
        "replication events applied"
    );

    Ok(Json(serde_json::json!({ "applied": count })))
}
