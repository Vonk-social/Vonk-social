//! Database bootstrap helpers.
//!
//! Query logic lives in the route modules; this file only owns pool
//! construction so `main.rs` stays thin.

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

/// Connect to Postgres with sensible pool defaults and run pending migrations.
pub async fn connect_and_migrate(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Some(Duration::from_secs(600)))
        .connect(database_url)
        .await
        .with_context(|| "connecting to Postgres")?;

    sqlx::migrate!("../db/migrations")
        .run(&pool)
        .await
        .with_context(|| "running database migrations")?;

    Ok(pool)
}
