//! Consistent hash ring for user → node placement.
//!
//! Uses rendezvous hashing (highest random weight): for a given user_id,
//! each active node gets a deterministic score. The N nodes with the
//! highest scores become the placement set (N = replication factor).
//!
//! Benefits over classic consistent hashing:
//! - Adding/removing a node only moves ~1/N of keys (minimal disruption)
//! - No virtual nodes needed — simple and predictable
//! - Easy to compute: hash(user_id, node_id) → sort → top N

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A node in the cluster ring.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RingNode {
    pub id: Uuid,
    pub name: String,
    pub api_url: String,
}

/// The hash ring — holds all active nodes and computes placements.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Methods used incrementally as cluster features land.
pub struct HashRing {
    nodes: Vec<RingNode>,
    replication_factor: usize,
}

#[allow(dead_code)]
impl HashRing {
    pub fn new(nodes: Vec<RingNode>, replication_factor: usize) -> Self {
        Self {
            nodes,
            replication_factor,
        }
    }

    /// Return true if the ring has enough nodes for the replication factor.
    pub fn is_healthy(&self) -> bool {
        self.nodes.len() >= self.replication_factor
    }

    /// Compute the placement for a user: returns the N nodes (sorted by
    /// score descending) that should hold this user's data. The first
    /// node in the list is the primary.
    pub fn placement(&self, user_id: i64) -> Vec<&RingNode> {
        if self.nodes.is_empty() {
            return Vec::new();
        }

        let n = self.replication_factor.min(self.nodes.len());
        let mut scored: Vec<(u64, &RingNode)> = self
            .nodes
            .iter()
            .map(|node| (Self::score(user_id, &node.id), node))
            .collect();

        // Sort descending by score.
        scored.sort_by(|a, b| b.0.cmp(&a.0));

        scored.into_iter().take(n).map(|(_, node)| node).collect()
    }

    /// Which node is the primary for this user?
    pub fn primary(&self, user_id: i64) -> Option<&RingNode> {
        self.placement(user_id).into_iter().next()
    }

    /// Is the given node_id in the placement set for this user?
    pub fn is_replica(&self, user_id: i64, node_id: &Uuid) -> bool {
        self.placement(user_id)
            .iter()
            .any(|n| &n.id == node_id)
    }

    /// Compute users that need to move when the node set changes.
    /// Returns (user_id, old_nodes, new_nodes) for each user whose
    /// placement changed.
    pub fn diff(
        old: &HashRing,
        new: &HashRing,
        user_ids: &[i64],
    ) -> Vec<PlacementChange> {
        user_ids
            .iter()
            .filter_map(|&uid| {
                let old_set: Vec<Uuid> =
                    old.placement(uid).iter().map(|n| n.id).collect();
                let new_set: Vec<Uuid> =
                    new.placement(uid).iter().map(|n| n.id).collect();
                if old_set != new_set {
                    Some(PlacementChange {
                        user_id: uid,
                        old_nodes: old_set,
                        new_nodes: new_set,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Rendezvous hash: SHA256(user_id || node_id) → u64.
    fn score(user_id: i64, node_id: &Uuid) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(user_id.to_le_bytes());
        hasher.update(node_id.as_bytes());
        let hash = hasher.finalize();
        // Take the first 8 bytes as a u64.
        u64::from_le_bytes(hash[..8].try_into().unwrap())
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PlacementChange {
    pub user_id: i64,
    pub old_nodes: Vec<Uuid>,
    pub new_nodes: Vec<Uuid>,
}
