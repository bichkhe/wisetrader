use std::sync::Arc;
use std::path::{Path, PathBuf};
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
use shared::entity::strategies;
use shared::FreqtradeApiClient;
use shared::{StrategyTemplate, BacktestReportTemplate, Config};
use std::fs;
use askama::Template;
use chrono::{Utc, Duration};
use crate::state::{AppState, BotState, BacktestState, MyDialogue};
use crate::i18n;

/// Helper function to HTML escape
fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
}

/// Split text into chunks at character boundaries (safe for UTF-8)
fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_pos = 0;
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();
    
    while current_pos < total_chars {
        let end_pos = std::cmp::min(current_pos + max_chars, total_chars);
        let chunk: String = chars[current_pos..end_pos].iter().collect();
        chunks.push(chunk);
        current_pos = end_pos;
    }
    
    chunks
}

/// Extract all tables from freqtrade output, returns vector of (title, content)
fn extract_all_tables(stdout: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = stdout.lines().collect();
    let mut tables: Vec<(String, String)> = Vec::new();
    let mut current_table_title = String::new();
    let mut current_table_lines: Vec<String> = Vec::new();
    let mut in_table = false;
    
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Detect table titles (centered text with REPORT/STATS/METRICS/SUMMARY, no box-drawing chars)
        let is_title = trimmed.len() > 10 && trimmed.len() < 100 && 
                       (trimmed.contains("REPORT") || 
                        trimmed.contains("STATS") || 
                        trimmed.contains("METRICS") ||
                        trimmed.contains("SUMMARY")) &&
                       !trimmed.contains("┃") && !trimmed.contains("│") && 
                       !trimmed.contains("┼") && !trimmed.contains("┡") &&
                       !trimmed.contains("━") && !trimmed.contains("═");
        
        // Detect table lines (box-drawing characters)
        let is_table_line = trimmed.contains("┃") || trimmed.contains("│") || 
                           trimmed.contains("┡") || trimmed.contains("┼") ||
                           trimmed.contains("┏") || trimmed.contains("┗") ||
                           (trimmed.contains("━━") && trimmed.len() > 20) ||
                           (trimmed.contains("══") && trimmed.len() > 20);
        
        if is_title {
            // Save previous table if exists
            if in_table && !current_table_lines.is_empty() {
                tables.push((current_table_title.clone(), current_table_lines.join("\n")));
                current_table_lines.clear();
            }
            // Start new table
            current_table_title = trimmed.to_string();
            in_table = true;
        } else if in_table {
            if is_table_line {
                current_table_lines.push(line.to_string());
            } else if trimmed.is_empty() {
                // Empty line within table (separator)
                if current_table_lines.len() > 0 {
                    current_table_lines.push(line.to_string());
                }
            } else {
                // Check if next line is still part of table
                let next_is_table = lines.get(idx + 1)
                    .map(|l| {
                        let t = l.trim();
                        t.contains("┃") || t.contains("│") || t.contains("┡") || 
                        t.contains("┼") || t.contains("┏") || t.contains("┗") ||
                        t.is_empty() || t.contains("━━") || t.contains("══")
                    })
                    .unwrap_or(false);
                
                if !next_is_table && current_table_lines.len() > 5 {
                    // End of table, save it
                    tables.push((current_table_title.clone(), current_table_lines.join("\n")));
                    current_table_lines.clear();
                    current_table_title.clear();
                    in_table = false;
                }
            }
        }
    }
    
    // Save last table if exists
    if !current_table_lines.is_empty() {
        tables.push((current_table_title, current_table_lines.join("\n")));
    }
    
    tables
}

/// Extract summary table section from freqtrade output (legacy, kept for compatibility)
fn extract_summary_table(stdout: &str) -> String {
    let lines: Vec<&str> = stdout.lines().collect();
    let mut summary_lines: Vec<String> = Vec::new();
    let mut in_summary = false;
    let mut table_lines = 0;
    
    for line in lines.iter() {
        let trimmed = line.trim();
        
        // Detect summary section
        if trimmed.contains("SUMMARY") || trimmed.contains("BACKTEST RESULT") || trimmed.contains("===================") {
            in_summary = true;
            if !trimmed.contains("========") {
                summary_lines.push(line.to_string());
            }
            continue;
        }
        
        if in_summary {
            // Collect table lines (usually contain | or multiple spaces)
            if trimmed.contains("|") || (trimmed.len() > 20 && trimmed.chars().filter(|c| c.is_whitespace()).count() > 5) {
                summary_lines.push(line.to_string());
                table_lines += 1;
                // Limit table size to avoid message too long
                if table_lines > 30 {
                    summary_lines.push("... (table truncated)".to_string());
                    break;
                }
            } else if trimmed.is_empty() {
                if summary_lines.len() > 5 {
                    summary_lines.push(line.to_string());
                }
            } else if table_lines > 5 && !trimmed.contains("=") && !trimmed.contains("-") {
                // End of summary section
                break;
            }
        }
    }
    
    // If no summary found, return key metrics lines
    if summary_lines.is_empty() || summary_lines.len() < 3 {
        for line in lines.iter() {
            let trimmed = line.trim();
            if trimmed.contains("Total") || trimmed.contains("Profit") || trimmed.contains("Win") || 
               trimmed.contains("Drawdown") || trimmed.contains("Trades") {
                summary_lines.push(line.to_string());
            }
            if summary_lines.len() > 15 {
                break;
            }
        }
    }
    
    summary_lines.join("\n")
}

/// Generate HTML report from backtest results
async fn generate_html_report(
    config: &Config,
    strategy_name: &str,
    exchange: &str,
    pair: &str,
    timeframe: &str,
    timerange: &str,
    result: &shared::BacktestResult,
    tables: &[(String, String)],
) -> Result<Option<String>, anyhow::Error> {
    // Create reports directory if it doesn't exist
    fs::create_dir_all(&config.html_reports_dir)?;
    
    // Generate unique filename
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("backtest_{}_{}.html", 
        strategy_name.replace(" ", "_").replace("/", "_"),
        timestamp
    );
    let filepath = Path::new(&config.html_reports_dir).join(&filename);
    
    // Create template
    let template = BacktestReportTemplate::new(
        strategy_name.to_string(),
        exchange.to_string(),
        pair.to_string(),
        timeframe.to_string(),
        timerange.to_string(),
        result.trades,
        result.profit_pct,
        result.win_rate,
        result.max_drawdown,
        result.starting_balance,
        result.final_balance,
        result.download_time_secs,
        result.backtest_time_secs,
        tables.to_vec(),
        result.stdout.clone(),
    );
    
    // Render template
    let html_content = template.render()?;
    
    // Write to file
    fs::write(&filepath, html_content)?;
    
    tracing::info!("HTML report saved to: {}", filepath.display());
    
    // Return URL - use API server if available, otherwise use file:// or custom base URL
    let url = if let Some(ref base_url) = config.html_reports_base_url {
        format!("{}/{}", base_url.trim_end_matches('/'), filename)
    } else {
        // Use API server by default
        format!("{}/reports/{}", config.api_base_url.trim_end_matches('/'), filename)
    };
    
    Ok(Some(url))
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

    let telegram_id = msg.from.as_ref().unwrap().id.0 as i64;
    let db = state.db.clone();

    // Get user language
    let user = shared::entity::users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");

    // Get user's strategies
    use sea_orm::QueryOrder;
    let user_strategies = strategies::Entity::find()
        .filter(strategies::Column::TelegramId.eq(telegram_id.to_string()))
        .order_by_desc(strategies::Column::CreatedAt)
        .all(db.as_ref())
        .await?;

    if user_strategies.is_empty() {
        let empty_msg = i18n::translate(locale, "error_no_strategies", None);
        bot.send_message(msg.chat.id, empty_msg)
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
        InlineKeyboardButton::callback(
            i18n::get_button_text(locale, "backtest_button_cancel"),
            "backtest_cancel"
        )
    ]);

    let welcome_msg = i18n::translate(locale, "backtest_welcome", Some(&[("count", &user_strategies.len().to_string())]));

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
    // Get user locale
    let user_id = q.from.id.0 as i64;
    let user = shared::entity::users::Entity::find_by_id(user_id)
        .one(state.db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    if let Some(data) = q.data {
        if let Some(msg) = q.message {
            let chat_id = msg.chat().id;
            let message_id = msg.id();

            match data.as_str() {
                "backtest_cancel" => {
                    bot.answer_callback_query(q.id).await?;
                    let cancel_msg = i18n::translate(locale, "backtest_cancelled", None);
                    bot.edit_message_text(chat_id, message_id, cancel_msg)
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
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "backtest_exchange_binance"),
                                    "backtest_exchange_binance"
                                ),
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "backtest_exchange_okx"),
                                    "backtest_exchange_okx"
                                ),
                            ],
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "backtest_button_cancel"),
                                    "backtest_cancel"
                                ),
                            ],
                        ];

                        let escaped_name = escape_html(&strategy_name);
                        let strategy_selected = i18n::translate(locale, "backtest_strategy_selected", Some(&[("strategy_name", &escaped_name)]));
                        bot.edit_message_text(chat_id, message_id, strategy_selected)
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
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_1day"),
                                    "backtest_timerange_1day"
                                ),
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_1week"),
                                    "backtest_timerange_1week"
                                ),
                            ],
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_1month"),
                                    "backtest_timerange_1month"
                                ),
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_3months"),
                                    "backtest_timerange_3months"
                                ),
                            ],
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_6months"),
                                    "backtest_timerange_6months"
                                ),
                            ],
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "backtest_button_cancel"),
                                    "backtest_cancel"
                                ),
                            ],
                        ];

                        let escaped_exchange = escape_html(&exchange);
                        let exchange_selected = i18n::translate(locale, "backtest_exchange_selected", Some(&[("exchange", &escaped_exchange)]));
                        bot.edit_message_text(chat_id, message_id, exchange_selected)
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
                        let running_msg = i18n::translate(locale, "backtest_running", None);
                        bot.edit_message_text(chat_id, message_id, running_msg)
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
                                        
                                        // Build result message with detailed report table
                                        let mut result_msg = format!(
                                            "✅ <b>Backtest Complete!</b>\n\n\
                                            <b>Strategy:</b> {}\n\
                                            <b>Exchange:</b> {}\n\
                                            <b>Pair:</b> {}\n\
                                            <b>Time Range:</b> {}\n\
                                            <b>Timeframe:</b> {}\n\n",
                                            escape_html(&strategy_name),
                                            escape_html(&exchange),
                                            freqtrade_pair,
                                            timerange,
                                            timeframe
                                        );
                                        
                                        // Add detailed results table
                                        result_msg.push_str("<b>📊 Backtest Report:</b>\n");
                                        result_msg.push_str("━━━━━━━━━━━━━━━━━━━━\n");
                                        
                                        result_msg.push_str(&format!("📈 Total Trades: <b>{}</b>\n", result.trades));
                                        
                                        // Profit with color indication
                                        let profit_symbol = if result.profit_pct >= 0.0 { "💰" } else { "📉" };
                                        result_msg.push_str(&format!("{} Profit: <b>{:.2}%</b>\n", profit_symbol, result.profit_pct));
                                        
                                        // Additional metrics if available
                                        if let Some(win_rate) = result.win_rate {
                                            result_msg.push_str(&format!("✅ Win Rate: <b>{:.2}%</b>\n", win_rate));
                                        }
                                        if let Some(drawdown) = result.max_drawdown {
                                            result_msg.push_str(&format!("📉 Max Drawdown: <b>{:.2}%</b>\n", drawdown));
                                        }
                                        if let (Some(start), Some(final_bal)) = (result.starting_balance, result.final_balance) {
                                            result_msg.push_str(&format!("💵 Starting: <b>${:.2}</b> → Final: <b>${:.2}</b>\n", start, final_bal));
                                        }
                                        
                                        result_msg.push_str("━━━━━━━━━━━━━━━━━━━━\n\n");
                                        
                                        // Add timing info
                                        result_msg.push_str("<b>⏱️ Performance:</b>\n");
                                        if let Some(dl_time) = result.download_time_secs {
                                            result_msg.push_str(&format!("📥 Data Download: <b>{}s</b>\n", dl_time));
                                        } else {
                                            result_msg.push_str("📥 Data Download: <b>Skipped</b>\n");
                                        }
                                        result_msg.push_str(&format!("🔄 Backtest Execution: <b>{}s</b>\n", result.backtest_time_secs));
                                        
                                        let total_time = result.download_time_secs.unwrap_or(0) + result.backtest_time_secs;
                                        result_msg.push_str(&format!("⏱️ Total Time: <b>{}s</b>\n\n", total_time));
                                        
                                        result_msg.push_str(&format!("💾 Strategy file: <code>{}</code>", filepath.display()));
                                        
                                        // Extract tables for HTML report and Telegram messages
                                        let tables = if let Some(ref stdout) = result.stdout {
                                            extract_all_tables(stdout)
                                        } else {
                                            Vec::new()
                                        };
                                        
                                        // Generate HTML report if enabled - use config from AppState
                                        let config = state.config.as_ref();
                                        
                                        let html_report_url = if config.generate_html_reports {
                                            match generate_html_report(
                                                &config,
                                                &strategy_name,
                                                &exchange,
                                                &freqtrade_pair,
                                                &timeframe,
                                                &timerange,
                                                &result,
                                                &tables,
                                            ).await {
                                                Ok(Some(url)) => {
                                                    tracing::info!("HTML report generated: {}", url);
                                                    Some(url)
                                                }
                                                Ok(None) => None,
                                                Err(e) => {
                                                    tracing::error!("Failed to generate HTML report: {}", e);
                                                    None
                                                }
                                            }
                                        } else {
                                            None
                                        };
                                        
                                        tracing::info!("html_report_url: {:?}", html_report_url);
                                        // Add HTML report link to summary message if available
                                        if let Some(ref html_url) = html_report_url {
                                            // Telegram HTML link format: <a href="URL">text</a>
                                            // Đảm bảo URL không có spaces và format đúng
                                            let clean_url = html_url.trim();
                                            
                                            result_msg.push_str("\n\n✅🌐 <b>View Full Report:</b>\n");
                                            result_msg.push_str(&format!("<code>{}</code>\n", clean_url));
                                            
                                            // Warning nếu URL là localhost
                                            if clean_url.contains("localhost") || clean_url.contains("127.0.0.1") {
                                                tracing::warn!("URL contains localhost, Telegram users won't be able to access it. URL: {}", clean_url);
                                                result_msg.push_str("\n⚠️ <i>Note: This is a localhost URL. Use a public domain for remote access.</i>");
                                            }
                                            
                                            // Debug: log full message để kiểm tra
                                            tracing::info!("Added HTML link to message. Full message length: {}, URL: {}", result_msg.len(), clean_url);
                                            tracing::debug!("Link HTML format: <a href=\"{}\">Open HTML Report</a>", clean_url);
                                        } else {
                                            tracing::warn!("html_report_url is None, not adding link to message");
                                        }
                                        
                                        bot.edit_message_text(
                                            chat_id,
                                            message_id,
                                            result_msg
                                        )
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .await?;
                                        
                                        // Log full output to console for debugging
                                        tracing::info!("=== Backtest Full Output ===");
                                        if let Some(ref stdout) = result.stdout {
                                            tracing::info!("STDOUT:\n{}", stdout);
                                        }
                                        if let Some(ref stderr) = result.stderr {
                                            if !stderr.is_empty() {
                                                tracing::info!("STDERR:\n{}", stderr);
                                            }
                                        }
                                        tracing::info!("=== End Backtest Output ===");
                                        
                                        // Extract and send all tables from backtest output
                                        if !tables.is_empty() {
                                            // Send each table as a separate message for better readability
                                            for (idx, (title, table_content)) in tables.iter().enumerate() {
                                                let table_num = idx + 1;
                                                let total_tables = tables.len();
                                                
                                                // Format title nicely with emoji based on content
                                                let emoji = if title.contains("SUMMARY") {
                                                    "📊"
                                                } else if title.contains("REPORT") {
                                                    "📈"
                                                } else if title.contains("STATS") {
                                                    "📉"
                                                } else {
                                                    "📋"
                                                };
                                                
                                                let formatted_title = format!(
                                                    "{} <b>{}</b> ({}/{})\n",
                                                    emoji,
                                                    escape_html(title),
                                                    table_num,
                                                    total_tables
                                                );
                                                
                                                // Split table content into chunks if needed
                                                let chunks = split_into_chunks(table_content, 3200); // Smaller chunk for table formatting
                                                
                                                for (chunk_idx, chunk) in chunks.iter().enumerate() {
                                                    let mut table_msg = if chunk_idx == 0 {
                                                        formatted_title.clone()
                                                    } else {
                                                        format!("{} <b>{} (cont.)</b>\n", emoji, escape_html(title))
                                                    };
                                                    
                                                    table_msg.push_str("<pre>");
                                                    table_msg.push_str(&escape_html(chunk));
                                                    table_msg.push_str("</pre>");
                                                    
                                                    bot.send_message(chat_id, table_msg)
                                                        .parse_mode(teloxide::types::ParseMode::Html)
                                                        .await?;
                                                    
                                                    // Small delay between messages to avoid rate limiting
                                                    if chunk_idx < chunks.len() - 1 || idx < total_tables - 1 {
                                                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                                                    }
                                                }
                                            }
                                        } else if let Some(ref stdout) = result.stdout {
                                            // Fallback: if no tables found, send full output in chunks
                                            let chunks = split_into_chunks(stdout, 3500);
                                            let total_chunks = chunks.len();
                                            
                                            for (idx, chunk) in chunks.iter().enumerate() {
                                                let chunk_num = idx + 1;
                                                let mut chunk_msg = format!(
                                                    "📋 <b>Backtest Output ({}/{})</b>\n\n",
                                                    chunk_num,
                                                    total_chunks
                                                );
                                                chunk_msg.push_str("<pre>");
                                                chunk_msg.push_str(&escape_html(chunk));
                                                chunk_msg.push_str("</pre>");
                                                
                                                bot.send_message(chat_id, chunk_msg)
                                                    .parse_mode(teloxide::types::ParseMode::Html)
                                                    .await?;
                                                
                                                if idx < chunks.len() - 1 {
                                                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                                                }
                                            }
                                        }
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

