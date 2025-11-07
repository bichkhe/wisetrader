//! Strategy Registry - manages available strategies

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use serde_json::Value;
use crate::services::strategy_engine::{
    Strategy, StrategyConfig, 
    implementations::{
        RsiStrategy, MacdStrategy, BollingerStrategy, 
        EmaStrategy, MaStrategy, StochasticStrategy, AdxStrategy,
    },
};

pub type StrategyFactory = Box<dyn Fn(StrategyConfig) -> Result<Box<dyn Strategy>> + Send + Sync>;

/// Strategy Registry - manages strategy types and their factories
pub struct StrategyRegistry {
    factories: HashMap<String, StrategyFactory>,
}

impl StrategyRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };
        
        // Register built-in strategies
        registry.register_strategy("RSI", |config| {
            let period = config.parameters
                .get("period")
                .and_then(|v| v.as_u64())
                .unwrap_or(14) as usize;
            Ok(Box::new(RsiStrategy::new(config, period)?))
        });
        
        registry.register_strategy("MACD", |config| {
            let fast = config.parameters
                .get("fast")
                .and_then(|v| v.as_u64())
                .unwrap_or(12) as usize;
            let slow = config.parameters
                .get("slow")
                .and_then(|v| v.as_u64())
                .unwrap_or(26) as usize;
            let signal = config.parameters
                .get("signal")
                .and_then(|v| v.as_u64())
                .unwrap_or(9) as usize;
            Ok(Box::new(MacdStrategy::new(config, fast, slow, signal)?))
        });
        
        registry.register_strategy("Bollinger", |config| {
            let period = config.parameters
                .get("period")
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;
            let std_dev = config.parameters
                .get("std_dev")
                .and_then(|v| v.as_f64())
                .unwrap_or(2.0);
            Ok(Box::new(BollingerStrategy::new(config, period, std_dev)?))
        });
        
        registry.register_strategy("EMA", |config| {
            let period = config.parameters
                .get("period")
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;
            Ok(Box::new(EmaStrategy::new(config, period)?))
        });
        
        registry.register_strategy("MA", |config| {
            let period = config.parameters
                .get("period")
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;
            Ok(Box::new(MaStrategy::new(config, period)?))
        });
        
        registry.register_strategy("STOCHASTIC", |config| {
            let period = config.parameters
                .get("period")
                .and_then(|v| v.as_u64())
                .unwrap_or(14) as usize;
            let smooth_k = config.parameters
                .get("smooth_k")
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as usize;
            let smooth_d = config.parameters
                .get("smooth_d")
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as usize;
            Ok(Box::new(StochasticStrategy::new(config, period, smooth_k, smooth_d)?))
        });
        
        registry.register_strategy("ADX", |config| {
            let period = config.parameters
                .get("period")
                .and_then(|v| v.as_u64())
                .unwrap_or(14) as usize;
            Ok(Box::new(AdxStrategy::new(config, period)?))
        });
        
        registry
    }
    
    /// Register a strategy factory
    pub fn register_strategy<F>(&mut self, name: &str, factory: F)
    where
        F: Fn(StrategyConfig) -> Result<Box<dyn Strategy>> + Send + Sync + 'static,
    {
        self.factories.insert(name.to_string(), Box::new(factory));
    }
    
    /// Create a strategy instance from config
    pub fn create_strategy(&self, config: StrategyConfig) -> Result<Box<dyn Strategy>> {
        let strategy_type = config.strategy_type.to_uppercase();
        let factory = self.factories
            .get(&strategy_type)
            .ok_or_else(|| anyhow::anyhow!("Unknown strategy type: {}", strategy_type))?;
        
        factory(config)
    }
    
    /// Get list of available strategy types
    pub fn get_available_strategies(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }
    
    /// Check if a strategy type is available
    pub fn has_strategy(&self, strategy_type: &str) -> bool {
        self.factories.contains_key(&strategy_type.to_uppercase())
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

