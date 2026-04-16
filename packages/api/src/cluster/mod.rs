//! Cluster management: node registry, heartbeat, placement, replication.
//!
//! Phase 3.5 — distributed Vonk with RAID5-style data replication.
//! Each user is placed on N nodes (replication factor). Nodes communicate
//! via internal HTTP APIs authenticated with per-node API keys.

pub mod heartbeat;
pub mod proxy;
pub mod rebalance;
pub mod replication;
pub mod ring;
pub mod ring_refresh;
