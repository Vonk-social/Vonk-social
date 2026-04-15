//! Background task: null out `sessions.ip_hash` after 48 hours.
//!
//! CLAUDE.md §9 requires that IP addresses be deleted after 48h. We never
//! store the raw IP (only a salted hash, see `auth/ip.rs`), but even the
//! hash is identifying enough to count as IP-linked data, so we expire it
//! on the same schedule.
//!
//! Runs every 15 minutes. Deliberately simple — a single UPDATE per tick;
//! the index on `sessions(created_at)` makes it cheap even with many rows.

use std::time::Duration;

use sqlx::PgPool;
use tokio::time::{interval, MissedTickBehavior};

const TICK_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let mut ticker = interval(TICK_INTERVAL);
        // If the API was paused / sleeping, don't try to catch up with a
        // burst of runs — one catch-up is plenty.
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            match sweep(&pool).await {
                Ok(n) if n > 0 => tracing::info!(rows = n, "ip_sweep: nulled old ip_hash values"),
                Ok(_) => tracing::debug!("ip_sweep: nothing to clean"),
                Err(e) => tracing::warn!(error = %e, "ip_sweep failed"),
            }
        }
    });
}

async fn sweep(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sessions \
            SET ip_hash = NULL \
          WHERE ip_hash IS NOT NULL \
            AND created_at < now() - interval '48 hours'",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
