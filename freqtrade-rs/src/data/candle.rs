//! OHLCV candle data structures

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// OHLCV candle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    /// Opening price
    pub open: f64,
    /// High price
    pub high: f64,
    /// Low price
    pub low: f64,
    /// Closing price
    pub close: f64,
    /// Volume
    pub volume: f64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Symbol (e.g., "BTC/USDT")
    pub symbol: String,
    /// Timeframe (e.g., "5m", "1h", "1d")
    pub timeframe: String,
}

impl Candle {
    /// Create a new candle
    pub fn new(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        timestamp: DateTime<Utc>,
        symbol: String,
        timeframe: String,
    ) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
            timestamp,
            symbol,
            timeframe,
        }
    }

    /// Convert from barter MarketEvent
    /// Note: MarketEvent structure may vary by barter-data version
    pub fn from_barter_event(event: &barter_data::event::MarketEvent) -> Self {
        use barter_data::event::DataKind;
        
        // Extract market info - MarketEvent in barter-data v0.10 structure
        // For now, use placeholder values - will be populated when event is processed
        let symbol = "UNKNOWN/USDT".to_string();
        let timeframe = "1m".to_string();
        
        match &event.kind {
            DataKind::Candle(candle) => {
                Self {
                    open: candle.open,
                    high: candle.high,
                    low: candle.low,
                    close: candle.close,
                    volume: candle.volume,
                    timestamp: event.time_received,
                    symbol,
                    timeframe,
                }
            }
            _ => {
                // Fallback: create candle from trade data if available
                Self {
                    open: 0.0,
                    high: 0.0,
                    low: 0.0,
                    close: 0.0,
                    volume: 0.0,
                    timestamp: event.time_received,
                    symbol,
                    timeframe,
                }
            }
        }
    }

    /// Get typical price (HLC/3)
    pub fn typical_price(&self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }

    /// Get weighted close price (HLCV/4)
    pub fn weighted_close(&self) -> f64 {
        (self.high + self.low + self.close + self.close) / 4.0
    }

    /// Get median price (HL/2)
    pub fn median_price(&self) -> f64 {
        (self.high + self.low) / 2.0
    }

    /// Check if candle is bullish
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Check if candle is bearish
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Get body size (absolute difference between open and close)
    pub fn body_size(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Get upper wick size
    pub fn upper_wick(&self) -> f64 {
        self.high - self.open.max(self.close)
    }

    /// Get lower wick size
    pub fn lower_wick(&self) -> f64 {
        self.open.min(self.close) - self.low
    }

    /// Get total range (high - low)
    pub fn range(&self) -> f64 {
        self.high - self.low
    }
}

/// Collection of candles
#[derive(Debug, Clone)]
pub struct CandleSeries {
    candles: Vec<Candle>,
}

impl CandleSeries {
    /// Create new empty series
    pub fn new() -> Self {
        Self {
            candles: Vec::new(),
        }
    }

    /// Create from vector of candles
    pub fn from_vec(candles: Vec<Candle>) -> Self {
        Self { candles }
    }

    /// Add a candle
    pub fn push(&mut self, candle: Candle) {
        self.candles.push(candle);
    }

    /// Get number of candles
    pub fn len(&self) -> usize {
        self.candles.len()
    }

    /// Check if series is empty
    pub fn is_empty(&self) -> bool {
        self.candles.is_empty()
    }

    /// Get candle at index
    pub fn get(&self, index: usize) -> Option<&Candle> {
        self.candles.get(index)
    }

    /// Get last candle
    pub fn last(&self) -> Option<&Candle> {
        self.candles.last()
    }

    /// Get all candles
    pub fn candles(&self) -> &[Candle] {
        &self.candles
    }

    /// Get candles as mutable slice
    pub fn candles_mut(&mut self) -> &mut [Candle] {
        &mut self.candles
    }

    /// Get close prices as vector
    pub fn closes(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.close).collect()
    }

    /// Get open prices as vector
    pub fn opens(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.open).collect()
    }

    /// Get high prices as vector
    pub fn highs(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.high).collect()
    }

    /// Get low prices as vector
    pub fn lows(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.low).collect()
    }

    /// Get volumes as vector
    pub fn volumes(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.volume).collect()
    }

    /// Reverse the series (oldest first -> newest first)
    pub fn reverse(&mut self) {
        self.candles.reverse();
    }

    /// Sort by timestamp (oldest first)
    pub fn sort_by_time(&mut self) {
        self.candles.sort_by_key(|c| c.timestamp);
    }
}

impl Default for CandleSeries {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<Candle>> for CandleSeries {
    fn from(candles: Vec<Candle>) -> Self {
        Self::from_vec(candles)
    }
}

