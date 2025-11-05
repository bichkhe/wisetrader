//! Data storage and retrieval

use crate::data::Candle;
use crate::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// In-memory data storage
#[derive(Debug, Default)]
pub struct DataStorage {
    /// Store candles by symbol and timeframe
    candles: HashMap<String, Vec<Candle>>,
}

impl DataStorage {
    /// Create new storage
    pub fn new() -> Self {
        Self {
            candles: HashMap::new(),
        }
    }

    /// Get storage key from symbol and timeframe
    fn key(symbol: &str, timeframe: &str) -> String {
        format!("{}:{}", symbol, timeframe)
    }

    /// Add a candle
    pub fn add_candle(&mut self, candle: Candle) {
        let key = Self::key(&candle.symbol, &candle.timeframe);
        self.candles.entry(key).or_insert_with(Vec::new).push(candle);
    }

    /// Add multiple candles
    pub fn add_candles(&mut self, candles: Vec<Candle>) {
        for candle in candles {
            self.add_candle(candle);
        }
    }

    /// Get candles for symbol and timeframe
    pub fn get_candles(&self, symbol: &str, timeframe: &str) -> Option<&Vec<Candle>> {
        let key = Self::key(symbol, timeframe);
        self.candles.get(&key)
    }

    /// Get candles within time range
    pub fn get_candles_range(
        &self,
        symbol: &str,
        timeframe: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&Candle> {
        if let Some(candles) = self.get_candles(symbol, timeframe) {
            candles
                .iter()
                .filter(|c| c.timestamp >= start && c.timestamp <= end)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get latest candle
    pub fn get_latest_candle(&self, symbol: &str, timeframe: &str) -> Option<&Candle> {
        self.get_candles(symbol, timeframe)?.last()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.candles.clear();
    }

    /// Get number of stored candles
    pub fn len(&self) -> usize {
        self.candles.values().map(|v| v.len()).sum()
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.candles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage() {
        let mut storage = DataStorage::new();
        let candle = Candle::new(
            100.0, 110.0, 95.0, 105.0, 1000.0,
            Utc::now(),
            "BTC/USDT".to_string(),
            "5m".to_string(),
        );
        
        storage.add_candle(candle);
        assert_eq!(storage.len(), 1);
        
        let candles = storage.get_candles("BTC/USDT", "5m");
        assert!(candles.is_some());
        assert_eq!(candles.unwrap().len(), 1);
    }
}

