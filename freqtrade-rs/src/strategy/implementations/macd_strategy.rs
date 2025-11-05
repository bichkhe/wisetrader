//! MACD Strategy implementation

use crate::data::Candle;
use crate::indicators::{MACD, Indicator};
use crate::strategy::{Strategy, Signal, SignalType};
use crate::Result;
use tracing::{debug, info, warn};

/// MACD Strategy configuration
#[derive(Debug, Clone)]
pub struct MACDStrategyConfig {
    /// MACD fast period
    pub fast_period: usize,
    /// MACD slow period
    pub slow_period: usize,
    /// MACD signal period
    pub signal_period: usize,
    /// Minimum confidence for signal
    pub min_confidence: f64,
    /// Minimum histogram value for signal
    pub min_histogram: f64,
}

impl Default for MACDStrategyConfig {
    fn default() -> Self {
        Self {
            fast_period: 12,
            slow_period: 26,
            signal_period: 9,
            min_confidence: 0.6,
            min_histogram: 0.0,
        }
    }
}

/// MACD-based trading strategy
pub struct MACDStrategy {
    config: MACDStrategyConfig,
    macd: MACD,
    candles: Vec<Candle>,
    is_initialized: bool,
    last_histogram: Option<f64>,
}

impl MACDStrategy {
    /// Create new MACD strategy
    pub fn new(config: MACDStrategyConfig) -> Self {
        Self {
            macd: MACD::new(config.fast_period, config.slow_period, config.signal_period),
            config,
            candles: Vec::new(),
            is_initialized: false,
            last_histogram: None,
        }
    }

    /// Calculate signal confidence based on MACD histogram
    fn calculate_confidence(&self, histogram: f64) -> f64 {
        // Normalize histogram to confidence (0.0 to 1.0)
        // Use absolute value and scale
        let abs_histogram = histogram.abs();
        let max_histogram = abs_histogram.max(1.0); // Prevent division by zero
        (abs_histogram / max_histogram).min(1.0)
    }

    /// Detect MACD crossover
    fn detect_crossover(&self, current_histogram: f64, previous_histogram: Option<f64>) -> Option<CrossoverType> {
        if let Some(prev) = previous_histogram {
            // Bullish crossover: histogram crosses from negative to positive
            if prev < 0.0 && current_histogram > 0.0 {
                return Some(CrossoverType::Bullish);
            }
            // Bearish crossover: histogram crosses from positive to negative
            if prev > 0.0 && current_histogram < 0.0 {
                return Some(CrossoverType::Bearish);
            }
        }
        None
    }
}

/// MACD crossover type
#[derive(Debug, Clone, Copy, PartialEq)]
enum CrossoverType {
    Bullish,
    Bearish,
}

impl Strategy for MACDStrategy {
    fn name(&self) -> &str {
        "MACD Strategy"
    }

    fn initialize(&mut self, candles: &[Candle]) -> Result<()> {
        info!(
            "Initializing MACD Strategy with {} historical candles",
            candles.len()
        );

        // Process historical candles to initialize MACD
        for candle in candles {
            self.macd.update(candle.close);
            self.candles.push(candle.clone());
        }

        // Get initial histogram value
        if self.macd.is_ready() {
            self.last_histogram = self.macd.histogram();
        }

        self.is_initialized = true;
        info!(
            "MACD Strategy initialized. MACD ready: {}",
            self.macd.is_ready()
        );

        Ok(())
    }

    fn process(&mut self, candle: &Candle) -> Result<Signal> {
        // Update MACD with new candle close price
        self.macd.update(candle.close);
        self.candles.push(candle.clone());

        // Keep only recent candles (last 1000)
        if self.candles.len() > 1000 {
            self.candles.remove(0);
        }

        // Check if MACD is ready
        if !self.macd.is_ready() {
            debug!("MACD not ready yet, holding position");
            return Ok(Signal::hold("MACD indicator not ready".to_string()));
        }

        let macd_value = self.macd.macd().unwrap();
        let signal_value = self.macd.signal().unwrap();
        let histogram = self.macd.histogram().unwrap();

        debug!(
            "MACD: {:.4}, Signal: {:.4}, Histogram: {:.4}",
            macd_value, signal_value, histogram
        );

        // Detect crossover
        let crossover = self.detect_crossover(histogram, self.last_histogram);
        self.last_histogram = Some(histogram);

        // Calculate confidence
        let confidence = self.calculate_confidence(histogram);

        // Generate signal based on MACD crossover and histogram
        if let Some(crossover_type) = crossover {
            match crossover_type {
                CrossoverType::Bullish => {
                    if confidence >= self.config.min_confidence
                        && histogram.abs() >= self.config.min_histogram
                    {
                        let signal = Signal::buy(
                            candle.close,
                            confidence,
                            format!("MACD bullish crossover: histogram={:.4}", histogram),
                        )
                        .with_stop_loss(candle.close * 0.95) // 5% stop loss
                        .with_take_profit(candle.close * 1.05); // 5% take profit

                        info!(
                            "BUY signal generated: price={:.2}, MACD={:.4}, histogram={:.4}, confidence={:.2}",
                            candle.close, macd_value, histogram, confidence
                        );

                        return Ok(signal);
                    }
                }
                CrossoverType::Bearish => {
                    if confidence >= self.config.min_confidence
                        && histogram.abs() >= self.config.min_histogram
                    {
                        let signal = Signal::sell(
                            candle.close,
                            confidence,
                            format!("MACD bearish crossover: histogram={:.4}", histogram),
                        );

                        info!(
                            "SELL signal generated: price={:.2}, MACD={:.4}, histogram={:.4}, confidence={:.2}",
                            candle.close, macd_value, histogram, confidence
                        );

                        return Ok(signal);
                    }
                }
            }
        }

        // Hold position if no crossover or low confidence
        Ok(Signal::hold(format!(
            "MACD in neutral zone: histogram={:.4}",
            histogram
        )))
    }

    fn is_ready(&self) -> bool {
        self.is_initialized && self.macd.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Candle;
    use chrono::Utc;

    fn create_test_candle(close: f64, timestamp: chrono::DateTime<Utc>) -> Candle {
        Candle::new(
            close,
            close + 1.0,
            close - 1.0,
            close,
            1000.0,
            timestamp,
            "BTC/USDT".to_string(),
            "5m".to_string(),
        )
    }

    #[test]
    fn test_macd_strategy_initialization() {
        let mut strategy = MACDStrategy::new(MACDStrategyConfig::default());

        // Initialize with enough candles (need at least slow_period + signal_period)
        let mut candles = Vec::new();
        let base_time = Utc::now();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1);
            candles.push(create_test_candle(price, base_time + chrono::Duration::minutes(i)));
        }

        strategy.initialize(&candles).unwrap();
        assert!(strategy.is_initialized);
    }

    #[test]
    fn test_macd_crossover_detection() {
        let config = MACDStrategyConfig::default();
        let strategy = MACDStrategy::new(config);

        // Test bullish crossover
        let bullish = strategy.detect_crossover(1.0, Some(-0.5));
        assert_eq!(bullish, Some(CrossoverType::Bullish));

        // Test bearish crossover
        let bearish = strategy.detect_crossover(-1.0, Some(0.5));
        assert_eq!(bearish, Some(CrossoverType::Bearish));

        // Test no crossover
        let none = strategy.detect_crossover(1.0, Some(0.5));
        assert_eq!(none, None);
    }
}

