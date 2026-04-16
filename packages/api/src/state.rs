//! Shared application state passed to every handler via [`axum::extract::State`].

use std::sync::Arc;

use redis::aio::ConnectionManager;
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::cluster::ring::HashRing;
use crate::config::AppConfig;

/// Cloneable handle holding all long-lived resources.
///
/// `PgPool`, `ConnectionManager`, `aws_sdk_s3::Client` and `reqwest::Client`
/// are all cheap to clone (internally `Arc`-backed).
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub s3: aws_sdk_s3::Client,
    pub http: reqwest::Client,
    pub config: Arc<AppConfig>,

    // ── Cluster (Phase 3.5) ─────────────────────────────────
    /// This node's UUID in cluster_nodes. None when running standalone.
    pub self_node_id: Option<Uuid>,
    /// The current hash ring. Refreshed every 30s by ring_refresh task.
    pub cluster_ring: Option<Arc<RwLock<HashRing>>>,
}
