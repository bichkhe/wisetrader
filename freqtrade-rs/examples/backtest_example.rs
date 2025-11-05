//! Example: Backtesting with RSI Strategy

use freqtrade_rs::data::{Candle, CandleSeries};
use freqtrade_rs::backtest::{BacktestEngine, BacktestReport};
use freqtrade_rs::strategy::Strategy;
use freqtrade_rs::strategy::implementations::{RSIStrategy, RSIStrategyConfig};
use freqtrade_rs::Result;
use chrono::Utc;

fn create_test_candles(count: usize, base_price: f64) -> Vec<Candle> {
    let mut candles = Vec::new();
    let base_time = Utc::now();

    // Create candles with some price movement
    for i in 0..count {
        // Add some volatility
        let volatility = (i as f64 % 10.0) * 0.5;
        let trend = if i < count / 2 { 0.1 } else { -0.1 }; // Up then down
        let price = base_price + (i as f64 * trend) + volatility;

        candles.push(Candle::new(
            price,
            price + 1.0,
            price - 1.0,
            price,
            1000.0,
            base_time + chrono::Duration::minutes(i as i64),
            "BTC/USDT".to_string(),
            "5m".to_string(),
        ));
    }

    candles
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Freqtrade-RS Backtest Example ===\n");

    // Create test data
    println!("Creating test candle data...");
    let candles = create_test_candles(500, 100.0);
    let series = CandleSeries::from_vec(candles);

    // Create strategy
    println!("Creating RSI Strategy...");
    let config = RSIStrategyConfig {
        rsi_period: 14,
        rsi_oversold: 30.0,
        rsi_overbought: 70.0,
        min_confidence: 0.6,
    };
    let mut strategy = RSIStrategy::new(config);

    // Initialize strategy
    println!("Initializing strategy with historical data...");
    strategy.initialize(series.candles())?;

    // Create backtest engine
    println!("Running backtest...");
    let initial_balance = 10000.0;
    let mut engine = BacktestEngine::new(initial_balance);
    let result = engine.run(&mut strategy, &series)?;

    // Generate report
    println!("\n=== Backtest Results ===");
    let report = BacktestReport::new(result);
    println!("{}", report.format());

    Ok(())
}

