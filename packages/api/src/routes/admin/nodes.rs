//! Admin node management — review join requests, monitor nodes, kick.
//!
//! * `GET    /api/admin/nodes`                  — list all nodes + health
//! * `GET    /api/admin/nodes/requests`         — pending join requests
//! * `POST   /api/admin/nodes/requests/:uuid/approve` — approve + generate API key
//! * `POST   /api/admin/nodes/requests/:uuid/reject`  — reject with reason
//! * `POST   /api/admin/nodes/:id/drain`        — graceful remove
//! * `DELETE /api/admin/nodes/:id`              — force remove (dead node)
//! * `GET    /api/admin/nodes/:id`              — single node detail

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/nodes", get(list_nodes))
        .route("/api/admin/nodes/requests", get(list_requests))
        .route(
            "/api/admin/nodes/requests/{uuid}/approve",
            post(approve_request),
        )
        .route(
            "/api/admin/nodes/requests/{uuid}/reject",
            post(reject_request),
        )
        .route("/api/admin/nodes/{id}", get(get_node))
        .route("/api/admin/nodes/{id}/drain", post(drain_node))
        .route("/api/admin/nodes/{id}", delete(remove_node))
}

/// Guard: only user id=1 (first registered) can access admin routes.
/// A proper role system replaces this in a later phase.
fn require_admin(user: &crate::models::User) -> ApiResult<()> {
    if user.id != 1 {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

// ── List nodes ──────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
struct NodeSummary {
    id: Uuid,
    name: String,
    api_url: String,
    region: Option<String>,
    status: String,
    cpu_usage: Option<f32>,
    memory_usage: Option<f32>,
    disk_usage: Option<f32>,
    user_count: Option<i32>,
    last_heartbeat: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

async fn list_nodes(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    let nodes: Vec<NodeSummary> = sqlx::query_as(
        "SELECT id, name, api_url, region, status, cpu_usage, memory_usage, \
         disk_usage, user_count, last_heartbeat, created_at \
         FROM cluster_nodes ORDER BY created_at",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(serde_json::json!({ "data": nodes })))
}

// ── Single node detail ──────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
struct NodeDetail {
    id: Uuid,
    name: String,
    api_url: String,
    public_url: String,
    region: Option<String>,
    status: String,
    cpu_usage: Option<f32>,
    memory_usage: Option<f32>,
    disk_usage: Option<f32>,
    user_count: Option<i32>,
    admin_email: Option<String>,
    admin_name: Option<String>,
    admin_note: Option<String>,
    last_heartbeat: Option<DateTime<Utc>>,
    approved_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

async fn get_node(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    let node: NodeDetail = sqlx::query_as(
        "SELECT id, name, api_url, public_url, region, status, cpu_usage, \
         memory_usage, disk_usage, user_count, admin_email, admin_name, \
         admin_note, last_heartbeat, approved_at, created_at \
         FROM cluster_nodes WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(serde_json::json!({ "data": node })))
}

// ── List join requests ──────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
struct JoinRequest {
    uuid: Uuid,
    name: String,
    email: String,
    note: Option<String>,
    proposed_region: Option<String>,
    proposed_url: Option<String>,
    cpu_cores: Option<i32>,
    ram_gb: Option<i32>,
    disk_gb: Option<i32>,
    status: String,
    review_note: Option<String>,
    created_at: DateTime<Utc>,
}

async fn list_requests(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    let requests: Vec<JoinRequest> = sqlx::query_as(
        "SELECT uuid, name, email, note, proposed_region, proposed_url, \
         cpu_cores, ram_gb, disk_gb, status, review_note, created_at \
         FROM node_join_requests ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(serde_json::json!({ "data": requests })))
}

// ── Approve request ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApproveBody {
    note: Option<String>,
}

async fn approve_request(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(req_uuid): Path<Uuid>,
    Json(body): Json<ApproveBody>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&user)?;

    // Find the pending request.
    #[derive(sqlx::FromRow)]
    struct PendingReq {
        id: i64,
        name: String,
        proposed_region: Option<String>,
        proposed_url: Option<String>,
    }
    let req: PendingReq = sqlx::query_as(
        "SELECT id, name, proposed_region, proposed_url \
         FROM node_join_requests WHERE uuid = $1 AND status = 'pending'",
    )
    .bind(req_uuid)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        ApiError::bad_request("not_pending", "Request not found or already reviewed")
    })?;
    let (req_id, name, region, proposed_url) =
        (req.id, req.name, req.proposed_region, req.proposed_url);

    // Generate a unique API key for this node.
    let api_key = generate_api_key();
    let api_key_hash = hash_api_key(&api_key);

    let mut tx = state.db.begin().await?;

    // Create the cluster_nodes row.
    let node_id: Uuid = sqlx::query_scalar(
        "INSERT INTO cluster_nodes \
            (name, api_url, public_url, region, status, api_key_hash, \
             admin_email, admin_name, approved_at, approved_by) \
         VALUES ($1, $2, $3, $4, 'joining', $5, \
                 (SELECT email FROM node_join_requests WHERE id = $6), \
                 (SELECT name FROM node_join_requests WHERE id = $6), \
                 now(), $7) \
         RETURNING id",
    )
    .bind(&name)
    .bind(proposed_url.as_deref().unwrap_or("https://pending.vonk.social"))
    .bind("https://vonk.social")
    .bind(region.as_deref())
    .bind(&api_key_hash)
    .bind(req_id)
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;

    // Update the request.
    sqlx::query(
        "UPDATE node_join_requests SET \
            status = 'approved', reviewed_by = $1, reviewed_at = now(), \
            review_note = $2, node_id = $3 \
         WHERE id = $4",
    )
    .bind(user.id)
    .bind(body.note.as_deref())
    .bind(node_id)
    .bind(req_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(node_id = %node_id, name = %name, "node join request approved");

    // Return the API key — this is the ONLY time it's shown in plaintext.
    // The volunteer needs it to configure their node.
    Ok(Json(serde_json::json!({
        "data": {
            "node_id": node_id,
            "api_key": api_key,
            "message": "Node goedgekeurd. Geef deze API key aan de vrijwilliger — hij wordt maar 1x getoond."
        }
    })))
}

// ── Reject request ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RejectBody {
    reason: Option<String>,
}

async fn reject_request(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(req_uuid): Path<Uuid>,
    Json(body): Json<RejectBody>,
) -> ApiResult<StatusCode> {
    require_admin(&user)?;
    let result = sqlx::query(
        "UPDATE node_join_requests SET \
            status = 'rejected', reviewed_by = $1, reviewed_at = now(), \
            review_note = $2 \
         WHERE uuid = $3 AND status = 'pending'",
    )
    .bind(user.id)
    .bind(body.reason.as_deref())
    .bind(req_uuid)
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::bad_request(
            "not_pending",
            "Request not found or already reviewed",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ── Drain node (graceful remove) ────────────────────────────

async fn drain_node(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    require_admin(&user)?;
    sqlx::query(
        "UPDATE cluster_nodes SET status = 'draining' \
         WHERE id = $1 AND status = 'active'",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    tracing::info!(node_id = %id, "node set to draining");
    Ok(StatusCode::NO_CONTENT)
}

// ── Force remove (dead node) ────────────────────────────────

async fn remove_node(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    require_admin(&user)?;
    // Remove placement entries first, then the node.
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM user_placement WHERE node_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM replication_queue WHERE source_node = $1 OR target_node = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM cluster_nodes WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    tracing::warn!(node_id = %id, "node force-removed from cluster");
    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ─────────────────────────────────────────────────

fn generate_api_key() -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    let mut buf = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut buf);
    format!("vnk_{}", URL_SAFE_NO_PAD.encode(buf))
}

fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(key.as_bytes());
    hex::encode(hash)
}
