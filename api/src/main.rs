use anyhow::Result;
use shared::{Config, get_pool};
use tracing::{info, error};
use axum::{
    routing::get,
    Router,
    Json,
};
use tower_http::services::ServeDir;
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

    // Setup static file serving for HTML reports
    let reports_dir = std::path::Path::new(&config.html_reports_dir);
    info!("Serving HTML reports from: {:?}", reports_dir);
    
    // Ensure reports directory exists
    if let Err(e) = std::fs::create_dir_all(reports_dir) {
        error!("Failed to create reports directory: {}", e);
    }

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/subscriptions/status", get(subscription_status))
        .nest_service(
            "/reports",
            ServeDir::new(reports_dir)
                .append_index_html_on_directories(false)
                .precompressed_gzip()
                .precompressed_br(),
        )
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9999").await?;
    info!("API server listening on http://0.0.0.0:9999");
    info!("HTML reports available at: http://localhost:9999/reports/");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn subscription_status() -> Json<Value> {
    Json(json!({ "message": "Subscription status endpoint (placeholder)" }))
}


