//! Freqtrade-RS: A Rust implementation of Freqtrade trading bot
//!
//! This crate provides a high-performance trading bot framework using:
//! - [barter-rs](https://github.com/barter-rs/barter) for exchange integration
//! - [ta-rs](https://github.com/greyblake/ta-rs) for technical analysis
//!
//! # Features
//!
//! - **Data Management**: OHLCV candle data fetching and storage
//! - **Technical Indicators**: RSI, MACD, EMA, SMA, BB, etc.
//! - **Strategy Engine**: Strategy definition and execution
//! - **Backtesting**: Historical backtesting with performance metrics
//! - **Portfolio Management**: Position tracking and risk management
//! - **Exchange Integration**: Multi-exchange support via barter-rs
//!
//! # Example
//!
//! ```no_run
//! use freqtrade_rs::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = ExchangeClient::new("binance").await?;
//!     let strategy = RSIStrategy::new(RSIStrategyConfig::default());
//!     let engine = TradingEngine::new(client, strategy);
//!     engine.run().await?;
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod data;
pub mod exchange;
pub mod indicators;
pub mod portfolio;
pub mod strategy;
pub mod backtest;

// Re-export commonly used types
pub mod prelude {
    pub use crate::config::*;
    pub use crate::data::*;
    pub use crate::exchange::*;
    pub use crate::indicators::*;
    pub use crate::portfolio::*;
    pub use crate::strategy::*;
    pub use crate::backtest::*;
    
    pub use anyhow::{Result, Context};
    // pub use barter::exchange::Exchange;  // Comment out until barter crate is added
    // pub use barter_data::Subscription;  // Comment out if not available
}

/// Result type alias
pub type Result<T> = anyhow::Result<T>;

