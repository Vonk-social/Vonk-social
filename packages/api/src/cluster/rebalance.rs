//! Node rebalancing — redistribute user data when nodes join/leave/crash.
//!
//! When the hash ring changes (node added, removed, or marked dead), some
//! users' placement sets shift. This module detects those changes and
//! triggers data migration:
//!
//! - **Node join**: users that now belong to the new node need their data
//!   synced from an existing replica.
//! - **Node drain/leave**: users that were on the leaving node need their
//!   data re-replicated to another node.
//! - **Node death**: same as drain but triggered by heartbeat timeout.
//!
//! Rebalancing is idempotent — running it twice produces no extra work.

use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::ring::{HashRing, RingNode};

/// Check for placement changes and update the `user_placement` table.
/// Called after the ring is refreshed (every 30s) or on-demand after
/// an admin action (approve/drain/remove node).
pub async fn rebalance(
    db: &PgPool,
    ring: &Arc<RwLock<HashRing>>,
    _self_node_id: &Uuid,
) -> anyhow::Result<RebalanceResult> {
    let ring = ring.read().await;

    if !ring.is_healthy() {
        return Ok(RebalanceResult::default());
    }

    // Get all user IDs.
    let user_ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM users WHERE deleted_at IS NULL")
        .fetch_all(db)
        .await?;

    let mut added = 0u64;
    let mut removed = 0u64;

    for &uid in &user_ids {
        let target_nodes: Vec<&RingNode> = ring.placement(uid);
        let target_set: HashSet<Uuid> = target_nodes.iter().map(|n| n.id).collect();

        // Current placements from DB.
        let current: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT node_id FROM user_placement WHERE user_id = $1",
        )
        .bind(uid)
        .fetch_all(db)
        .await?;
        let current_set: HashSet<Uuid> = current.iter().map(|r| r.0).collect();

        // Nodes that should have this user but don't yet.
        for &nid in &target_set {
            if !current_set.contains(&nid) {
                let is_primary = target_nodes.first().map(|n| n.id) == Some(nid);
                sqlx::query(
                    "INSERT INTO user_placement (user_id, node_id, is_primary, sync_status) \
                     VALUES ($1, $2, $3, 'pending') \
                     ON CONFLICT (user_id, node_id) DO UPDATE SET \
                         is_primary = EXCLUDED.is_primary, \
                         sync_status = CASE \
                             WHEN user_placement.sync_status = 'synced' THEN 'synced' \
                             ELSE 'pending' END",
                )
                .bind(uid)
                .bind(nid)
                .bind(is_primary)
                .execute(db)
                .await?;
                added += 1;
            }
        }

        // Nodes that have this user but shouldn't anymore.
        for &(nid,) in &current {
            if !target_set.contains(&nid) {
                sqlx::query(
                    "DELETE FROM user_placement WHERE user_id = $1 AND node_id = $2",
                )
                .bind(uid)
                .bind(nid)
                .execute(db)
                .await?;
                removed += 1;
            }
        }

        // Update primary flag if it changed.
        if let Some(primary) = target_nodes.first() {
            sqlx::query(
                "UPDATE user_placement SET is_primary = (node_id = $2) \
                 WHERE user_id = $1",
            )
            .bind(uid)
            .bind(primary.id)
            .execute(db)
            .await?;
        }
    }

    if added > 0 || removed > 0 {
        tracing::info!(
            added,
            removed,
            users = user_ids.len(),
            "rebalance complete"
        );
    }

    Ok(RebalanceResult { added, removed })
}

#[derive(Debug, Default)]
pub struct RebalanceResult {
    pub added: u64,
    pub removed: u64,
}

/// Spawn a background task that rebalances every 60 seconds.
pub fn spawn(db: PgPool, ring: Arc<RwLock<HashRing>>, self_node_id: Uuid) {
    tokio::spawn(async move {
        // Wait 30s before first rebalance to let the ring populate.
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            match rebalance(&db, &ring, &self_node_id).await {
                Ok(r) if r.added > 0 || r.removed > 0 => {
                    tracing::info!(added = r.added, removed = r.removed, "rebalanced");
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "rebalance failed"),
            }
        }
    });
}
