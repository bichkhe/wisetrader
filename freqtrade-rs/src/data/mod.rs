//! Data management module
//!
//! Handles OHLCV candle data fetching, storage, and validation.

pub mod candle;
pub mod storage;

pub use candle::*;
pub use storage::*;

