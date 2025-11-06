use shared::{Config, get_pool, get_db_connection};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, sqlx::MySqlPool};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::Dialogue};
use crate::services::user_service::UserService;
use crate::services::strategy_engine::StrategyExecutor;
use crate::services::strategy_service::StrategyService;

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
    pub strategy_service: Arc<StrategyService>,
    pub strategy_executor: Arc<StrategyExecutor>,
    pub config: Arc<Config>,
}

impl AppState {
    pub async fn new() -> Result<Self, anyhow::Error> {
        tracing::info!("Loading configuration from environment...");
        let config = match Config::from_env() {
            Ok(cfg) => {
                tracing::info!("Configuration loaded successfully");
                tracing::info!("Bot name: {}", cfg.bot_name);
                tracing::info!("Database URL: {}", cfg.database_url);
                tracing::info!("Redis URL: {}", cfg.redis_url);
                tracing::info!("API Base URL: {}", cfg.api_base_url);
                cfg
            }
            Err(e) => {
                tracing::error!("Failed to load configuration: {:?}", e);
                return Err(e);
            }
        };

        tracing::info!("Connecting to MySQL database...");
        let pool = match get_pool(&config.database_url).await {
            Ok(p) => {
                tracing::info!("MySQL pool created successfully");
                p
            }
            Err(e) => {
                tracing::error!("Failed to create MySQL pool: {:?}", e);
                return Err(e);
            }
        };

        tracing::info!("Connecting to database via SeaORM...");
        let db = match get_db_connection(&config.database_url).await {
            Ok(conn) => {
                tracing::info!("Database connection established successfully");
                conn
            }
            Err(e) => {
                tracing::error!("Failed to connect to database: {:?}", e);
                return Err(e);
            }
        };

        // Initialize services
        tracing::info!("Initializing services...");
        let db_arc = Arc::new(db);
        let user_service = Arc::new(UserService::new(db_arc.clone()));
        tracing::info!("UserService initialized");

        let strategy_service = Arc::new(StrategyService::new(db_arc.clone()));
        tracing::info!("StrategyService initialized");

        // Create StrategyExecutor (it doesn't need AppState)
        let executor = Arc::new(StrategyExecutor::new());
        tracing::info!("StrategyExecutor initialized");

        // Create the AppState
        let app_state = Self {
            bot_token: config.bot_token.clone(),
            bot_name: config.bot_name.clone(),
            pool: Arc::new(pool),
            database_url: config.database_url.clone(),
            redis_url: config.redis_url.clone(),
            db: db_arc,
            user_service,
            strategy_service,
            strategy_executor: executor,
            config: Arc::new(config),
        };

        tracing::info!("AppState created successfully");
        Ok(app_state)
    }
}

#[derive(Clone, Default, Debug)]
pub enum BotState {
    #[default]
    Normal,
    WaitingForLanguage,
    CreateStrategy(CreateStrategyState),
    Trading(TradingState),
    Backtest(BacktestState),
}


#[derive(Clone, Debug, Default)]
pub enum CreateStrategyState {
    #[default]
    Start,
    WaitingForStrategyType, // Choose between Custom or Preset
    WaitingForPresetSelection, // Waiting for user to select a preset strategy
    WaitingForPresetName {
        algorithm: String,
        buy_condition: String,
        sell_condition: String,
        timeframe: String,
    },
    WaitingForName,
    WaitingForAlgorithm,
    WaitingForBuyCondition {
        algorithm: String,
    },
    WaitingForSellCondition {
        algorithm: String,
        buy_condition: String,
    },
    WaitingForTimeframe {
        algorithm: String,
        buy_condition: String,
        sell_condition: String,
    },
    WaitingForPair {
        algorithm: String,
        buy_condition: String,
        sell_condition: String,
        timeframe: String,
        strategy_name: String,
    },
    WaitingForConfirmation {
        algorithm: String,
        buy_condition: String,
        sell_condition: String,
        timeframe: String,
        pair: String,
    },
}

#[derive(Clone, Debug, Default)]
pub enum TradingState {
    #[default]
    Idle,
    WaitingForPair,
    WaitingForAmount,
    WaitingForConfirmation,
}

#[derive(Clone, Debug, Default)]
pub enum BacktestState {
    #[default]
    Start,
    WaitingForStrategy,
    WaitingForExchange {
        strategy_id: u64,
        strategy_name: String,
    },
    WaitingForTimeRange {
        strategy_id: u64,
        strategy_name: String,
        exchange: String,
    },
}