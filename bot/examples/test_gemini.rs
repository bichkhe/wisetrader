/// Example: Test Gemini API Integration
/// 
/// This example demonstrates:
/// - Loading GEMINI_API_KEY from environment variables
/// - Creating a GeminiService instance
/// - Calling analyze_backtest with sample backtest data
/// - Displaying the AI-generated analysis

use anyhow::Result;
use dotenv::dotenv;
use std::env;
use tracing::info;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// GeminiService implementation (copied from bot/src/services/gemini.rs for example purposes)
#[derive(Debug, Clone)]
struct GeminiService {
    api_key: String,
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
    /// Create a new Gemini service instance
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            api_key,
            client,
        }
    }

    /// Analyze backtest results using Gemini AI
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
    ) -> Result<String> {
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

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            self.api_key
        );

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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load environment variables from .env file
    dotenv().ok();

    let separator = "=".repeat(60);
    info!("{}", separator);
    info!("Gemini API Test Example");
    info!("{}", separator);

    // Get GEMINI_API_KEY from environment
    let api_key = env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow::anyhow!("GEMINI_API_KEY environment variable is not set. Please set it in your .env file or environment."))?;

    info!("✅ GEMINI_API_KEY loaded successfully");
    info!("🔑 API Key: {}...{}", &api_key[..8.min(api_key.len())], &api_key[api_key.len().saturating_sub(4)..]);

    // Create Gemini service
    let gemini_service = GeminiService::new(api_key);

    // Test 1: Simple test using analyze_backtest with minimal data
    info!("\n📝 Test 1: Simple Analysis Test");
    info!("Sending a simple test request to Gemini...");
    
    match gemini_service.analyze_backtest(
        "Test Strategy",
        "binance",
        "BTC/USDT",
        "1h",
        "2024-01-01 to 2024-01-07",
        10,
        5.0,
        Some(60.0),
        Some(3.0),
        Some(1000.0),
        Some(1050.0),
        &[],
        Some("This is a simple test to verify Gemini API connection."),
    ).await {
        Ok(response) => {
            info!("✅ Success! Response from Gemini:");
            println!("\n{}", response);
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
        }
    }

    // Test 2: Backtest analysis test with detailed data
    info!("\n📊 Test 2: Backtest Analysis Test");
    info!("Sending sample backtest data for analysis...");

    let sample_tables = vec![
        (
            "Top 5 Winning Trades".to_string(),
            "Trade 1: +5.2%\nTrade 2: +3.8%\nTrade 3: +2.1%\nTrade 4: +1.9%\nTrade 5: +1.5%".to_string(),
        ),
        (
            "Top 5 Losing Trades".to_string(),
            "Trade 1: -2.3%\nTrade 2: -1.8%\nTrade 3: -1.5%\nTrade 4: -1.2%\nTrade 5: -0.9%".to_string(),
        ),
    ];

    match gemini_service.analyze_backtest(
        "RSI Strategy",
        "binance",
        "BNB/USDT",
        "5m",
        "2024-01-01 to 2024-01-31",
        150,
        12.5,
        Some(65.0),
        Some(8.3),
        Some(1000.0),
        Some(1125.0),
        &sample_tables,
        None,
    ).await {
        Ok(analysis) => {
            info!("✅ Success! Analysis from Gemini:");
            println!("\n{}", analysis);
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
        }
    }

    info!("\n{}", separator);
    info!("✨ All tests completed!");
    info!("{}", separator);

    Ok(())
}
