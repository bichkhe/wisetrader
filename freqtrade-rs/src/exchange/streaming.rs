//! Real-time data streaming using barter-data

use crate::data::Candle;
use crate::Result;
use barter_data::exchange::binance::spot::BinanceSpot;
use barter_data::streams::Streams;
use barter_data::subscription::candle::Candles;
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use futures::StreamExt;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Data streamer for real-time candle data
pub struct DataStreamer {
    candle_tx: mpsc::Sender<Candle>,
    _handle: tokio::task::JoinHandle<()>,
}

impl DataStreamer {
    /// Create new data streamer for Binance
    pub async fn binance(
        base: &str,
        quote: &str,
        interval: &str,
    ) -> Result<(Self, mpsc::Receiver<Candle>)> {
        let (candle_tx, candle_rx) = mpsc::channel(1000);

        let handle = tokio::spawn({
            let base = base.to_string();
            let quote = quote.to_string();
            let candle_tx = candle_tx.clone();
            let _interval = interval.to_string();

            async move {
                // TODO: BinanceSpot with Candles subscription may not be fully supported
                // For now, return error - this needs to be implemented properly
                error!("Candle streaming not yet fully supported - please use trade data aggregation");
                let _base = base;
                let _quote = quote;
                // let streams_result = Streams::<Candles>::builder()
                //     .subscribe([
                //         (
                //             BinanceSpot::default(),
                //             base.as_str(),
                //             quote.as_str(),
                //             MarketDataInstrumentKind::Spot,
                //             Candles, // Unit struct
                //         ),
                //     ])
                //     .init()
                //     .await;

                // TODO: Implement when candle subscription is supported
                // match streams_result {
                //     Ok(mut streams) => {
                //         let mut joined_stream = streams.select_all();
                //         info!("Binance candle stream started for {}/{}", base, quote);

                //         while let Some(event_result) = joined_stream.next().await {
                //             match event_result {
                //                 Ok(event) => {
                //                     let candle = Candle::from_barter_event(&event);
                //                     if candle_tx.send(candle).await.is_err() {
                //                         error!("Failed to send candle to channel");
                //                         break;
                //                     }
                //                 }
                //                 Err(e) => {
                //                     error!("Error receiving candle event: {}", e);
                //                 }
                //             }
                //         }
                //     }
                //     Err(e) => {
                //         error!("Failed to initialize Binance streams: {}", e);
                //     }
                // }
            }
        });

        Ok((
            Self {
                candle_tx,
                _handle: handle,
            },
            candle_rx,
        ))
    }

    // Note: Interval parsing is handled by Candles::new() method
}

/// Streaming example
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Ignore by default as it requires network connection
    async fn test_binance_streaming() {
        let (_streamer, mut rx) = DataStreamer::binance("btc", "usdt", "1m")
            .await
            .expect("Failed to create streamer");

        // Receive a few candles
        for _ in 0..5 {
            if let Some(candle) = rx.recv().await {
                println!("Received candle: {:?}", candle);
            }
        }
    }
}

