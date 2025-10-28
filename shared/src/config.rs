use dotenv::dotenv;

pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub bot_token: String,
    pub bot_name: String,
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
        })
    }
}
