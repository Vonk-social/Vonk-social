-- ============================================================
-- VONK — Phase 3.5: Cluster foundation (RAID5-model)
-- ============================================================
-- Distributed Vonk: multiple nodes form one platform. Each user
-- is replicated across N nodes (replication factor). If a node
-- dies, data survives on the remaining replicas.
--
-- This migration adds:
--   - cluster_nodes: registry of all nodes in the cluster
--   - node_join_requests: pending applications from volunteers
--   - user_placement: which users live on which nodes
--   - cluster_config: cluster-wide settings (replication factor etc.)
-- ============================================================

-- ── Cluster-wide configuration ──────────────────────────────
CREATE TABLE cluster_config (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Defaults: replication factor 2, cluster name
INSERT INTO cluster_config (key, value) VALUES
    ('replication_factor', '2'),
    ('cluster_name', 'vonk-main'),
    ('cluster_secret', ''),          -- set by admin on first boot
    ('allow_join_requests', 'true');

-- ── Cluster nodes ───────────────────────────────────────────
CREATE TABLE cluster_nodes (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name            TEXT NOT NULL,
    -- Internal API URL (node-to-node communication, not public)
    api_url         TEXT NOT NULL,
    -- Public-facing URL (what end-users see, typically vonk.social for all)
    public_url      TEXT NOT NULL,
    -- Geographic hint for smart placement
    region          TEXT,
    -- Node lifecycle
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'joining', 'syncing', 'active', 'draining', 'dead')),
    -- Capacity info (updated by heartbeat)
    cpu_usage       REAL,
    memory_usage    REAL,
    disk_usage      REAL,
    user_count      INT DEFAULT 0,
    -- Auth: each node has a unique API key for cluster communication
    api_key_hash    TEXT NOT NULL,
    -- Contact info for the volunteer running this node
    admin_email     TEXT,
    admin_name      TEXT,
    admin_note      TEXT,            -- "Why I want to host a Vonk node"
    -- Timestamps
    last_heartbeat  TIMESTAMPTZ,
    approved_at     TIMESTAMPTZ,
    approved_by     BIGINT REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_cluster_nodes_status ON cluster_nodes (status);
CREATE UNIQUE INDEX uq_cluster_nodes_api_url ON cluster_nodes (api_url);

-- ── Join requests (before a node is approved) ───────────────
-- Volunteers submit a request via vonk.social/host. Admin reviews
-- and either approves (→ node moves to 'joining') or rejects
-- (→ row gets rejected_at set).
CREATE TABLE node_join_requests (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT uuid_generate_v4() UNIQUE,
    -- The volunteer
    name            TEXT NOT NULL,
    email           TEXT NOT NULL,
    note            TEXT,            -- "Hi, ik wil een node hosten omdat..."
    -- Technical details (filled by the volunteer)
    proposed_region TEXT,
    proposed_url    TEXT,            -- https://my-vonk-node.example.com
    -- Server specs (self-reported)
    cpu_cores       INT,
    ram_gb          INT,
    disk_gb         INT,
    -- Review
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'approved', 'rejected')),
    reviewed_by     BIGINT REFERENCES users(id),
    reviewed_at     TIMESTAMPTZ,
    review_note     TEXT,            -- "Welkom!" or "Onvoldoende specs"
    -- If approved, the generated node_id and API key
    node_id         UUID REFERENCES cluster_nodes(id),
    -- Timestamps
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_join_requests_status ON node_join_requests (status);

-- ── User placement (consistent hash ring materialised) ──────
-- Maps each user to the nodes that hold their data. The primary
-- node handles writes; replicas serve reads and take over on
-- primary failure.
CREATE TABLE user_placement (
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    node_id     UUID NOT NULL REFERENCES cluster_nodes(id) ON DELETE CASCADE,
    is_primary  BOOLEAN NOT NULL DEFAULT false,
    -- Replication state
    sync_status TEXT NOT NULL DEFAULT 'pending'
                CHECK (sync_status IN ('pending', 'syncing', 'synced', 'stale')),
    last_synced TIMESTAMPTZ,
    PRIMARY KEY (user_id, node_id)
);

CREATE INDEX idx_placement_node ON user_placement (node_id);
CREATE INDEX idx_placement_primary ON user_placement (user_id) WHERE is_primary;

-- ── Replication queue (cross-node data sync) ────────────────
-- When a write happens on the primary node, it queues a replication
-- event for each replica. A background worker on each node processes
-- incoming events.
CREATE TABLE replication_queue (
    id          BIGSERIAL PRIMARY KEY,
    source_node UUID NOT NULL REFERENCES cluster_nodes(id),
    target_node UUID NOT NULL REFERENCES cluster_nodes(id),
    -- What changed
    table_name  TEXT NOT NULL,
    row_id      BIGINT NOT NULL,
    operation   TEXT NOT NULL CHECK (operation IN ('insert', 'update', 'delete')),
    payload     JSONB NOT NULL,
    -- Delivery tracking
    attempts    INT NOT NULL DEFAULT 0,
    next_retry  TIMESTAMPTZ,
    delivered   BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_repl_queue_pending ON replication_queue (target_node, created_at)
    WHERE NOT delivered;
