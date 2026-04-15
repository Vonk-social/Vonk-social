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

mod auth;
mod config;
mod db;
mod error;
mod models;
mod routes;
mod s3;
mod state;

use crate::config::AppConfig;
use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    google_oauth_configured: bool,
}

async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        google_oauth_configured: state.config.google_configured(),
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

    let state = AppState {
        db,
        redis,
        s3: s3_client,
        http,
        config: cfg.clone(),
    };

    // CORS: lock to the configured WEB_URL, allow credentials for cookies.
    let cors = CorsLayer::new()
        .allow_origin(
            cfg.web_url
                .parse::<HeaderValue>()
                .map_err(|e| anyhow::anyhow!("invalid WEB_URL: {e}"))?,
        )
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION]);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/open/summary", get(open_finances))
        .merge(routes::auth::router())
        .merge(routes::users::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
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
