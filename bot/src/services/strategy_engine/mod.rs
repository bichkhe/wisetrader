//! Strategy Engine Module
//! 
//! This module provides a flexible strategy engine where:
//! - Each strategy is a trait implementation
//! - Users can select strategies from their created strategies
//! - Live trading runs the selected strategy for each user independently

pub mod strategy;
pub mod registry;
pub mod executor;
pub mod implementations;
pub mod indicator_configs;

pub use strategy::{Strategy, StrategyConfig, StrategySignal, Candle, parse_condition};
pub use registry::StrategyRegistry;
pub use executor::StrategyExecutor;
pub use indicator_configs::{IndicatorConfigRegistry, IndicatorConfig};

