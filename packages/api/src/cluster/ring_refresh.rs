//! Periodically refresh the cluster hash ring from the database.

use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::RwLock;

use super::ring::{HashRing, RingNode};

/// Rebuild the ring every 30 seconds from active cluster_nodes.
pub fn spawn(db: PgPool, ring: Arc<RwLock<HashRing>>, replication_factor: usize) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            match refresh(&db, &ring, replication_factor).await {
                Ok(n) => tracing::debug!(nodes = n, "ring refreshed"),
                Err(e) => tracing::warn!(error = %e, "ring refresh failed"),
            }
        }
    });
}

#[derive(sqlx::FromRow)]
struct NodeRow {
    id: uuid::Uuid,
    name: String,
    api_url: String,
}

async fn refresh(
    db: &PgPool,
    ring: &Arc<RwLock<HashRing>>,
    replication_factor: usize,
) -> Result<usize, sqlx::Error> {
    let rows: Vec<NodeRow> = sqlx::query_as(
        "SELECT id, name, api_url FROM cluster_nodes WHERE status = 'active'",
    )
    .fetch_all(db)
    .await?;

    let nodes: Vec<RingNode> = rows
        .into_iter()
        .map(|r| RingNode {
            id: r.id,
            name: r.name,
            api_url: r.api_url,
        })
        .collect();

    let count = nodes.len();
    let new_ring = HashRing::new(nodes, replication_factor);

    let mut guard = ring.write().await;
    *guard = new_ring;

    Ok(count)
}
