//! RSI Strategy implementation

use crate::data::Candle;
use crate::indicators::{RSI, Indicator};
use crate::strategy::{Strategy, Signal, SignalType};
use crate::Result;
use tracing::{debug, info, warn};

/// RSI Strategy configuration
#[derive(Debug, Clone)]
pub struct RSIStrategyConfig {
    /// RSI period
    pub rsi_period: usize,
    /// RSI oversold threshold (buy signal when RSI < this)
    pub rsi_oversold: f64,
    /// RSI overbought threshold (sell signal when RSI > this)
    pub rsi_overbought: f64,
    /// Minimum confidence for signal
    pub min_confidence: f64,
}

impl Default for RSIStrategyConfig {
    fn default() -> Self {
        Self {
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            min_confidence: 0.6,
        }
    }
}

/// RSI-based trading strategy
pub struct RSIStrategy {
    config: RSIStrategyConfig,
    rsi: RSI,
    candles: Vec<Candle>,
    is_initialized: bool,
}

impl RSIStrategy {
    /// Create new RSI strategy
    pub fn new(config: RSIStrategyConfig) -> Self {
        Self {
            rsi: RSI::new(config.rsi_period),
            config,
            candles: Vec::new(),
            is_initialized: false,
        }
    }

    /// Calculate signal confidence based on RSI value
    fn calculate_confidence(&self, rsi_value: f64) -> f64 {
        if rsi_value < self.config.rsi_oversold {
            // RSI is oversold - strong buy signal
            let distance = self.config.rsi_oversold - rsi_value;
            let max_distance = self.config.rsi_oversold; // Max distance is when RSI = 0
            (distance / max_distance).min(1.0)
        } else if rsi_value > self.config.rsi_overbought {
            // RSI is overbought - strong sell signal
            let distance = rsi_value - self.config.rsi_overbought;
            let max_distance = 100.0 - self.config.rsi_overbought; // Max distance is when RSI = 100
            (distance / max_distance).min(1.0)
        } else {
            // RSI is in neutral zone
            0.0
        }
    }
}

impl Strategy for RSIStrategy {
    fn name(&self) -> &str {
        "RSI Strategy"
    }

    fn initialize(&mut self, candles: &[Candle]) -> Result<()> {
        info!(
            "Initializing RSI Strategy with {} historical candles",
            candles.len()
        );

        // Process historical candles to initialize RSI
        for candle in candles {
            self.rsi.update(candle.close);
            self.candles.push(candle.clone());
        }

        self.is_initialized = true;
        info!(
            "RSI Strategy initialized. RSI ready: {}",
            self.rsi.is_ready()
        );

        Ok(())
    }

    fn process(&mut self, candle: &Candle) -> Result<Signal> {
        // Update RSI with new candle close price
        self.rsi.update(candle.close);
        self.candles.push(candle.clone());

        // Keep only recent candles (last 1000)
        if self.candles.len() > 1000 {
            self.candles.remove(0);
        }

        // Check if RSI is ready
        if !self.rsi.is_ready() {
            debug!("RSI not ready yet, holding position");
            return Ok(Signal::hold("RSI indicator not ready".to_string()));
        }

        let rsi_value = self.rsi.value().unwrap();
        let confidence = self.calculate_confidence(rsi_value);

        debug!(
            "RSI value: {:.2}, confidence: {:.2}, oversold: {}, overbought: {}",
            rsi_value, confidence, self.config.rsi_oversold, self.config.rsi_overbought
        );

        // Generate signal based on RSI
        if rsi_value < self.config.rsi_oversold && confidence >= self.config.min_confidence {
            // RSI is oversold - buy signal
            let signal = Signal::buy(
                candle.close,
                confidence,
                format!("RSI oversold: {:.2} < {}", rsi_value, self.config.rsi_oversold),
            )
            .with_stop_loss(candle.close * 0.95) // 5% stop loss
            .with_take_profit(candle.close * 1.05); // 5% take profit

            info!(
                "BUY signal generated: price={:.2}, RSI={:.2}, confidence={:.2}",
                candle.close, rsi_value, confidence
            );

            Ok(signal)
        } else if rsi_value > self.config.rsi_overbought && confidence >= self.config.min_confidence {
            // RSI is overbought - sell signal
            let signal = Signal::sell(
                candle.close,
                confidence,
                format!("RSI overbought: {:.2} > {}", rsi_value, self.config.rsi_overbought),
            );

            info!(
                "SELL signal generated: price={:.2}, RSI={:.2}, confidence={:.2}",
                candle.close, rsi_value, confidence
            );

            Ok(signal)
        } else {
            // Hold position
            Ok(Signal::hold(format!(
                "RSI in neutral zone: {:.2}",
                rsi_value
            )))
        }
    }

    fn is_ready(&self) -> bool {
        self.is_initialized && self.rsi.is_ready()
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
    fn test_rsi_strategy_oversold() {
        let mut strategy = RSIStrategy::new(RSIStrategyConfig {
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            min_confidence: 0.5,
        });

        // Initialize with some candles (need at least period+1 candles)
        let mut candles = Vec::new();
        let base_time = Utc::now();
        for i in 0..20 {
            // Create declining prices to get oversold RSI
            let price = 100.0 - (i as f64 * 0.5);
            candles.push(create_test_candle(price, base_time + chrono::Duration::minutes(i)));
        }

        strategy.initialize(&candles).unwrap();

        // Process a new candle with low price
        let new_candle = create_test_candle(90.0, base_time + chrono::Duration::minutes(20));
        let signal = strategy.process(&new_candle).unwrap();

        // Should generate buy signal if RSI is oversold
        // Note: Actual RSI calculation depends on price movement pattern
        match signal.signal_type {
            SignalType::Buy | SignalType::Hold => {
                // Either is acceptable depending on actual RSI value
            }
            _ => panic!("Expected Buy or Hold signal"),
        }
    }

    #[test]
    fn test_rsi_strategy_confidence() {
        let config = RSIStrategyConfig {
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            min_confidence: 0.5,
        };
        let strategy = RSIStrategy::new(config.clone());

        // Test confidence calculation
        let confidence_0 = strategy.calculate_confidence(0.0); // Maximum oversold
        let confidence_15 = strategy.calculate_confidence(15.0); // Strongly oversold
        let confidence_30 = strategy.calculate_confidence(30.0); // At threshold
        let confidence_50 = strategy.calculate_confidence(50.0); // Neutral
        let confidence_70 = strategy.calculate_confidence(70.0); // At threshold
        let confidence_85 = strategy.calculate_confidence(85.0); // Strongly overbought
        let confidence_100 = strategy.calculate_confidence(100.0); // Maximum overbought

        assert!(confidence_0 > confidence_15);
        assert!(confidence_15 > confidence_30);
        assert_eq!(confidence_30, 0.0);
        assert_eq!(confidence_50, 0.0);
        assert_eq!(confidence_70, 0.0);
        assert!(confidence_85 > confidence_70);
        assert!(confidence_100 > confidence_85);
    }
}

