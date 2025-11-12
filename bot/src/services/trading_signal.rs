use std::sync::Arc;
use std::collections::HashMap;
use std::str::FromStr;
use barter_data::{
    exchange::binance::spot::BinanceSpot,
    streams::Streams,
    subscription::trade::PublicTrades,
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use sea_orm::{EntityTrait, ActiveValue};
use shared::entity::live_trading_signals;
use barter_data::streams::reconnect::Event;
use futures::StreamExt;
use ta::{
    indicators::RelativeStrengthIndex,
    Next,
};
use tokio::sync::{RwLock, broadcast};
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use teloxide::prelude::*;
use chrono::{Utc, DateTime};

use crate::state::AppState;

/// Stream key to identify unique streams (exchange + base + quote)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StreamKey {
    exchange: String,
    base: String,
    quote: String,
}

impl StreamKey {
    fn new(exchange: &str, base: &str, quote: &str) -> Self {
        Self {
            exchange: exchange.to_lowercase(),
            base: base.to_lowercase(),
            quote: quote.to_lowercase(),
        }
    }
    
    fn from_pair(exchange: &str, pair: &str) -> Option<Self> {
        normalize_pair(pair).map(|(base, quote)| {
            Self::new(exchange, &base, &quote)
        })
    }
}

/// Stream information for a trading pair
struct StreamInfo {
    subscribers: Arc<RwLock<Vec<i64>>>, // List of user IDs subscribed to this stream
    sender: broadcast::Sender<MarketEvent>, // Broadcast channel to send events to all subscribers
}

/// Market event wrapper for broadcast
#[derive(Debug, Clone)]
struct MarketEvent {
    price: f64,
    timestamp: i64,
}

/// Stream Manager to share streams across users for the same trading pair
pub struct StreamManager {
    streams: Arc<RwLock<HashMap<StreamKey, StreamInfo>>>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Subscribe a user to a stream for a trading pair
    /// Returns a receiver for market events
    pub async fn subscribe(
        &self,
        exchange: &str,
        pair: &str,
        user_id: i64,
    ) -> Result<broadcast::Receiver<MarketEvent>, anyhow::Error> {
        let key = StreamKey::from_pair(exchange, pair)
            .ok_or_else(|| anyhow::anyhow!("Invalid pair format: {}", pair))?;
        
        let mut streams = self.streams.write().await;
        
        // Check if stream already exists
        if let Some(stream_info) = streams.get(&key) {
            // Add user to subscribers
            let mut subscribers = stream_info.subscribers.write().await;
            if !subscribers.contains(&user_id) {
                subscribers.push(user_id);
                info!("User {} subscribed to existing stream for {} ({})", user_id, pair, exchange);
            }
            
            // Return receiver for this stream
            Ok(stream_info.sender.subscribe())
        } else {
            // Create new stream
            let (sender, receiver) = broadcast::channel(1000); // Buffer up to 1000 events
            
            let stream_info = StreamInfo {
                subscribers: Arc::new(RwLock::new(vec![user_id])),
                sender,
            };
            
            streams.insert(key.clone(), stream_info);
            info!("Created new stream for {} ({}) with subscriber {}", pair, exchange, user_id);
            
            // Spawn task to initialize and run the stream
            // Use std::thread::spawn with LocalSet for non-Send futures
            let key_clone = key.clone();
            let exchange_clone = exchange.to_string();
            let base = key.base.clone();
            let quote = key.quote.clone();
            let streams_map = self.streams.clone();
            
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    use tokio::task::LocalSet;
                    let local = LocalSet::new();
                    local.spawn_local(async move {
                        if let Err(e) = Self::run_stream(
                            &key_clone,
                            &exchange_clone,
                            &base,
                            &quote,
                            streams_map.clone(),
                        ).await {
                            error!("Error running stream for {:?}: {}", key_clone, e);
                            
                            // Remove stream from map on error
                            let mut streams = streams_map.write().await;
                            streams.remove(&key_clone);
                        }
                    });
                    local.await;
                });
            });
            
            Ok(receiver)
        }
    }
    
    /// Unsubscribe a user from a stream
    pub async fn unsubscribe(&self, exchange: &str, pair: &str, user_id: i64) {
        let key = StreamKey::from_pair(exchange, pair);
        
        if let Some(key) = key {
            let mut streams = self.streams.write().await;
            
            if let Some(stream_info) = streams.get(&key) {
                let mut subscribers = stream_info.subscribers.write().await;
                subscribers.retain(|&id| id != user_id);
                
                let subscriber_count = subscribers.len();
                drop(subscribers); // Release lock before removing from map
                
                info!("User {} unsubscribed from stream for {} ({})", user_id, pair, exchange);
                
                // If no more subscribers, remove the stream
                if subscriber_count == 0 {
                    streams.remove(&key);
                    info!("Removed stream for {} ({}) - no more subscribers", pair, exchange);
                }
            }
        }
    }
    
    /// Get number of subscribers for a stream
    pub async fn subscriber_count(&self, exchange: &str, pair: &str) -> usize {
        let key = StreamKey::from_pair(exchange, pair);
        
        if let Some(key) = key {
            let streams = self.streams.read().await;
            if let Some(stream_info) = streams.get(&key) {
                let subscribers = stream_info.subscribers.read().await;
                return subscribers.len();
            }
        }
        0
    }
    
    /// Get all active streams with their information
    /// Returns vector of (exchange, pair, subscriber_count, subscriber_ids)
    pub async fn get_active_streams(&self) -> Vec<(String, String, usize, Vec<i64>)> {
        let streams = self.streams.read().await;
        let mut result = Vec::new();
        
        for (key, stream_info) in streams.iter() {
            let subscribers = stream_info.subscribers.read().await;
            let subscriber_ids = subscribers.clone();
            let subscriber_count = subscriber_ids.len();
            
            let pair = format!("{}/{}", key.base.to_uppercase(), key.quote.to_uppercase());
            result.push((
                key.exchange.clone(),
                pair,
                subscriber_count,
                subscriber_ids,
            ));
        }
        
        result
    }
    
    /// Run the actual stream and broadcast events to all subscribers
    async fn run_stream(
        key: &StreamKey,
        exchange: &str,
        base: &str,
        quote: &str,
        streams_map: Arc<RwLock<HashMap<StreamKey, StreamInfo>>>,
    ) -> Result<(), anyhow::Error> {
        // Initialize stream based on exchange
        let streams_result = match exchange {
            "binance" => {
                Streams::<PublicTrades>::builder()
                    .subscribe([(
                        BinanceSpot::default(),
                        base,
                        quote,
                        MarketDataInstrumentKind::Spot,
                        PublicTrades,
                    )])
                    .init()
                    .await
            }
            "okx" => {
                return Err(anyhow::anyhow!("OKX exchange not yet supported in barter-data"));
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported exchange: {}", exchange));
            }
        };
        
        let streams = streams_result?;
        info!("✅ Stream initialized for {} ({})", format!("{}/{}", base, quote), exchange);
        
        let mut market_stream = streams.select_all();
        
        loop {
            match market_stream.next().await {
                Some(event) => {
                    match event {
                        Event::Item(market_event_result) => {
                            match market_event_result {
                                Ok(market_event) => {
                                    let price = market_event.kind.price;
                                    let timestamp = market_event.time_received.timestamp();
                                    
                                    // Broadcast event to all subscribers
                                    let event = MarketEvent { price, timestamp };
                                    
                                    // Get sender from streams map
                                    let streams = streams_map.read().await;
                                    if let Some(stream_info) = streams.get(key) {
                                        // Send to all subscribers (ignore errors if no receivers)
                                        let _ = stream_info.sender.send(event);
                                    }
                                }
                                Err(e) => {
                                    error!("Error in market event for {:?}: {}", key, e);
                                }
                            }
                        }
                        Event::Reconnecting(_origin) => {
                            warn!("Reconnecting stream for {:?}...", key);
                        }
                    }
                }
                None => {
                    warn!("Market stream ended for {:?}, exiting...", key);
                    break;
                }
            }
        }
        
        // Remove stream from map when it ends
        let mut streams = streams_map.write().await;
        streams.remove(key);
        info!("Stream removed for {:?}", key);
        
        Ok(())
    }
}

/// Normalize pair format to "BASE/QUOTE" (e.g., "BTCUSDT" -> "BTC/USDT", "BTC/USDT" -> "BTC/USDT")
fn normalize_pair(pair: &str) -> Option<(String, String)> {
    let pair_upper = pair.to_uppercase();
    
    // If already has "/", split it
    if pair_upper.contains('/') {
        let parts: Vec<&str> = pair_upper.split('/').collect();
        if parts.len() == 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    
    // Try to detect common quote currencies at the end
    let common_quotes = vec!["USDT", "BTC", "ETH", "BNB", "BUSD", "USDC", "EUR", "USD"];
    
    for quote in common_quotes {
        if pair_upper.ends_with(quote) && pair_upper.len() > quote.len() {
            let base = &pair_upper[..pair_upper.len() - quote.len()];
            if !base.is_empty() {
                return Some((base.to_string(), quote.to_string()));
            }
        }
    }
    
    // If no quote detected, assume USDT (most common)
    if pair_upper.len() > 4 {
        let base = &pair_upper[..pair_upper.len() - 4];
        if base.len() >= 2 {
            return Some((base.to_string(), "USDT".to_string()));
        }
    }
    
    None
}

/// 1-Minute Candle aggregator
#[derive(Debug, Clone)]
struct OneMinuteCandle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    start_minute: i64,
    processed: bool, // Flag to prevent double processing
}

impl OneMinuteCandle {
    fn new(price: f64, timestamp: i64) -> Self {
        let start_minute = (timestamp / 60) * 60;
        Self {
            open: price,
            high: price,
            low: price,
            close: price,
            start_minute,
            processed: false,
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

/// Trading Bot State
#[derive(Debug, Clone)]
struct TradingState {
    rsi: RelativeStrengthIndex,
    prices: Vec<f64>,
    period: usize,
    current_candle: Option<OneMinuteCandle>,
    last_signal: Option<TradingSignalType>, // Track last signal type to avoid duplicates
}

/// Signal type enum to track state changes
#[derive(Debug, Clone, PartialEq)]
enum TradingSignalType {
    Buy,
    Sell,
}

impl TradingState {
    fn new(period: usize) -> Self {
        Self {
            rsi: RelativeStrengthIndex::new(period).unwrap(),
            prices: Vec::with_capacity(period + 10),
            period,
            current_candle: None,
            last_signal: None,
        }
    }

    fn process_trade(&mut self, price: f64, timestamp: i64, app_state: Arc<crate::state::AppState>) -> Option<TradingSignal> {
        let current_minute = (timestamp / 60) * 60;
        
        // Check if candle exists and if it's expired
        if let Some(ref mut candle) = self.current_candle {
            if candle.is_expired(timestamp) {
                // Candle completed! Process it only if not already processed by timer
                if candle.processed {
                    info!("🕐 Trade triggered: Candle already processed by timer, starting new candle");
                    // Just start new candle, don't process again
                    self.current_candle = Some(OneMinuteCandle::new(price, timestamp));
                    return None;
                }
                
                let completed_close = candle.close;
                info!("🕐 Trade triggered: 1-minute candle completed! Close: {:.4}", completed_close);
                
                // Create Candle for strategy executor
                use crate::services::strategy_engine::Candle as StrategyCandle;
                let strategy_candle = StrategyCandle {
                    open: candle.open,
                    high: candle.high,
                    low: candle.low,
                    close: candle.close,
                    volume: 0.0, // Volume not available from trades
                    timestamp,
                };
                
                // Process candle through strategy executor for all users (async)
                let executor = app_state.strategy_executor.clone();
                tokio::spawn(async move {
                    let signals = executor.process_candle_for_all(&strategy_candle).await;
                    for (user_id, signal) in signals {
                        info!("📊 User {} strategy signal: {:?}", user_id, signal);
                        // TODO: In future, send personalized signals to each user
                        // For now, signals are logged
                    }
                });
                
                // Mark as processed BEFORE processing
                candle.processed = true;
                let signal = self.process_price(completed_close);
                
                // Start new candle for current minute
                self.current_candle = Some(OneMinuteCandle::new(price, timestamp));
                return signal;
            } else {
                // Update current candle
                candle.update(price);
            }
        } else {
            // No candle yet - initialize new one
            info!("🕐 Starting new 1-minute candle at minute: {}", current_minute);
            self.current_candle = Some(OneMinuteCandle::new(price, timestamp));
        }
        
        None
    }

    fn force_process_candle(&mut self) -> Option<TradingSignal> {
        if let Some(ref mut candle) = self.current_candle {
            // Check if already processed
            if candle.processed {
                info!("🕐 Timer: Candle already processed, skipping");
                return None;
            }
            
            let completed_close = candle.close;
            info!("🕐 Timer: Processing completed candle with close price: {:.4}", completed_close);
            
            // Mark as processed BEFORE processing to prevent race condition
            candle.processed = true;
            let signal = self.process_price(completed_close);
            
            // Don't reset candle here - let next trade create new one when it comes
            return signal;
        } else {
            info!("🕐 Timer: No candle to process yet (waiting for first trade)");
        }
        None
    }

    fn process_price(&mut self, price: f64) -> Option<TradingSignal> {
        self.prices.push(price);
        
        if self.prices.len() > self.period + 10 {
            self.prices.remove(0);
        }

        if self.prices.len() < self.period + 1 {
            info!("⏳ Collecting prices for RSI: {}/{} (need {} prices for RSI calculation)", 
                  self.prices.len(), self.period + 1, self.period + 1);
            return None;
        }

        // Always calculate and log RSI, even if no signal
        let rsi_value = self.rsi.next(price);
        info!("📊 1m Candle Close: {:.4} USDT, RSI: {:.2}", price, rsi_value);

        // Strategy: Buy when RSI < 30, Sell when RSI > 70
        // Only send signal when signal type CHANGES (to avoid spam)
        let current_signal_type = if rsi_value < 30.0 {
            Some(TradingSignalType::Buy)
        } else if rsi_value > 70.0 {
            Some(TradingSignalType::Sell)
        } else {
            None // RSI in neutral zone, reset last_signal
        };

        // Check if signal type changed
        let should_send = match (&self.last_signal, &current_signal_type) {
            (None, Some(_)) => true,  // First signal
            (Some(TradingSignalType::Buy), Some(TradingSignalType::Sell)) => true,  // Buy -> Sell
            (Some(TradingSignalType::Sell), Some(TradingSignalType::Buy)) => true,  // Sell -> Buy
            (Some(_), None) => true,  // Signal -> No signal (reset)
            (Some(same), Some(other)) if same != other => true,  // Signal type changed
            _ => false,  // Same signal type, don't send again
        };

        if should_send {
            // Update last signal
            self.last_signal = current_signal_type.clone();
            
            // Return signal
            match current_signal_type {
                Some(TradingSignalType::Buy) => Some(TradingSignal::Buy { price, rsi: rsi_value }),
                Some(TradingSignalType::Sell) => Some(TradingSignal::Sell { price, rsi: rsi_value }),
                None => {
                    // Signal cleared (RSI back to neutral)
                    self.last_signal = None;
                    None
                }
            }
        } else {
            // Same signal type, don't send
            info!("⏸️  Signal already sent for this condition (RSI: {:.2}), skipping duplicate", rsi_value);
            None
        }
    }

    /// Process trade and create candle for user's strategy (returns Candle instead of TradingSignal)
    fn process_trade_for_user(
        &mut self,
        price: f64,
        timestamp: i64,
        strategy_config: &crate::services::strategy_engine::StrategyConfig,
        _app_state: &Arc<crate::state::AppState>,
        user_id: i64,
    ) -> Option<crate::services::strategy_engine::Candle> {
        // Parse timeframe to seconds
        let timeframe_secs = parse_timeframe_to_seconds(&strategy_config.timeframe);
        let current_period = (timestamp / timeframe_secs as i64) * timeframe_secs as i64;
        
        // Check if candle exists and if it's expired
        if let Some(ref mut candle) = self.current_candle {
            let candle_period = (candle.start_minute / timeframe_secs as i64) * timeframe_secs as i64;
            if current_period > candle_period {
                // Candle completed!
                if candle.processed {
                    // Already processed, start new candle
                    debug!("🕯️ [User {}] Candle already processed, starting new candle at price {:.4}", user_id, price);
                    self.current_candle = Some(OneMinuteCandle::new(price, timestamp));
                    return None;
                }
                
                let completed_candle = crate::services::strategy_engine::Candle {
                    open: candle.open,
                    high: candle.high,
                    low: candle.low,
                    close: candle.close,
                    volume: 0.0,
                    timestamp: candle.start_minute,
                };
                
                candle.processed = true;
                self.current_candle = Some(OneMinuteCandle::new(price, timestamp));
                return Some(completed_candle);
            } else {
                // Update current candle
                candle.update(price);
            }
        } else {
            // No candle yet - initialize new one
            debug!("🕯️ [User {}] Initializing new candle at price {:.4} for {} timeframe", user_id, price, strategy_config.timeframe);
            self.current_candle = Some(OneMinuteCandle::new(price, timestamp));
        }
        
        None
    }

    /// Force process candle for user (timer-based)
    fn force_process_candle_for_user(
        &mut self,
        strategy_config: &crate::services::strategy_engine::StrategyConfig,
        _app_state: &Arc<crate::state::AppState>,
        user_id: i64,
    ) -> Option<crate::services::strategy_engine::Candle> {
        if let Some(ref mut candle) = self.current_candle {
            if candle.processed {
                debug!("🕯️ [User {}] Candle already processed, skipping", user_id);
                return None;
            }
            
            let completed_candle = crate::services::strategy_engine::Candle {
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                volume: 0.0,
                timestamp: candle.start_minute,
            };
            
            candle.processed = true;
            return Some(completed_candle);
        }
        debug!("🕯️ [User {}] No active candle to process", user_id);
        None
    }
}

#[derive(Debug, Clone)]
enum TradingSignal {
    Buy { price: f64, rsi: f64 },
    Sell { price: f64, rsi: f64 },
}

/// Parse timeframe string to seconds
fn parse_timeframe_to_seconds(timeframe: &str) -> u64 {
    let timeframe_lower = timeframe.to_lowercase();
    if timeframe_lower.ends_with('m') {
        if let Ok(minutes) = timeframe_lower.trim_end_matches('m').parse::<u64>() {
            return minutes * 60;
        }
    } else if timeframe_lower.ends_with('h') {
        if let Ok(hours) = timeframe_lower.trim_end_matches('h').parse::<u64>() {
            return hours * 3600;
        }
    } else if timeframe_lower.ends_with('d') {
        if let Ok(days) = timeframe_lower.trim_end_matches('d').parse::<u64>() {
            return days * 86400;
        }
    }
    // Default to 1 minute if parsing fails
    60
}

/// Format trading signal message for Telegram
fn format_signal_message(signal: &TradingSignal, pair: &str, bot_name: &str) -> String {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    match signal {
        TradingSignal::Buy { price, rsi } => {
            format!(
                "🟢 <b>BUY SIGNAL - {}</b>\n\n\
💰 <b>Price:</b> <code>{:.4}</code> USDT\n\
📊 <b>RSI:</b> <code>{:.2}</code>\n\
⏰ <b>Time:</b> <code>{}</code>\n\
📈 <b>Strategy:</b> RSI &lt; 30 (Oversold)\n\
📍 <b>Timeframe:</b> 1 minute candles\n\n\
🤖 <b>Bot:</b> {}\n\
🔄 <b>Status:</b> <code>Monitoring Live</code>\n\
🌐 <b>Exchange:</b> Binance Spot\n\n\
⚠️ <i>This is a paper trading signal. Always do your own research!</i>",
                pair, price, rsi, timestamp, bot_name
            )
        },
        TradingSignal::Sell { price, rsi } => {
            format!(
                "🔴 <b>SELL SIGNAL - {}</b>\n\n\
💰 <b>Price:</b> <code>{:.4}</code> USDT\n\
📊 <b>RSI:</b> <code>{:.2}</code>\n\
⏰ <b>Time:</b> <code>{}</code>\n\
📉 <b>Strategy:</b> RSI &gt; 70 (Overbought)\n\
📍 <b>Timeframe:</b> 1 minute candles\n\n\
🤖 <b>Bot:</b> {}\n\
🔄 <b>Status:</b> <code>Monitoring Live</code>\n\
🌐 <b>Exchange:</b> Binance Spot\n\n\
⚠️ <i>This is a paper trading signal. Always do your own research!</i>",
                pair, price, rsi, timestamp, bot_name
            )
        },
    }
}

/// Start trading signal service (runs forever in background)
pub fn start_trading_signal_service(
    app_state: Arc<AppState>,
    bot: Bot,
    channel_id: i64,
    pair: String,
) {
    let bot_name = app_state.bot_name.clone();
    info!("🚀 Starting Trading Signal Service for {}", pair);
    
    let state = Arc::new(RwLock::new(TradingState::new(14))); // Period 2 = need 3 prices for RSI
    
    // Clone variables for tasks
    let state_for_stream = state.clone();
    let bot_for_stream = bot.clone();
    let pair_for_stream = pair.clone();
    let channel_id_for_stream = channel_id;
    let bot_name_for_stream = bot_name.clone();
    let app_state_for_stream = app_state.clone();
    
    // Use LocalSet for non-Send futures
    use tokio::task::LocalSet;
    
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let local = LocalSet::new();
            local.spawn_local(async move {
                // Initialize streams
                let streams_result = Streams::<PublicTrades>::builder()
                    .subscribe([(
                        BinanceSpot::default(),
                        "bnb",
                        "usdt",
                        MarketDataInstrumentKind::Spot,
                        PublicTrades,
                    )])
                    .init()
                    .await;
                    
                let streams = match streams_result {
                    Ok(streams) => streams,
                    Err(e) => {
                        error!("Failed to initialize streams: {}", e);
                        return;
                    }
                };

                info!("✅ Connected to Binance! Monitoring {} for trading signals...", pair_for_stream);

                let mut market_stream = streams.select_all();
                let state_trades = state_for_stream.clone();
                let bot_trades = bot_for_stream.clone();
                let pair_trades = pair_for_stream.clone();

                // Process trades
                while let Some(event) = market_stream.next().await {
                    match event {
                        Event::Item(market_event_result) => {
                            match market_event_result {
                                Ok(market_event) => {
                                    let price = market_event.kind.price;
                                    let timestamp = market_event.time_received.timestamp();
                                    
                                    let mut state_guard = state_trades.write().await;
                                    if let Some(signal) = state_guard.process_trade(price, timestamp, app_state_for_stream.clone()) {
                                        let message = format_signal_message(&signal, &pair_trades, &bot_name_for_stream);
                                        
                                        if let Err(e) = bot_trades.send_message(
                                            ChatId(channel_id_for_stream),
                                            message
                                        )
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .await {
                                            error!("Failed to send trading signal: {}", e);
                                        } else {
                                            info!("✅ Trading signal sent to channel {}", channel_id_for_stream);
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
            });
            local.await;
        });
    });

    // Start timer to force process candle every minute
    let state_timer = state.clone();
    let bot_timer = bot.clone();
    let pair_timer = pair.clone();
    let bot_name_timer = bot_name.clone();
    
    // Spawn task for timer-based processing
    tokio::spawn(async move {
        let mut minute_timer = interval(Duration::from_secs(60));
        minute_timer.tick().await; // Skip first tick
        info!("⏰ Timer started: will process candles every 60 seconds");
        let mut count = 0;
        
        loop {
            minute_timer.tick().await;
            count += 1;
            info!("⏰ Timer tick #{}", count);
            let mut state_guard = state_timer.write().await;
            
            // Only process if candle exists (was started by a trade)
            // After processing, reset candle to prevent double processing when next trade comes
            if state_guard.current_candle.is_some() {
                if let Some(signal) = state_guard.force_process_candle() {
                    let message = format_signal_message(&signal, &pair_timer, &bot_name_timer);
                    
                    if let Err(e) = bot_timer.send_message(
                        ChatId(channel_id),
                        message
                    )
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await {
                        error!("Failed to send trading signal: {}", e);
                    } else {
                        info!("✅ Trading signal sent to channel {} (timer-based)", channel_id);
                    }
                }
            } else {
                info!("⏰ Timer: No active candle to process (waiting for trades to start candle)");
            }
        }
    });

    info!("✅ Trading Signal Service started successfully");
}

/// Start user-specific trading service (runs forever in background for a specific user)
/// This service monitors market data and sends trading signals to the user's chat/channel
/// Uses StreamManager to share streams across users for the same trading pair
pub fn start_user_trading_service(
    app_state: Arc<AppState>,
    bot: Bot,
    user_id: i64,
    user_chat_id: i64, // Telegram chat ID to send signals to
    strategy_config: crate::services::strategy_engine::StrategyConfig,
    exchange: String,
    pair: String,
) {
    let bot_name = app_state.bot_name.clone();
    info!("🚀 Starting User Trading Service for user {} with strategy {} on {} ({})", 
        user_id, strategy_config.strategy_type, exchange, pair);
    
    // Normalize pair format (supports both "BTC/USDT" and "BTCUSDT")
    let (base, quote) = match normalize_pair(&pair) {
        Some((b, q)) => (b.to_lowercase(), q.to_lowercase()),
        None => {
            error!("Invalid pair format: {} (expected format: BTC/USDT or BTCUSDT)", pair);
            return;
        }
    };
    
    // Log normalized pair for debugging
    info!("Normalized pair: {} -> {}/{}", pair, base, quote);
    
    // Create candle aggregator for this user's timeframe
    let state = Arc::new(RwLock::new(TradingState::new(14))); // RSI period
    
    // Clone variables for tasks
    let state_for_stream = state.clone();
    let bot_for_stream = bot.clone();
    let pair_for_stream = pair.clone();
    let user_chat_id_for_stream = user_chat_id;
    let bot_name_for_stream = bot_name.clone();
    let app_state_for_stream = app_state.clone();
    let user_id_for_stream = user_id;
    let strategy_config_for_stream = strategy_config.clone();
    let exchange_for_stream = exchange.clone();
    let stream_manager = app_state.stream_manager.clone();
    
    // Subscribe to stream using StreamManager (will reuse existing stream if available)
    let stream_manager_clone = stream_manager.clone();
    tokio::spawn(async move {
        // Subscribe to stream
        let mut receiver = match stream_manager_clone.subscribe(&exchange_for_stream, &pair_for_stream, user_id_for_stream).await {
            Ok(receiver) => {
                let subscriber_count = stream_manager_clone.subscriber_count(&exchange_for_stream, &pair_for_stream).await;
                info!("✅ User {} subscribed to stream for {} ({}). Total subscribers: {}", 
                    user_id_for_stream, pair_for_stream, exchange_for_stream, subscriber_count);
                receiver
            }
            Err(e) => {
                error!("Failed to subscribe user {} to stream for {}: {}", user_id_for_stream, pair_for_stream, e);
                return;
            }
        };
        
        // Process market events from shared stream
        let mut last_heartbeat = std::time::Instant::now();
        let mut event_count = 0u64;
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let price = event.price;
                    let timestamp = event.timestamp;
                    event_count += 1;
                    
                    // Log heartbeat every 2 minutes
                    if last_heartbeat.elapsed().as_secs() >= 120 {
                        info!("💓 [User {}] Heartbeat: Service active, received {} market events, strategy: {} on {} ({})", 
                            user_id_for_stream, event_count, strategy_config_for_stream.strategy_type, 
                            exchange_for_stream, pair_for_stream);
                        last_heartbeat = std::time::Instant::now();
                    }
                    
                    // Use timeout to prevent blocking
                    match tokio::time::timeout(
                        Duration::from_secs(5),
                        state_for_stream.write()
                    ).await {
                        Ok(mut state_guard) => {
                            if let Some(candle) = state_guard.process_trade_for_user(
                                price, 
                                timestamp, 
                                &strategy_config_for_stream,
                                &app_state_for_stream,
                                user_id_for_stream
                            ) {
                                info!("📊 [User {}] Candle completed: O={:.4} H={:.4} L={:.4} C={:.4} ({} timeframe)", 
                                    user_id_for_stream, candle.open, candle.high, candle.low, candle.close,
                                    strategy_config_for_stream.timeframe);
                                
                                // Process candle through user's strategy
                                info!("🔍 [User {}] Evaluating strategy '{}' on candle...", 
                                    user_id_for_stream, strategy_config_for_stream.strategy_type);
                                
                                // Check if user is still trading before processing
                                let is_trading = app_state_for_stream.strategy_executor.is_user_trading(user_id_for_stream).await;
                                if !is_trading {
                                    warn!("⚠️ [User {}] User is not trading anymore, skipping candle processing", user_id_for_stream);
                                    continue;
                                }
                                
                                if let Some(signal) = app_state_for_stream.strategy_executor
                                    .process_candle(user_id_for_stream, &candle).await 
                                {
                                    match &signal {
                                        crate::services::strategy_engine::StrategySignal::Buy { price, confidence, reason } => {
                                            info!("✅ [User {}] Strategy '{}' generated BUY signal: price={:.4}, confidence={:.2}, reason={}", 
                                                user_id_for_stream, strategy_config_for_stream.strategy_type, 
                                                price, confidence, reason);
                                        }
                                        crate::services::strategy_engine::StrategySignal::Sell { price, confidence, reason } => {
                                            info!("✅ [User {}] Strategy '{}' generated SELL signal: price={:.4}, confidence={:.2}, reason={}", 
                                                user_id_for_stream, strategy_config_for_stream.strategy_type, 
                                                price, confidence, reason);
                                        }
                                        crate::services::strategy_engine::StrategySignal::Hold => {
                                            info!("✅ [User {}] Strategy '{}' generated HOLD signal", 
                                                user_id_for_stream, strategy_config_for_stream.strategy_type);
                                        }
                                    }
                                    // Clone signal for database save
                                    let signal_clone = signal.clone();
                                    
                                    // Save order to database (non-blocking)
                                    let app_state_for_db = app_state_for_stream.clone();
                                    let user_id_for_db = user_id_for_stream;
                                    let exchange_for_db = exchange_for_stream.clone();
                                    let pair_for_db = pair_for_stream.clone();
                                    let strategy_config_for_db = strategy_config_for_stream.clone();
                                    
                                    // Clone candle timestamp for save
                                    let candle_timestamp_for_save = DateTime::from_timestamp(candle.timestamp, 0).unwrap_or_else(|| Utc::now());
                                    
                                    // Check if we should send this signal based on existing positions
                                    let should_send_signal = match &signal {
                                        crate::services::strategy_engine::StrategySignal::Buy { .. } => {
                                            // Only send BUY if user doesn't have an open position for this pair
                                            match crate::services::position_service::has_open_position_for_pair(
                                                app_state_for_stream.db.as_ref(),
                                                user_id_for_stream,
                                                &pair_for_stream,
                                            ).await {
                                                Ok(false) => {
                                                    info!("✅ [User {}] No open position for {}, sending BUY signal", user_id_for_stream, pair_for_stream);
                                                    true
                                                }
                                                Ok(true) => {
                                                    info!("⏭️ [User {}] Already has open position for {}, skipping BUY signal", user_id_for_stream, pair_for_stream);
                                                    false
                                                }
                                                Err(e) => {
                                                    error!("❌ [User {}] Failed to check open position for {}: {}, will send signal anyway", user_id_for_stream, pair_for_stream, e);
                                                    true // Send anyway if check fails
                                                }
                                            }
                                        }
                                        crate::services::strategy_engine::StrategySignal::Sell { .. } => {
                                            // Only send SELL if user has an open position for this pair
                                            match crate::services::position_service::has_open_position_for_pair(
                                                app_state_for_stream.db.as_ref(),
                                                user_id_for_stream,
                                                &pair_for_stream,
                                            ).await {
                                                Ok(true) => {
                                                    info!("✅ [User {}] Has open position for {}, sending SELL signal", user_id_for_stream, pair_for_stream);
                                                    true
                                                }
                                                Ok(false) => {
                                                    info!("⏭️ [User {}] No open position for {}, skipping SELL signal", user_id_for_stream, pair_for_stream);
                                                    false
                                                }
                                                Err(e) => {
                                                    error!("❌ [User {}] Failed to check open position for {}: {}, will send signal anyway", user_id_for_stream, pair_for_stream, e);
                                                    true // Send anyway if check fails
                                                }
                                            }
                                        }
                                        crate::services::strategy_engine::StrategySignal::Hold => {
                                            false // Never send Hold signals
                                        }
                                    };
                                    
                                    if should_send_signal {
                                        // Spawn task to save signal (non-blocking)
                                        let app_state_for_db = app_state_for_stream.clone();
                                        let user_id_for_db = user_id_for_stream;
                                        let exchange_for_db = exchange_for_stream.clone();
                                        let pair_for_db = pair_for_stream.clone();
                                        let strategy_config_for_db = strategy_config_for_stream.clone();
                                        let signal_clone_for_db = signal.clone();
                                        
                                        tokio::spawn(async move {
                                            if let Err(e) = save_trading_order(
                                                &app_state_for_db,
                                                user_id_for_db,
                                                &exchange_for_db,
                                                &pair_for_db,
                                                &strategy_config_for_db,
                                                &signal_clone_for_db,
                                                Some(candle_timestamp_for_save),
                                                None, // Indicator values - TODO: extract from strategy state
                                            ).await {
                                                error!("Failed to save trading signal for user {}: {}", user_id_for_db, e);
                                            }
                                        });
                                        
                                        // Format and send signal to user
                                        let message = format_user_signal_message(
                                            &signal, 
                                            &pair_for_stream, 
                                            &bot_name_for_stream,
                                            &strategy_config_for_stream
                                        );
                                        
                                        if message.is_empty() {
                                            warn!("⚠️ [User {}] Message is empty for signal {:?}, skipping send", user_id_for_stream, signal);
                                        } else {
                                            info!("📤 [User {}] Sending signal message to chat {} (length: {} chars)", 
                                                user_id_for_stream, user_chat_id_for_stream, message.len());
                                            
                                            if let Err(e) = bot_for_stream.send_message(
                                                ChatId(user_chat_id_for_stream),
                                                &message
                                            )
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .await {
                                                error!("❌ [User {}] Failed to send trading signal to user {} (chat: {}): {}", 
                                                    user_id_for_stream, user_id_for_stream, user_chat_id_for_stream, e);
                                            } else {
                                                info!("✅ [User {}] Trading signal sent successfully to user (chat: {})", 
                                                    user_id_for_stream, user_chat_id_for_stream);
                                            }
                                        }
                                    } else {
                                        // Still save the signal to database for tracking, but don't send message
                                        let app_state_for_db = app_state_for_stream.clone();
                                        let user_id_for_db = user_id_for_stream;
                                        let exchange_for_db = exchange_for_stream.clone();
                                        let pair_for_db = pair_for_stream.clone();
                                        let strategy_config_for_db = strategy_config_for_stream.clone();
                                        let signal_clone_for_db = signal.clone();
                                        
                                        tokio::spawn(async move {
                                            if let Err(e) = save_trading_order(
                                                &app_state_for_db,
                                                user_id_for_db,
                                                &exchange_for_db,
                                                &pair_for_db,
                                                &strategy_config_for_db,
                                                &signal_clone_for_db,
                                                Some(candle_timestamp_for_save),
                                                None,
                                            ).await {
                                                error!("Failed to save trading signal for user {}: {}", user_id_for_db, e);
                                            }
                                        });
                                    }
                                } else {
                                    debug!("🔍 [User {}] Strategy '{}' evaluated: No signal generated (hold or None)", 
                                        user_id_for_stream, strategy_config_for_stream.strategy_type);
                                    
                                    // For RSI strategy, log why no signal was generated
                                    if strategy_config_for_stream.strategy_type.to_uppercase() == "RSI" {
                                        if let Some(state_info) = app_state_for_stream.strategy_executor.get_user_state_info(user_id_for_stream).await {
                                            debug!("📊 [User {}] RSI Strategy state: {}", user_id_for_stream, state_info);
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            warn!("⏱️ [User {}] Timeout acquiring lock for trading state", user_id_for_stream);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!("Stream closed for user {}, unsubscribing...", user_id_for_stream);
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!("User {} lagged behind stream, skipped {} events", user_id_for_stream, skipped);
                    // Continue processing
                }
            }
        }
        
        // Unsubscribe when loop ends
        stream_manager_clone.unsubscribe(&exchange_for_stream, &pair_for_stream, user_id_for_stream).await;
        info!("User {} unsubscribed from stream for {}", user_id_for_stream, pair_for_stream);
    });
    
    info!("✅ Trading service task spawned for user {}", user_id);

    // Start timer to force process candle based on strategy timeframe
    let state_timer = state.clone();
    let bot_timer = bot.clone();
    let pair_timer = pair.clone();
    let bot_name_timer = bot_name.clone();
    let strategy_config_timer = strategy_config.clone();
    let user_id_timer = user_id;
    let user_chat_id_timer = user_chat_id;
    let app_state_timer = app_state.clone();
    let exchange_timer = exchange.clone();
    
    // Parse timeframe to seconds (e.g., "1m" -> 60, "5m" -> 300, "1h" -> 3600)
    let timeframe_secs = parse_timeframe_to_seconds(&strategy_config.timeframe);
    
    // Spawn task for timer-based processing
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(timeframe_secs));
        timer.tick().await; // Skip first tick
        info!("⏰ [User {}] Timer started: will process candles every {} seconds ({} timeframe)", 
            user_id_timer, timeframe_secs, strategy_config_timer.timeframe);
        
        let mut timer_count = 0u64;
        let mut last_heartbeat_timer = std::time::Instant::now();
        let mut last_minute_log = std::time::Instant::now();
        
        loop {
            timer.tick().await;
            timer_count += 1;
            
            // Log heartbeat every 2 minutes
            if last_heartbeat_timer.elapsed().as_secs() >= 120 {
                info!("💓 [User {}] Timer heartbeat: Service active, processed {} timer ticks, strategy: {} on {} ({})", 
                    user_id_timer, timer_count, strategy_config_timer.strategy_type, 
                    exchange_timer, pair_timer);
                last_heartbeat_timer = std::time::Instant::now();
            }
            
            // For RSI strategy, log detailed stats every 1 minute
            if strategy_config_timer.strategy_type.to_uppercase() == "RSI" && last_minute_log.elapsed().as_secs() >= 60 {
                // Get strategy state info for logging
                if let Some(state_info) = app_state_timer.strategy_executor.get_user_state_info(user_id_timer).await {
                    info!("📊 [User {}] RSI Strategy Status (1-min heartbeat): {}", user_id_timer, state_info);
                } else {
                    warn!("⚠️ [User {}] Could not get RSI strategy state info for 1-min log", user_id_timer);
                }
                
                // Also log current candle state if available
                let state_read = state_timer.read().await;
                if let Some(ref candle) = state_read.current_candle {
                    info!("📊 [User {}] RSI Strategy - Current Candle: O={:.4} H={:.4} L={:.4} C={:.4} ({} timeframe)", 
                        user_id_timer, candle.open, candle.high, candle.low, candle.close,
                        strategy_config_timer.timeframe);
                } else {
                    info!("📊 [User {}] RSI Strategy - No active candle yet (waiting for market data)", user_id_timer);
                }
                drop(state_read);
                
                last_minute_log = std::time::Instant::now();
            }
            
            let mut state_guard = state_timer.write().await;
            
            if state_guard.current_candle.is_some() {
                info!("⏰ [User {}] Timer tick #{}: Processing candle...", user_id_timer, timer_count);
                
                if let Some(candle) = state_guard.force_process_candle_for_user(
                    &strategy_config_timer,
                    &app_state_timer,
                    user_id_timer
                ) {
                    info!("📊 [User {}] Timer: Candle completed: O={:.4} H={:.4} L={:.4} C={:.4} ({} timeframe)", 
                        user_id_timer, candle.open, candle.high, candle.low, candle.close,
                        strategy_config_timer.timeframe);
                    
                    // Process candle through user's strategy
                    info!("🔍 [User {}] Timer: Evaluating strategy '{}' on candle...", 
                        user_id_timer, strategy_config_timer.strategy_type);
                    
                    // Check if user is still trading before processing
                    let is_trading = app_state_timer.strategy_executor.is_user_trading(user_id_timer).await;
                    if !is_trading {
                        warn!("⚠️ [User {}] User is not trading anymore, skipping candle processing", user_id_timer);
                        continue;
                    }
                    
                    if let Some(signal) = app_state_timer.strategy_executor
                        .process_candle(user_id_timer, &candle).await 
                    {
                        match &signal {
                            crate::services::strategy_engine::StrategySignal::Buy { price, confidence, reason } => {
                                info!("✅ [User {}] Timer: Strategy '{}' generated BUY signal: price={:.4}, confidence={:.2}, reason={}", 
                                    user_id_timer, strategy_config_timer.strategy_type, 
                                    price, confidence, reason);
                            }
                            crate::services::strategy_engine::StrategySignal::Sell { price, confidence, reason } => {
                                info!("✅ [User {}] Timer: Strategy '{}' generated SELL signal: price={:.4}, confidence={:.2}, reason={}", 
                                    user_id_timer, strategy_config_timer.strategy_type, 
                                    price, confidence, reason);
                            }
                            crate::services::strategy_engine::StrategySignal::Hold => {
                                info!("✅ [User {}] Timer: Strategy '{}' generated HOLD signal", 
                                    user_id_timer, strategy_config_timer.strategy_type);
                            }
                        }
                        // Clone signal for database save
                        let signal_clone = signal.clone();
                        
                        // Save order to database
                        let app_state_for_db = app_state_timer.clone();
                        let user_id_for_db = user_id_timer;
                        let exchange_for_db = exchange_timer.clone();
                        let pair_for_db = pair_timer.clone();
                        let strategy_config_for_db = strategy_config_timer.clone();
                        
                        // Clone candle timestamp for save
                        let candle_timestamp_for_save = DateTime::from_timestamp(candle.timestamp, 0).unwrap_or_else(|| Utc::now());
                        
                        // Check if we should send this signal based on existing positions
                        let should_send_signal_timer = match &signal {
                            crate::services::strategy_engine::StrategySignal::Buy { .. } => {
                                // Only send BUY if user doesn't have an open position for this pair
                                match crate::services::position_service::has_open_position_for_pair(
                                    app_state_timer.db.as_ref(),
                                    user_id_timer,
                                    &pair_timer,
                                ).await {
                                    Ok(false) => {
                                        info!("✅ [User {}] Timer: No open position for {}, sending BUY signal", user_id_timer, pair_timer);
                                        true
                                    }
                                    Ok(true) => {
                                        info!("⏭️ [User {}] Timer: Already has open position for {}, skipping BUY signal", user_id_timer, pair_timer);
                                        false
                                    }
                                    Err(e) => {
                                        error!("❌ [User {}] Timer: Failed to check open position for {}: {}, will send signal anyway", user_id_timer, pair_timer, e);
                                        true // Send anyway if check fails
                                    }
                                }
                            }
                            crate::services::strategy_engine::StrategySignal::Sell { .. } => {
                                // Only send SELL if user has an open position for this pair
                                match crate::services::position_service::has_open_position_for_pair(
                                    app_state_timer.db.as_ref(),
                                    user_id_timer,
                                    &pair_timer,
                                ).await {
                                    Ok(true) => {
                                        info!("✅ [User {}] Timer: Has open position for {}, sending SELL signal", user_id_timer, pair_timer);
                                        true
                                    }
                                    Ok(false) => {
                                        info!("⏭️ [User {}] Timer: No open position for {}, skipping SELL signal", user_id_timer, pair_timer);
                                        false
                                    }
                                    Err(e) => {
                                        error!("❌ [User {}] Timer: Failed to check open position for {}: {}, will send signal anyway", user_id_timer, pair_timer, e);
                                        true // Send anyway if check fails
                                    }
                                }
                            }
                            crate::services::strategy_engine::StrategySignal::Hold => {
                                false // Never send Hold signals
                            }
                        };
                        
                        if should_send_signal_timer {
                            // Spawn task to save signal (non-blocking)
                            let app_state_for_db_timer = app_state_timer.clone();
                            let user_id_for_db_timer = user_id_timer;
                            let exchange_for_db_timer = exchange_timer.clone();
                            let pair_for_db_timer = pair_timer.clone();
                            let strategy_config_for_db_timer = strategy_config_timer.clone();
                            let signal_clone_for_db_timer = signal.clone();
                            
                            tokio::spawn(async move {
                                if let Err(e) = save_trading_order(
                                    &app_state_for_db_timer,
                                    user_id_for_db_timer,
                                    &exchange_for_db_timer,
                                    &pair_for_db_timer,
                                    &strategy_config_for_db_timer,
                                    &signal_clone_for_db_timer,
                                    Some(candle_timestamp_for_save),
                                    None, // Indicator values - TODO: extract from strategy state
                                ).await {
                                    error!("Failed to save trading signal for user {}: {}", user_id_for_db_timer, e);
                                }
                            });
                            
                            let message = format_user_signal_message(
                                &signal, 
                                &pair_timer, 
                                &bot_name_timer,
                                &strategy_config_timer
                            );
                            
                            if message.is_empty() {
                                warn!("⚠️ [User {}] Timer: Message is empty for signal {:?}, skipping send", user_id_timer, signal);
                            } else {
                                info!("📤 [User {}] Timer: Sending signal message to chat {} (length: {} chars)", 
                                    user_id_timer, user_chat_id_timer, message.len());
                                
                                if let Err(e) = bot_timer.send_message(
                                    ChatId(user_chat_id_timer),
                                    &message
                                )
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await {
                                    error!("❌ [User {}] Timer: Failed to send trading signal to user {} (chat: {}): {}", 
                                        user_id_timer, user_id_timer, user_chat_id_timer, e);
                                } else {
                                    info!("✅ [User {}] Timer: Trading signal sent successfully to user", user_id_timer);
                                }
                            }
                        } else {
                            // Still save the signal to database for tracking, but don't send message
                            let app_state_for_db_timer = app_state_timer.clone();
                            let user_id_for_db_timer = user_id_timer;
                            let exchange_for_db_timer = exchange_timer.clone();
                            let pair_for_db_timer = pair_timer.clone();
                            let strategy_config_for_db_timer = strategy_config_timer.clone();
                            let signal_clone_for_db_timer = signal.clone();
                            
                            tokio::spawn(async move {
                                if let Err(e) = save_trading_order(
                                    &app_state_for_db_timer,
                                    user_id_for_db_timer,
                                    &exchange_for_db_timer,
                                    &pair_for_db_timer,
                                    &strategy_config_for_db_timer,
                                    &signal_clone_for_db_timer,
                                    Some(candle_timestamp_for_save),
                                    None,
                                ).await {
                                    error!("Failed to save trading signal for user {}: {}", user_id_for_db_timer, e);
                                }
                            });
                        }
                    }
                } else {
                    info!("⏰ [User {}] Timer tick #{}: No active candle to process", user_id_timer, timer_count);
                }
            } else {
                debug!("⏰ [User {}] Timer tick #{}: No candle initialized yet (waiting for market data)", user_id_timer, timer_count);
            }
        }
    });

    info!("✅ User Trading Service started successfully for user {}", user_id);
}

/// Helper function to escape HTML characters for Telegram messages
/// Must escape & first to avoid double-escaping!
fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
}

/// Format user-specific trading signal message
fn format_user_signal_message(
    signal: &crate::services::strategy_engine::StrategySignal,
    pair: &str,
    bot_name: &str,
    strategy_config: &crate::services::strategy_engine::StrategyConfig,
) -> String {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    
    // Escape all user input to prevent HTML parsing errors
    let escaped_pair = escape_html(pair);
    let escaped_bot_name = escape_html(bot_name);
    let escaped_strategy_type = escape_html(&strategy_config.strategy_type);
    let escaped_timeframe = escape_html(&strategy_config.timeframe);
    
    match signal {
        crate::services::strategy_engine::StrategySignal::Buy { confidence, price, reason } => {
            let escaped_reason = escape_html(reason);
            format!(
                "🟢 <b>BUY SIGNAL - {}</b>\n\n\
💰 <b>Price:</b> <code>{:.4}</code> USDT\n\
📊 <b>Confidence:</b> <code>{:.1}%</code>\n\
📝 <b>Reason:</b> {}\n\
⏰ <b>Time:</b> <code>{}</code>\n\
📈 <b>Strategy:</b> {}\n\
📍 <b>Timeframe:</b> {}\n\
📍 <b>Pair:</b> {}\n\n\
🤖 <b>Bot:</b> {}\n\
🔄 <b>Status:</b> <code>Live Trading Active</code>\n\n\
⚠️ <i>This is a live trading signal. Always do your own research!</i>",
                escaped_pair, price, confidence * 100.0, escaped_reason, timestamp, 
                escaped_strategy_type, escaped_timeframe, escaped_pair, escaped_bot_name
            )
        },
        crate::services::strategy_engine::StrategySignal::Sell { confidence, price, reason } => {
            let escaped_reason = escape_html(reason);
            format!(
                "🔴 <b>SELL SIGNAL - {}</b>\n\n\
💰 <b>Price:</b> <code>{:.4}</code> USDT\n\
📊 <b>Confidence:</b> <code>{:.1}%</code>\n\
📝 <b>Reason:</b> {}\n\
⏰ <b>Time:</b> <code>{}</code>\n\
📉 <b>Strategy:</b> {}\n\
📍 <b>Timeframe:</b> {}\n\
📍 <b>Pair:</b> {}\n\n\
🤖 <b>Bot:</b> {}\n\
🔄 <b>Status:</b> <code>Live Trading Active</code>\n\n\
⚠️ <i>This is a live trading signal. Always do your own research!</i>",
                escaped_pair, price, confidence * 100.0, escaped_reason, timestamp,
                escaped_strategy_type, escaped_timeframe, escaped_pair, escaped_bot_name
            )
        },
        crate::services::strategy_engine::StrategySignal::Hold => {
            // Don't send messages for Hold signals
            return String::new();
        }
    }
}

/// Save trading signal to database and manage positions/trades
async fn save_trading_order(
    app_state: &Arc<AppState>,
    user_id: i64,
    exchange: &str,
    pair: &str,
    strategy_config: &crate::services::strategy_engine::StrategyConfig,
    signal: &crate::services::strategy_engine::StrategySignal,
    candle_timestamp: Option<DateTime<Utc>>,
    indicator_values: Option<serde_json::Value>,
) -> Result<(), anyhow::Error> {
    use crate::services::strategy_engine::StrategySignal;
    use crate::services::position_service;
    
    let (signal_type, side, price, confidence, reason) = match signal {
        StrategySignal::Buy { confidence, price, reason } => {
            ("buy".to_string(), "buy".to_string(), *price, *confidence, reason.clone())
        },
        StrategySignal::Sell { confidence, price, reason } => {
            ("sell".to_string(), "sell".to_string(), *price, *confidence, reason.clone())
        },
        StrategySignal::Hold => {
            // Don't save Hold signals
            return Ok(());
        }
    };
    
    // Get strategy ID if available (from strategy_config or lookup)
    let strategy_id = None; // TODO: Get strategy ID from strategy_config if available
    
    // Save signal first
    let signal = live_trading_signals::ActiveModel {
        user_id: ActiveValue::Set(user_id),
        strategy_id: ActiveValue::Set(strategy_id),
        strategy_name: ActiveValue::Set(Some(strategy_config.strategy_type.clone())),
        exchange: ActiveValue::Set(exchange.to_string()),
        pair: ActiveValue::Set(pair.to_string()),
        side: ActiveValue::Set(side.clone()),
        signal_type: ActiveValue::Set(signal_type.clone()),
        price: ActiveValue::Set(price.to_string()),
        confidence: ActiveValue::Set(Some(confidence.to_string())),
        reason: ActiveValue::Set(Some(reason)),
        timeframe: ActiveValue::Set(Some(strategy_config.timeframe.clone())),
        status: ActiveValue::Set("signal".to_string()),
        external_order_id: ActiveValue::NotSet,
        executed_price: ActiveValue::Set(Some(price.to_string())),
        executed_quantity: ActiveValue::NotSet, // TODO: Calculate based on available balance
        executed_at: ActiveValue::Set(Some(Utc::now())),
        created_at: ActiveValue::Set(Some(Utc::now())),
        updated_at: ActiveValue::Set(Some(Utc::now())),
        candle_timestamp: ActiveValue::Set(candle_timestamp),
        indicator_values: ActiveValue::Set(indicator_values.map(|v| v.to_string())),
        telegram_message_id: ActiveValue::NotSet, // Will be set when message is sent
        related_signal_id: ActiveValue::NotSet, // Can be set to link to previous signal
        ..Default::default()
    };
    
    let signal_result = live_trading_signals::Entity::insert(signal)
        .exec(app_state.db.as_ref())
        .await?;
    
    let signal_id = signal_result.last_insert_id;
    
    // For now, use a default quantity (in production, calculate based on balance)
    let quantity = 0.001; // Default quantity for demo
    
    // Handle Buy signal: Create position
    if side == "buy" {
        match position_service::create_position(
            app_state.db.as_ref(),
            user_id,
            Some(signal_id),
            strategy_id,
            Some(strategy_config.strategy_type.clone()),
            exchange.to_string(),
            pair.to_string(),
            price,
            quantity,
        ).await {
            Ok(position_id) => {
                info!("✅ Created position {} for user {}: {} {} at {}", position_id, user_id, side, pair, price);
            }
            Err(e) => {
                error!("Failed to create position for user {}: {}", user_id, e);
            }
        }
    }
    
    // Handle Sell signal: Close position and create trade
    if side == "sell" {
        // Find open position for this user and pair
        let open_positions = position_service::get_open_positions(app_state.db.as_ref(), user_id).await?;
        
        // Find matching position (same pair)
        if let Some(position) = open_positions.iter().find(|p| p.pair == pair && p.status == "open") {
            match position_service::close_position_and_create_trade(
                app_state.db.as_ref(),
                user_id,
                position.id,
                Some(signal_id),
                price,
            ).await {
                Ok(trade_id) => {
                    let pnl: f64 = f64::from_str(&position.unrealized_pnl.to_string()).unwrap_or(0.0);
                    info!("✅ Closed position {} and created trade {} for user {}: {} {} at {} (P&L: {:.2})", 
                        position.id, trade_id, user_id, side, pair, price, pnl);
                }
                Err(e) => {
                    error!("Failed to close position for user {}: {}", user_id, e);
                }
            }
        } else {
            warn!("No open position found for user {} to close with sell signal for {}", user_id, pair);
        }
    }
    
    info!("✅ Saved trading signal {} to database for user {}: {} {} at {}", signal_id, user_id, side, pair, price);
    
    Ok(())
}

/// Restore active live trading sessions from database on bot startup
/// This function queries all active sessions and restarts the trading services for them
pub async fn restore_active_sessions(
    app_state: Arc<crate::state::AppState>,
    bot: teloxide::Bot,
) -> Result<(), anyhow::Error> {
    use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
    use shared::entity::live_trading_sessions;
    
    info!("🔄 Restoring active live trading sessions from database...");
    
    // Query all active sessions
    let active_sessions = live_trading_sessions::Entity::find()
        .filter(live_trading_sessions::Column::Status.eq("active"))
        .all(app_state.db.as_ref())
        .await?;
    
    if active_sessions.is_empty() {
        info!("ℹ️ No active sessions found to restore");
        return Ok(());
    }
    
    info!("📋 Found {} active session(s) to restore", active_sessions.len());
    
    let mut restored_count = 0;
    let mut failed_count = 0;
    
    for session in active_sessions {
        let user_id = session.user_id;
        let user_chat_id = user_id; // In Telegram, user_id is typically the same as chat_id for private chats
        let strategy_id = session.strategy_id;
        let exchange = session.exchange.clone();
        let pair = session.pair.clone();
        let timeframe = session.timeframe.clone().unwrap_or_else(|| "1m".to_string());
        
        info!("🔄 Restoring session for user {}: strategy_id={:?}, exchange={}, pair={}, timeframe={}", 
            user_id, strategy_id, exchange, pair, timeframe);
        
        // Get strategy from database if strategy_id exists
        let strategy_config = if let Some(sid) = strategy_id {
            match app_state.strategy_service.get_strategy_by_id(sid).await? {
                Some(strategy) => {
                    match app_state.strategy_service.strategy_to_config(&strategy) {
                        Ok(config) => {
                            info!("✅ Loaded strategy config for user {}: {}", user_id, config.strategy_type);
                            config
                        }
                        Err(e) => {
                            error!("❌ Failed to convert strategy to config for user {}: {}", user_id, e);
                            failed_count += 1;
                            continue;
                        }
                    }
                }
                None => {
                    warn!("⚠️ Strategy {} not found for user {}, skipping session restore", sid, user_id);
                    failed_count += 1;
                    continue;
                }
            }
        } else {
            // If no strategy_id, try to create a basic config from session data
            warn!("⚠️ Session for user {} has no strategy_id, creating basic config from session data", user_id);
            use crate::services::strategy_engine::StrategyConfig;
            StrategyConfig {
                strategy_type: session.strategy_name.clone().unwrap_or_else(|| "RSI".to_string()),
                parameters: serde_json::json!({}),
                pair: pair.clone(),
                timeframe: timeframe.clone(),
                buy_condition: "RSI < 30".to_string(), // Default
                sell_condition: "RSI > 70".to_string(), // Default
            }
        };
        
        // Start strategy executor
        match app_state.strategy_executor.start_trading(
            user_id,
            strategy_config.clone(),
            Some(exchange.clone())
        ).await {
            Ok(_) => {
                info!("✅ Strategy executor started for user {}", user_id);
            }
            Err(e) => {
                error!("❌ Failed to start strategy executor for user {}: {}", user_id, e);
                failed_count += 1;
                continue;
            }
        }
        
        // Start user trading service (this will subscribe to stream and start monitoring)
        start_user_trading_service(
            app_state.clone(),
            bot.clone(),
            user_id,
            user_chat_id,
            strategy_config,
            exchange.clone(),
            pair.clone(),
        );
        
        info!("✅ Restored live trading session for user {}: {} on {} ({})", 
            user_id, session.strategy_name.as_ref().unwrap_or(&"Unknown".to_string()), exchange, pair);
        restored_count += 1;
    }
    
    info!("✅ Session restoration complete: {} restored, {} failed", restored_count, failed_count);
    
    Ok(())
}

