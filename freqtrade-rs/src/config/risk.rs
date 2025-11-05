//! Risk management configuration

use serde::{Deserialize, Serialize};

/// Risk management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Maximum position size (as percentage of balance, e.g., 0.1 = 10%)
    pub max_position_size: f64,
    /// Maximum number of open positions
    pub max_open_positions: usize,
    /// Maximum daily loss (as percentage, e.g., 0.05 = 5%)
    pub max_daily_loss: f64,
    /// Maximum drawdown (as percentage, e.g., 0.20 = 20%)
    pub max_drawdown: f64,
    /// Use stop loss
    pub use_stop_loss: bool,
    /// Use take profit
    pub use_take_profit: bool,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_size: 0.1, // 10% of balance
            max_open_positions: 3,
            max_daily_loss: 0.05, // 5%
            max_drawdown: 0.20, // 20%
            use_stop_loss: true,
            use_take_profit: true,
        }
    }
}

