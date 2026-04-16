//! Request routing: decide whether to handle locally or proxy to another node.

use crate::state::AppState;

/// Where a request should be handled.
#[allow(dead_code)]
pub enum Routing {
    /// Handle on this node.
    Local,
    /// Forward to another node's internal API URL.
    Proxy(String),
}

/// Determine if a user-scoped request should be handled locally or proxied.
///
/// Returns `Local` when:
/// - Cluster is not enabled (standalone mode)
/// - This node is in the placement set for the user
///
/// Returns `Proxy(url)` when another node is the primary for this user.
#[allow(dead_code)]
pub async fn route_for_user(state: &AppState, user_id: i64) -> Routing {
    let (self_id, ring_lock) = match (&state.self_node_id, &state.cluster_ring) {
        (Some(id), Some(ring)) => (id, ring),
        _ => return Routing::Local, // standalone mode
    };

    let ring = ring_lock.read().await;

    if !ring.is_healthy() {
        // Not enough nodes for the replication factor — handle locally
        // to avoid routing failures.
        return Routing::Local;
    }

    // Check if we're in the placement set.
    if ring.is_replica(user_id, self_id) {
        return Routing::Local;
    }

    // We're not a replica for this user — find the primary and proxy.
    match ring.primary(user_id) {
        Some(node) if node.id != *self_id => Routing::Proxy(node.api_url.clone()),
        _ => Routing::Local, // fallback: handle locally
    }
}
