//! Replication engine — queues and processes data sync between nodes.
//!
//! Write path:
//!   1. Local write happens (insert/update/delete on a user-scoped table)
//!   2. `queue_replication()` inserts a row per replica-node into
//!      `replication_queue` with the full payload as JSONB.
//!   3. The background worker (`spawn_worker`) polls the queue and POSTs
//!      each event to the target node's `/api/cluster/replicate` endpoint.
//!
//! Receive path:
//!   - `POST /api/cluster/replicate` (X-Cluster-Key auth) receives the
//!     payload and applies it to the local database. Idempotent via
//!     upsert (ON CONFLICT DO UPDATE).

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use super::ring::HashRing;

/// A single replication event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationEvent {
    pub table_name: String,
    pub row_id: i64,
    pub operation: String, // "insert" | "update" | "delete"
    pub payload: serde_json::Value,
}

/// Queue a write for replication to all replica nodes (excluding self).
#[allow(dead_code)] // Called from write-path hooks once cluster mode is active.
pub async fn queue_replication(
    db: &PgPool,
    ring: &HashRing,
    self_node_id: &Uuid,
    user_id: i64,
    event: &ReplicationEvent,
) -> Result<(), sqlx::Error> {
    let replicas = ring.placement(user_id);
    for node in replicas {
        if &node.id == self_node_id {
            continue; // don't replicate to ourselves
        }
        sqlx::query(
            "INSERT INTO replication_queue \
                (source_node, target_node, table_name, row_id, operation, payload) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(self_node_id)
        .bind(node.id)
        .bind(&event.table_name)
        .bind(event.row_id)
        .bind(&event.operation)
        .bind(&event.payload)
        .execute(db)
        .await?;
    }
    Ok(())
}

/// Background worker that processes the replication queue.
/// Polls every 2 seconds, sends pending events to target nodes.
#[allow(dead_code)] // Spawned from main.rs once cluster mode is active.
pub fn spawn_worker(db: PgPool, self_node_id: Uuid) {
    tokio::spawn(async move {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            if let Err(e) = process_batch(&db, &http, &self_node_id).await {
                tracing::warn!(error = %e, "replication worker error");
            }
        }
    });
}

/// Payload sent to the target node.
#[derive(Debug, Serialize)]
struct ReplicateRequest {
    source_node: Uuid,
    events: Vec<QueuedEvent>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct QueuedEvent {
    pub id: i64,
    pub table_name: String,
    pub row_id: i64,
    pub operation: String,
    pub payload: serde_json::Value,
}

/// Target node info.
#[derive(Debug, sqlx::FromRow)]
struct TargetNode {
    id: Uuid,
    api_url: String,
    #[allow(dead_code)]
    api_key_hash: String,
}

async fn process_batch(db: &PgPool, http: &reqwest::Client, self_node_id: &Uuid) -> anyhow::Result<()> {
    // Get distinct target nodes with pending events.
    let targets: Vec<TargetNode> = sqlx::query_as(
        "SELECT DISTINCT cn.id, cn.api_url, cn.api_key_hash \
         FROM replication_queue rq \
         JOIN cluster_nodes cn ON cn.id = rq.target_node \
         WHERE rq.source_node = $1 \
           AND NOT rq.delivered \
           AND (rq.next_retry IS NULL OR rq.next_retry <= now()) \
           AND cn.status IN ('active', 'syncing') \
         LIMIT 10",
    )
    .bind(self_node_id)
    .fetch_all(db)
    .await?;

    for target in targets {
        // Fetch up to 100 events for this target.
        let events: Vec<QueuedEvent> = sqlx::query_as(
            "SELECT id, table_name, row_id, operation, payload \
             FROM replication_queue \
             WHERE source_node = $1 AND target_node = $2 \
               AND NOT delivered \
               AND (next_retry IS NULL OR next_retry <= now()) \
             ORDER BY id \
             LIMIT 100",
        )
        .bind(self_node_id)
        .bind(target.id)
        .fetch_all(db)
        .await?;

        if events.is_empty() {
            continue;
        }

        let event_ids: Vec<i64> = events.iter().map(|e| e.id).collect();

        let req = ReplicateRequest {
            source_node: *self_node_id,
            events,
        };

        // POST to the target node.
        // We need the actual API key, not the hash. Since we only store
        // hashes, the target authenticates US via our own cluster key
        // which they know. For now we use a shared cluster secret from
        // cluster_config.
        let cluster_secret: Option<String> = sqlx::query_scalar(
            "SELECT value FROM cluster_config WHERE key = 'cluster_secret'",
        )
        .fetch_optional(db)
        .await?;

        let secret = cluster_secret.unwrap_or_default();
        if secret.is_empty() {
            tracing::debug!("cluster_secret not set, skipping replication");
            return Ok(());
        }

        let url = format!("{}/api/cluster/replicate", target.api_url.trim_end_matches('/'));

        match http
            .post(&url)
            .header("X-Cluster-Key", &secret)
            .json(&req)
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => {
                // Mark delivered.
                sqlx::query(
                    "UPDATE replication_queue SET delivered = true \
                     WHERE id = ANY($1)",
                )
                .bind(&event_ids)
                .execute(db)
                .await?;
                tracing::debug!(
                    target_node = %target.id,
                    count = event_ids.len(),
                    "replicated events"
                );
            }
            Ok(res) => {
                let status = res.status();
                tracing::warn!(
                    target_node = %target.id,
                    status = %status,
                    "replication delivery failed"
                );
                // Exponential backoff: retry in attempts^2 * 10 seconds.
                sqlx::query(
                    "UPDATE replication_queue SET \
                        attempts = attempts + 1, \
                        next_retry = now() + (power(attempts + 1, 2) * interval '10 seconds') \
                     WHERE id = ANY($1)",
                )
                .bind(&event_ids)
                .execute(db)
                .await?;
            }
            Err(e) => {
                tracing::warn!(
                    target_node = %target.id,
                    error = %e,
                    "replication delivery error"
                );
                sqlx::query(
                    "UPDATE replication_queue SET \
                        attempts = attempts + 1, \
                        next_retry = now() + (power(attempts + 1, 2) * interval '10 seconds') \
                     WHERE id = ANY($1)",
                )
                .bind(&event_ids)
                .execute(db)
                .await?;
            }
        }
    }

    // Cleanup: remove delivered events older than 24h.
    sqlx::query(
        "DELETE FROM replication_queue \
         WHERE delivered AND created_at < now() - interval '24 hours'",
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Incoming replication: apply events from another node.
/// Called from the `/api/cluster/replicate` HTTP endpoint.
pub async fn apply_events(
    db: &PgPool,
    events: Vec<QueuedEvent>,
) -> anyhow::Result<usize> {
    let mut applied = 0;

    for event in &events {
        match event.operation.as_str() {
            "insert" | "update" => {
                // Generic upsert: we use the payload's columns to build
                // an INSERT ... ON CONFLICT (id) DO UPDATE. For Phase 3.5
                // MVP we support the core user-scoped tables.
                match apply_upsert(db, &event.table_name, event.row_id, &event.payload).await {
                    Ok(()) => applied += 1,
                    Err(e) => {
                        tracing::warn!(
                            table = %event.table_name,
                            row_id = event.row_id,
                            error = %e,
                            "replication apply failed"
                        );
                    }
                }
            }
            "delete" => {
                let query = format!(
                    "DELETE FROM {} WHERE id = $1",
                    sanitize_table_name(&event.table_name)
                );
                sqlx::query(&query)
                    .bind(event.row_id)
                    .execute(db)
                    .await?;
                applied += 1;
            }
            op => {
                tracing::warn!(operation = op, "unknown replication operation");
            }
        }
    }

    Ok(applied)
}

/// Apply an upsert for a replicated row. The payload is a JSON object
/// with column → value pairs.
async fn apply_upsert(
    db: &PgPool,
    table: &str,
    row_id: i64,
    payload: &serde_json::Value,
) -> anyhow::Result<()> {
    let table = sanitize_table_name(table);
    let obj = payload
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("payload is not an object"))?;

    if obj.is_empty() {
        return Ok(());
    }

    // Build: INSERT INTO table (id, col1, col2, ...) VALUES ($1, $2, $3, ...)
    // ON CONFLICT (id) DO UPDATE SET col1=EXCLUDED.col1, col2=EXCLUDED.col2, ...
    let mut cols = vec!["id".to_string()];
    let mut placeholders = vec!["$1".to_string()];
    let mut updates = Vec::new();
    let mut values: Vec<serde_json::Value> = vec![serde_json::Value::Number(row_id.into())];
    let mut idx = 2;

    for (col, val) in obj {
        if col == "id" {
            continue;
        }
        let safe_col = sanitize_column_name(col);
        cols.push(safe_col.clone());
        placeholders.push(format!("${idx}"));
        updates.push(format!("{safe_col} = EXCLUDED.{safe_col}"));
        values.push(val.clone());
        idx += 1;
    }

    let sql = format!(
        "INSERT INTO {table} ({cols}) VALUES ({placeholders}) \
         ON CONFLICT (id) DO UPDATE SET {updates}",
        cols = cols.join(", "),
        placeholders = placeholders.join(", "),
        updates = updates.join(", "),
    );

    // We use raw sqlx::query with JSON binds. For the MVP we bind
    // everything as JSONB and let Postgres coerce. A production version
    // would use typed binds per column, but this works for replication
    // where the source already validated the types.
    let mut query = sqlx::query(&sql);
    for val in &values {
        query = query.bind(val);
    }
    query.execute(db).await?;

    Ok(())
}

/// Only allow known table names to prevent SQL injection.
fn sanitize_table_name(name: &str) -> String {
    const ALLOWED: &[&str] = &[
        "users",
        "posts",
        "media",
        "likes",
        "bookmarks",
        "follows",
        "blocks",
        "messages",
        "conversations",
        "conversation_members",
        "user_auth_providers",
        "sessions",
        "email_invites",
        "push_subscriptions",
        "snap_views",
        "story_views",
        "notifications",
    ];
    if ALLOWED.contains(&name) {
        name.to_string()
    } else {
        // Safety: if an unknown table comes through, use a known-safe default
        // that will cause the upsert to fail gracefully.
        tracing::warn!(table = name, "unknown table in replication, skipping");
        "__unknown__".to_string()
    }
}

/// Basic column name sanitization — only allow alphanumeric + underscore.
fn sanitize_column_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}
