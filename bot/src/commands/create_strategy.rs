use std::sync::Arc;
use std::time::Instant;
use teloxide::{prelude::*, types::InlineKeyboardButton};
use sea_orm::{EntityTrait, ActiveValue};
use shared::entity::{users, strategies};
use tracing;

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
                            let strategy_name = format!("{}_{}_{}", algorithm, timeframe, pair);
                            let new_strategy = strategies::ActiveModel {
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

/// Handler to process strategy name
pub async fn receive_strategy_name(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    if let Some(text) = msg.text() {
        if text.to_lowercase() == "cancel" {
            dialogue.exit().await?;
            bot.send_message(msg.chat.id, "Strategy creation cancelled.").await?;
            return Ok(());
        }

        let algorithms_menu = format!(
            "✅ Strategy name: <b>{}</b>\n\n\
            <b>Step 2:</b> Choose algorithm indicator:\n\n\
            /rsi - Relative Strength Index\n\
            /bollinger - Bollinger Bands\n\
            /ema - Exponential Moving Average\n\
            /macd - MACD\n\
            /ma - Simple Moving Average\n",
            text
        );

        bot.send_message(msg.chat.id, algorithms_menu)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }
    Ok(())
}

/// Handler for RSI algorithm
pub async fn select_rsi(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let instruction = format!(
        "📊 <b>RSI Strategy Configuration</b>\n\n\
        RSI ranges from 0-100:\n\
        • <b>Oversold:</b> RSI &lt; 30 (buy signal)\n\
        • <b>Overbought:</b> RSI &gt; 70 (sell signal)\n\n\
        <b>Step 3:</b> Enter buy condition:\n\
        Example: <code>RSI &lt; 30</code>\n\
        (Enter the exact condition)",
    );

    bot.send_message(msg.chat.id, instruction)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}

/// Handler for Bollinger Bands
pub async fn select_bollinger(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let instruction = format!(
        "📊 <b>Bollinger Bands Strategy</b>\n\n\
        • <b>Lower Band:</b> Buy signal (price touches lower band)\n\
        • <b>Upper Band:</b> Sell signal (price touches upper band)\n\n\
        <b>Step 3:</b> Enter buy condition:\n\
        Example: <code>Price &lt; LowerBand</code>",
    );

    bot.send_message(msg.chat.id, instruction)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}

/// Handler for EMA
pub async fn select_ema(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let instruction = format!(
        "📊 <b>EMA Crossover Strategy</b>\n\n\
        • <b>Buy:</b> Fast EMA crosses above Slow EMA\n\
        • <b>Sell:</b> Fast EMA crosses below Slow EMA\n\n\
        <b>Step 3:</b> Enter buy condition:\n\
        Example: <code>EMA(12) &gt; EMA(26)</code>",
    );

    bot.send_message(msg.chat.id, instruction)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

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
                    let strategy_name = format!("{}_{}_{}", algorithm, timeframe, pair);
                    let new_strategy = strategies::ActiveModel {
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

/// Handler to receive sell condition and ask for timeframe
pub async fn receive_sell_condition(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    _state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    if let Some(text) = msg.text() {
        if text.to_lowercase() == "cancel" {
            dialogue.exit().await?;
            return Ok(());
        }

        let timeframe_prompt = format!(
            "✅ Sell condition: <b>{}</b>\n\n\
            <b>Step 5:</b> Enter timeframe:\n\
            Options: 1m, 5m, 15m, 30m, 1h, 4h, 1d, 1w\n\n\
            Example: <code>1h</code>",
            text
        );

        bot.send_message(msg.chat.id, timeframe_prompt)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }
    Ok(())
}

/// Handler to receive timeframe and ask for trading pair
pub async fn receive_timeframe(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    if let Some(text) = msg.text() {
        let pair_prompt = format!(
            "✅ Timeframe: <b>{}</b>\n\n\
            <b>Step 6:</b> Enter trading pair:\n\
            Example: <code>BTCUSDT</code> or <code>ETHUSDT</code>",
            text
        );

        bot.send_message(msg.chat.id, pair_prompt)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }
    Ok(())
}

/// Handler to receive pair and show confirmation
pub async fn receive_pair(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    if let Some(text) = msg.text() {
        if text.to_lowercase() == "cancel" {
            dialogue.exit().await?;
            bot.send_message(msg.chat.id, "❌ Strategy creation cancelled.").await?;
            return Ok(());
        }

        // Validate trading pair format
        let pair = text.trim().to_uppercase();
        
        // Save strategy to database
        let strategy_name = format!("CustomStrategy_{}", chrono::Utc::now().timestamp());
        
        let new_strategy = strategies::ActiveModel {
            name: ActiveValue::Set(Some(strategy_name.clone())),
            description: ActiveValue::Set(Some(format!("Trading Pair: {}", pair))),
            repo_ref: ActiveValue::Set(Some(format!("custom_{}_{}", pair, chrono::Utc::now().timestamp()))),
            created_at: ActiveValue::Set(Some(chrono::Utc::now())),
            ..Default::default()
        };

        let confirm_buttons = vec![
            vec![
                InlineKeyboardButton::callback("✅ Confirm & Save", format!("confirm_strategy_{}", strategy_name.clone())),
                InlineKeyboardButton::callback("❌ Cancel", "cancel_strategy"),
            ],
        ];

        match strategies::Entity::insert(new_strategy).exec(state.db.as_ref()).await {
            Ok(_) => {
                let summary = format!(
                    "✅ <b>Strategy Created!</b>\n\n\
                    <b>Name:</b> {}\n\
                    <b>Pair:</b> {}\n\n\
                    Your strategy has been saved successfully!\n\n\
                    Use <code>/backtest {} {} 1h 7days</code> to test it.",
                    strategy_name,
                    pair,
                    strategy_name,
                    pair
                );

                bot.send_message(msg.chat.id, summary)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new(confirm_buttons))
                    .await?;
                
                dialogue.exit().await?;
            }
            Err(e) => {
                bot.send_message(
                    msg.chat.id,
                    format!("❌ Failed to save strategy: {}", e)
                ).await?;
            }
        }
    }
    Ok(())
}

