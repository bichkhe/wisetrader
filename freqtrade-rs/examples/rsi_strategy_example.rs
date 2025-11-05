//! Example: RSI Strategy with real-time data streaming

use freqtrade_rs::data::Candle;
use freqtrade_rs::exchange::streaming::DataStreamer;
use freqtrade_rs::strategy::Strategy;
use freqtrade_rs::strategy::implementations::{RSIStrategy, RSIStrategyConfig};
use freqtrade_rs::Result;
use tokio::sync::mpsc;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create RSI Strategy
    let config = RSIStrategyConfig {
        rsi_period: 14,
        rsi_oversold: 30.0,
        rsi_overbought: 70.0,
        min_confidence: 0.6,
    };
    let mut strategy = RSIStrategy::new(config);

    // Start streaming data from Binance
    info!("Starting Binance data stream for BTC/USDT 5m candles...");
    let (_streamer, mut candle_rx) = DataStreamer::binance("btc", "usdt", "5m").await?;

    // Collect initial candles for strategy initialization
    let mut initial_candles = Vec::new();
    info!("Collecting initial candles for strategy initialization...");

    for _ in 0..30 {
        if let Some(candle) = candle_rx.recv().await {
            initial_candles.push(candle);
        }
    }

    // Initialize strategy with historical candles
    strategy.initialize(&initial_candles)?;
    info!("Strategy initialized and ready");

    // Process real-time candles
    info!("Processing real-time candles...");
    let mut candle_count = 0;

    while let Some(candle) = candle_rx.recv().await {
        candle_count += 1;

        // Process candle through strategy
        match strategy.process(&candle) {
            Ok(signal) => {
                match signal.signal_type {
                    freqtrade_rs::strategy::SignalType::Buy => {
                        info!(
                            "🔵 BUY SIGNAL: Price={:.2}, Confidence={:.2}, Reason: {}",
                            candle.close, signal.confidence, signal.reason
                        );
                    }
                    freqtrade_rs::strategy::SignalType::Sell => {
                        info!(
                            "🔴 SELL SIGNAL: Price={:.2}, Confidence={:.2}, Reason: {}",
                            candle.close, signal.confidence, signal.reason
                        );
                    }
                    freqtrade_rs::strategy::SignalType::Hold => {
                        if candle_count % 10 == 0 {
                            // Only log every 10th candle to reduce noise
                            info!(
                                "⚪ HOLD: Price={:.2}, Reason: {}",
                                candle.close, signal.reason
                            );
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error processing candle: {}", e);
            }
        }

        // Process for 100 candles as example
        if candle_count >= 100 {
            info!("Processed {} candles, stopping...", candle_count);
            break;
        }
    }

    Ok(())
}

