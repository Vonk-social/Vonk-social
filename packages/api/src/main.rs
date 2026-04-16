//! Vonk API — entrypoint.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    response::Json,
    routing::get,
    Router,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod activitypub;
mod auth;
mod cluster;
mod config;
mod db;
mod email;
mod error;
mod feed;
mod jobs;
mod media;
mod models;
mod push;
mod routes;
mod s3;
mod state;
mod ws;

use crate::config::AppConfig;
use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    google_oauth_configured: bool,
    github_oauth_configured: bool,
    apple_oauth_configured: bool,
    smtp_configured: bool,
    vapid_configured: bool,
    cluster_enabled: bool,
    cluster_node_id: Option<String>,
}

async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        google_oauth_configured: state.config.google_configured(),
        github_oauth_configured: state.config.github_configured(),
        apple_oauth_configured: state.config.apple_configured(),
        smtp_configured: state.config.smtp_configured(),
        vapid_configured: state.config.vapid_configured(),
        cluster_enabled: state.config.cluster_enabled,
        cluster_node_id: state.self_node_id.map(|id| id.to_string()),
    })
}

#[derive(Serialize)]
struct OpenFinancesResponse {
    message: &'static str,
    url: &'static str,
}

async fn open_finances() -> Json<OpenFinancesResponse> {
    Json(OpenFinancesResponse {
        message: "Vonk's volledige boekhouding is publiek. Coming soon.",
        url: "https://vonk.social/open",
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vonk_api=info,tower_http=info".into()),
        )
        .init();

    let cfg = Arc::new(AppConfig::from_env()?);
    tracing::info!(
        env = ?cfg.environment,
        api = %cfg.api_url,
        web = %cfg.web_url,
        "loaded configuration"
    );

    // Database + migrations.
    let db = db::connect_and_migrate(&cfg.database_url).await?;
    tracing::info!("database connected, migrations up-to-date");

    // Background: null out `sessions.ip_hash` older than 48h every 15 min
    // (CLAUDE.md §9).
    jobs::ip_sweep::spawn(db.clone());

    // Background: cluster heartbeat — mark dead nodes after 90s silence.
    cluster::heartbeat::spawn(db.clone());

    // Valkey (Redis-compatible).
    let redis_client = redis::Client::open(cfg.redis_url.as_str())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;
    tracing::info!("valkey connected");

    // MinIO / S3.
    let s3_client = s3::build_client(&cfg).await;
    ensure_bucket(&s3_client, &cfg.s3_bucket).await;
    tracing::info!(bucket = %cfg.s3_bucket, "object storage ready");

    // Shared HTTP client (Google userinfo, token exchange).
    let http = reqwest::Client::builder()
        .user_agent(format!("vonk-api/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // ── Cluster setup (Phase 3.5) ─────────────────────────────
    let (self_node_id, cluster_ring) = if cfg.cluster_enabled {
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let node_id_str = cfg.cluster_node_id.as_deref().unwrap_or("");
        let node_id: Option<uuid::Uuid> = node_id_str.parse().ok();

        if let Some(nid) = node_id {
            tracing::info!(node_id = %nid, "cluster mode enabled");

            // Build initial ring from active nodes.
            let ring = cluster::ring::HashRing::new(Vec::new(), 2);
            let ring = Arc::new(RwLock::new(ring));

            // Spawn ring refresh (rebuilds every 30s).
            cluster::ring_refresh::spawn(db.clone(), ring.clone(), 2);

            // Spawn replication worker.
            cluster::replication::spawn_worker(db.clone(), nid);

            // Spawn rebalancer (checks every 60s for placement drift).
            cluster::rebalance::spawn(db.clone(), ring.clone(), nid);

            (Some(nid), Some(ring))
        } else {
            tracing::warn!("CLUSTER_ENABLED=true but CLUSTER_NODE_ID is missing or invalid");
            (None, None)
        }
    } else {
        (None, None)
    };

    let ws_hub = ws::WsHub::new();

    let state = AppState {
        db,
        redis,
        s3: s3_client,
        http,
        config: cfg.clone(),
        self_node_id,
        cluster_ring,
        ws_hub,
    };

    // CORS: reflect the request Origin only if it's in the allowed list.
    // This supports multi-domain (vonk.social + vonk.openview.be) without
    // the wildcard (which is incompatible with `allow_credentials`).
    let allowed: std::collections::HashSet<HeaderValue> = cfg
        .cors_origins
        .iter()
        .filter_map(|o| o.parse::<HeaderValue>().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(
            move |origin: &HeaderValue, _parts: &axum::http::request::Parts| {
                allowed.contains(origin)
            },
        ))
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION]);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/open/summary", get(open_finances))
        .merge(routes::auth::router())
        .merge(routes::users::router())
        .merge(routes::media::router())
        .merge(routes::posts::router())
        .merge(routes::feed::router())
        .merge(routes::follows::router())
        .merge(routes::snaps::router())
        .merge(routes::invites::router())
        .merge(routes::push::router())
        .merge(routes::dm::router())
        .merge(ws::router())
        .merge(routes::cluster::router())
        .merge(routes::admin::nodes::router())
        // ActivityPub federation (outside /api/ prefix — standard paths)
        .merge(routes::webfinger::router())
        .merge(routes::nodeinfo::router())
        .merge(routes::activitypub::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Port is configurable via API_PORT; default 8080 for local dev. The
    // staging box has something else on 8080 so we bind 3501 there.
    let port: u16 = std::env::var("API_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Vonk API listening on {}", addr);

    // `into_make_service_with_connect_info` so handlers can extract client IP.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// Idempotently create the avatar/media bucket on startup.
/// Running against MinIO, this makes the dev experience "just work" after a
/// fresh `docker compose down -v`.
async fn ensure_bucket(s3: &aws_sdk_s3::Client, bucket: &str) {
    match s3.head_bucket().bucket(bucket).send().await {
        Ok(_) => {}
        Err(_) => {
            if let Err(e) = s3.create_bucket().bucket(bucket).send().await {
                tracing::warn!(error = %e, bucket, "could not create bucket (may already exist)");
            } else {
                tracing::info!(bucket, "created S3 bucket");
            }
        }
    }
}
