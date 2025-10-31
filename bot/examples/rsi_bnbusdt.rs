/// Example: BNBUSDT RSI Trading Strategy using barter-rs
/// 
/// This example demonstrates:
/// - Connecting to Binance WebSocket for BNB/USDT
/// - Calculating RSI indicator
/// - Trading strategy: Buy when RSI < 30, Sell when RSI > 70

use std::collections::VecDeque;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use tracing::info;

// Note: Using barter-data and barter-execution for WebSocket and order execution
// For simplicity, we'll create a basic implementation

/// RSI (Relative Strength Index) Calculator
struct RSI {
    period: usize,
    gains: VecDeque<f64>,
    losses: VecDeque<f64>,
    prices: VecDeque<f64>,
}

impl RSI {
    fn new(period: usize) -> Self {
        Self {
            period,
            gains: VecDeque::with_capacity(period + 1),
            losses: VecDeque::with_capacity(period + 1),
            prices: VecDeque::with_capacity(period + 2),
        }
    }

    fn update(&mut self, price: f64) -> Option<f64> {
        self.prices.push_back(price);

        if self.prices.len() < 2 {
            return None;
        }

        let change = price - self.prices[self.prices.len() - 2];
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { -change } else { 0.0 };

        self.gains.push_back(gain);
        self.losses.push_back(loss);

        if self.gains.len() > self.period {
            self.gains.pop_front();
        }
        if self.losses.len() > self.period {
            self.losses.pop_front();
        }

        if self.gains.len() < self.period {
            return None;
        }

        let avg_gain = self.gains.iter().sum::<f64>() / self.period as f64;
        let avg_loss = self.losses.iter().sum::<f64>() / self.period as f64;

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        let rsi = 100.0 - (100.0 / (1.0 + rs));

        Some(rsi)
    }
}

/// Trading Strategy State
enum TradingAction {
    Buy,
    Sell,
    Hold,
}

/// Main Trading Bot
struct BnbusdtRSIBot {
    rsi: RSI,
    last_action: Option<TradingAction>,
    position: Option<f64>, // Current position price if holding
}

impl BnbusdtRSIBot {
    fn new() -> Self {
        Self {
            rsi: RSI::new(14), // Standard RSI period is 14
            last_action: None,
            position: None,
        }
    }

    fn process_price(&mut self, price: f64) -> Option<TradingAction> {
        if let Some(rsi_value) = self.rsi.update(price) {
            info!("BNB/USDT Price: {:.4}, RSI: {:.2}", price, rsi_value);

            // Strategy: Buy when RSI < 30 (oversold), Sell when RSI > 70 (overbought)
            if rsi_value < 30.0 {
                // Check if we're not already in a position or last action wasn't buy
                match self.last_action {
                    Some(TradingAction::Buy) => {
                        // Already bought, hold
                        None
                    }
                    _ => {
                        info!("🟢 RSI < 30 (Oversold): BUY signal at price {:.4}", price);
                        self.last_action = Some(TradingAction::Buy);
                        self.position = Some(price);
                        Some(TradingAction::Buy)
                    }
                }
            } else if rsi_value > 70.0 {
                // Check if we have a position to sell
                match self.last_action {
                    Some(TradingAction::Buy) if self.position.is_some() => {
                        let entry_price = self.position.unwrap();
                        let profit_pct = ((price - entry_price) / entry_price) * 100.0;
                        info!("🔴 RSI > 70 (Overbought): SELL signal at price {:.4}", price);
                        info!("💰 Profit/Loss: {:.2}% (Entry: {:.4}, Exit: {:.4})", 
                              profit_pct, entry_price, price);
                        self.last_action = Some(TradingAction::Sell);
                        self.position = None;
                        Some(TradingAction::Sell)
                    }
                    _ => {
                        // No position to sell
                        None
                    }
                }
            } else {
                // RSI between 30-70, hold
                None
            }
        } else {
            None
        }
    }
}

/// Simulate WebSocket data stream
/// In real implementation, this would connect to Binance WebSocket API
async fn simulate_price_stream(mut bot: BnbusdtRSIBot) -> Result<()> {
    info!("🚀 Starting BNBUSDT RSI Trading Bot");
    info!("📊 Strategy: Buy when RSI < 30, Sell when RSI > 70");
    info!("🔗 Simulating price stream (connect to Binance WebSocket in production)");
    
    // Simulated price data (in production, this comes from Binance WebSocket)
    // Starting with realistic BNB/USDT price around $580
    let mut current_price = 580.0;
    let mut price_direction = 1.0; // 1.0 for up, -1.0 for down
    
    loop {
        // Simulate price movement (random walk with some trend)
        let change = (rand::random() - 0.5) * 2.0 * price_direction;
        current_price += change;
        
        // Keep price in reasonable range
        if current_price > 650.0 {
            price_direction = -1.0;
        } else if current_price < 520.0 {
            price_direction = 1.0;
        }
        
        // Process price through bot
        if let Some(action) = bot.process_price(current_price) {
            match action {
                TradingAction::Buy => {
                    info!("✅ EXECUTING BUY ORDER for BNB/USDT at {:.4}", current_price);
                    // In production, this would send order to exchange
                }
                TradingAction::Sell => {
                    info!("✅ EXECUTING SELL ORDER for BNB/USDT at {:.4}", current_price);
                    // In production, this would send order to exchange
                }
                TradingAction::Hold => {}
            }
        }
        
        // Wait 1 second before next price update (in production, WebSocket provides real-time updates)
        sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let bot = BnbusdtRSIBot::new();
    
    let separator = "=".repeat(60);
    info!("{}", separator);
    info!("BNBUSDT RSI Trading Bot Example");
    info!("{}", separator);
    
    simulate_price_stream(bot).await?;
    
    Ok(())
}

// Simple random number generator for simulation
mod rand {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    thread_local! {
        static RNG: Cell<u64> = Cell::new(0);
    }
    
    pub fn random() -> f64 {
        RNG.with(|rng| {
            let mut seed = rng.get();
            if seed == 0 {
                seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
            }
            // Linear congruential generator
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            rng.set(seed);
            (seed as f64) / (u64::MAX as f64)
        })
    }
}

