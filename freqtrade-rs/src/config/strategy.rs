//! Strategy configuration

use serde::{Deserialize, Serialize};

/// Strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Strategy name
    pub name: String,
    /// Timeframe (e.g., "5m", "1h", "1d")
    pub timeframe: String,
    /// Minimum ROI (e.g., 0.01 = 1%)
    pub minimal_roi: f64,
    /// Stop loss (e.g., -0.10 = -10%)
    pub stoploss: f64,
    /// Trailing stop enabled
    pub trailing_stop: bool,
    /// Trailing stop positive
    pub trailing_stop_positive: f64,
    /// Trailing stop offset
    pub trailing_stop_offset: f64,
    /// Startup candle count
    pub startup_candle_count: usize,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            name: "DefaultStrategy".to_string(),
            timeframe: "5m".to_string(),
            minimal_roi: 0.01,
            stoploss: -0.10,
            trailing_stop: false,
            trailing_stop_positive: 0.02,
            trailing_stop_offset: 0.01,
            startup_candle_count: 200,
        }
    }
}

