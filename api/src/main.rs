use anyhow::{Result, Context};
use shared::{Config, get_pool};
use tracing::{info, error, warn};
use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::Query,
    response::Html,
};
use axum_extra::extract::Form;
use tower_http::services::ServeDir;
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use totp_rs::{TOTP, Algorithm};
use askama::Template;

/// TOTP secret state (shared across requests)
#[derive(Clone)]
struct TotpState {
    secret: Arc<RwLock<Option<String>>>,
    totp_instance: Arc<RwLock<Option<TOTP>>>,
}

/// TOTP setup request
#[derive(Deserialize)]
struct TotpSetupRequest {
    secret: String,
}

/// TOTP setup response
#[derive(Serialize)]
struct TotpSetupResponse {
    success: bool,
    message: String,
}

/// TOTP verify request
#[derive(Deserialize)]
struct TotpVerifyRequest {
    code: String,
}

/// TOTP setup page template
#[derive(Template)]
#[template(path = "totp_setup.html")]
struct TotpSetupTemplate {
    qr_code: String,
    secret: String,
    error: String,
    success: String,
}

/// Deploy query parameters
#[derive(Deserialize)]
struct DeployQuery {
    totp: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting WiseTrader API server...");

    let config = Config::from_env()?;
    let _pool = get_pool(&config.database_url).await?;
    info!("Connected to database");

    // Setup static file serving for HTML reports
    let reports_dir = std::path::Path::new(&config.html_reports_dir);
    info!("Serving HTML reports from: {:?}", reports_dir);
    
    // Ensure reports directory exists
    if let Err(e) = std::fs::create_dir_all(reports_dir) {
        error!("Failed to create reports directory: {}", e);
    }

    // Initialize TOTP state
    let totp_state = TotpState {
        secret: Arc::new(RwLock::new(None)),
        totp_instance: Arc::new(RwLock::new(None)),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/subscriptions/status", get(subscription_status))
        .route("/api/deploy/setup", get(show_totp_setup))
        .route("/api/deploy/generate-totp", post(generate_totp))
        .route("/api/deploy/verify-totp", post(verify_totp))
        .route("/api/deploy/bot", get(deploy_bot))
        .nest_service(
            "/reports",
            ServeDir::new(reports_dir)
                .append_index_html_on_directories(false)
                .precompressed_gzip()
                .precompressed_br(),
        )
        .with_state(totp_state);

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

/// Deploy bot response
#[derive(Debug, Serialize)]
struct DeployResponse {
    success: bool,
    message: String,
    output: Option<String>,
    error: Option<String>,
}

/// Show TOTP setup page
async fn show_totp_setup(
    axum::extract::State(totp_state): axum::extract::State<TotpState>,
) -> Html<String> {
    let secret_guard = totp_state.secret.read().await;
    let totp_guard = totp_state.totp_instance.read().await;
    
    let template = if secret_guard.is_some() {
        // Already configured, show success message
        TotpSetupTemplate {
            qr_code: String::new(),
            secret: String::new(),
            error: String::new(),
            success: "TOTP has already been configured. You can use it to deploy the bot.".to_string(),
        }
    } else {
        // Not configured yet
        TotpSetupTemplate {
            qr_code: String::new(),
            secret: String::new(),
            error: String::new(),
            success: String::new(),
        }
    };
    
    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)))
}

/// Generate TOTP secret and QR code
async fn generate_totp(
    axum::extract::State(totp_state): axum::extract::State<TotpState>,
) -> Html<String> {
    info!("Generate TOTP request received");

    let mut secret_guard = totp_state.secret.write().await;
    let mut totp_guard = totp_state.totp_instance.write().await;
    
    if secret_guard.is_some() {
        warn!("TOTP secret already set");
        let template = TotpSetupTemplate {
            qr_code: String::new(),
            secret: String::new(),
            error: "TOTP secret has already been set and cannot be changed".to_string(),
            success: String::new(),
        };
        return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
    }

    // Generate a new secret (32 bytes random)
    let mut secret_bytes = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut secret_bytes);
    let secret_string = base32::encode(base32::Alphabet::RFC4648 { padding: false }, &secret_bytes);
    
    // Create TOTP instance  
    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes.to_vec(),
        Some("WiseTrader Deploy".to_string()),
        "wisetrader".to_string(),
    ) {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to create TOTP: {:?}", e);
            let template = TotpSetupTemplate {
                qr_code: String::new(),
                secret: String::new(),
                error: format!("Failed to create TOTP: {:?}", e),
                success: String::new(),
            };
            return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
        }
    };
    
    // Generate QR code as base64
    let qr_code_base64 = match totp.get_qr() {
        Ok(qr) => qr,
        Err(e) => {
            error!("Failed to generate QR code: {:?}", e);
            let template = TotpSetupTemplate {
                qr_code: String::new(),
                secret: String::new(),
                error: format!("Failed to generate QR code: {:?}", e),
                success: String::new(),
            };
            return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
        }
    };
    use base64::{Engine as _, engine::general_purpose};
    let qr_code_base64_str = general_purpose::STANDARD.encode(&qr_code_base64);
    
    // Store secret and TOTP instance (but don't mark as verified yet)
    *secret_guard = Some(secret_string.clone());
    *totp_guard = Some(totp);
    
    info!("TOTP secret generated successfully");
    
    let template = TotpSetupTemplate {
        qr_code: qr_code_base64_str,
        secret: secret_string,
        error: String::new(),
        success: String::new(),
    };
    
    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)))
}

/// Verify TOTP code and complete setup
async fn verify_totp(
    axum::extract::State(totp_state): axum::extract::State<TotpState>,
    Form(req): Form<TotpVerifyRequest>,
) -> Html<String> {
    info!("Verify TOTP request received");

    let totp_guard = totp_state.totp_instance.read().await;
    
    let totp = match totp_guard.as_ref() {
        Some(t) => t,
        None => {
            let template = TotpSetupTemplate {
                qr_code: String::new(),
                secret: String::new(),
                error: "TOTP not initialized. Please generate a secret first.".to_string(),
                success: String::new(),
            };
            return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
        }
    };
    
    // Verify the code
    let template = match totp.check_current(&req.code) {
        Ok(true) => {
            info!("TOTP code verified successfully");
            TotpSetupTemplate {
                qr_code: String::new(),
                secret: String::new(),
                error: String::new(),
                success: "✅ TOTP verified successfully! You can now use it to deploy the bot.".to_string(),
            }
        }
        Ok(false) => {
            error!("Invalid TOTP code provided");
            TotpSetupTemplate {
                qr_code: String::new(),
                secret: String::new(),
                error: "Invalid TOTP code. Please try again.".to_string(),
                success: String::new(),
            }
        }
        Err(e) => {
            error!("Error verifying TOTP: {}", e);
            TotpSetupTemplate {
                qr_code: String::new(),
                secret: String::new(),
                error: format!("Error verifying code: {}", e),
                success: String::new(),
            }
        }
    };
    
    Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)))
}

/// Validate TOTP code
fn validate_totp(totp: &TOTP, code: &str) -> bool {
    totp.check_current(code).unwrap_or(false)
}

/// Deploy bot endpoint
/// Executes: cd /opt/wisetrader/wisetrader && git pull origin && docker compose up -d bot --build
/// Runs as user 'bichkhe'
/// Requires TOTP code in query parameter: ?totp=123456
async fn deploy_bot(
    axum::extract::State(totp_state): axum::extract::State<TotpState>,
    Query(params): Query<DeployQuery>,
) -> Json<DeployResponse> {
    info!("Deploy bot request received");

    // Check TOTP instance is set
    let totp_guard = totp_state.totp_instance.read().await;
    let totp = match totp_guard.as_ref() {
        Some(t) => t,
        None => {
            error!("TOTP not configured");
            return Json(DeployResponse {
                success: false,
                message: "TOTP not configured. Please set it first via /api/deploy/setup".to_string(),
                output: None,
                error: None,
            });
        }
    };

    // Validate TOTP code
    let totp_code = match params.totp {
        Some(code) => code,
        None => {
            error!("TOTP code not provided");
            return Json(DeployResponse {
                success: false,
                message: "TOTP code is required. Please provide ?totp=123456".to_string(),
                output: None,
                error: None,
            });
        }
    };

    if !validate_totp(totp, &totp_code) {
        error!("Invalid TOTP code provided");
        return Json(DeployResponse {
            success: false,
            message: "Invalid TOTP code".to_string(),
            output: None,
            error: None,
        });
    }
    
    drop(totp_guard);

    info!("TOTP validated successfully, proceeding with deploy");

    let work_dir = "/opt/wisetrader/wisetrader";
    let user = "bichkhe";

    // Execute commands as user bichkhe
    // We'll use a shell script approach to chain commands
    let script = format!(
        r#"
        cd {} || exit 1
        git pull origin || exit 1
        docker compose up -d bot --build || exit 1
        "#,
        work_dir
    );

    // Run commands with sudo -u bichkhe
    let output = Command::new("sudo")
        .arg("-u")
        .arg(user)
        .arg("sh")
        .arg("-c")
        .arg(&script)
        .current_dir("/")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                info!("Deploy bot completed successfully");
                info!("Output: {}", stdout);
                
                Json(DeployResponse {
                    success: true,
                    message: "Bot deployed successfully".to_string(),
                    output: Some(stdout.to_string()),
                    error: if stderr.is_empty() {
                        None
                    } else {
                        warn!("Deploy warnings: {}", stderr);
                        Some(stderr.to_string())
                    },
                })
            } else {
                error!("Deploy bot failed with status: {:?}", output.status);
                error!("Stdout: {}", stdout);
                error!("Stderr: {}", stderr);
                
                Json(DeployResponse {
                    success: false,
                    message: format!("Deploy failed with exit code: {:?}", output.status.code()),
                    output: Some(stdout.to_string()),
                    error: Some(stderr.to_string()),
                })
            }
        }
        Err(e) => {
            error!("Failed to execute deploy command: {}", e);
            
            Json(DeployResponse {
                success: false,
                message: format!("Failed to execute deploy command: {}", e),
                output: None,
                error: Some(e.to_string()),
            })
        }
    }
}


