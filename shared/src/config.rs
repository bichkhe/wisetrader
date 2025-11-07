use dotenv::dotenv;

pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub bot_token: String,
    pub bot_name: String,
    pub generate_html_reports: bool,
    pub html_reports_dir: String,
    pub html_reports_base_url: Option<String>,
    pub api_base_url: String,
    pub webhook_url: Option<String>,
    pub webhook_path: String,
    pub webhook_port: u16,
    pub mobile_friendly_tables: bool,
    pub gemini_api_key: Option<String>,
    pub enable_gemini_analysis: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        dotenv().ok();
        
        Ok(Config {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "mysql://wisetrader:wisetrader2025@localhost:3306/wisetrader_db".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            bot_token: std::env::var("BOT_TOKEN")?,
            bot_name: std::env::var("BOT_NAME").unwrap_or_else(|_| "WiseTrader".to_string()),
            generate_html_reports: std::env::var("GENERATE_HTML_REPORTS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            html_reports_dir: std::env::var("HTML_REPORTS_DIR")
                .unwrap_or_else(|_| "./html_reports".to_string()),
            html_reports_base_url: std::env::var("HTML_REPORTS_BASE_URL").ok(),
            api_base_url: std::env::var("API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:9999".to_string()),
            webhook_url: std::env::var("WEBHOOK_URL").ok(),
            webhook_path: std::env::var("WEBHOOK_PATH")
                .unwrap_or_else(|_| "/webhook".to_string()),
            webhook_port: std::env::var("WEBHOOK_PORT")
                .unwrap_or_else(|_| "8443".to_string())
                .parse()
                .unwrap_or(8443),
            mobile_friendly_tables: std::env::var("MOBILE_FRIENDLY_TABLES")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
            enable_gemini_analysis: std::env::var("ENABLE_GEMINI_ANALYSIS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        })
    }
}
