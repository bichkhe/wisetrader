/// Example: BNBUSDT RSI Trading Strategy using barter-rs and ta-rs
/// 
/// This example demonstrates Paper Trading with:
/// - Live market data from Binance via barter-data
/// - RSI calculation using ta-rs
/// - Trading strategy: Buy when RSI < 30, Sell when RSI > 70
/// - Mock execution (paper trading)

use anyhow::Result;
use barter_data::{
    exchange::binance::spot::BinanceSpot,
    streams::Streams,
    subscription::trade::PublicTrades,
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use chrono::Utc;
use futures::StreamExt;
use ta::{
    indicators::RelativeStrengthIndex,
    Next,
};
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 1-Minute Candle aggregator
#[derive(Debug, Clone)]
struct OneMinuteCandle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    start_minute: i64, // Minute timestamp (seconds since epoch, rounded to minute)
}

impl OneMinuteCandle {
    fn new(price: f64, timestamp: i64) -> Self {
        let start_minute = (timestamp / 60) * 60; // Round down to minute
        Self {
            open: price,
            high: price,
            low: price,
            close: price,
            start_minute,
        }
    }

    fn update(&mut self, price: f64) {
        self.high = self.high.max(price);
        self.low = self.low.min(price);
        self.close = price;
    }

    fn is_expired(&self, current_timestamp: i64) -> bool {
        let current_minute = (current_timestamp / 60) * 60;
        current_minute > self.start_minute
    }
}

/// Paper Trading Bot State
#[derive(Debug, Clone)]
struct PaperTradingState {
    rsi: RelativeStrengthIndex,
    position: Option<f64>, // Entry price if holding
    balance_usdt: f64,    // USDT balance
    balance_bnb: f64,     // BNB balance
    trades_count: u64,
    prices: Vec<f64>,     // Price history for RSI
    period: usize,
    current_candle: Option<OneMinuteCandle>, // Current 1-minute candle being built
}

impl PaperTradingState {
    fn new(period: usize, initial_balance: f64) -> Self {
        Self {
            rsi: RelativeStrengthIndex::new(period).unwrap(),
            position: None,
            balance_usdt: initial_balance,
            balance_bnb: 0.0,
            trades_count: 0,
            prices: Vec::with_capacity(period + 10),
            period,
            current_candle: None,
        }
    }

    /// Process a trade and aggregate into 1-minute candles
    /// Returns signal only when a 1-minute candle is completed
    fn process_trade(&mut self, price: f64, timestamp: i64) -> Option<TradingSignal> {
        let current_minute = (timestamp / 60) * 60; // Round down to minute
        
        // Check if we need to start a new candle or update current one
        if let Some(ref mut candle) = self.current_candle {
            if candle.is_expired(timestamp) {
                // Candle completed! Process it
                let completed_close = candle.close;
                info!("🕐 1-minute candle completed! Close: {:.4}", completed_close);
                let signal = self.process_price(completed_close);
                
                // Start new candle for current minute
                self.current_candle = Some(OneMinuteCandle::new(price, timestamp));
                return signal;
            } else {
                // Update current candle
                candle.update(price);
            }
        } else {
            // First candle - initialize
            info!("🕐 Starting first 1-minute candle at minute: {}", (timestamp / 60) * 60);
            self.current_candle = Some(OneMinuteCandle::new(price, timestamp));
        }
        
        None // No signal yet, waiting for candle to complete
    }

    /// Force process current candle (called by timer every minute)
    fn force_process_candle(&mut self) -> Option<TradingSignal> {
        if let Some(ref candle) = self.current_candle {
            let completed_close = candle.close;
            info!("🕐 Timer triggered! Processing 1-minute candle. Close: {:.4}", completed_close);
            let signal = self.process_price(completed_close);
            
            // Reset candle to None, will be recreated on next trade
            self.current_candle = None;
            return signal;
        }
        None
    }

    /// Process a new price and update RSI, generate signals
    fn process_price(&mut self, price: f64) -> Option<TradingSignal> {
        // Add price to history
        self.prices.push(price);
        
        // Keep only recent prices
        if self.prices.len() > self.period + 10 {
            self.prices.remove(0);
        }

        // Need at least period + 1 prices for RSI
        if self.prices.len() < self.period + 1 {
            return None;
        }

        // Calculate RSI on completed 1-minute candle close price
        let rsi_value = self.rsi.next(price);
        
        info!("📊 1m Candle Close: {:.4} USDT, RSI: {:.2}", price, rsi_value);

        // Strategy: Buy when RSI < 30, Sell when RSI > 70
        if rsi_value < 30.0 && self.position.is_none() {
            // Buy signal
            Some(TradingSignal::Buy(price))
        } else if rsi_value > 70.0 && self.position.is_some() {
            // Sell signal
            let entry_price = self.position.unwrap();
            Some(TradingSignal::Sell(price, entry_price))
        } else {
            None
        }
    }

    /// Execute buy order (mock execution)
    fn execute_buy(&mut self, price: f64) {
        if self.balance_usdt > 0.0 && self.position.is_none() {
            // Use 90% of balance to buy
            let buy_amount_usdt = self.balance_usdt * 0.9;
            let buy_amount_bnb = buy_amount_usdt / price;
            
            self.balance_usdt -= buy_amount_usdt;
            self.balance_bnb += buy_amount_bnb;
            self.position = Some(price);
            self.trades_count += 1;
            
            info!("✅ BUY EXECUTED: {:.6} BNB @ {:.4} USDT", buy_amount_bnb, price);
            info!("   Balance: {:.2} USDT, {:.6} BNB", self.balance_usdt, self.balance_bnb);
        }
    }

    /// Execute sell order (mock execution)
    fn execute_sell(&mut self, price: f64, entry_price: f64) {
        if self.balance_bnb > 0.0 && self.position.is_some() {
            let sell_amount_usdt = self.balance_bnb * price;
            
            self.balance_usdt += sell_amount_usdt;
            let sell_amount_bnb = self.balance_bnb;
            self.balance_bnb = 0.0;
            self.position = None;
            self.trades_count += 1;
            
            let profit_pct = ((price - entry_price) / entry_price) * 100.0;
            let profit_usdt = sell_amount_usdt - (sell_amount_bnb * entry_price);
            
            info!("✅ SELL EXECUTED: {:.6} BNB @ {:.4} USDT", sell_amount_bnb, price);
            info!("   Entry: {:.4}, Exit: {:.4}, Profit: {:.2}% ({:.2} USDT)", 
                  entry_price, price, profit_pct, profit_usdt);
            info!("   Balance: {:.2} USDT, {:.6} BNB", self.balance_usdt, self.balance_bnb);
        }
    }

    fn print_summary(&self) {
        let total_value = self.balance_usdt + (self.balance_bnb * self.prices.last().copied().unwrap_or(0.0));
        let separator = "=".repeat(60);
        info!("");
        info!("{}", separator);
        info!("📊 TRADING SUMMARY");
        info!("{}", separator);
        info!("Total Trades: {}", self.trades_count);
        info!("Current Position: {:?}", self.position);
        info!("Balance: {:.2} USDT, {:.6} BNB", self.balance_usdt, self.balance_bnb);
        info!("Total Portfolio Value: {:.2} USDT", total_value);
        info!("{}", separator);
    }
}

#[derive(Debug, Clone)]
enum TradingSignal {
    Buy(f64),
    Sell(f64, f64), // (current_price, entry_price)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into())
        )
        .init();

    let separator = "=".repeat(60);
    info!("{}", separator);
    info!("BNBUSDT Paper Trading Bot using barter-rs & ta-rs");
    info!("{}", separator);
    info!("📊 Strategy: Buy when RSI < 30, Sell when RSI > 70");
    info!("📈 RSI Period: 14");
    info!("⏰ Timeframe: 1 minute candles (aggregated from trades)");
    info!("💰 Initial Balance: 1000 USDT");
    info!("");

    // Create paper trading state
    let state = Arc::new(RwLock::new(PaperTradingState::new(14, 1000.0)));

    // Build market data stream from Binance
    // We'll aggregate trades into 1-minute candles and process once per minute
    info!("🔗 Connecting to Binance for BNB/USDT trades (aggregating to 1m candles)...");
    
    let streams = Streams::<PublicTrades>::builder()
        .subscribe([(
            BinanceSpot::default(),
            "bnb",
            "usdt",
            MarketDataInstrumentKind::Spot,
            PublicTrades,
        )])
        .init()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize streams: {}", e))?;

    info!("✅ Connected to Binance! Aggregating trades into 1-minute candles...");
    info!("   Processing signals once per minute when candle closes");
    info!("");

    // Get combined stream - select_all returns a Stream, not a Future
    let mut market_stream = streams.select_all();

    // Handle Ctrl+C gracefully
    let state_clone = state.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("");
        warn!("⚠️  Shutting down...");
        let state = state_clone.read().await;
        state.print_summary();
        std::process::exit(0);
    });

    // Start a timer to force process candle every minute
    // This ensures we process candles even if no trades come in
    use tokio::time::{interval, Duration};
    let mut minute_timer = interval(Duration::from_secs(60));
    minute_timer.tick().await; // Skip first tick (immediate)
    info!("⏰ Starting 1-minute timer to process candles...");
    
    let state_timer = state.clone();
    tokio::spawn(async move {
        let mut count = 0;
        loop {
            minute_timer.tick().await;
            count += 1;
            info!("⏰ Timer tick #{}, processing candle...", count);
            let mut state_guard = state_timer.write().await;
            if let Some(signal) = state_guard.force_process_candle() {
                match signal {
                    TradingSignal::Buy(buy_price) => {
                        state_guard.execute_buy(buy_price);
                    }
                    TradingSignal::Sell(sell_price, entry_price) => {
                        state_guard.execute_sell(sell_price, entry_price);
                    }
                }
            } else {
                info!("⏰ No candle to process (waiting for trades...)");
            }
        }
    });

    // Process market events
    use barter_data::streams::reconnect::Event;
    
    while let Some(event) = market_stream.next().await {
        match event {
            Event::Item(market_event_result) => {
                match market_event_result {
                    Ok(market_event) => {
                        // market_event.kind is PublicTrade, extract price and timestamp
                        let price = market_event.kind.price;
                        let timestamp = market_event.time_received.timestamp();
                        let current_minute = (timestamp / 60) * 60;
                        
                        // Process trade and aggregate into 1-minute candles
                        // Returns signal only when a 1-minute candle is completed (when minute changes)
                        let mut state_guard = state.write().await;
                        if let Some(signal) = state_guard.process_trade(price, timestamp) {
                            match signal {
                                TradingSignal::Buy(buy_price) => {
                                    state_guard.execute_buy(buy_price);
                                }
                                TradingSignal::Sell(sell_price, entry_price) => {
                                    state_guard.execute_sell(sell_price, entry_price);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error in market event: {}", e);
                    }
                }
            }
            Event::Reconnecting(_origin) => {
                warn!("Reconnecting to Binance...");
            }
        }
    }

    // Print final summary
    let state = state.read().await;
    state.print_summary();

    Ok(())
}
