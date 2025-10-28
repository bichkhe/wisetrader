use anyhow::Result;
use shared::{Config, get_pool};
use tracing::{info};
use axum::{
    routing::get,
    Router,
    Json,
};

use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting WiseTrader API server...");

    let config = Config::from_env()?;
    let pool = get_pool(&config.database_url).await?;
    info!("Connected to database");

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/subscriptions/status", get(subscription_status))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("API server listening on http://0.0.0.0:3000");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn subscription_status() -> Json<Value> {
    Json(json!({ "message": "Subscription status endpoint (placeholder)" }))
}

