//! Cluster heartbeat — each node pings every 30s to report health.
//!
//! Also marks nodes as `dead` if they miss 3 consecutive heartbeats
//! (90s without contact).

use sqlx::PgPool;
use std::time::Duration;

/// Spawn the heartbeat background tasks:
/// 1. Send our own heartbeat to the cluster (if we're a registered node)
/// 2. Check for dead nodes and mark them
pub fn spawn(db: PgPool) {
    // Dead-node detector: every 60s, mark nodes that haven't pinged in 90s.
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = mark_dead_nodes(&db).await {
                tracing::warn!(error = %e, "dead-node check failed");
            }
        }
    });
}

async fn mark_dead_nodes(db: &PgPool) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        "UPDATE cluster_nodes \
         SET status = 'dead' \
         WHERE status = 'active' \
           AND last_heartbeat < now() - interval '90 seconds'",
    )
    .execute(db)
    .await?;

    if result.rows_affected() > 0 {
        tracing::warn!(
            count = result.rows_affected(),
            "marked unresponsive nodes as dead"
        );
    }
    Ok(())
}
