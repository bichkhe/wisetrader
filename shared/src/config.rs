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
        })
    }
}
