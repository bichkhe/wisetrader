use anyhow::Result;
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreqtradeApiClient {
    pub base_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FreqtradeStatus {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FreqtradeVersion {
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BacktestResult {
    pub strategy: String,
    pub trades: i32,
    pub profit_pct: f64,
    pub download_time_secs: Option<u64>,
    pub backtest_time_secs: u64,
    pub stdout: Option<String>, // Full stdout output for detailed information
    pub stderr: Option<String>, // Full stderr output for debugging
    pub win_rate: Option<f64>, // Win rate percentage
    pub max_drawdown: Option<f64>, // Max drawdown percentage
    pub starting_balance: Option<f64>, // Starting capital
    pub final_balance: Option<f64>, // Final balance
}

impl FreqtradeApiClient {
    pub fn new(base_url: String, username: String, password: String) -> Self {
        Self {
            base_url,
            username,
            password,
        }
    }

    pub async fn ping(&self) -> Result<FreqtradeStatus> {
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/api/v1/ping", self.base_url))
            .send()
            .await?;
        
        let status: FreqtradeStatus = response.json().await?;
        Ok(status)
    }

    pub async fn status(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/api/v1/status", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await?;
        
        let text = response.text().await?;
        Ok(text)
    }

    /// Stop the trading bot if it's running
    pub async fn stop(&self) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/api/v1/stop", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await?;
        
        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to stop bot: {}", error_text))
        }
    }

    /// Start the trading bot
    pub async fn start(&self) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/api/v1/start", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await?;
        
        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Failed to start bot: {}", error_text))
        }
    }

    pub async fn backtest(&self, strategy_name: &str, symbol: &str, timeframe: &str, timerange: &str) -> Result<BacktestResult> {
        // This is a simplified version - real implementation would need proper auth
        let client = reqwest::Client::new();
        
        let response = client
            .post(&format!("{}/api/v1/backtest", self.base_url))
            .json(&serde_json::json!({
                "strategy": strategy_name,
                "timeframe": timeframe,
                "timerange": timerange,
            }))
            .send()
            .await?;
        
        let result: BacktestResult = response.json().await?;
        Ok(result)
    }

    /// Check if data exists for given exchange, pair, and timeframe
    pub async fn check_data_exists(
        &self,
        container_name: &str,
        exchange: &str,
        pair: &str,
        timeframe: &str,
    ) -> Result<bool> {
        use tokio::process::Command;

        // Try to list data files - if command succeeds, data likely exists
        // Note: This is a simple check - freqtrade doesn't have a direct "check data" command
        // So we try to list the data directory or check if backtest can find data
        
        tracing::debug!("Checking if data exists for {}/{} on {}", exchange, pair, timeframe);
        
        // For now, we'll assume data doesn't exist and always download
        // A better approach would be to check the data directory, but that's complex
        // We can improve this later by checking specific data files
        
        Ok(false) // Always assume data needs to be downloaded for safety
    }

    /// Download historical data for backtesting
    /// Downloads data only for the specified pair (not all pairs in config)
    /// Returns the time taken in seconds
    pub async fn download_data(
        &self,
        container_name: &str,
        exchange: &str,
        pair: &str, // Download only this specific pair
        timeframe: &str,
        days: u32,
    ) -> Result<u64> {
        use std::time::Instant;
        use tokio::process::Command;
        use std::process::Stdio;

        tracing::info!(
            "Downloading data for specific pair: container={}, exchange={}, pair={}, timeframe={}, days={}",
            container_name,
            exchange,
            pair,
            timeframe,
            days
        );

        let start_time = Instant::now();
        // Download data only for the specific pair
        // Ensure data directory exists first
        let _mkdir_output = Command::new("docker")
            .arg("exec")
            .arg(container_name)
            .arg("mkdir")
            .arg("-p")
            .arg("/freqtrade/user_data/data")
            .output()
            .await;
        
        // Download data only for the specific pair
        let output = Command::new("docker")
            .arg("exec")
            .arg(container_name)
            .arg("freqtrade")
            .arg("download-data")
            .arg("--exchange")
            .arg(exchange)
            .arg("--pairs")
            .arg(pair) // Specify the exact pair to download
            .arg("--timeframes")
            .arg(timeframe)
            .arg("--days")
            .arg(days.to_string())
            .arg("--config")
            .arg("/freqtrade/user_data/config.json")
            .arg("--user-data-dir")
            .arg("/freqtrade/user_data")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        tracing::info!(
            "Running command: docker exec {} freqtrade download-data --exchange {} --pairs {} --timeframes {} --days {} --config /freqtrade/user_data/config.json --user-data-dir /freqtrade/user_data",
            container_name,
            exchange,
            pair,
            timeframe,
            days
        );

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            tracing::error!("Download data failed. Exit code: {:?}", output.status.code());
            tracing::error!("stderr: {}", stderr);
            tracing::error!("stdout: {}", stdout);
            
            return Err(anyhow::anyhow!(
                "Failed to download data: {}. stderr: {}",
                output.status,
                stderr
            ));
        }

        let elapsed = start_time.elapsed().as_secs();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Log full output for debugging
        tracing::info!("Download data stdout:\n{}", stdout);
        if !stderr.is_empty() {
            tracing::info!("Download data stderr:\n{}", stderr);
        }
        tracing::info!("Data downloaded successfully in {}s", elapsed);

        Ok(elapsed)
    }

    /// Run backtest via CLI command in Docker container
    pub async fn backtest_via_cli(
        &self,
        container_name: &str,
        strategy_name: &str,
        exchange: &str,
        pair: &str,
        timeframe: &str,
        timerange: &str,
    ) -> Result<BacktestResult> {
        use tokio::process::Command;
        use std::process::Stdio;
        use std::time::Instant;

        tracing::info!(
            "Running backtest via CLI: container={}, strategy={}, exchange={}, pair={}, timeframe={}, timerange={}",
            container_name,
            strategy_name,
            exchange,
            pair,
            timeframe,
            timerange
        );

        // Check if data exists, if not, download it
        let mut download_time: Option<u64> = None;
        match self.check_data_exists(container_name, exchange, pair, timeframe).await {
            Ok(exists) => {
                if !exists {
                    tracing::info!("Data not found, downloading...");
                    
                    // Calculate days from timerange
                    let timerange_trimmed = timerange.trim_end_matches('-');
                    let days = if timerange_trimmed.contains("day") || timerange_trimmed.len() <= 4 {
                        // Handle preset ranges like "1day", "1week", etc.
                        match timerange_trimmed {
                            "1day" => 1,
                            "1week" => 7,
                            "1month" => 30,
                            "3months" => 90,
                            "6months" => 180,
                            _ => 7, // Default to 7 days for short ranges
                        }
                    } else {
                        // Parse YYYYMMDD format to calculate days
                        let now = chrono::Utc::now();
                        if let Ok(start_date) = chrono::NaiveDate::parse_from_str(timerange_trimmed, "%Y%m%d") {
                            let start_naive_datetime = start_date.and_hms_opt(0, 0, 0).unwrap_or_default();
                            let start_datetime = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                                start_naive_datetime,
                                chrono::Utc,
                            );
                            let duration = now.signed_duration_since(start_datetime);
                            duration.num_days().abs() as u32 + 1 // Add 1 for safety
                        } else {
                            30 // Default to 30 days
                        }
                    };
                    
                    // Download data
                    match self.download_data(container_name, exchange, pair, timeframe, days).await {
                        Ok(elapsed) => {
                            download_time = Some(elapsed);
                            tracing::info!("Data downloaded successfully in {}s, proceeding with backtest", elapsed);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to download data (may already exist): {}", e);
                            // Continue anyway, data might already exist
                        }
                    }
                } else {
                    tracing::info!("Data already exists, skipping download");
                }
            }
            Err(e) => {
                tracing::warn!("Could not check data existence: {}, proceeding anyway", e);
            }
        }

        // Execute docker exec command to run freqtrade backtesting
        // Bubble: Add --pairs to only test with the specific pair (not all pairs in pair_whitelist)
        let backtest_start = Instant::now();
        let output = Command::new("docker")
            .arg("exec")
            .arg(container_name)
            .arg("freqtrade")
            .arg("backtesting")
            .arg("--strategy")
            .arg(strategy_name)
            .arg("--pairs")
            .arg(pair) // Only test with the specific pair, not all pairs in config
            .arg("--timeframe")
            .arg(timeframe)
            .arg("--timerange")
            .arg(timerange)
            .arg("--config")
            .arg("/freqtrade/user_data/config.json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Check if command succeeded
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            tracing::error!("Backtest CLI failed. Exit code: {:?}", output.status.code());
            tracing::error!("stderr: {}", stderr);
            tracing::error!("stdout: {}", stdout);
            
            // Truncate error message to avoid Telegram message too long error (max 4096 chars)
            let error_msg = if stderr.len() > 2000 {
                format!("{}... (truncated)", &stderr[..2000])
            } else {
                stderr.to_string()
            };
            
            return Err(anyhow::anyhow!(
                "Backtest failed: {}. Error: {}",
                output.status,
                error_msg
            ));
        }

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Log full output for debugging
        tracing::info!("Backtest stdout:\n{}", stdout);
        tracing::info!("Backtest stderr:\n{}", stderr);

        // Parse freqtrade backtest output
        // Freqtrade outputs results in text format with tables, extract all metrics
        let mut trades = 0;
        let mut profit_pct = 0.0;
        let mut win_rate = 0.0;
        let mut max_drawdown = 0.0;
        let mut starting_balance = 0.0;
        let mut final_balance = 0.0;
        
        for line in stdout.lines() {
            let line_trimmed = line.trim();
            
            // Extract total trades
            if line_trimmed.contains("Total") && (line_trimmed.contains("trade") || line_trimmed.contains("Trade")) && trades == 0 {
                if let Some(num) = extract_number(line_trimmed) {
                    trades = num;
                }
            } else if (line_trimmed.contains("Trades:") || line_trimmed.contains("trades:")) && trades == 0 {
                if let Some(num) = extract_number(line_trimmed) {
                    trades = num;
                }
            }
            
            // Extract profit percentage
            if (line_trimmed.contains("Total profit") || line_trimmed.contains("Total Profit") || line_trimmed.contains("Profit %")) && profit_pct == 0.0 {
                if let Some(pct) = extract_percentage(line_trimmed) {
                    profit_pct = pct;
                }
            } else if line_trimmed.contains("Profit:") && line_trimmed.contains("%") && profit_pct == 0.0 {
                if let Some(pct) = extract_percentage(line_trimmed) {
                    profit_pct = pct;
                }
            }
            
            // Extract win rate
            if (line_trimmed.contains("Win") && line_trimmed.contains("Rate")) || line_trimmed.contains("Win %") || (line_trimmed.contains("Win%") && win_rate == 0.0) {
                if let Some(pct) = extract_percentage(line_trimmed) {
                    win_rate = pct;
                }
            }
            
            // Extract max drawdown
            if line_trimmed.contains("Max Drawdown") || (line_trimmed.contains("drawdown") && max_drawdown == 0.0) {
                if let Some(pct) = extract_percentage(line_trimmed) {
                    max_drawdown = pct.abs();
                }
            }
            
            // Extract starting balance
            if line_trimmed.contains("Starting capital") || line_trimmed.contains("Starting balance") || line_trimmed.contains("Starting amount") {
                if let Some(num) = extract_decimal(line_trimmed) {
                    starting_balance = num;
                }
            }
            
            // Extract final balance
            if line_trimmed.contains("Final balance") || line_trimmed.contains("Final equity") || line_trimmed.contains("End balance") {
                if let Some(num) = extract_decimal(line_trimmed) {
                    final_balance = num;
                }
            }
        }

        // If we couldn't parse, try searching in stderr as well
        if trades == 0 || profit_pct == 0.0 {
            for line in stderr.lines() {
                if trades == 0 {
                    if (line.contains("Total") && line.contains("trade")) || line.contains("Trades:") {
                        if let Some(num) = extract_number(line) {
                            trades = num;
                        }
                    }
                }
                if profit_pct == 0.0 {
                    if line.contains("Profit:") && line.contains("%") {
                        if let Some(pct) = extract_percentage(line) {
                            profit_pct = pct;
                        }
                    }
                }
            }
        }

        let backtest_elapsed = backtest_start.elapsed().as_secs();
        
        // Store full output for detailed information
        let stdout_full = if stdout.is_empty() { None } else { Some(stdout.to_string()) };
        let stderr_full = if stderr.is_empty() { None } else { Some(stderr.to_string()) };
        
        // Log parsed metrics
        tracing::info!("Parsed metrics: trades={}, profit={:.2}%, win_rate={:.2}%, drawdown={:.2}%, start={:.2}, final={:.2}", 
            trades, profit_pct, win_rate, max_drawdown, starting_balance, final_balance);
        
        Ok(BacktestResult {
            strategy: strategy_name.to_string(),
            trades,
            profit_pct,
            download_time_secs: download_time,
            backtest_time_secs: backtest_elapsed,
            stdout: stdout_full,
            stderr: stderr_full,
            win_rate: if win_rate > 0.0 { Some(win_rate) } else { None },
            max_drawdown: if max_drawdown > 0.0 { Some(max_drawdown) } else { None },
            starting_balance: if starting_balance > 0.0 { Some(starting_balance) } else { None },
            final_balance: if final_balance > 0.0 { Some(final_balance) } else { None },
        })
    }

    pub async fn backtest_with_exchange(&self, strategy_name: &str, exchange: &str, timeframe: &str, timerange: &str) -> Result<BacktestResult> {
        // Stop bot first if it's running (required for backtest)
        match self.stop().await {
            Ok(_) => {
                tracing::info!("Bot stopped successfully before backtest");
                // Wait a bit for bot to fully stop
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                // If stop fails, it might be already stopped, continue anyway
                tracing::warn!("Could not stop bot (might already be stopped): {}", e);
            }
        }
        
        // Create client with longer timeout for backtest (can take minutes)
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600)) // 10 minutes timeout
            .build()?;
        
        // Freqtrade API expects timerange in format: timestamp_start-timestamp_end
        // For now, use timerange as start, and current time as end
        let timerange_param = format!("{}-", timerange);
        
        tracing::info!("Starting backtest: strategy={}, exchange={}, timeframe={}, timerange={}", 
            strategy_name, exchange, timeframe, timerange_param);
        
        let response = client
            .post(&format!("{}/api/v1/backtest", self.base_url))
            .basic_auth(&self.username, Some(&self.password))
            .json(&serde_json::json!({
                "strategy": strategy_name,
                "timeframe": timeframe,
                "timerange": timerange_param,
                "exchange": exchange,
            }))
            .send()
            .await?;
        
        let status = response.status();
        let response_text = response.text().await?;
        
        if status.is_success() {
            // Try to parse as JSON first
            match serde_json::from_str::<BacktestResult>(&response_text) {
                Ok(result) => Ok(result),
                Err(e) => {
                    // If parsing fails, try to extract data from response
                    tracing::warn!("Failed to parse backtest response as BacktestResult: {}", e);
                    tracing::debug!("Response body: {}", response_text);
                    
                    // Try to parse as generic JSON to see structure
                    match serde_json::from_str::<serde_json::Value>(&response_text) {
                        Ok(json) => {
                            // Try to extract fields manually
                            let trades = json.get("trades")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i32;
                            let profit_pct = json.get("profit_pct")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            
                            Ok(BacktestResult {
                                strategy: strategy_name.to_string(),
                                trades,
                                profit_pct,
                                download_time_secs: None,
                                backtest_time_secs: 0,
                                stdout: None,
                                stderr: None,
                                win_rate: None,
                                max_drawdown: None,
                                starting_balance: None,
                                final_balance: None,
                            })
                        }
                        Err(_) => Err(anyhow::anyhow!(
                            "Failed to parse Freqtrade response. Status: {}. Body: {}", 
                            status, 
                            response_text
                        ))
                    }
                }
            }
        } else {
            Err(anyhow::anyhow!(
                "Freqtrade API error. Status: {}. Response: {}", 
                status, 
                response_text
            ))
        }
    }
}

/// Helper function to extract number from text
fn extract_number(text: &str) -> Option<i32> {
    let re = Regex::new(r"(\d+)").ok()?;
    re.find(text)?.as_str().parse().ok()
}

/// Helper function to extract percentage from text
fn extract_percentage(text: &str) -> Option<f64> {
    let re = Regex::new(r"([+-]?\d+\.?\d*)%").ok()?;
    let matched = re.find(text)?;
    matched.as_str().trim_end_matches('%').parse().ok()
}

/// Helper function to extract decimal number from text
fn extract_decimal(text: &str) -> Option<f64> {
    let re = Regex::new(r"(\d+\.?\d*)").ok()?;
    let matched = re.find(text)?;
    matched.as_str().parse().ok()
}

