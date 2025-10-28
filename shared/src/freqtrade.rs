use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreqtradeApiClient {
    pub base_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FreqtradeStatus {
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FreqtradeVersion {
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BacktestResult {
    pub strategy: String,
    pub trades: i32,
    pub profit_pct: f64,
}

impl FreqtradeApiClient {
    pub fn new(base_url: String, username: String, password: String) -> Self {
        Self {
            base_url,
            username,
            password,
        }
    }

    pub async fn ping(&self) -> Result<FreqtradeVersion> {
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/api/v1/ping", self.base_url))
            .send()
            .await?;
        
        let version: FreqtradeVersion = response.json().await?;
        Ok(version)
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

    pub async fn backtest(&self, strategy_name: &str, timeframe: &str, timerange: &str) -> Result<BacktestResult> {
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
}

