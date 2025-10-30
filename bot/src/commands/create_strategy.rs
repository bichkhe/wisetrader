use std::sync::Arc;
use std::time::Instant;
use teloxide::{prelude::*, types::InlineKeyboardButton};
use sea_orm::{EntityTrait, ActiveValue};
use shared::entity::{users, strategies};
use tracing;
use sea_orm::{QueryFilter, QueryOrder};
use crate::state::{AppState, BotState, CreateStrategyState, MyDialogue};

/// Handler for inline keyboard callbacks in strategy creation
pub async fn handle_strategy_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    tracing::info!("handle_strategy_callback called with data: {:?}", q.data);
    
    if let Some(data) = q.data {
        if let Some(msg) = q.message {
            let chat_id = msg.chat().id;
            let message_id = msg.id();
            
            tracing::info!("Processing callback: {}", data);
            
            match data.as_str() {
                "algorithm_rsi" => {
                    bot.answer_callback_query(q.id).await?;
                    let instruction = format!(
                        "📊 <b>RSI Strategy Selected</b>\n\n\
                        RSI ranges from 0-100:\n\
                        • <b>Oversold:</b> RSI &lt; 30 (buy signal)\n\
                        • <b>Overbought:</b> RSI &gt; 70 (sell signal)\n\n\
                        <b>Step 2:</b> Enter buy condition:\n\
                        Example: <code>RSI &lt; 30</code>"
                    );
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    // Update dialogue state to WaitingForBuyCondition with algorithm
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "RSI".to_string(),
                    })).await?;
                }
                "algorithm_bollinger" => {
                    bot.answer_callback_query(q.id).await?;
                    let instruction = format!(
                        "📊 <b>Bollinger Bands Strategy Selected</b>\n\n\
                        • <b>Lower Band:</b> Buy signal (price touches lower band)\n\
                        • <b>Upper Band:</b> Sell signal (price touches upper band)\n\n\
                        <b>Step 2:</b> Enter buy condition:\n\
                        Example: <code>Price &lt; LowerBand</code>"
                    );
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "Bollinger Bands".to_string(),
                    })).await?;
                }
                "algorithm_ema" => {
                    bot.answer_callback_query(q.id).await?;
                    let instruction = format!(
                        "📊 <b>EMA Crossover Strategy Selected</b>\n\n\
                        • <b>Buy:</b> Fast EMA crosses above Slow EMA\n\
                        • <b>Sell:</b> Fast EMA crosses below Slow EMA\n\n\
                        <b>Step 2:</b> Enter buy condition:\n\
                        Example: <code>EMA(12) &gt; EMA(26)</code>"
                    );
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "EMA".to_string(),
                    })).await?;
                }
                "algorithm_macd" => {
                    bot.answer_callback_query(q.id).await?;
                    let instruction = format!(
                        "📊 <b>MACD Strategy Selected</b>\n\n\
                        • <b>Buy:</b> MACD line crosses above signal line\n\
                        • <b>Sell:</b> MACD line crosses below signal line\n\n\
                        <b>Step 2:</b> Enter buy condition:\n\
                        Example: <code>MACD &gt; Signal</code>"
                    );
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "MACD".to_string(),
                    })).await?;
                }
                "algorithm_ma" => {
                    bot.answer_callback_query(q.id).await?;
                    let instruction = format!(
                        "📊 <b>MA Crossover Strategy Selected</b>\n\n\
                        • <b>Buy:</b> Fast MA crosses above Slow MA\n\
                        • <b>Sell:</b> Fast MA crosses below Slow MA\n\n\
                        <b>Step 2:</b> Enter buy condition:\n\
                        Example: <code>MA(9) &gt; MA(21)</code>"
                    );
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "MA".to_string(),
                    })).await?;
                }
                "cancel_strategy" => {
                    bot.answer_callback_query(q.id).await?;
                    bot.edit_message_text(
                        chat_id,
                        message_id,
                        "❌ Strategy creation cancelled."
                    ).await?;
                }
                _ if data.starts_with("timeframe_") => {
                    bot.answer_callback_query(q.id).await?;
                    let timeframe = data.replace("timeframe_", "");
                    // Get current state to extract data
                    if let Ok(Some(BotState::CreateStrategy(CreateStrategyState::WaitingForTimeframe { algorithm, buy_condition, sell_condition }))) = dialogue.get().await {
                        let pair_buttons = vec![
                            vec![
                                InlineKeyboardButton::callback("BTC/USDT", "pair_BTCUSDT"),
                                InlineKeyboardButton::callback("ETH/USDT", "pair_ETHUSDT"),
                            ],
                            vec![
                                InlineKeyboardButton::callback("BNB/USDT", "pair_BNBUSDT"),
                                InlineKeyboardButton::callback("ADA/USDT", "pair_ADAUSDT"),
                            ],
                            vec![
                                InlineKeyboardButton::callback("SOL/USDT", "pair_SOLUSDT"),
                                InlineKeyboardButton::callback("DOT/USDT", "pair_DOTUSDT"),
                            ],
                            vec![
                                InlineKeyboardButton::callback("Manual", "pair_manual"),
                            ],
                        ];
                        
                        let instruction = format!(
                            "✅ <b>Step 3 Complete!</b>\n\n\
                            📋 <b>Summary:</b>\n\
                            • <b>Algorithm:</b> {}\n\
                            • <b>Buy Condition:</b> {}\n\
                            • <b>Sell Condition:</b> {}\n\
                            • <b>Timeframe:</b> {}\n\n\
                            <b>Step 4:</b> Choose trading pair:",
                            algorithm,
                            buy_condition.replace("<", "&lt;").replace(">", "&gt;"),
                            sell_condition.replace("<", "&lt;").replace(">", "&gt;"),
                            timeframe
                        );
                        
                        bot.send_message(chat_id, instruction)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(pair_buttons))
                            .await?;
                        dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForPair {
                            algorithm,
                            buy_condition,
                            sell_condition,
                            timeframe,
                        })).await?;
                    }
                }
                _ if data.starts_with("pair_") => {
                    bot.answer_callback_query(q.id).await?;
                    if data == "pair_manual" {
                        bot.send_message(chat_id, "Please enter the trading pair manually (e.g., BTCUSDT):").await?;
                    } else {
                        let pair = data.replace("pair_", "");
                        // Get current state to extract all data and save strategy
                        if let Ok(Some(BotState::CreateStrategy(CreateStrategyState::WaitingForPair { algorithm, buy_condition, sell_condition, timeframe }))) = dialogue.get().await {
                            // Get telegram_id from callback query
                            let telegram_id = q.from.id.0.to_string();
                            let strategy_name = format!("{}_{}_{}", algorithm, timeframe, pair);
                            let new_strategy = strategies::ActiveModel {
                                telegram_id: ActiveValue::Set(telegram_id.clone()),
                                name: ActiveValue::Set(Some(strategy_name.clone())),
                                description: ActiveValue::Set(Some(format!(
                                    "Algorithm: {}\nBuy: {}\nSell: {}\nTimeframe: {}\nPair: {}",
                                    algorithm, buy_condition, sell_condition, timeframe, pair
                                ))),
                                repo_ref: ActiveValue::Set(Some(format!("custom_{}_{}", pair, chrono::Utc::now().timestamp()))),
                                created_at: ActiveValue::Set(Some(chrono::Utc::now())),
                                ..Default::default()
                            };

                            match strategies::Entity::insert(new_strategy).exec(state.db.as_ref()).await {
                                Ok(_) => {
                                    bot.send_message(
                                        chat_id,
                                        format!("✅ <b>Strategy Created Successfully!</b>\n\n\
                                        📋 <b>Complete Summary:</b>\n\n\
                                        🎯 <b>Strategy Name:</b> {}\n\
                                        📊 <b>Algorithm:</b> {}\n\
                                        📈 <b>Buy Condition:</b> {}\n\
                                        📉 <b>Sell Condition:</b> {}\n\
                                        ⏰ <b>Timeframe:</b> {}\n\
                                        💱 <b>Trading Pair:</b> {}\n\n\
                                        Your strategy has been saved and is ready to use!",
                                        strategy_name, algorithm, 
                                        buy_condition.replace("<", "&lt;").replace(">", "&gt;"),
                                        sell_condition.replace("<", "&lt;").replace(">", "&gt;"),
                                        timeframe, pair)
                                    )
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .await?;
                                    dialogue.exit().await?;
                                }
                                Err(e) => {
                                    bot.send_message(chat_id, format!("❌ Failed to save: {}", e)).await?;
                                }
                            }
                        }
                    }
                }
                _ if data.starts_with("confirm_strategy_") => {
                    bot.answer_callback_query(q.id).await?;
                    bot.send_message(
                        chat_id,
                        "✅ Strategy already saved! Use /strategies to view all your strategies."
                    ).await?;
                }
                _ => {
                    bot.answer_callback_query(q.id).await?;
                }
            }
        }
    }
    Ok(())
}



/// Handler to start strategy creation wizard
pub async fn handle_create_strategy(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    dialogue.update(BotState::CreateStrategy(CreateStrategyState::Start)).await?;
    let start_time = Instant::now();
    
    let from = msg.from.unwrap();
    let telegram_id = from.id.0 as i64;
    let username = from.username.unwrap_or("Unknown".to_string());

    tracing::info!(
        "Handling /create_strategy command from user: {} (id: {})",
        username,
        telegram_id
    );

    // Check if user exists
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;

    if user.is_none() {
        bot.send_message(
            msg.chat.id,
            "❌ User not found. Please run /start first."
        ).await?;
        return Ok(());
    }

    let algorithm_buttons = vec![
        vec![
            InlineKeyboardButton::callback("📊 RSI", "algorithm_rsi"),
            InlineKeyboardButton::callback("📈 Bollinger", "algorithm_bollinger"),
        ],
        vec![
            InlineKeyboardButton::callback("📉 EMA", "algorithm_ema"),
            InlineKeyboardButton::callback("📊 MACD", "algorithm_macd"),
        ],
        vec![
            InlineKeyboardButton::callback("📊 MA", "algorithm_ma"),
            InlineKeyboardButton::callback("❌ Cancel", "cancel_strategy"),
        ],
    ];

    let welcome_msg = format!(
        "🤖 <b>Create Custom Trading Strategy</b>\n\n\
        <b>Step 1:</b> Choose an algorithm indicator:\n\n\
        • <b>RSI</b> - Relative Strength Index (0-100)\n\
        • <b>Bollinger Bands</b> - Price volatility bands\n\
        • <b>EMA</b> - Exponential Moving Average\n\
        • <b>MACD</b> - Moving Average Convergence Divergence\n\
        • <b>MA</b> - Simple Moving Average\n\n\
        Click a button below to start:",
    );

    bot.send_message(msg.chat.id, welcome_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(algorithm_buttons))
        .await?;

    // Update dialogue state to CreateStrategy
    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForAlgorithm)).await?;

    Ok(())
}

pub async fn handle_strategy_input_callback(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    _state: Arc<AppState>,
) ->  Result<(), anyhow::Error>{
    if let Ok(state) = dialogue.get().await {
        tracing::info!("handle_strategy_input_callback called. Dialogue state: {:?}", state);
        match state.unwrap() {
            BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition { algorithm }) => {
                if let Some(text) = msg.text() {
                    let buy_condition = text.trim().to_string();
                    let instruction = format!(
                        "✅ <b>Step 1 Complete!</b>\n\n\
                        <b>Algorithm:</b> {}\n\
                        <b>Buy Condition:</b> {}\n\n\
                        <b>Step 2:</b> Enter sell condition:\n\
                        Example: <code>RSI &gt;= 70</code>",
                        algorithm,
                        buy_condition.replace("<", "&lt;").replace(">", "&gt;")
                    );
                    bot.send_message(msg.chat.id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForSellCondition {
                        algorithm,
                        buy_condition,
                    })).await?;
                }
            }
            BotState::CreateStrategy(CreateStrategyState::WaitingForSellCondition { algorithm, buy_condition }) => {
                if let Some(text) = msg.text() {
                    let sell_condition = text.trim().to_string();
                    let timeframe_buttons = vec![
                        vec![
                            InlineKeyboardButton::callback("1m", "timeframe_1m"),
                            InlineKeyboardButton::callback("5m", "timeframe_5m"),
                        ],
                        vec![
                            InlineKeyboardButton::callback("15m", "timeframe_15m"),
                            InlineKeyboardButton::callback("30m", "timeframe_30m"),
                        ],
                        vec![
                            InlineKeyboardButton::callback("1h", "timeframe_1h"),
                            InlineKeyboardButton::callback("4h", "timeframe_4h"),
                        ],
                        vec![
                            InlineKeyboardButton::callback("1d", "timeframe_1d"),
                            InlineKeyboardButton::callback("1w", "timeframe_1w"),
                        ],
                    ];
                    
                    let instruction = format!(
                        "✅ <b>Step 2 Complete!</b>\n\n\
                        📋 <b>Summary:</b>\n\
                        • <b>Algorithm:</b> {}\n\
                        • <b>Buy Condition:</b> {}\n\
                        • <b>Sell Condition:</b> {}\n\n\
                        <b>Step 3:</b> Choose timeframe:\n\
                        Click a button below:",
                        algorithm,
                        buy_condition.replace("<", "&lt;").replace(">", "&gt;"),
                        sell_condition.replace("<", "&lt;").replace(">", "&gt;")
                    );
                    
                    bot.send_message(msg.chat.id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(timeframe_buttons))
                        .await?;
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForTimeframe {
                        algorithm,
                        buy_condition,
                        sell_condition,
                    })).await?;
                }
            }
            BotState::CreateStrategy(CreateStrategyState::WaitingForPair { algorithm, buy_condition, sell_condition, timeframe }) => {
                if let Some(text) = msg.text() {
                    let pair = text.trim().to_uppercase();
                    // Save strategy to database
                    let telegram_id = msg.from.as_ref().unwrap().id.0.to_string();
                    let strategy_name = format!("{}_{}_{}", algorithm, timeframe, pair);
                    let new_strategy = strategies::ActiveModel {
                        telegram_id: ActiveValue::Set(telegram_id.clone()),
                        name: ActiveValue::Set(Some(strategy_name.clone())),
                        description: ActiveValue::Set(Some(format!(
                            "Algorithm: {}\nBuy: {}\nSell: {}\nTimeframe: {}\nPair: {}",
                            algorithm, buy_condition, sell_condition, timeframe, pair
                        ))),
                        repo_ref: ActiveValue::Set(Some(format!("custom_{}_{}", pair, chrono::Utc::now().timestamp()))),
                        created_at: ActiveValue::Set(Some(chrono::Utc::now())),
                        ..Default::default()
                    };

                    match strategies::Entity::insert(new_strategy).exec(_state.db.as_ref()).await {
                        Ok(_) => {
                            bot.send_message(
                                msg.chat.id,
                                format!("✅ <b>Strategy Created Successfully!</b>\n\n\
                                📋 <b>Complete Summary:</b>\n\n\
                                🎯 <b>Strategy Name:</b> {}\n\
                                📊 <b>Algorithm:</b> {}\n\
                                📈 <b>Buy Condition:</b> {}\n\
                                📉 <b>Sell Condition:</b> {}\n\
                                ⏰ <b>Timeframe:</b> {}\n\
                                💱 <b>Trading Pair:</b> {}\n\n\
                                Your strategy has been saved and is ready to use!",
                                strategy_name, algorithm, 
                                buy_condition.replace("<", "&lt;").replace(">", "&gt;"),
                                sell_condition.replace("<", "&lt;").replace(">", "&gt;"),
                                timeframe, pair)
                            )
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                            dialogue.exit().await?;
                        }
                        Err(e) => {
                            bot.send_message(msg.chat.id, format!("❌ Failed to save: {}", e)).await?;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}


/// Handler to list all strategies created by the current user
pub async fn handle_my_strategies(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
) -> anyhow::Result<()> {
    let telegram_id = msg.from.as_ref().unwrap().id.0.to_string();
    let db = state.db.clone();

    // Query strategies filtered by telegram_id column
    use sea_orm::ColumnTrait;
    use shared::entity::strategies;
    
    let user_strategies = strategies::Entity::find()
        .filter(strategies::Column::TelegramId.eq(telegram_id.clone()))
        .order_by_desc(strategies::Column::CreatedAt)
        .all(db.as_ref())
        .await?;

    if user_strategies.is_empty() {
        bot.send_message(
            msg.chat.id,
            "📋 <b>My Strategies</b>\n\nYou haven't created any strategies yet.\n\nUse <code>/create_strategy</code> to create your first strategy!"
        )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }

    // Helper function to HTML escape (must escape & first!)
    fn escape_html(text: &str) -> String {
        text.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&#x27;")
    }
    
    let mut msg_text = format!("📋 <b>My Strategies</b> ({})\n\n", user_strategies.len());
    
    let unnamed_str = "Unnamed".to_string();
    let no_desc_str = "No description".to_string();
    
    for (idx, strategy) in user_strategies.iter().enumerate() {
        let name = strategy.name.as_ref().unwrap_or(&unnamed_str);
        let desc_str = strategy.description.as_ref().unwrap_or(&no_desc_str);
        
        // Parse description to extract fields
        let mut algorithm = "N/A".to_string();
        let mut buy_condition = "N/A".to_string();
        let mut sell_condition = "N/A".to_string();
        let mut timeframe = "N/A".to_string();
        let mut pair = "N/A".to_string();
        
        for line in desc_str.lines() {
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
        
        // HTML escape all user data
        let escaped_name = escape_html(name);
        let escaped_algorithm = escape_html(&algorithm);
        let escaped_buy = escape_html(&buy_condition);
        let escaped_sell = escape_html(&sell_condition);
        let escaped_timeframe = escape_html(&timeframe);
        let escaped_pair = escape_html(&pair);
        
        let created = strategy.created_at
            .as_ref()
            .map(|dt| escape_html(&dt.format("%Y-%m-%d %H:%M").to_string()))
            .unwrap_or_else(|| "Unknown".to_string());

        // Build message with beautiful icons
        msg_text.push_str(if idx == 0 { "⭐ " } else { "📌 " });
        msg_text.push_str("<b>");
        msg_text.push_str(&(idx + 1).to_string());
        msg_text.push_str(". ");
        msg_text.push_str(&escaped_name);
        msg_text.push_str("</b>\n\n");
        
        msg_text.push_str("📊 <b>Algorithm:</b> ");
        msg_text.push_str(&escaped_algorithm);
        msg_text.push_str("\n");
        
        msg_text.push_str("📈 <b>Buy:</b> <code>");
        msg_text.push_str(&escaped_buy);
        msg_text.push_str("</code>\n");
        
        msg_text.push_str("📉 <b>Sell:</b> <code>");
        msg_text.push_str(&escaped_sell);
        msg_text.push_str("</code>\n");
        
        msg_text.push_str("⏰ <b>Timeframe:</b> ");
        msg_text.push_str(&escaped_timeframe);
        msg_text.push_str("\n");
        
        msg_text.push_str("💱 <b>Pair:</b> ");
        msg_text.push_str(&escaped_pair);
        msg_text.push_str("\n");
        
        msg_text.push_str("📅 <b>Created:</b> ");
        msg_text.push_str(&created);
        msg_text.push_str("\n");
        
        msg_text.push_str("🆔 <b>ID:</b> <code>");
        msg_text.push_str(&strategy.id.to_string());
        msg_text.push_str("</code>\n\n");
    }

    msg_text.push_str("\n💡 <b>Tip:</b> Use <code>/backtest &lt;strategy_name&gt;</code> to test your strategies!");

    bot.send_message(msg.chat.id, msg_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}