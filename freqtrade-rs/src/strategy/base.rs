//! Base strategy trait and common strategy implementations

use crate::data::Candle;
use crate::Result;

/// Base trait for all trading strategies
pub trait Strategy {
    /// Get strategy name
    fn name(&self) -> &str;
    
    /// Initialize strategy with historical candles
    fn initialize(&mut self, candles: &[Candle]) -> Result<()>;
    
    /// Process new candle and generate signal
    fn process(&mut self, candle: &Candle) -> Result<Signal>;
    
    /// Check if strategy is ready (has enough data)
    fn is_ready(&self) -> bool;
}

/// Buy signal type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalType {
    /// Buy/Long signal
    Buy,
    /// Sell/Short signal
    Sell,
    /// Hold/No action
    Hold,
}

/// Trading signal
#[derive(Debug, Clone)]
pub struct Signal {
    /// Signal type
    pub signal_type: SignalType,
    /// Entry price (if buy)
    pub entry_price: Option<f64>,
    /// Stop loss price
    pub stop_loss: Option<f64>,
    /// Take profit price
    pub take_profit: Option<f64>,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Reason for signal
    pub reason: String,
}

impl Signal {
    /// Create buy signal
    pub fn buy(entry_price: f64, confidence: f64, reason: String) -> Self {
        Self {
            signal_type: SignalType::Buy,
            entry_price: Some(entry_price),
            stop_loss: None,
            take_profit: None,
            confidence,
            reason,
        }
    }

    /// Create sell signal
    pub fn sell(entry_price: f64, confidence: f64, reason: String) -> Self {
        Self {
            signal_type: SignalType::Sell,
            entry_price: Some(entry_price),
            stop_loss: None,
            take_profit: None,
            confidence,
            reason,
        }
    }

    /// Create hold signal
    pub fn hold(reason: String) -> Self {
        Self {
            signal_type: SignalType::Hold,
            entry_price: None,
            stop_loss: None,
            take_profit: None,
            confidence: 0.0,
            reason,
        }
    }

    /// Set stop loss
    pub fn with_stop_loss(mut self, stop_loss: f64) -> Self {
        self.stop_loss = Some(stop_loss);
        self
    }

    /// Set take profit
    pub fn with_take_profit(mut self, take_profit: f64) -> Self {
        self.take_profit = Some(take_profit);
        self
    }
}

