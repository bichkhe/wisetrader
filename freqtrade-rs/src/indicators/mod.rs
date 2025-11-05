//! Technical indicators module
//!
//! Provides technical analysis indicators using the `ta` crate.

pub mod rsi;
pub mod macd;
pub mod ema;
pub mod sma;
pub mod bb;

pub use rsi::*;
pub use macd::*;
pub use ema::*;
pub use sma::*;
pub use bb::*;

/// Indicator trait for all indicators
pub trait Indicator {
    /// Get the name of the indicator
    fn name(&self) -> &str;
    
    /// Update indicator with new value
    fn update(&mut self, value: f64);
    
    /// Get current indicator value
    fn value(&self) -> Option<f64>;
    
    /// Check if indicator is ready (has enough data)
    fn is_ready(&self) -> bool;
}

