//! Strategy Service - loads strategies from database and converts to StrategyConfig

use std::sync::Arc;
use anyhow::{Result, Context, bail};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use shared::entity::{strategies, users};
use crate::services::strategy_engine::StrategyConfig;
use serde_json::{Value, Map};

pub struct StrategyService {
    db: Arc<DatabaseConnection>,
}

impl StrategyService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
    
    /// Get all strategies created by a user
    pub async fn get_user_strategies(&self, telegram_id: i64) -> Result<Vec<strategies::Model>> {
        let strategies = strategies::Entity::find()
            .filter(strategies::Column::TelegramId.eq(telegram_id.to_string()))
            .all(self.db.as_ref())
            .await
            .context("Failed to fetch user strategies")?;
        
        Ok(strategies)
    }
    
    /// Get a specific strategy by ID
    pub async fn get_strategy_by_id(&self, strategy_id: u64) -> Result<Option<strategies::Model>> {
        let strategy = strategies::Entity::find_by_id(strategy_id)
            .one(self.db.as_ref())
            .await
            .context("Failed to fetch strategy")?;
        
        Ok(strategy)
    }
    
    /// Convert database strategy to StrategyConfig
    /// First tries to parse and validate from content field (JSON), falls back to description field for backward compatibility
    pub fn strategy_to_config(&self, strategy: &strategies::Model) -> Result<StrategyConfig> {
        // Try to parse from content field (JSON) first
        if let Some(content) = strategy.content.as_ref() {
            match self.parse_and_validate_content(content) {
                Ok(config) => {
                    // Validate the config
                    if let Err(e) = self.validate_config(&config) {
                        tracing::warn!("Strategy validation failed, falling back to description. Strategy ID: {}, Error: {}", strategy.id, e);
                    } else {
                        return Ok(config);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse strategy content as JSON, falling back to description. Strategy ID: {}, Error: {}", strategy.id, e);
                }
            }
        }
        
        // Fallback: Parse from description field (backward compatibility)
        let description = strategy.description.as_ref()
            .context("Strategy description is missing and content is also missing")?;
        
        // Parse description format: "Algorithm: RSI\nBuy: RSI < 30\nSell: RSI > 70\nTimeframe: 1m\nPair: BTC/USDT"
        let mut algorithm = None;
        let mut buy_condition = None;
        let mut sell_condition = None;
        let mut timeframe = None;
        let mut pair = None;
        
        for line in description.lines() {
            let line = line.trim();
            if line.starts_with("Algorithm:") {
                algorithm = Some(line.strip_prefix("Algorithm:").unwrap().trim().to_string());
            } else if line.starts_with("Buy:") {
                buy_condition = Some(line.strip_prefix("Buy:").unwrap().trim().to_string());
            } else if line.starts_with("Sell:") {
                sell_condition = Some(line.strip_prefix("Sell:").unwrap().trim().to_string());
            } else if line.starts_with("Timeframe:") {
                timeframe = Some(line.strip_prefix("Timeframe:").unwrap().trim().to_string());
            } else if line.starts_with("Pair:") {
                pair = Some(line.strip_prefix("Pair:").unwrap().trim().to_string());
            }
        }
        
        let algorithm = algorithm.context("Algorithm not found in strategy description")?;
        let buy_condition = buy_condition.context("Buy condition not found in strategy description")?;
        let sell_condition = sell_condition.context("Sell condition not found in strategy description")?;
        let timeframe = timeframe.context("Timeframe not found in strategy description")?;
        let pair = pair.context("Pair not found in strategy description")?;
        
        // Extract parameters based on algorithm type
        let parameters = self.extract_parameters(&algorithm, &buy_condition, &sell_condition);
        
        let config = StrategyConfig {
            strategy_type: algorithm.clone(),
            parameters,
            pair,
            timeframe,
            buy_condition,
            sell_condition,
        };
        
        // Validate even when parsed from description
        self.validate_config(&config)?;
        
        Ok(config)
    }
    
    /// Parse JSON content and return StrategyConfig
    /// This function handles flexible JSON structures based on strategy type
    fn parse_and_validate_content(&self, content: &str) -> Result<StrategyConfig> {
        let json: Value = serde_json::from_str(content)
            .context("Failed to parse content as JSON")?;
        
        let obj = json.as_object()
            .context("Content must be a JSON object")?;
        
        // Extract required fields
        let strategy_type = obj.get("strategy_type")
            .and_then(|v| v.as_str())
            .context("Missing or invalid 'strategy_type' field")?
            .to_string();
        
        let pair = obj.get("pair")
            .and_then(|v| v.as_str())
            .context("Missing or invalid 'pair' field")?
            .to_string();
        
        let timeframe = obj.get("timeframe")
            .and_then(|v| v.as_str())
            .context("Missing or invalid 'timeframe' field")?
            .to_string();
        
        let buy_condition = obj.get("buy_condition")
            .and_then(|v| v.as_str())
            .context("Missing or invalid 'buy_condition' field")?
            .to_string();
        
        let sell_condition = obj.get("sell_condition")
            .and_then(|v| v.as_str())
            .context("Missing or invalid 'sell_condition' field")?
            .to_string();
        
        // Extract parameters (can be object or null/empty)
        // If not present, we'll extract from conditions or use defaults
        let parameters = if let Some(params) = obj.get("parameters") {
            if params.is_null() {
                // Extract parameters from conditions if not provided
                self.extract_parameters(&strategy_type, &buy_condition, &sell_condition)
            } else if params.is_object() {
                params.clone()
            } else {
                bail!("'parameters' must be a JSON object or null");
            }
        } else {
            // Extract parameters from conditions if not provided
            self.extract_parameters(&strategy_type, &buy_condition, &sell_condition)
        };
        
        Ok(StrategyConfig {
            strategy_type,
            parameters,
            pair,
            timeframe,
            buy_condition,
            sell_condition,
        })
    }
    
    /// Validate StrategyConfig based on strategy type
    /// This ensures the config is valid for both backtest and live trading
    fn validate_config(&self, config: &StrategyConfig) -> Result<()> {
        // Validate common required fields
        if config.strategy_type.is_empty() {
            bail!("Strategy type cannot be empty");
        }
        
        if config.pair.is_empty() {
            bail!("Pair cannot be empty");
        }
        
        if config.timeframe.is_empty() {
            bail!("Timeframe cannot be empty");
        }
        
        if config.buy_condition.is_empty() {
            bail!("Buy condition cannot be empty");
        }
        
        if config.sell_condition.is_empty() {
            bail!("Sell condition cannot be empty");
        }
        
        // Validate pair format (should be like BTC/USDT or BTCUSDT)
        let pair_upper = config.pair.to_uppercase().replace("/", "");
        if !pair_upper.ends_with("USDT") && !pair_upper.ends_with("BTC") && !pair_upper.ends_with("ETH") {
            tracing::warn!("Pair format might be invalid: {}", config.pair);
        }
        
        // Validate timeframe format (should be like 1m, 5m, 1h, 1d, etc.)
        let valid_timeframes = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];
        if !valid_timeframes.contains(&config.timeframe.as_str()) {
            tracing::warn!("Timeframe might be invalid: {}. Valid timeframes: {:?}", config.timeframe, valid_timeframes);
        }
        
        // Validate strategy-specific parameters
        self.validate_strategy_parameters(&config.strategy_type, &config.parameters)?;
        
        Ok(())
    }
    
    /// Validate parameters based on strategy type
    /// This allows different strategy types to have different parameter structures
    fn validate_strategy_parameters(&self, strategy_type: &str, parameters: &Value) -> Result<()> {
        let params = parameters.as_object()
            .context("Parameters must be a JSON object")?;
        
        match strategy_type.to_uppercase().as_str() {
            "RSI" => {
                // RSI should have 'period' parameter
                if let Some(period) = params.get("period") {
                    if let Some(p) = period.as_u64() {
                        if p < 1 || p > 100 {
                            bail!("RSI period must be between 1 and 100, got {}", p);
                        }
                    } else if let Some(p) = period.as_i64() {
                        if p < 1 || p > 100 {
                            bail!("RSI period must be between 1 and 100, got {}", p);
                        }
                    } else {
                        bail!("RSI period must be a positive number");
                    }
                } else {
                    // Default period if not specified
                    tracing::info!("RSI period not specified, will use default: 14");
                }
            }
            "MACD" => {
                // MACD should have fast, slow, signal parameters
                for param_name in &["fast", "slow", "signal"] {
                    if let Some(val) = params.get(*param_name) {
                        if val.as_u64().is_none() && val.as_i64().is_none() {
                            bail!("MACD {} must be a positive number", param_name);
                        }
                    }
                }
                // Validate ranges
                if let Some(fast) = params.get("fast") {
                    let fast_val = fast.as_u64().or_else(|| fast.as_i64().map(|v| v as u64)).unwrap_or(12);
                    if fast_val < 1 || fast_val > 50 {
                        bail!("MACD fast must be between 1 and 50, got {}", fast_val);
                    }
                }
                if let Some(slow) = params.get("slow") {
                    let slow_val = slow.as_u64().or_else(|| slow.as_i64().map(|v| v as u64)).unwrap_or(26);
                    if slow_val < 1 || slow_val > 200 {
                        bail!("MACD slow must be between 1 and 200, got {}", slow_val);
                    }
                }
                if let Some(signal) = params.get("signal") {
                    let signal_val = signal.as_u64().or_else(|| signal.as_i64().map(|v| v as u64)).unwrap_or(9);
                    if signal_val < 1 || signal_val > 50 {
                        bail!("MACD signal must be between 1 and 50, got {}", signal_val);
                    }
                }
            }
            "BOLLINGER" | "BOLLINGER BANDS" | "BB" | "BOLLINGERBANDS" => {
                // Bollinger Bands should have period and std_dev
                if let Some(period) = params.get("period") {
                    let period_val = period.as_u64().or_else(|| period.as_i64().map(|v| v as u64)).unwrap_or(20);
                    if period_val < 1 || period_val > 200 {
                        bail!("Bollinger period must be between 1 and 200, got {}", period_val);
                    }
                }
                if let Some(std_dev) = params.get("std_dev") {
                    if let Some(sd) = std_dev.as_f64() {
                        if sd <= 0.0 || sd > 5.0 {
                            bail!("Bollinger std_dev must be between 0 and 5, got {}", sd);
                        }
                    } else {
                        bail!("Bollinger std_dev must be a number");
                    }
                }
            }
            "EMA" | "MA" | "SMA" => {
                // Moving averages should have period
                if let Some(period) = params.get("period") {
                    let period_val = period.as_u64().or_else(|| period.as_i64().map(|v| v as u64)).unwrap_or(20);
                    if period_val < 1 || period_val > 500 {
                        bail!("{} period must be between 1 and 500, got {}", strategy_type, period_val);
                    }
                }
            }
            _ => {
                // For unknown strategy types, allow flexible parameters but log a warning
                tracing::warn!("Unknown strategy type: {}, allowing flexible parameters", strategy_type);
            }
        }
        
        Ok(())
    }
    
    /// Extract parameters from strategy conditions
    pub fn extract_parameters(&self, algorithm: &str, buy_condition: &str, sell_condition: &str) -> Value {
        let mut params = serde_json::Map::new();
        
        match algorithm.to_uppercase().as_str() {
            "RSI" => {
                // Extract period from conditions like "RSI < 30" or "RSI > 70"
                // Default period is 14
                let period = self.extract_number(buy_condition)
                    .or_else(|| self.extract_number(sell_condition))
                    .unwrap_or(14);
                params.insert("period".to_string(), Value::Number(period.into()));
            }
            "MACD" => {
                params.insert("fast".to_string(), Value::Number(12.into()));
                params.insert("slow".to_string(), Value::Number(26.into()));
                params.insert("signal".to_string(), Value::Number(9.into()));
            }
            "BOLLINGER" | "BOLLINGER BANDS" | "BB" | "BOLLINGERBANDS" => {
                params.insert("period".to_string(), Value::Number(20.into()));
                params.insert("std_dev".to_string(), Value::Number(serde_json::Number::from_f64(2.0).unwrap()));
            }
            "EMA" => {
                let period = self.extract_number(buy_condition)
                    .or_else(|| self.extract_number(sell_condition))
                    .unwrap_or(20);
                params.insert("period".to_string(), Value::Number(period.into()));
            }
            "MA" | "SMA" => {
                let period = self.extract_number(buy_condition)
                    .or_else(|| self.extract_number(sell_condition))
                    .unwrap_or(20);
                params.insert("period".to_string(), Value::Number(period.into()));
            }
            _ => {
                // Default parameters for unknown strategies
                params.insert("period".to_string(), Value::Number(14.into()));
            }
        }
        
        Value::Object(params)
    }
    
    /// Extract number from a string (for extracting thresholds)
    fn extract_number(&self, text: &str) -> Option<i64> {
        // Simple regex-like extraction - find numbers after operators
        for part in text.split_whitespace() {
            if let Ok(num) = part.parse::<i64>() {
                return Some(num);
            }
        }
        None
    }
}
