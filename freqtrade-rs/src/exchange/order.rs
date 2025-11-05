//! Order management

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// Market order
    Market,
    /// Limit order
    Limit,
    /// Stop order
    Stop,
    /// Stop limit order
    StopLimit,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    /// Buy
    Buy,
    /// Sell
    Sell,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Pending
    Pending,
    /// Filled
    Filled,
    /// Partially filled
    PartiallyFilled,
    /// Cancelled
    Cancelled,
    /// Rejected
    Rejected,
}

/// Order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Order ID
    pub id: String,
    /// Symbol
    pub symbol: String,
    /// Order type
    pub order_type: OrderType,
    /// Order side
    pub side: OrderSide,
    /// Quantity
    pub quantity: f64,
    /// Price (for limit orders)
    pub price: Option<f64>,
    /// Filled quantity
    pub filled_quantity: f64,
    /// Average fill price
    pub avg_fill_price: Option<f64>,
    /// Status
    pub status: OrderStatus,
    /// Created time
    pub created_at: DateTime<Utc>,
    /// Updated time
    pub updated_at: DateTime<Utc>,
}

impl Order {
    /// Create new order
    pub fn new(
        id: String,
        symbol: String,
        order_type: OrderType,
        side: OrderSide,
        quantity: f64,
        price: Option<f64>,
    ) -> Self {
        Self {
            id,
            symbol,
            order_type,
            side,
            quantity,
            price,
            filled_quantity: 0.0,
            avg_fill_price: None,
            status: OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Check if order is filled
    pub fn is_filled(&self) -> bool {
        self.status == OrderStatus::Filled
    }

    /// Check if order is active (pending or partially filled)
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Pending | OrderStatus::PartiallyFilled
        )
    }
}

