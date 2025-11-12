//! Strategy Executor - manages running strategies for users

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use std::collections::HashMap;
use crate::services::strategy_engine::{Strategy, StrategyConfig, Candle, StrategySignal};

/// User's trading state with strategy
pub struct UserTradingState {
    pub user_id: i64,
    pub strategy: Box<dyn Strategy>,
    pub pair: String,
    pub exchange: String, // Store exchange for stream management
    pub is_active: bool,
}

/// Strategy Executor - manages multiple users' strategies
pub struct StrategyExecutor {
    /// Map of user_id -> UserTradingState
    users: Arc<RwLock<HashMap<i64, UserTradingState>>>,
}

impl StrategyExecutor {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start trading for a user with a specific strategy
    pub async fn start_trading(
        &self,
        user_id: i64,
        strategy_config: StrategyConfig,
        exchange: Option<String>, // Optional exchange for stream management
    ) -> Result<()> {
        use crate::services::strategy_engine::StrategyRegistry;
        
        let registry = StrategyRegistry::new();
        let strategy = registry.create_strategy(strategy_config.clone())?;
        
        let mut users = self.users.write().await;
        users.insert(user_id, UserTradingState {
            user_id,
            strategy,
            pair: strategy_config.pair,
            exchange: exchange.unwrap_or_else(|| "binance".to_string()),
            is_active: true,
        });
        
        tracing::info!("✅ Started trading for user {} with strategy {}", user_id, strategy_config.strategy_type);
        Ok(())
    }
    
    /// Stop trading for a user
    /// Returns (exchange, pair) if user was trading, for stream cleanup
    pub async fn stop_trading(&self, user_id: i64) -> Result<Option<(String, String)>> {
        let mut users = self.users.write().await;
        if let Some(state) = users.get(&user_id) {
            let exchange = state.exchange.clone();
            let pair = state.pair.clone();
        users.remove(&user_id);
            tracing::info!("🛑 Stopped trading for user {} ({} on {})", user_id, pair, exchange);
            Ok(Some((exchange, pair)))
        } else {
            Ok(None)
        }
    }
    
    /// Process a candle for a specific user
    pub async fn process_candle(
        &self,
        user_id: i64,
        candle: &Candle,
    ) -> Option<StrategySignal> {
        let mut users = self.users.write().await;
        
        if let Some(state) = users.get_mut(&user_id) {
            if state.is_active {
                // Process candle regardless of pair match (pair matching is done at stream level)
                // Log strategy evaluation for debugging
                tracing::debug!("🔍 [User {}] Processing candle for strategy '{}' (pair: {}, timeframe: {})", 
                    user_id, state.strategy.name(), state.pair, state.strategy.config().timeframe);
                return state.strategy.process_candle(candle);
            } else {
                tracing::warn!("⚠️ [User {}] Strategy is not active, skipping candle processing", user_id);
            }
        } else {
            tracing::warn!("⚠️ [User {}] No trading state found, skipping candle processing", user_id);
        }
        
        None
    }
    
    /// Process candle for all active users
    /// Returns vector of (user_id, signal) pairs
    pub async fn process_candle_for_all(
        &self,
        candle: &Candle,
    ) -> Vec<(i64, StrategySignal)> {
        let mut signals = Vec::new();
        let mut users = self.users.write().await;
        
        for (user_id, state) in users.iter_mut() {
            if state.is_active {
                // Process candle for this user's strategy
                if let Some(signal) = state.strategy.process_candle(candle) {
                    signals.push((*user_id, signal));
                }
            }
        }
        
        signals
    }
    
    /// Check if user is trading
    pub async fn is_user_trading(&self, user_id: i64) -> bool {
        let users = self.users.read().await;
        users.get(&user_id)
            .map(|s| s.is_active)
            .unwrap_or(false)
    }
    
    /// Get user's trading state info
    pub async fn get_user_state_info(&self, user_id: i64) -> Option<String> {
        let users = self.users.read().await;
        users.get(&user_id)
            .map(|s| format!("Strategy: {}, Pair: {}, Active: {}", 
                s.strategy.name(), s.pair, s.is_active))
    }
    
    /// Get all active trading users with their details
    pub async fn get_active_trading_users(&self) -> Vec<(i64, String, String, String)> {
        let users = self.users.read().await;
        users.iter()
            .filter(|(_, state)| state.is_active)
            .map(|(user_id, state)| {
                (
                    *user_id,
                    state.strategy.name().to_string(),
                    state.pair.clone(),
                    state.strategy.config().timeframe.clone(),
                )
            })
            .collect()
    }
    
    /// Get user's trading details
    pub async fn get_user_trading_details(&self, user_id: i64) -> Option<(String, String, String)> {
        let users = self.users.read().await;
        users.get(&user_id)
            .filter(|s| s.is_active)
            .map(|s| {
                (
                    s.strategy.name().to_string(),
                    s.pair.clone(),
                    s.strategy.config().timeframe.clone(),
                )
            })
    }
}

