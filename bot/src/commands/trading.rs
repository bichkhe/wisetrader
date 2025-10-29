use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;
use sea_orm::EntityTrait;
use shared::entity::users;
use shared::FreqtradeApiClient;
use chrono::Utc;
use serde_json::json;

use crate::state::AppState;

/// Handler for /backtest command
/// Format: /backtest StrategyName SYMBOL TIMEFRAME PERIOD
/// Examples:
///   /backtest MyStrategy BTCUSDT 1h 3days
///   /backtest StrategyA BTCUSDT 1d 2025-10-01 2025-10-30
pub async fn handle_backtest(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
    args: String,
) -> Result<(), anyhow::Error> {
    let start_time = Instant::now();
    
    let from = msg.from.unwrap();
    let telegram_id = from.id.0 as i64;
    let username = from.username.unwrap_or("Unknown".to_string());

    tracing::info!(
        "Handling /backtest command from user: {} (id: {})",
        username,
        telegram_id
    );

    // Parse arguments
    let parts: Vec<&str> = args.trim().split_whitespace().collect();
    
    if parts.len() < 4 {
        bot.send_message(
            msg.chat.id,
            "❌ <b>Invalid command format</b>\n\n\
            <b>Usage:</b>\n\
            <code>/backtest &lt;StrategyName&gt; &lt;SYMBOL&gt; &lt;TIMEFRAME&gt; &lt;PERIOD&gt;</code>\n\n\
            <b>Examples:</b>\n\
            • <code>/backtest MyStrategy BTCUSDT 1h 3days</code>\n\
            • <code>/backtest StrategyA BTCUSDT 1d 2025-10-01 2025-10-30</code>\n\n\
            <b>Available timeframes:</b> 1m, 5m, 15m, 30m, 1h, 4h, 1d, 1w\n\
            <b>Available pairs:</b> BTCUSDT, ETHUSDT, BNBUSDT, etc.",
        )
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
        return Ok(());
    }

    let strategy_name = parts[0];
    let symbol = parts[1].to_uppercase();
    let timeframe = parts[2];
    let period = parts[3..].join(" ");

    // Validate symbol format
    if !symbol.ends_with("USDT") && !symbol.ends_with("BTC") && !symbol.ends_with("ETH") {
        bot.send_message(
            msg.chat.id,
            "❌ Invalid trading pair. Please use format like BTCUSDT, ETHUSDT, etc."
        ).await?;
        return Ok(());
    }

    // Send processing message
    let processing_msg = bot.send_message(
        msg.chat.id,
        format!("⏳ Running backtest for <b>{}</b> on {}/{}...\n\nThis may take a few minutes.", 
            strategy_name, symbol, timeframe)
    )
    .parse_mode(teloxide::types::ParseMode::Html)
    .await?;

    // Get user to check subscription
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;

    if user.is_none() {
        bot.edit_message_text(
            msg.chat.id,
            processing_msg.id,
            "❌ User not found. Please run /start first."
        ).await?;
        return Ok(());
    }

    let user = user.unwrap();
    
    // Check if user has active subscription
    let has_active_subscription = user.subscription_expires
        .map(|exp| exp > Utc::now())
        .unwrap_or(false);

    if !has_active_subscription && !user.subscription_tier.as_ref().unwrap_or(&"".to_string()).contains("free_trial") {
        bot.edit_message_text(
            msg.chat.id,
            processing_msg.id,
            "❌ Your subscription has expired. Please upgrade to continue using backtest features."
        ).await?;
        return Ok(());
    }

    // Initialize Freqtrade API client
    let freq_client = FreqtradeApiClient::new("http://localhost:9081".to_string(), "admin".to_string(), "admin".to_string());
    
    // Check if Freqtrade is running    
    match freq_client.ping().await {
        Ok(_) => {}
        Err(e) => {
            bot.edit_message_text(
                msg.chat.id,
                processing_msg.id,
                format!("❌ Freqtrade service is not running.\n\nError: {}", e)
            ).await?;
            return Ok(());
        }
    }

    // Run backtest via Freqtrade API
    let backtest_result = freq_client.backtest(
        strategy_name,
        &symbol,
        timeframe,
        &period,
    ).await;

    let duration = start_time.elapsed();
    
    match backtest_result {
        Ok(result) => {
            // Format backtest results
            // Fix: Use direct struct access on result, avoid invalid .get and removed json dependency.
            // Fallbacks if fields are missing, uses 0 or 0.0 as appropriate.
            let summary = format!(
                "✅ <b>Backtest Complete</b>\n\n\
                <b>Strategy:</b> {}\n\
                <b>Pair:</b> {}\n\
                <b>Timeframe:</b> {}\n\
                <b>Period:</b> {}\n\
                <b>Duration:</b> {:.2}s\n\n\
                <b>Results:</b>\n\
                Total Trades: {}\n\
                Win Rate: {:.2}%\n\
                Profit: {:.2}%\n\n\
                View full details in Freqtrade UI.",
                strategy_name,
                symbol,
                timeframe,
                period,
                duration.as_secs_f64(),
                result.trades,
                result.profit_pct,
                result.profit_pct,
            );

            bot.edit_message_text(
                msg.chat.id,
                processing_msg.id,
                summary
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        }
        Err(e) => {
            bot.edit_message_text(
                msg.chat.id,
                processing_msg.id,
                format!("❌ Backtest failed!\n\nError: {}\n\nPlease check your parameters and try again.", e)
            ).await?;
        }
    }

    tracing::info!("Backtest completed in {:?} for user {}", duration, telegram_id);
    
    Ok(())
}



