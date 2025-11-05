//! Risk management

use crate::config::RiskConfig;
use crate::portfolio::{Balance, Position};

/// Risk manager
#[derive(Debug)]
pub struct RiskManager {
    config: RiskConfig,
}

impl RiskManager {
    /// Create new risk manager
    pub fn new(config: RiskConfig) -> Self {
        Self { config }
    }

    /// Check if position size is within limits
    pub fn can_open_position(
        &self,
        balance: &Balance,
        position_value: f64,
        current_positions: usize,
    ) -> bool {
        // Check max position size
        let max_position_value = balance.total * self.config.max_position_size;
        if position_value > max_position_value {
            return false;
        }

        // Check max open positions
        if current_positions >= self.config.max_open_positions {
            return false;
        }

        // Check if can afford
        if !balance.can_afford(position_value) {
            return false;
        }

        true
    }

    /// Calculate position size based on risk
    pub fn calculate_position_size(
        &self,
        balance: &Balance,
        entry_price: f64,
        stop_loss: f64,
        risk_per_trade: f64, // Risk as percentage of balance (e.g., 0.01 = 1%)
    ) -> f64 {
        let risk_amount = balance.total * risk_per_trade;
        let price_risk = (entry_price - stop_loss).abs();
        
        if price_risk == 0.0 {
            return 0.0;
        }

        let quantity = risk_amount / price_risk;
        let max_quantity = (balance.total * self.config.max_position_size) / entry_price;
        
        quantity.min(max_quantity)
    }

    /// Check if daily loss limit is exceeded
    pub fn is_daily_loss_exceeded(&self, starting_balance: f64, current_balance: f64) -> bool {
        let loss = (starting_balance - current_balance) / starting_balance;
        loss > self.config.max_daily_loss
    }

    /// Check if drawdown limit is exceeded
    pub fn is_drawdown_exceeded(
        &self,
        peak_balance: f64,
        current_balance: f64,
    ) -> bool {
        let drawdown = (peak_balance - current_balance) / peak_balance;
        drawdown > self.config.max_drawdown
    }
}

