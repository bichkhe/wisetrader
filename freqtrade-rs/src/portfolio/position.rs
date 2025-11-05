//! Position tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Trading position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Position ID
    pub id: String,
    /// Symbol (e.g., "BTC/USDT")
    pub symbol: String,
    /// Position side (Long/Short)
    pub side: PositionSide,
    /// Entry price
    pub entry_price: f64,
    /// Current price
    pub current_price: f64,
    /// Quantity
    pub quantity: f64,
    /// Stop loss price
    pub stop_loss: Option<f64>,
    /// Take profit price
    pub take_profit: Option<f64>,
    /// Entry time
    pub entry_time: DateTime<Utc>,
    /// Unrealized P&L
    pub unrealized_pnl: f64,
    /// Unrealized P&L percentage
    pub unrealized_pnl_percent: f64,
}

/// Position side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionSide {
    /// Long position
    Long,
    /// Short position
    Short,
}

impl Position {
    /// Create new position
    pub fn new(
        id: String,
        symbol: String,
        side: PositionSide,
        entry_price: f64,
        quantity: f64,
    ) -> Self {
        Self {
            id,
            symbol,
            side,
            entry_price,
            current_price: entry_price,
            quantity,
            stop_loss: None,
            take_profit: None,
            entry_time: Utc::now(),
            unrealized_pnl: 0.0,
            unrealized_pnl_percent: 0.0,
        }
    }

    /// Update current price and calculate P&L
    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
        self.calculate_pnl();
    }

    /// Calculate P&L
    fn calculate_pnl(&mut self) {
        match self.side {
            PositionSide::Long => {
                self.unrealized_pnl = (self.current_price - self.entry_price) * self.quantity;
                self.unrealized_pnl_percent =
                    ((self.current_price - self.entry_price) / self.entry_price) * 100.0;
            }
            PositionSide::Short => {
                self.unrealized_pnl = (self.entry_price - self.current_price) * self.quantity;
                self.unrealized_pnl_percent =
                    ((self.entry_price - self.current_price) / self.entry_price) * 100.0;
            }
        }
    }

    /// Check if stop loss is hit
    pub fn is_stop_loss_hit(&self) -> bool {
        if let Some(stop_loss) = self.stop_loss {
            match self.side {
                PositionSide::Long => self.current_price <= stop_loss,
                PositionSide::Short => self.current_price >= stop_loss,
            }
        } else {
            false
        }
    }

    /// Check if take profit is hit
    pub fn is_take_profit_hit(&self) -> bool {
        if let Some(take_profit) = self.take_profit {
            match self.side {
                PositionSide::Long => self.current_price >= take_profit,
                PositionSide::Short => self.current_price <= take_profit,
            }
        } else {
            false
        }
    }

    /// Get position value
    pub fn value(&self) -> f64 {
        self.current_price * self.quantity
    }

    /// Get entry value
    pub fn entry_value(&self) -> f64 {
        self.entry_price * self.quantity
    }

    /// Set stop loss price
    pub fn set_stop_loss(&mut self, stop_loss: f64) {
        self.stop_loss = Some(stop_loss);
    }

    /// Set take profit price
    pub fn set_take_profit(&mut self, take_profit: f64) {
        self.take_profit = Some(take_profit);
    }
}

