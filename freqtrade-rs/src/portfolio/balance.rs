//! Balance management

use serde::{Deserialize, Serialize};

/// Account balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    /// Total balance
    pub total: f64,
    /// Available balance (not in positions)
    pub available: f64,
    /// Balance in positions
    pub in_positions: f64,
}

impl Balance {
    /// Create new balance
    pub fn new(total: f64) -> Self {
        Self {
            total,
            available: total,
            in_positions: 0.0,
        }
    }

    /// Update balance
    pub fn update(&mut self, total: f64, in_positions: f64) {
        self.total = total;
        self.in_positions = in_positions;
        self.available = total - in_positions;
    }

    /// Check if can afford amount
    pub fn can_afford(&self, amount: f64) -> bool {
        self.available >= amount
    }
}

