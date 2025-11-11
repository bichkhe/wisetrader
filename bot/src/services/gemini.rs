//! Gemini AI Service for Backtest Analysis
//! 
//! This module provides integration with Google Gemini API to analyze backtest results.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct GeminiService {
    api_key: String,
    model_name: String,
    base_url: String,
    timeout_secs: u64,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

impl GeminiService {
    /// Create a new Gemini service instance with default configuration
    pub fn new(api_key: String) -> Self {
        Self::with_config(
            api_key,
            "gemini-pro".to_string(),
            "https://generativelanguage.googleapis.com/v1beta".to_string(),
            60,
        )
    }

    /// Create a new Gemini service instance with custom configuration
    pub fn with_config(
        api_key: String,
        model_name: String,
        base_url: String,
        timeout_secs: u64,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            api_key,
            model_name,
            base_url,
            timeout_secs,
            client,
        }
    }

    /// Build the API URL for Gemini requests
    /// 
    /// Supports two formats:
    /// 1. If GEMINI_MODEL_URL env var is set, use it directly (can contain {key} placeholder)
    /// 2. Otherwise, build from base_url, model_name, and api_key
    fn build_api_url(&self) -> String {
        // Check if custom URL is provided via environment variable
        if let Ok(custom_url) = std::env::var("GEMINI_MODEL_URL") {
            // Only use custom URL if it's not empty
            if !custom_url.trim().is_empty() {
                // Replace {key} placeholder if present, otherwise use as-is
                if custom_url.contains("{key}") {
                    custom_url.replace("{key}", &self.api_key)
                } else {
                    custom_url
                }
            } else {
                // Build URL from components if custom URL is empty
                format!(
                    "{}/models/{}:generateContent?key={}",
                    self.base_url.trim_end_matches('/'),
                    self.model_name,
                    self.api_key
                )
            }
        } else {
            // Build URL from components
            format!(
                "{}/models/{}:generateContent?key={}",
                self.base_url.trim_end_matches('/'),
                self.model_name,
                self.api_key
            )
        }
    }

    /// Analyze backtest results using Gemini AI
    /// 
    /// Takes backtest metrics and tables, returns AI-generated analysis
    pub async fn analyze_backtest(
        &self,
        strategy_name: &str,
        exchange: &str,
        pair: &str,
        timeframe: &str,
        timerange: &str,
        trades: i32,
        profit_pct: f64,
        win_rate: Option<f64>,
        max_drawdown: Option<f64>,
        starting_balance: Option<f64>,
        final_balance: Option<f64>,
        tables: &[(String, String)],
        raw_output: Option<&str>,
    ) -> Result<String, anyhow::Error> {
        // Build prompt with backtest data
        let mut prompt = format!(
            r#"Bạn là một chuyên gia phân tích trading strategy. Hãy phân tích kết quả backtest sau đây và đưa ra nhận xét chi tiết:

**Thông tin Strategy:**
- Tên: {}
- Exchange: {}
- Cặp: {}
- Timeframe: {}
- Khoảng thời gian: {}

**Kết quả Backtest:**
- Số lượng trades: {}
- Lợi nhuận: {:.2}%
- Win rate: {}
- Max drawdown: {}
- Số dư ban đầu: {}
- Số dư cuối: {}

"#,
            strategy_name,
            exchange,
            pair,
            timeframe,
            timerange,
            trades,
            profit_pct,
            win_rate.map(|w| format!("{:.2}%", w)).unwrap_or_else(|| "N/A".to_string()),
            max_drawdown.map(|d| format!("{:.2}%", d)).unwrap_or_else(|| "N/A".to_string()),
            starting_balance.map(|b| format!("${:.2}", b)).unwrap_or_else(|| "N/A".to_string()),
            final_balance.map(|b| format!("${:.2}", b)).unwrap_or_else(|| "N/A".to_string()),
        );

        // Add tables to prompt
        if !tables.is_empty() {
            prompt.push_str("\n**Các bảng số liệu chi tiết:**\n\n");
            for (title, content) in tables {
                prompt.push_str(&format!("=== {} ===\n{}\n\n", title, content));
            }
        }

        // Add raw output if available (truncated to avoid token limits)
        if let Some(output) = raw_output {
            let truncated = if output.len() > 5000 {
                &output[..5000]
            } else {
                output
            };
            prompt.push_str(&format!("\n**Raw Output (truncated):**\n{}\n\n", truncated));
        }

        prompt.push_str(
            r#"**Yêu cầu phân tích:**

Hãy đưa ra phân tích chi tiết về:
1. **Đánh giá tổng quan**: Strategy này có hiệu quả không? Tại sao?
2. **Điểm mạnh**: Những điểm tích cực của strategy
3. **Điểm yếu**: Những vấn đề cần cải thiện
4. **Khuyến nghị**: Các đề xuất để tối ưu strategy (điều chỉnh parameters, điều kiện entry/exit, etc.)
5. **Rủi ro**: Các rủi ro tiềm ẩn cần lưu ý
6. **Kết luận**: Tóm tắt và đánh giá cuối cùng

Hãy viết bằng tiếng Việt, rõ ràng và chi tiết. Sử dụng định dạng markdown để dễ đọc."#
        );

        // Call Gemini API
        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt,
                }],
            }],
        };

        let url = self.build_api_url();

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Gemini API error ({}): {}",
                status,
                error_text
            ));
        }

        let gemini_response: GeminiResponse = response.json().await?;

        // Extract text from response
        let analysis = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from Gemini API"))?;

        Ok(analysis)
    }

    /// Analyze backtest with English prompt (for international users)
    pub async fn analyze_backtest_en(
        &self,
        strategy_name: &str,
        exchange: &str,
        pair: &str,
        timeframe: &str,
        timerange: &str,
        trades: i32,
        profit_pct: f64,
        win_rate: Option<f64>,
        max_drawdown: Option<f64>,
        starting_balance: Option<f64>,
        final_balance: Option<f64>,
        tables: &[(String, String)],
        raw_output: Option<&str>,
    ) -> Result<String, anyhow::Error> {
        // Build prompt with backtest data
        let mut prompt = format!(
            r#"You are a trading strategy analysis expert. Please analyze the following backtest results and provide detailed insights:

**Strategy Information:**
- Name: {}
- Exchange: {}
- Pair: {}
- Timeframe: {}
- Time Range: {}

**Backtest Results:**
- Number of trades: {}
- Profit: {:.2}%
- Win rate: {}
- Max drawdown: {}
- Starting balance: {}
- Final balance: {}

"#,
            strategy_name,
            exchange,
            pair,
            timeframe,
            timerange,
            trades,
            profit_pct,
            win_rate.map(|w| format!("{:.2}%", w)).unwrap_or_else(|| "N/A".to_string()),
            max_drawdown.map(|d| format!("{:.2}%", d)).unwrap_or_else(|| "N/A".to_string()),
            starting_balance.map(|b| format!("${:.2}", b)).unwrap_or_else(|| "N/A".to_string()),
            final_balance.map(|b| format!("${:.2}", b)).unwrap_or_else(|| "N/A".to_string()),
        );

        // Add tables to prompt
        if !tables.is_empty() {
            prompt.push_str("\n**Detailed Statistics Tables:**\n\n");
            for (title, content) in tables {
                prompt.push_str(&format!("=== {} ===\n{}\n\n", title, content));
            }
        }

        // Add raw output if available
        if let Some(output) = raw_output {
            let truncated = if output.len() > 5000 {
                &output[..5000]
            } else {
                output
            };
            prompt.push_str(&format!("\n**Raw Output (truncated):**\n{}\n\n", truncated));
        }

        prompt.push_str(
            r#"**Analysis Requirements:**

Please provide detailed analysis on:
1. **Overall Assessment**: Is this strategy effective? Why?
2. **Strengths**: Positive aspects of the strategy
3. **Weaknesses**: Areas that need improvement
4. **Recommendations**: Suggestions to optimize the strategy (parameter adjustments, entry/exit conditions, etc.)
5. **Risks**: Potential risks to be aware of
6. **Conclusion**: Summary and final assessment

Please write in clear, detailed English. Use markdown formatting for readability."#
        );

        // Call Gemini API
        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt,
                }],
            }],
        };

        let url = self.build_api_url();

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Gemini API error ({}): {}",
                status,
                error_text
            ));
        }

        let gemini_response: GeminiResponse = response.json().await?;

        // Extract text from response
        let analysis = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from Gemini API"))?;

        Ok(analysis)
    }

    /// Ask a general question to Gemini AI
    /// 
    /// Takes a question/prompt and returns AI-generated response
    pub async fn ask_question(&self, question: &str) -> Result<String, anyhow::Error> {
        // Call Gemini API
        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: question.to_string(),
                }],
            }],
        };

        let url = self.build_api_url();

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Gemini API error ({}): {}",
                status,
                error_text
            ));
        }

        let gemini_response: GeminiResponse = response.json().await?;

        // Extract text from response
        let answer = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from Gemini API"))?;

        Ok(answer)
    }
}

