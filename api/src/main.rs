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
use totp_rs::{TOTP, Algorithm, Secret};
use askama::Template;
use base32;
use rand::Rng;

/// TOTP secret state (shared across requests)
#[derive(Clone)]
struct TotpState {
    secret: Arc<RwLock<Option<String>>>,
    totp_instance: Arc<RwLock<Option<TOTP>>>,
}

/// Generate a random TOTP secret (base32 encoded)
fn generate_random_totp_secret() -> String {
    // Generate 20 random bytes (160 bits, standard TOTP secret length)
    let mut rng = rand::thread_rng();
    let mut secret_bytes = vec![0u8; 20];
    rng.fill(&mut secret_bytes[..]);
    
    // Encode as base32
    base32::encode(base32::Alphabet::RFC4648 { padding: false }, &secret_bytes)
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

/// TOTP setup fragment template (for HTMX partial updates)
#[derive(Template)]
#[template(path = "totp_setup_fragment.html")]
struct TotpSetupFragment {
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

    // Initialize TOTP state and load from environment variable or generate random fallback
    let totp_state = TotpState {
        secret: Arc::new(RwLock::new(None)),
        totp_instance: Arc::new(RwLock::new(None)),
    };

    // Try to load TOTP secret from environment variable
    let secret_str = if let Ok(env_secret) = std::env::var("TOTP_SECRET") {
        let env_secret = env_secret.trim();
        if !env_secret.is_empty() {
            info!("Loading TOTP secret from TOTP_SECRET environment variable");
            env_secret.to_string()
        } else {
            // Empty env var, generate random fallback
            warn!("TOTP_SECRET is empty, generating random secret as fallback");
            let random_secret = generate_random_totp_secret();
            info!("Generated random TOTP secret: {}", random_secret);
            random_secret
        }
    } else {
        // No env var, generate random fallback
        warn!("TOTP_SECRET environment variable not set, generating random secret as fallback");
        let random_secret = generate_random_totp_secret();
        info!("Generated random TOTP secret: {}", random_secret);
        random_secret
    };
    
    // Parse secret string (base32 encoded)
    let secret_bytes = match base32::decode(base32::Alphabet::RFC4648 { padding: false }, &secret_str) {
        Some(bytes) => bytes,
        None => {
            error!("Failed to decode TOTP secret from base32");
            return Err(anyhow::anyhow!("Invalid TOTP_SECRET format (must be base32)"));
        }
    };
    
    match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("WiseTrader Deploy".to_string()),
        "wisetrader".to_string(),
    ) {
        Ok(totp) => {
            let mut secret_guard = totp_state.secret.write().await;
            let mut totp_guard = totp_state.totp_instance.write().await;
            *secret_guard = Some(secret_str.clone());
            *totp_guard = Some(totp);
            info!("✅ TOTP secret loaded successfully (from env var or generated fallback)");
        }
        Err(e) => {
            error!("Failed to create TOTP from secret: {:?}", e);
            return Err(anyhow::anyhow!("Invalid TOTP configuration"));
        }
    }

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/subscriptions/status", get(subscription_status))
        // .route("/api/deploy/setup", get(show_totp_setup))
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
    
    // Check if TOTP is already configured
    let is_configured = secret_guard.is_some() && totp_guard.is_some();
    let has_env_var = std::env::var("TOTP_SECRET").is_ok();
    
    let template = if is_configured {
        // Already configured
        let secret_display = secret_guard.as_ref().map(|s| s.clone()).unwrap_or_default();
        let source_msg = if has_env_var {
            "✅ TOTP has been configured via TOTP_SECRET environment variable."
        } else {
            "✅ TOTP is using a randomly generated secret (fallback mode)."
        };
        
        TotpSetupTemplate {
            qr_code: String::new(),
            secret: secret_display,
            error: String::new(),
            success: format!("{} Current secret: {}\n\nYou can use this secret to deploy the bot. To make it persistent, add TOTP_SECRET={} to your docker-compose.yml", 
                source_msg, 
                secret_guard.as_ref().unwrap_or(&String::new()),
                secret_guard.as_ref().unwrap_or(&String::new())
            ),
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
    
    // Check if secret already exists in memory
    if secret_guard.is_some() {
        warn!("TOTP secret already set in memory");
        let template = TotpSetupFragment {
            qr_code: String::new(),
            secret: String::new(),
            error: "TOTP secret has already been set and cannot be changed. Please use the existing secret.".to_string(),
            success: String::new(),
        };
        return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
    }

    // Get TOTP_SECRET from environment variable
    let secret_string = match std::env::var("TOTP_SECRET") {
        Ok(env_secret) => {
            let trimmed = env_secret.trim();
            if trimmed.is_empty() {
                error!("TOTP_SECRET environment variable is empty");
                let template = TotpSetupFragment {
                    qr_code: String::new(),
                    secret: String::new(),
                    error: "TOTP_SECRET environment variable is set but empty. Please set a valid base32-encoded secret.".to_string(),
                    success: String::new(),
                };
                return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
            }
            info!("Using TOTP_SECRET from environment variable");
            trimmed.to_string()
        }
        Err(_) => {
            error!("TOTP_SECRET environment variable is not set");
            let template = TotpSetupFragment {
                qr_code: String::new(),
                secret: String::new(),
                error: "TOTP_SECRET environment variable is not set. Please set it in docker-compose.yml before generating TOTP.".to_string(),
                success: String::new(),
            };
            return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
        }
    };
    
    // Convert secret string to bytes for TOTP::new
    let secret_bytes = match base32::decode(base32::Alphabet::RFC4648 { padding: false }, &secret_string) {
        Some(bytes) => bytes,
        None => {
            error!("Failed to decode TOTP_SECRET from base32");
            let template = TotpSetupFragment {
                qr_code: String::new(),
                secret: String::new(),
                error: "Failed to decode TOTP_SECRET. Please ensure it's a valid base32-encoded string.".to_string(),
                success: String::new(),
            };
            return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
        }
    };
    
    // Create TOTP instance
    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("WiseTrader Deploy".to_string()),
        "wisetrader".to_string(),
    ) {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to create TOTP: {:?}", e);
            let template = TotpSetupFragment {
                qr_code: String::new(),
                secret: String::new(),
                error: format!("Failed to create TOTP: {:?}", e),
                success: String::new(),
            };
            return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
        }
    };
    
    // Generate QR code as base64
    let qr_code_base64_str = match totp.get_qr_base64() {
        Ok(qr) => qr,
        Err(e) => {
            error!("Failed to generate QR code: {:?}", e);
            let template = TotpSetupFragment {
                qr_code: String::new(),
                secret: String::new(),
                error: format!("Failed to generate QR code: {:?}", e),
                success: String::new(),
            };
            return Html(template.render().unwrap_or_else(|e| format!("Template error: {}", e)));
        }
    };
    
    // Store secret and TOTP instance in memory
    *secret_guard = Some(secret_string.clone());
    *totp_guard = Some(totp.clone());
    
    info!("TOTP secret loaded from environment variable successfully");
    
    // Show success message
    let success_msg = format!(
        "✅ TOTP secret loaded from TOTP_SECRET environment variable!\n\nSecret: {}\n\nYou can now use this secret to set up your authenticator app.",
        secret_string
    );
    
    let template = TotpSetupFragment {
        qr_code: qr_code_base64_str,
        secret: secret_string.clone(),
        error: String::new(),
        success: success_msg,
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
            let template = TotpSetupFragment {
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
            TotpSetupFragment {
                qr_code: String::new(),
                secret: String::new(),
                error: String::new(),
                success: "✅ TOTP verified successfully! You can now use it to deploy the bot.".to_string(),
            }
        }
        Ok(false) => {
            error!("Invalid TOTP code provided");
            TotpSetupFragment {
                qr_code: String::new(),
                secret: String::new(),
                error: "Invalid TOTP code. Please try again.".to_string(),
                success: String::new(),
            }
        }
        Err(e) => {
            error!("Error verifying TOTP: {}", e);
            TotpSetupFragment {
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

    // Get work directory from environment variable or use default
    let work_dir = std::env::var("DEPLOY_WORK_DIR")
        .unwrap_or_else(|_| "/opt/wisetrader/wisetrader".to_string());

    info!("Using work directory: {}", work_dir);

    // Check if work directory exists (it should be mounted as volume)
    if !std::path::Path::new(&work_dir).exists() {
        error!("Work directory does not exist: {}", work_dir);
        return Json(DeployResponse {
            success: false,
            message: format!("Work directory does not exist: {}. Make sure it's mounted as a volume.", work_dir),
            output: None,
            error: Some(format!("Directory {} not found. Check docker-compose.yml volumes configuration.", work_dir)),
        });
    }

    // Check if docker-compose.yml exists in work directory
    let compose_file = format!("{}/docker-compose.yml", work_dir);
    if !std::path::Path::new(&compose_file).exists() {
        error!("docker-compose.yml not found in: {}", work_dir);
        return Json(DeployResponse {
            success: false,
            message: format!("docker-compose.yml not found in {}", work_dir),
            output: None,
            error: Some(format!("File {} not found", compose_file)),
        });
    }

    // Check if custom deploy.sh script exists
    let deploy_script = format!("{}/deploy.sh", work_dir);
    let use_custom_script = std::path::Path::new(&deploy_script).exists();
    
    if use_custom_script {
        info!("Found custom deploy script: {}", deploy_script);
        
        // Get host user UID
        let get_uid_script = format!(
            r#"stat -c '%u' {}/.git 2>/dev/null || stat -c '%u' {} 2>/dev/null || id -u bichkhe 2>/dev/null || echo '1000'"#,
            work_dir, work_dir
        );
        let uid_output = Command::new("sh")
            .arg("-c")
            .arg(&get_uid_script)
            .output()
            .await;
        
        let bichkhe_uid = match uid_output {
            Ok(output) => {
                let uid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                uid_str.parse::<u32>().unwrap_or(1000)
            }
            Err(_) => {
                warn!("Failed to get UID, using default 1000");
                1000
            }
        };
        
        info!("Running deploy.sh as host user (UID: {})", bichkhe_uid);
        
        // Run deploy.sh with host user permissions
        let hash_uid_str = format!("#{}", bichkhe_uid);
        let run_script = format!(
            r#"EXISTING_USER=$(getent passwd {} | cut -d: -f1 2>/dev/null || echo "")
if [ -n "$EXISTING_USER" ]; then
    sudo -u "$EXISTING_USER" bash {}
elif command -v runuser >/dev/null 2>&1; then
    runuser -u "{}" -- bash {} 2>/dev/null
else
    sudo -u "{}" bash {} 2>/dev/null || sudo -u {} bash {} 2>/dev/null || bash {}
fi"#,
            bichkhe_uid, deploy_script,
            hash_uid_str, deploy_script,
            hash_uid_str, deploy_script,
            bichkhe_uid, deploy_script,
            deploy_script
        );
        
        let deploy_output = Command::new("sh")
            .arg("-c")
            .arg(&run_script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        
        let mut all_stdout = String::new();
        let mut all_stderr = String::new();
        
        match deploy_output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                all_stdout.push_str(&format!("=== Deploy Script Output ===\n{}\n", stdout));
                if !stderr.is_empty() {
                    all_stderr.push_str(&format!("=== Deploy Script (stderr) ===\n{}\n", stderr));
                }
                
                if !output.status.success() {
                    error!("Deploy script failed with exit code: {:?}", output.status.code());
                    return Json(DeployResponse {
                        success: false,
                        message: format!("Deploy script failed with exit code: {:?}. Error: {}", 
                            output.status.code(),
                            if stderr.is_empty() { &stdout } else { &stderr }
                        ),
                        output: Some(all_stdout),
                        error: Some(all_stderr),
                    });
                }
                
                info!("Deploy script completed successfully");
                return Json(DeployResponse {
                    success: true,
                    message: "Bot deployed successfully using deploy.sh".to_string(),
                    output: Some(all_stdout),
                    error: if all_stderr.is_empty() { None } else { Some(all_stderr) },
                });
            }
            Err(e) => {
                error!("Failed to execute deploy script: {}", e);
                return Json(DeployResponse {
                    success: false,
                    message: format!("Failed to execute deploy script: {}. Make sure user 'bichkhe' exists and appuser has sudo permissions.", e),
                    output: Some(all_stdout),
                    error: Some(format!("Deploy script error: {}\n{}", e, all_stderr)),
                });
            }
        }
    }
    
    // Default deployment logic (if deploy.sh doesn't exist)
    info!("Using default deployment logic (deploy.sh not found)");
    
    // Get host user UID for git operations
    let get_uid_script = format!(
        r#"stat -c '%u' {}/.git 2>/dev/null || stat -c '%u' {} 2>/dev/null || id -u bichkhe 2>/dev/null || echo '1000'"#,
        work_dir, work_dir
    );
    let uid_output = Command::new("sh")
        .arg("-c")
        .arg(&get_uid_script)
        .output()
        .await;
    
    let bichkhe_uid = match uid_output {
        Ok(output) => {
            let uid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            uid_str.parse::<u32>().unwrap_or(1000)
        }
        Err(_) => {
            warn!("Failed to get UID, using default 1000");
            1000
        }
    };
    
    info!("Detected UID {} for git operations", bichkhe_uid);
    
    let mut all_stdout = String::new();
    let mut all_stderr = String::new();
    
    // Step 1: Git pull as host user - run directly with sudo, no script needed
    info!("Executing git pull as host user in {}", work_dir);
    
    // Create script in /tmp (writable location) instead of work_dir
    let script_content = format!(
        r#"#!/bin/bash
cd {}
git config --local --add safe.directory {} 2>/dev/null || git config --global --add safe.directory {} 2>/dev/null || true
git pull origin
"#,
        work_dir, work_dir, work_dir
    );
    
    let script_path = format!("/tmp/git_pull_{}.sh", std::process::id());
    if let Err(e) = std::fs::write(&script_path, script_content) {
        error!("Failed to create git pull script in /tmp: {}", e);
        return Json(DeployResponse {
            success: false,
            message: format!("Failed to create git pull script: {}", e),
            output: None,
            error: Some(format!("Script creation error: {}", e)),
        });
    }
    
    // Make script executable
    if let Err(e) = std::fs::metadata(&script_path).and_then(|m| {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = m.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms)
    }) {
        warn!("Failed to set script permissions: {}", e);
    }
    
    // Run git pull script with host user
    let hash_uid_str = format!("#{}", bichkhe_uid);
    let git_script = format!(
        r#"EXISTING_USER=$(getent passwd {} | cut -d: -f1 2>/dev/null || echo "")
if [ -n "$EXISTING_USER" ]; then
    sudo -u "$EXISTING_USER" bash {}
elif command -v runuser >/dev/null 2>&1; then
    runuser -u "{}" -- bash {} 2>/dev/null
else
    sudo -u "{}" bash {} 2>/dev/null || sudo -u {} bash {} 2>/dev/null || bash {}
fi"#,
        bichkhe_uid, script_path,
        hash_uid_str, script_path,
        hash_uid_str, script_path,
        bichkhe_uid, script_path,
        script_path
    );
    
    let git_output = Command::new("sh")
        .arg("-c")
        .arg(&git_script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;
    
    // Clean up temporary script
    let _ = std::fs::remove_file(&script_path);
    
    match git_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            all_stdout.push_str(&format!("=== Git Pull ===\n{}\n", stdout));
            if !stderr.is_empty() {
                all_stderr.push_str(&format!("=== Git Pull (stderr) ===\n{}\n", stderr));
            }
            if !output.status.success() {
                error!("Git pull failed with exit code: {:?}", output.status.code());
                return Json(DeployResponse {
                    success: false,
                    message: format!("Git pull failed with exit code: {:?}. Error: {}", 
                        output.status.code(),
                        if stderr.is_empty() { &stdout } else { &stderr }
                    ),
                    output: Some(all_stdout),
                    error: Some(all_stderr),
                });
            }
            info!("Git pull completed successfully");
        }
        Err(e) => {
            error!("Failed to execute git pull: {}", e);
            return Json(DeployResponse {
                success: false,
                message: format!("Failed to execute git pull: {}", e),
                output: Some(all_stdout),
                error: Some(format!("Git pull error: {}\n{}", e, all_stderr)),
            });
        }
    }
    
    // Step 2: Docker compose up
    // Use docker compose from the mounted directory
    // The docker socket is mounted, so we can control host Docker daemon
    info!("Executing docker compose up -d bot --build in {}", work_dir);
    let docker_output = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(&compose_file)
        .arg("--project-directory")
        .arg(&work_dir)
        .arg("up")
        .arg("-d")
        .arg("bot")
        .arg("--build")
        .current_dir(&work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    let output = match docker_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            all_stdout.push_str(&format!("\n=== Docker Compose ===\n{}\n", stdout));
            if !stderr.is_empty() {
                all_stderr.push_str(&format!("\n=== Docker Compose (stderr) ===\n{}\n", stderr));
            }
            output
        }
        Err(e) => {
            error!("Failed to execute docker compose: {}", e);
            return Json(DeployResponse {
                success: false,
                message: format!("Failed to execute docker compose: {}", e),
                output: Some(all_stdout),
                error: Some(format!("Docker compose error: {}\n{}", e, all_stderr)),
            });
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    info!("Deploy command exit status: {:?}", output.status.code());
    info!("Deploy stdout: {}", stdout);
    if !stderr.is_empty() {
        warn!("Deploy stderr: {}", stderr);
    }

    if output.status.success() {
        info!("Deploy bot completed successfully");
        
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
        error!("Exit code: {:?}", output.status.code());
        error!("Stdout: {}", stdout);
        error!("Stderr: {}", stderr);
        
        Json(DeployResponse {
            success: false,
            message: format!(
                "Deploy failed with exit code: {:?}. Error: {}", 
                output.status.code(),
                if stderr.is_empty() { &stdout } else { &stderr }
            ),
            output: Some(stdout.to_string()),
            error: Some(stderr.to_string()),
        })
    }
}


