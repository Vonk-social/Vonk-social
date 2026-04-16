//! Web Push helper (VAPID).
//!
//! Subscriptions are persisted in `push_subscriptions`. Actual dispatch
//! is done from routes/notification code when we have something to send.
//!
//! For v1 we keep the surface tiny: the routes module just stores and
//! deletes subscriptions. Dispatch is deferred until we add the first
//! notification source (Phase 3.1 — new-DM / new-mention notifications).
//! The `web-push` crate is pinned in `Cargo.toml` so we can move fast
//! when that day arrives without a dependency bump.

use anyhow::{anyhow, Result};

use crate::config::AppConfig;

#[allow(dead_code)]
pub struct Subscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

/// Placeholder — always returns an error until the notification dispatch
/// layer is built (Phase 3.1).
#[allow(dead_code)]
pub async fn send(cfg: &AppConfig, _sub: &Subscription, _payload: &[u8]) -> Result<()> {
    if !cfg.vapid_configured() {
        return Err(anyhow!("vapid_not_configured"));
    }
    Err(anyhow!("push_dispatch_not_implemented"))
}
