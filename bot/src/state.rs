use shared::{Config, get_pool, get_db_connection};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, prelude::*, sqlx::MySqlPool};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::Dialogue};
use crate::services::user_service::UserService;

pub type MyDialogue = Dialogue<BotState, InMemStorage<BotState>>;
pub type HandlerResult = Result<(), anyhow::Error>;

#[derive(Clone)]
pub struct AppState {
    pub bot_token: String,
    pub bot_name: String,
    pub pool: Arc<MySqlPool>,
    pub database_url: String,
    pub redis_url: String,
    pub db: Arc<DatabaseConnection>,
    pub user_service: Arc<UserService>,
}

impl AppState {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let config = Config::from_env()?;
        let pool = get_pool(&config.database_url).await?;
        let db = get_db_connection(&config.database_url).await?;
        tracing::info!("Connected to database successfully");
        
        // Initialize UserService
        let user_service = Arc::new(UserService::new(Arc::new(db.clone())));
        
        Ok(AppState {
            bot_token: config.bot_token.clone(),
            bot_name: config.bot_name.clone(),
            pool: Arc::new(pool),
            database_url: config.database_url.clone(),
            redis_url: config.redis_url.clone(),
            db: Arc::new(db),
            user_service,
        })
    }
}

#[derive(Clone, Default, Debug)]
pub enum BotState {
    #[default]
    Normal,
    Start,
    Help,
    Subscription,
    Strategies,
    MyStrategies,
}
