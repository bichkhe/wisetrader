use std::sync::Arc;
use barter_data::{
    exchange::binance::spot::BinanceSpot,
    streams::Streams,
    subscription::trade::PublicTrades,
};
use barter_instrument::instrument::market_data::kind::MarketDataInstrumentKind;
use barter_data::streams::reconnect::Event;
use futures::StreamExt;
use ta::{
    indicators::RelativeStrengthIndex,
    Next,
};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};
use teloxide::prelude::*;
use chrono::Utc;

use crate::state::AppState;

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
        _user_id: i64,
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
            self.current_candle = Some(OneMinuteCandle::new(price, timestamp));
        }
        
        None
    }

    /// Force process candle for user (timer-based)
    fn force_process_candle_for_user(
        &mut self,
        _strategy_config: &crate::services::strategy_engine::StrategyConfig,
        _app_state: &Arc<crate::state::AppState>,
        _user_id: i64,
    ) -> Option<crate::services::strategy_engine::Candle> {
        if let Some(ref mut candle) = self.current_candle {
            if candle.processed {
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
    
    // Parse pair (e.g., "BTC/USDT" -> base="BTC", quote="USDT")
    let pair_parts: Vec<&str> = pair.split('/').collect();
    if pair_parts.len() != 2 {
        error!("Invalid pair format: {}", pair);
        return;
    }
    let base = pair_parts[0].to_lowercase();
    let quote = pair_parts[1].to_lowercase();
    
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
    
    // Use LocalSet for non-Send futures
    use tokio::task::LocalSet;
    
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let local = LocalSet::new();
            local.spawn_local(async move {
                // Initialize streams based on exchange
                let streams_result = match exchange_for_stream.as_str() {
                    "binance" => {
                        Streams::<PublicTrades>::builder()
                            .subscribe([(
                                BinanceSpot::default(),
                                base.as_str(),
                                quote.as_str(),
                                MarketDataInstrumentKind::Spot,
                                PublicTrades,
                            )])
                            .init()
                            .await
                    }
                    "okx" => {
                        // TODO: Add OKX support when barter-data supports it
                        error!("OKX exchange not yet supported in barter-data");
                        return;
                    }
                    _ => {
                        error!("Unsupported exchange: {}", exchange_for_stream);
                        return;
                    }
                };
                    
                let streams = match streams_result {
                    Ok(streams) => streams,
                    Err(e) => {
                        error!("Failed to initialize streams for user {}: {}", user_id_for_stream, e);
                        return;
                    }
                };

                info!("✅ Connected to {}! Monitoring {} for user {} trading signals...", 
                    exchange_for_stream, pair_for_stream, user_id_for_stream);

                let mut market_stream = streams.select_all();
                let state_trades = state_for_stream.clone();
                let bot_trades = bot_for_stream.clone();
                let pair_trades = pair_for_stream.clone();
                let strategy_config_trades = strategy_config_for_stream.clone();

                // Process trades
                while let Some(event) = market_stream.next().await {
                    match event {
                        Event::Item(market_event_result) => {
                            match market_event_result {
                                Ok(market_event) => {
                                    let price = market_event.kind.price;
                                    let timestamp = market_event.time_received.timestamp();
                                    
                                    let mut state_guard = state_trades.write().await;
                                    if let Some(candle) = state_guard.process_trade_for_user(
                                        price, 
                                        timestamp, 
                                        &strategy_config_trades,
                                        &app_state_for_stream,
                                        user_id_for_stream
                                    ) {
                                        // Process candle through user's strategy
                                        if let Some(signal) = app_state_for_stream.strategy_executor
                                            .process_candle(user_id_for_stream, &candle).await 
                                        {
                                            // Format and send signal to user
                                            let message = format_user_signal_message(
                                                &signal, 
                                                &pair_trades, 
                                                &bot_name_for_stream,
                                                &strategy_config_trades
                                            );
                                            
                                            if let Err(e) = bot_trades.send_message(
                                                ChatId(user_chat_id_for_stream),
                                                message
                                            )
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .await {
                                                error!("Failed to send trading signal to user {}: {}", user_id_for_stream, e);
                                            } else {
                                                info!("✅ Trading signal sent to user {} (chat: {})", user_id_for_stream, user_chat_id_for_stream);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Error in market event for user {}: {}", user_id_for_stream, e);
                                }
                            }
                        }
                        Event::Reconnecting(_origin) => {
                            warn!("Reconnecting to {} for user {}...", exchange_for_stream, user_id_for_stream);
                        }
                    }
                }
            });
            local.await;
        });
    });

    // Start timer to force process candle based on strategy timeframe
    let state_timer = state.clone();
    let bot_timer = bot.clone();
    let pair_timer = pair.clone();
    let bot_name_timer = bot_name.clone();
    let strategy_config_timer = strategy_config.clone();
    let user_id_timer = user_id;
    let user_chat_id_timer = user_chat_id;
    let app_state_timer = app_state.clone();
    
    // Parse timeframe to seconds (e.g., "1m" -> 60, "5m" -> 300, "1h" -> 3600)
    let timeframe_secs = parse_timeframe_to_seconds(&strategy_config.timeframe);
    
    // Spawn task for timer-based processing
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(timeframe_secs));
        timer.tick().await; // Skip first tick
        info!("⏰ Timer started for user {}: will process candles every {} seconds", user_id_timer, timeframe_secs);
        
        loop {
            timer.tick().await;
            let mut state_guard = state_timer.write().await;
            
            if state_guard.current_candle.is_some() {
                if let Some(candle) = state_guard.force_process_candle_for_user(
                    &strategy_config_timer,
                    &app_state_timer,
                    user_id_timer
                ) {
                    // Process candle through user's strategy
                    if let Some(signal) = app_state_timer.strategy_executor
                        .process_candle(user_id_timer, &candle).await 
                    {
                        let message = format_user_signal_message(
                            &signal, 
                            &pair_timer, 
                            &bot_name_timer,
                            &strategy_config_timer
                        );
                        
                        if !message.is_empty() {
                            if let Err(e) = bot_timer.send_message(
                                ChatId(user_chat_id_timer),
                                message
                            )
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await {
                                error!("Failed to send trading signal to user {}: {}", user_id_timer, e);
                            } else {
                                info!("✅ Trading signal sent to user {} (timer-based)", user_id_timer);
                            }
                        }
                    }
                }
            }
        }
    });

    info!("✅ User Trading Service started successfully for user {}", user_id);
}

/// Format user-specific trading signal message
fn format_user_signal_message(
    signal: &crate::services::strategy_engine::StrategySignal,
    pair: &str,
    bot_name: &str,
    strategy_config: &crate::services::strategy_engine::StrategyConfig,
) -> String {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    match signal {
        crate::services::strategy_engine::StrategySignal::Buy { confidence, price, reason } => {
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
                pair, price, confidence * 100.0, reason, timestamp, 
                strategy_config.strategy_type, strategy_config.timeframe, pair, bot_name
            )
        },
        crate::services::strategy_engine::StrategySignal::Sell { confidence, price, reason } => {
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
                pair, price, confidence * 100.0, reason, timestamp,
                strategy_config.strategy_type, strategy_config.timeframe, pair, bot_name
            )
        },
        crate::services::strategy_engine::StrategySignal::Hold => {
            // Don't send messages for Hold signals
            return String::new();
        }
    }
}

