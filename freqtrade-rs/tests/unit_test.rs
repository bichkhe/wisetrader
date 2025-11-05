//! Unit tests for freqtrade-rs modules

#[cfg(test)]
mod tests {
    use freqtrade_rs::data::Candle;
    use freqtrade_rs::indicators::{RSI, MACD, EMA, SMA, Indicator};
    use chrono::Utc;

    #[test]
    fn test_candle_creation() {
        let candle = Candle::new(
            100.0,
            110.0,
            95.0,
            105.0,
            1000.0,
            Utc::now(),
            "BTC/USDT".to_string(),
            "5m".to_string(),
        );

        assert_eq!(candle.open, 100.0);
        assert_eq!(candle.high, 110.0);
        assert_eq!(candle.low, 95.0);
        assert_eq!(candle.close, 105.0);
        assert_eq!(candle.volume, 1000.0);
        assert!(candle.is_bullish());
        assert!(!candle.is_bearish());
        assert_eq!(candle.range(), 15.0);
    }

    #[test]
    fn test_candle_utilities() {
        let candle = Candle::new(
            100.0,
            110.0,
            95.0,
            105.0,
            1000.0,
            Utc::now(),
            "BTC/USDT".to_string(),
            "5m".to_string(),
        );

        assert_eq!(candle.typical_price(), (110.0 + 95.0 + 105.0) / 3.0);
        assert_eq!(candle.median_price(), (110.0 + 95.0) / 2.0);
        assert_eq!(candle.body_size(), 5.0);
        assert_eq!(candle.upper_wick(), 5.0);
        assert_eq!(candle.lower_wick(), 5.0);
    }

    #[test]
    fn test_rsi_indicator() {
        let mut rsi = RSI::new(14);
        assert_eq!(rsi.name(), "RSI");
        assert_eq!(rsi.period(), 14);
        assert!(!rsi.is_ready());

        // Update with values
        for i in 0..20 {
            rsi.update(100.0 + (i as f64 * 0.1));
        }

        // Should be ready after enough updates
        assert!(rsi.is_ready());
        let value = rsi.value();
        assert!(value.is_some());
        if let Some(v) = value {
            assert!(v >= 0.0 && v <= 100.0);
        }
    }

    #[test]
    fn test_macd_indicator() {
        let mut macd = MACD::new(12, 26, 9);
        assert_eq!(macd.name(), "MACD");
        assert!(!macd.is_ready());

        // Update with values
        for i in 0..50 {
            macd.update(100.0 + (i as f64 * 0.1));
        }

        assert!(macd.is_ready());
        assert!(macd.macd().is_some());
        assert!(macd.signal().is_some());
        assert!(macd.histogram().is_some());
    }

    #[test]
    fn test_ema_indicator() {
        let mut ema = EMA::new(10);
        assert_eq!(ema.name(), "EMA");
        assert_eq!(ema.period(), 10);
        assert!(!ema.is_ready());

        for i in 0..20 {
            ema.update(100.0 + (i as f64 * 0.1));
        }

        assert!(ema.is_ready());
        assert!(ema.value().is_some());
    }

    #[test]
    fn test_sma_indicator() {
        let mut sma = SMA::new(10);
        assert_eq!(sma.name(), "SMA");
        assert_eq!(sma.period(), 10);
        assert!(!sma.is_ready());

        for i in 0..20 {
            sma.update(100.0 + (i as f64 * 0.1));
        }

        assert!(sma.is_ready());
        assert!(sma.value().is_some());
    }
}

