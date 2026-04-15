use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::Serialize;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod config;
mod db;
mod routes;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[derive(Serialize)]
struct OpenFinancesResponse {
    message: String,
    url: String,
}

async fn open_finances() -> Json<OpenFinancesResponse> {
    Json(OpenFinancesResponse {
        message: "Vonk's volledige boekhouding is publiek. Coming soon.".into(),
        url: "https://vonk.social/open".into(),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vonk_api=info,tower_http=info".into()),
        )
        .init();

    // Database
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let db = sqlx::PgPool::connect(&database_url).await?;
    sqlx::migrate!("../db/migrations").run(&db).await?;
    
    tracing::info!("Database connected and migrated");

    let state = AppState { db };

    // Router
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/open/summary", get(open_finances))
        // TODO: Add routes
        // .merge(routes::auth::router())
        // .merge(routes::users::router())
        // .merge(routes::posts::router())
        // .merge(routes::feed::router())
        .layer(CorsLayer::permissive()) // Tighten for production
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("Vonk API listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
