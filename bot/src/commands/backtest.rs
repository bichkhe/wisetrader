use std::sync::Arc;
use std::path::{Path, PathBuf};
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
use shared::entity::strategies;
use shared::FreqtradeApiClient;
use shared::StrategyTemplate;
use askama::Template;
use chrono::{Utc, Duration};
use crate::state::{AppState, BotState, BacktestState, MyDialogue};

/// Helper function to HTML escape
fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
}

/// Calculate timerange string for Freqtrade CLI (format: YYYYMMDD-)
fn calculate_timerange(range: &str) -> String {
    let now = Utc::now();
    let start_date = match range {
        "1day" => now - Duration::days(1),
        "1week" => now - Duration::days(7),
        "1month" => now - Duration::days(30),
        "3months" => now - Duration::days(90),
        "6months" => now - Duration::days(180),
        _ => now - Duration::days(7),
    };
    
    // Format as YYYYMMDD- for Freqtrade CLI
    format!("{}-", start_date.format("%Y%m%d"))
}

/// Parse strategy description to extract parameters (returns algorithm, buy, sell, timeframe, pair)
fn parse_strategy_description(description: &str) -> (String, String, String, String, String) {
    let mut algorithm = "RSI".to_string();
    let mut buy_condition = "RSI < 30".to_string();
    let mut sell_condition = "RSI > 70".to_string();
    let mut timeframe = "1h".to_string();
    let mut pair = "BTC/USDT".to_string();

    for line in description.lines() {
        if line.starts_with("Algorithm: ") {
            algorithm = line[11..].to_string();
        } else if line.starts_with("Buy: ") {
            buy_condition = line[5..].to_string();
        } else if line.starts_with("Sell: ") {
            sell_condition = line[6..].to_string();
        } else if line.starts_with("Timeframe: ") {
            timeframe = line[11..].to_string();
        } else if line.starts_with("Pair: ") {
            pair = line[6..].to_string();
        }
    }

    (algorithm, buy_condition, sell_condition, timeframe, pair)
}

/// Generate Python strategy file from strategy data
fn generate_strategy_file(
    strategy_id: u64,
    strategy_name: &str,
    algorithm: &str,
    buy_condition: &str,
    sell_condition: &str,
    timeframe: &str,
    strategies_path: &Path,
) -> Result<PathBuf, anyhow::Error> {
    use std::fs;
    
    // Ensure strategies directory exists
    fs::create_dir_all(strategies_path)?;
    
    // Determine indicators based on algorithm
    let (use_rsi, use_macd, use_ema, use_bb) = match algorithm {
        "RSI" => (true, false, false, false),
        "MACD" => (false, true, false, false),
        "EMA" => (false, false, true, false),
        "Bollinger Bands" => (false, false, false, true),
        "MA" => (false, false, true, false),
        _ => (true, false, false, false),
    };

    // Parse buy/sell conditions to determine entry/exit conditions
    let entry_condition_rsi = buy_condition.contains("RSI") && buy_condition.contains("<");
    let exit_condition_rsi = sell_condition.contains("RSI") && sell_condition.contains(">");
    let entry_condition_macd = buy_condition.contains("MACD");
    let entry_condition_ema = buy_condition.contains("EMA");
    let entry_condition_bb = buy_condition.contains("Bollinger") || buy_condition.contains("LowerBand");

    // Generate filename first to use for class name
    let safe_name = strategy_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>();
    let filename = format!("{}_{}.py", safe_name, strategy_id);
    
    // Class name must match filename (without .py) for Freqtrade
    // Freqtrade expects: filename RSI_5m_BTCUSDT_3.py -> class RSI_5m_BTCUSDT_3
    let class_name = filename
        .trim_end_matches(".py")
        .to_string();
    
    // Create template data with class name matching filename (Freqtrade requires exact match)
    let template = StrategyTemplate {
        strategy_name: class_name,
        minimal_roi_60: "0.05".to_string(),
        minimal_roi_30: "0.03".to_string(),
        minimal_roi_0: "0.01".to_string(),
        stoploss: "-0.10".to_string(),
        trailing_stop: false, // Will be converted to True/False in template
        trailing_stop_positive: "0.02".to_string(),
        trailing_stop_offset: "0.01".to_string(),
        timeframe: timeframe.to_string(),
        startup_candle_count: 200,
        
        use_rsi,
        rsi_period: 14,
        use_macd,
        macd_fast: 12,
        macd_slow: 26,
        macd_signal: 9,
        use_ema,
        ema_fast: 12,
        ema_slow: 26,
        use_bb,
        bb_period: 20,
        
        entry_condition_rsi,
        rsi_oversold: 30,
        entry_condition_macd,
        entry_condition_ema,
        entry_condition_bb,
        
        exit_condition_rsi,
        rsi_overbought: 70,
    };

    // Render template (class name already matches filename)
    let code = template.render()?;

    // Write file
    let filepath = strategies_path.join(&filename);
    fs::write(&filepath, code)?;

    Ok(filepath)
}

/// Handler to start backtest wizard
pub async fn handle_backtest(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    dialogue.update(BotState::Backtest(BacktestState::Start)).await?;

    let telegram_id = msg.from.as_ref().unwrap().id.0.to_string();
    let db = state.db.clone();

    // Get user's strategies
    use sea_orm::QueryOrder;
    let user_strategies = strategies::Entity::find()
        .filter(strategies::Column::TelegramId.eq(telegram_id.clone()))
        .order_by_desc(strategies::Column::CreatedAt)
        .all(db.as_ref())
        .await?;

    if user_strategies.is_empty() {
        bot.send_message(
            msg.chat.id,
            "❌ <b>No Strategies Found</b>\n\nYou haven't created any strategies yet.\n\nUse <code>/create_strategy</code> to create your first strategy!"
        )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        dialogue.exit().await?;
        return Ok(());
    }

    // Create inline keyboard with strategies
    let mut buttons = Vec::new();
    for strategy in &user_strategies {
        let name = strategy.name.as_ref()
            .map(|n| escape_html(n))
            .unwrap_or_else(|| format!("Strategy #{}", strategy.id));
        let button_text = if name.len() > 30 {
            format!("{}...", &name[..27])
        } else {
            name
        };
        buttons.push(vec![
            InlineKeyboardButton::callback(
                button_text,
                format!("backtest_strategy_{}", strategy.id)
            )
        ]);
    }
    buttons.push(vec![
        InlineKeyboardButton::callback("❌ Cancel", "backtest_cancel")
    ]);

    let welcome_msg = format!(
        "🤖 <b>Backtest Wizard</b>\n\n\
        <b>Step 1:</b> Choose a strategy to backtest:\n\n\
        You have <b>{}</b> strategy(ies) available.",
        user_strategies.len()
    );

    bot.send_message(msg.chat.id, welcome_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
        .await?;

    dialogue.update(BotState::Backtest(BacktestState::WaitingForStrategy)).await?;

    Ok(())
}

/// Handler for backtest callback queries
pub async fn handle_backtest_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    if let Some(data) = q.data {
        if let Some(msg) = q.message {
            let chat_id = msg.chat().id;
            let message_id = msg.id();

            match data.as_str() {
                "backtest_cancel" => {
                    bot.answer_callback_query(q.id).await?;
                    bot.edit_message_text(chat_id, message_id, "❌ Backtest cancelled.")
                        .await?;
                    dialogue.exit().await?;
                    return Ok(());
                }
                _ if data.starts_with("backtest_strategy_") => {
                    bot.answer_callback_query(q.id).await?;
                    let strategy_id: u64 = data.replace("backtest_strategy_", "").parse()?;
                    
                    // Get strategy from database
                    let strategy = strategies::Entity::find_by_id(strategy_id)
                        .one(state.db.as_ref())
                        .await?;

                    if let Some(strategy) = strategy {
                        let strategy_name = strategy.name.as_ref()
                            .unwrap_or(&format!("Strategy{}", strategy_id))
                            .clone();

                        // Show exchange selection
                        let exchange_buttons = vec![
                            vec![
                                InlineKeyboardButton::callback("🔵 Binance", "backtest_exchange_binance"),
                                InlineKeyboardButton::callback("🟢 OKX", "backtest_exchange_okx"),
                            ],
                            vec![
                                InlineKeyboardButton::callback("❌ Cancel", "backtest_cancel"),
                            ],
                        ];

                        bot.edit_message_text(
                            chat_id,
                            message_id,
                            format!(
                                "✅ <b>Strategy Selected:</b> {}\n\n\
                                <b>Step 2:</b> Choose exchange:",
                                escape_html(&strategy_name)
                            )
                        )
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(exchange_buttons))
                            .await?;

                        dialogue.update(BotState::Backtest(BacktestState::WaitingForExchange {
                            strategy_id,
                            strategy_name,
                        })).await?;
                    }
                }
                _ if data.starts_with("backtest_exchange_") => {
                    bot.answer_callback_query(q.id).await?;
                    let exchange = data.replace("backtest_exchange_", "");

                    if let Ok(Some(BotState::Backtest(BacktestState::WaitingForExchange { strategy_id, strategy_name }))) = dialogue.get().await {
                        // Show time range selection
                        let timerange_buttons = vec![
                            vec![
                                InlineKeyboardButton::callback("📅 1 Day", "backtest_timerange_1day"),
                                InlineKeyboardButton::callback("📅 1 Week", "backtest_timerange_1week"),
                            ],
                            vec![
                                InlineKeyboardButton::callback("📅 1 Month", "backtest_timerange_1month"),
                                InlineKeyboardButton::callback("📅 3 Months", "backtest_timerange_3months"),
                            ],
                            vec![
                                InlineKeyboardButton::callback("📅 6 Months", "backtest_timerange_6months"),
                            ],
                            vec![
                                InlineKeyboardButton::callback("❌ Cancel", "backtest_cancel"),
                            ],
                        ];

                        bot.edit_message_text(
                            chat_id,
                            message_id,
                            format!(
                                "✅ <b>Exchange:</b> {}\n\n\
                                <b>Step 3:</b> Choose time range for backtest:",
                                escape_html(&exchange)
                            )
                        )
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(timerange_buttons))
                            .await?;

                        dialogue.update(BotState::Backtest(BacktestState::WaitingForTimeRange {
                            strategy_id,
                            strategy_name,
                            exchange,
                        })).await?;
                    }
                }
                _ if data.starts_with("backtest_timerange_") => {
                    bot.answer_callback_query(q.id).await?;
                    let timerange = data.replace("backtest_timerange_", "");

                    if let Ok(Some(BotState::Backtest(BacktestState::WaitingForTimeRange { strategy_id, strategy_name, exchange }))) = dialogue.get().await {
                        // Get strategy details
                        let strategy = strategies::Entity::find_by_id(strategy_id)
                            .one(state.db.as_ref())
                            .await?;

                        let strategy_desc = strategy.as_ref()
                            .and_then(|s| s.description.as_ref())
                            .map(|s| s.as_str())
                            .unwrap_or("");

                        let (algorithm, buy_condition, sell_condition, timeframe, pair) = 
                            parse_strategy_description(strategy_desc);
                        
                        // Convert pair format if needed (BTCUSDT -> BTC/USDT)
                        let freqtrade_pair = if pair.contains('/') {
                            pair.clone()
                        } else {
                            // Convert BTCUSDT to BTC/USDT
                            if pair.len() > 4 && pair.ends_with("USDT") {
                                format!("{}/{}", &pair[..pair.len()-4], &pair[pair.len()-4..])
                            } else if pair.len() > 3 && pair.ends_with("BTC") {
                                format!("{}/{}", &pair[..pair.len()-3], &pair[pair.len()-3..])
                            } else {
                                format!("{}/USDT", pair) // Default to USDT pair
                            }
                        };

                        // Show processing message
                        bot.edit_message_text(
                            chat_id,
                            message_id,
                            format!(
                                "⏳ <b>Starting Backtest...</b>\n\n\
                                <b>Strategy:</b> {}\n\
                                <b>Exchange:</b> {}\n\
                                <b>Time Range:</b> {}\n\n\
                                Generating strategy file and running backtest...",
                                escape_html(&strategy_name),
                                escape_html(&exchange),
                                timerange
                            )
                        )
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;

                        // Generate strategy file
                        let strategies_path = Path::new("./docker/freqtrade/strategies");
                        
                        match generate_strategy_file(
                            strategy_id,
                            &strategy_name,
                            &algorithm,
                            &buy_condition,
                            &sell_condition,
                            &timeframe,
                            strategies_path,
                        ) {
                            Ok(filepath) => {
                                tracing::info!("Generated strategy file: {:?}", filepath);

                                // Get strategy name for Freqtrade
                                let default_name = format!("Strategy{}", strategy_id);
                                let freq_strategy_name = filepath.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(&default_name);

                                // Prepare timerange
                                let timerange_str = calculate_timerange(&timerange);

                                // Initialize Freqtrade client
                                let freq_client = FreqtradeApiClient::new(
                                    "http://127.0.0.1:9081".to_string(),
                                    "freqtrader".to_string(),
                                    "freqtraderpass".to_string(),
                                );

                                // Check if Freqtrade is running
                                // match freq_client.ping().await {
                                //     Ok(_) => {}
                                //     Err(e) => {
                                //         bot.edit_message_text(
                                //             chat_id,
                                //             message_id,
                                //             format!("❌ <b>Freqtrade Not Available</b>\n\nError: {}", e)
                                //         )
                                //             .parse_mode(teloxide::types::ParseMode::Html)
                                //             .await?;
                                //         dialogue.exit().await?;
                                //         return Ok(());
                                //     }
                                // }

                                // Update message - checking/downloading data
                                bot.edit_message_text(
                                    chat_id,
                                    message_id,
                                    format!(
                                        "⏳ <b>Step 1: Checking Data...</b>\n\n\
                                        <b>Strategy:</b> {}\n\
                                        <b>Exchange:</b> {}\n\
                                        <b>Pair:</b> {}\n\
                                        <b>Timeframe:</b> {}\n\
                                        <b>Time Range:</b> {}\n\n\
                                        🔍 Checking if historical data exists...",
                                        escape_html(&strategy_name),
                                        escape_html(&exchange),
                                        freqtrade_pair,
                                        timeframe,
                                        timerange
                                    )
                                )
                                    .parse_mode(teloxide::types::ParseMode::Html)
                                    .await?;

                                // Run backtest via CLI (with data download check)
                                let result = match freq_client.backtest_via_cli(
                                    "wisetrader_freqtrade",
                                    freq_strategy_name,
                                    &exchange,
                                    &freqtrade_pair,
                                    &timeframe,
                                    &timerange_str,
                                ).await {
                                    Ok(result) => Ok(result),
                                    Err(e) => {
                                        // Truncate error message to avoid Telegram MESSAGE_TOO_LONG error
                                        let error_str = e.to_string();
                                        let truncated_error = if error_str.len() > 1500 {
                                            format!("{}...\n\n(Error message truncated)", &error_str[..1500])
                                        } else {
                                            error_str.clone()
                                        };
                                        
                                        bot.edit_message_text(
                                            chat_id,
                                            message_id,
                                            format!(
                                                "❌ <b>Backtest Failed</b>\n\n\
                                                <b>Error:</b>\n\
                                                <code>{}</code>\n\n\
                                                💾 Strategy file: <code>{}</code>\n\n\
                                                💡 <i>Tip: Make sure data is downloaded for all required pairs.</i>",
                                                escape_html(&truncated_error),
                                                filepath.display()
                                            )
                                        )
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .await?;
                                        Err(e)
                                    }
                                };

                                match result {
                                    Ok(result) => {
                                        tracing::info!(
                                            "Backtest succeeded: strategy={} exchange={} trades={} profit_pct={:.2}",
                                            strategy_name,
                                            exchange,
                                            result.trades,
                                            result.profit_pct
                                        );
                                        
                                        // Build result message with timing info
                                        let mut result_msg = format!(
                                            "✅ <b>Backtest Complete!</b>\n\n\
                                            <b>Strategy:</b> {}\n\
                                            <b>Exchange:</b> {}\n\
                                            <b>Pair:</b> {}\n\
                                            <b>Time Range:</b> {}\n\
                                            <b>Timeframe:</b> {}\n\n\
                                            <b>📊 Results:</b>\n\
                                            📈 Total Trades: <b>{}</b>\n\
                                            💰 Profit: <b>{:.2}%</b>\n\n\
                                            <b>⏱️ Timing:</b>\n",
                                            escape_html(&strategy_name),
                                            escape_html(&exchange),
                                            freqtrade_pair,
                                            timerange,
                                            timeframe,
                                            result.trades,
                                            result.profit_pct
                                        );
                                        
                                        if let Some(dl_time) = result.download_time_secs {
                                            result_msg.push_str(&format!("📥 Data Download: <b>{}s</b>\n", dl_time));
                                        } else {
                                            result_msg.push_str("📥 Data Download: <b>Skipped (already exists)</b>\n");
                                        }
                                        result_msg.push_str(&format!("🔄 Backtest Execution: <b>{}s</b>\n", result.backtest_time_secs));
                                        
                                        let total_time = result.download_time_secs.unwrap_or(0) + result.backtest_time_secs;
                                        result_msg.push_str(&format!("⏱️ Total Time: <b>{}s</b>\n\n", total_time));
                                        result_msg.push_str(&format!("💾 Strategy file: <code>{}</code>", filepath.display()));
                                        
                                        bot.edit_message_text(
                                            chat_id,
                                            message_id,
                                            result_msg
                                        )
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .await?;
                                    }
                                    Err(_) => {
                                        // Error already handled above
                                    }
                                }

                                dialogue.exit().await?;
                            }
                            Err(e) => {
                                tracing::error!("Failed to generate strategy file: {}", e);
                                bot.edit_message_text(
                                    chat_id,
                                    message_id,
                                    format!("❌ <b>Failed to generate strategy file</b>\n\nError: {}", e)
                                )
                                    .parse_mode(teloxide::types::ParseMode::Html)
                                    .await?;
                                dialogue.exit().await?;
                            }
                        }
                    }
                }
                _ => {
                    bot.answer_callback_query(q.id).await?;
                }
            }
        }
    }

    Ok(())
}

