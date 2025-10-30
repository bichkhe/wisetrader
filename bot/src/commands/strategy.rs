use std::sync::Arc;
use std::time::Instant;
use teloxide::{prelude::*, types::InlineKeyboardButton};
use sea_orm::{EntityTrait, ActiveValue};
use shared::entity::{users, strategies};
use tracing;
use sea_orm::{QueryFilter, QueryOrder};
use crate::state::{AppState, BotState, CreateStrategyState, MyDialogue};
use crate::i18n;

// Helper function to HTML escape (must escape & first!)
fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
}

// Helper function to get user locale from callback
async fn get_locale_from_callback(state: &Arc<AppState>, user_id: i64) -> String {
    use shared::entity::users;
    if let Ok(Some(user)) = users::Entity::find_by_id(user_id).one(state.db.as_ref()).await {
        if let Some(ref lang) = user.language {
            return i18n::get_user_language(Some(lang)).to_string();
        }
    }
    "en".to_string()
}

/// Handler for inline keyboard callbacks in strategy creation
pub async fn handle_strategy_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    tracing::info!("handle_strategy_callback called with data: {:?}", q.data);
    
    let user_id = q.from.id.0 as i64;
    let locale = get_locale_from_callback(&state, user_id).await;
    
    if let Some(data) = q.data {
        if let Some(msg) = q.message {
            let chat_id = msg.chat().id;
            let message_id = msg.id();
            
            tracing::info!("Processing callback: {}", data);
            
            match data.as_str() {
                "algorithm_rsi" => {
                    bot.answer_callback_query(q.id).await?;
                    let algorithm_msg = i18n::translate(&locale, "strategy_algorithm_selected", Some(&[("algorithm", "RSI")]));
                    let info_msg = i18n::translate(&locale, "strategy_algorithm_rsi_info", None);
                    let step2_msg = i18n::translate(&locale, "strategy_step2_enter_buy", Some(&[("example", "RSI < 30")]));
                    let instruction = format!("{}\n\n{}\n\n{}", algorithm_msg, info_msg, step2_msg);
                    
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
                    let algorithm_msg = i18n::translate(&locale, "strategy_algorithm_selected", Some(&[("algorithm", "Bollinger Bands")]));
                    let info_msg = i18n::translate(&locale, "strategy_algorithm_bb_info", None);
                    let step2_msg = i18n::translate(&locale, "strategy_step2_enter_buy", Some(&[("example", "Price < LowerBand")]));
                    let instruction = format!("{}\n\n{}\n\n{}", algorithm_msg, info_msg, step2_msg);
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "Bollinger Bands".to_string(),
                    })).await?;
                }
                "algorithm_ema" => {
                    bot.answer_callback_query(q.id).await?;
                    let algorithm_msg = i18n::translate(&locale, "strategy_algorithm_selected", Some(&[("algorithm", "EMA")]));
                    let info_msg = i18n::translate(&locale, "strategy_algorithm_ema_info", None);
                    let step2_msg = i18n::translate(&locale, "strategy_step2_enter_buy", Some(&[("example", "EMA(12) > EMA(26)")]));
                    let instruction = format!("{}\n\n{}\n\n{}", algorithm_msg, info_msg, step2_msg);
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "EMA".to_string(),
                    })).await?;
                }
                "algorithm_macd" => {
                    bot.answer_callback_query(q.id).await?;
                    let algorithm_msg = i18n::translate(&locale, "strategy_algorithm_selected", Some(&[("algorithm", "MACD")]));
                    let info_msg = i18n::translate(&locale, "strategy_algorithm_macd_info", None);
                    let step2_msg = i18n::translate(&locale, "strategy_step2_enter_buy", Some(&[("example", "MACD > Signal")]));
                    let instruction = format!("{}\n\n{}\n\n{}", algorithm_msg, info_msg, step2_msg);
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "MACD".to_string(),
                    })).await?;
                }
                "algorithm_ma" => {
                    bot.answer_callback_query(q.id).await?;
                    let algorithm_msg = i18n::translate(&locale, "strategy_algorithm_selected", Some(&[("algorithm", "MA")]));
                    let info_msg = i18n::translate(&locale, "strategy_algorithm_ma_info", None);
                    let step2_msg = i18n::translate(&locale, "strategy_step2_enter_buy", Some(&[("example", "MA(9) > MA(21)")]));
                    let instruction = format!("{}\n\n{}\n\n{}", algorithm_msg, info_msg, step2_msg);
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "MA".to_string(),
                    })).await?;
                }
                "cancel_strategy" => {
                    bot.answer_callback_query(q.id).await?;
                    let cancel_msg = i18n::translate(&locale, "strategy_creation_cancelled", None);
                    bot.edit_message_text(chat_id, message_id, cancel_msg).await?;
                    dialogue.exit().await?;
                    return Ok(());
                }
                _ if data.starts_with("timeframe_") => {
                    bot.answer_callback_query(q.id).await?;
                    let timeframe = data.replace("timeframe_", "");
                    // Get current state to extract data
                    if let Ok(Some(BotState::CreateStrategy(CreateStrategyState::WaitingForTimeframe { algorithm, buy_condition, sell_condition }))) = dialogue.get().await {
                        let mut pair_buttons = vec![
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
                                InlineKeyboardButton::callback(i18n::get_button_text(&locale, "pair_manual"), "pair_manual"),
                            ],
                        ];
                        pair_buttons.push(vec![
                            InlineKeyboardButton::callback(i18n::get_button_text(&locale, "strategy_cancel_button"), "cancel_strategy"),
                        ]);
                        
                        // Don't escape here - translate() will handle HTML escaping
                        let step3_complete = i18n::translate(&locale, "strategy_step3_complete", Some(&[
                            ("algorithm", &algorithm),
                            ("buy_condition", &buy_condition),
                            ("sell_condition", &sell_condition),
                            ("timeframe", &timeframe),
                        ]));
                        let step5_msg = i18n::translate(&locale, "strategy_step5_choose_pair", None);
                        let instruction = format!("{}\n\n{}", step3_complete, step5_msg);
                        
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
                        let manual_msg = i18n::translate(&locale, "strategy_enter_pair_manual", None);
                        bot.send_message(chat_id, manual_msg).await?;
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
                                    // Use i18n translation with escaped values
                                    // translate() will handle HTML escaping automatically
                                    // locale is already available from callback handler scope
                                    let success_msg = i18n::translate(&locale, "strategy_created_success", Some(&[
                                        ("strategy_name", &strategy_name),
                                        ("algorithm", &algorithm),
                                        ("buy_condition", &buy_condition),
                                        ("sell_condition", &sell_condition),
                                        ("timeframe", &timeframe),
                                        ("pair", &pair),
                                    ]));
                                    
                                    bot.send_message(chat_id, success_msg)
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
                    let already_saved = i18n::translate(&locale, "strategy_already_saved", None);
                    bot.send_message(chat_id, already_saved).await?;
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

    // Check if user exists and get language
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;

    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");

    if user.is_none() {
        let error_msg = i18n::translate(locale, "error_user_not_found", None);
        bot.send_message(msg.chat.id, error_msg).await?;
        return Ok(());
    }

    let algorithm_buttons = vec![
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "algorithm_rsi"),
                "algorithm_rsi"
            ),
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "algorithm_bollinger"),
                "algorithm_bollinger"
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "algorithm_ema"),
                "algorithm_ema"
            ),
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "algorithm_macd"),
                "algorithm_macd"
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "algorithm_ma"),
                "algorithm_ma"
            ),
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "strategy_cancel_button"),
                "cancel_strategy"
            ),
        ],
    ];

    let welcome_msg = i18n::translate(locale, "strategy_welcome", None);

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
    state: Arc<AppState>,
) ->  Result<(), anyhow::Error>{
    // Get user locale
    let telegram_id = msg.from.as_ref().unwrap().id.0 as i64;
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    if let Ok(dialogue_state) = dialogue.get().await {
        tracing::info!("handle_strategy_input_callback called. Dialogue state: {:?}", dialogue_state);
        match dialogue_state.unwrap() {
            BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition { algorithm }) => {
                if let Some(text) = msg.text() {
                    let buy_condition = text.trim().to_string();
                    // Don't escape here - translate() will handle HTML escaping
                    let step1_complete = i18n::translate(locale, "strategy_step1_complete", Some(&[
                        ("algorithm", &algorithm),
                        ("buy_condition", &buy_condition),
                    ]));
                    let step2_msg = i18n::translate(locale, "strategy_step3_enter_sell", Some(&[("example", "RSI >= 70")]));
                    let instruction = step1_complete + &step2_msg;
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
                    let mut timeframe_buttons = vec![
                        vec![
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "timeframe_1m"), "timeframe_1m"),
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "timeframe_5m"), "timeframe_5m"),
                        ],
                        vec![
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "timeframe_15m"), "timeframe_15m"),
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "timeframe_30m"), "timeframe_30m"),
                        ],
                        vec![
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "timeframe_1h"), "timeframe_1h"),
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "timeframe_4h"), "timeframe_4h"),
                        ],
                        vec![
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "timeframe_1d"), "timeframe_1d"),
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "timeframe_1w"), "timeframe_1w"),
                        ],
                    ];
                    timeframe_buttons.push(vec![
                        InlineKeyboardButton::callback(i18n::get_button_text(locale, "strategy_cancel_button"), "cancel_strategy"),
                    ]);
                    
                    // Don't escape here - translate() will handle HTML escaping
                    let step2_complete = i18n::translate(locale, "strategy_step2_complete", Some(&[
                        ("algorithm", &algorithm),
                        ("buy_condition", &buy_condition),
                        ("sell_condition", &sell_condition),
                    ]));
                    let step4_msg = i18n::translate(locale, "strategy_step4_choose_timeframe", None);
                    let instruction = step2_complete + &step4_msg;
                    
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

                    match strategies::Entity::insert(new_strategy).exec(state.db.as_ref()).await {
                        Ok(_) => {
                            let success_msg = i18n::translate(locale, "strategy_saved_success", None);
                            bot.send_message(msg.chat.id, success_msg)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                            dialogue.exit().await?;
                        }
                        Err(e) => {
                            let error_msg = i18n::translate(locale, "strategy_saved_error", None);
                            bot.send_message(msg.chat.id, format!("{}: {}", error_msg, e)).await?;
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

    // Get user language
    let user = users::Entity::find_by_id(telegram_id.parse::<i64>().unwrap_or(0))
        .one(db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    if user_strategies.is_empty() {
        let empty_msg = i18n::translate(locale, "strategy_my_strategies_empty", None);
        bot.send_message(msg.chat.id, empty_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }

    
    let title = i18n::translate(locale, "strategy_my_strategies_title", Some(&[("count", &user_strategies.len().to_string())]));
    let mut msg_text = title + "\n\n";
    
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

    msg_text.push_str(&i18n::translate(locale, "strategy_my_strategies_tip", None));

    bot.send_message(msg.chat.id, msg_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}