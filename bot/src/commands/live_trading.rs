//! Live Trading Command - allows users to trade with real exchanges using OAuth tokens

use std::sync::Arc;
use anyhow::Result;
use teloxide::dispatching::dialogue;
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use sea_orm::{EntityTrait, ActiveValue, ColumnTrait, QueryFilter, QueryOrder, Order};
use crate::state::{AppState, MyDialogue, BotState, LiveTradingState};
use crate::i18n;
use shared::entity::{users, exchange_tokens, live_trading_sessions};
use chrono::Utc;

/// Handler for /livetrading command
pub async fn handle_live_trading(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let from = msg.from.unwrap();
    let telegram_id = from.id.0 as i64;
    
    // Get user locale
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    // Check if user already has an active live trading session
    let active_session = live_trading_sessions::Entity::find()
        .filter(live_trading_sessions::Column::UserId.eq(telegram_id))
        .filter(live_trading_sessions::Column::Status.eq("active"))
        .one(state.db.as_ref())
        .await?;
    
    if active_session.is_some() {
        let msg_text = i18n::translate(locale, "trading_already_active", None);
        bot.send_message(msg.chat.id, msg_text).await?;
        return Ok(());
    }
    
    // Get user's strategies
    let strategies_list = state.strategy_service.get_user_strategies(telegram_id).await?;
    
    if strategies_list.is_empty() {
        let msg_text = i18n::translate(locale, "trading_no_strategies", None);
        bot.send_message(msg.chat.id, msg_text).await?;
        return Ok(());
    }
    
    // Get all user's tokens (active and inactive)
    let exchange_tokens_list = exchange_tokens::Entity::find()
        .filter(exchange_tokens::Column::UserId.eq(telegram_id))
        .all(state.db.as_ref())
        .await?;
    
    // Get active tokens
    let active_tokens: Vec<_> = exchange_tokens_list.iter()
        .filter(|t| t.is_active == 1)
        .collect();
    
    // Check which exchanges are configured
    let has_binance = exchange_tokens_list.iter().any(|t| t.exchange == "binance");
    let has_okx = exchange_tokens_list.iter().any(|t| t.exchange == "okx");
    
    let mut setup_buttons = Vec::new();
    
    // Binance button - on its own row
    let binance_text = if has_binance {
        format!("{} {}", 
            i18n::get_button_text(locale, "live_trading_setup_binance"),
            if active_tokens.iter().any(|t| t.exchange == "binance") {
                "✅"
            } else {
                "⚠️"
            }
        )
    } else {
        i18n::get_button_text(locale, "live_trading_setup_binance").to_string()
    };
    setup_buttons.push(vec![InlineKeyboardButton::callback(
        binance_text,
        "live_trading_setup_binance"
    )]);
    
    // OKX button - on its own row
    let okx_text = if has_okx {
        format!("{} {}", 
            i18n::get_button_text(locale, "live_trading_setup_okx"),
            if active_tokens.iter().any(|t| t.exchange == "okx") {
                "✅"
            } else {
                "⚠️"
            }
        )
    } else {
        i18n::get_button_text(locale, "live_trading_setup_okx").to_string()
    };
    setup_buttons.push(vec![InlineKeyboardButton::callback(
        okx_text,
        "live_trading_setup_okx"
    )]);
    
    if !active_tokens.is_empty() {
        setup_buttons.push(vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "live_trading_start_trading"),
                "live_trading_show_strategies"
            )
        ]);
    }
    
    setup_buttons.push(vec![
        InlineKeyboardButton::callback(
            i18n::get_button_text(locale, "trading_cancel"),
            "cancel_live_trading"
        )
    ]);
    
    let exchanges_list: Vec<String> = active_tokens.iter()
        .map(|t| {
            match t.exchange.as_str() {
                "binance" => "🔵 Binance".to_string(),
                "okx" => "🟢 OKX".to_string(),
                _ => t.exchange.clone(),
            }
        })
        .collect();
    
    let status_msg = if active_tokens.is_empty() {
        i18n::translate(locale, "live_trading_no_tokens", None)
    } else {
        i18n::translate(locale, "live_trading_tokens_configured", Some(&[
            ("exchanges", &exchanges_list.join(", ")),
            ("count", &active_tokens.len().to_string()),
        ]))
    };
    
    bot.send_message(msg.chat.id, status_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(setup_buttons))
        .await?;
    
    dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForExchangeSetup)).await?;
    
    Ok(())
}

/// Handler for live trading callbacks
pub async fn handle_live_trading_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let telegram_id = q.from.id.0 as i64;
    
    // Get user locale
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    if let Some(data) = q.data {
        if data == "live_trading_setup_binance" {
            let exchange_name = "🔵 Binance";
            let callback_id = q.id.clone();
            bot.answer_callback_query(callback_id)
                .text(exchange_name)
                .await?;
            
            let msg_text = i18n::translate(locale, "live_trading_enter_api_key", Some(&[("exchange", exchange_name)]));
            if let Some(msg) = q.message {
                // Edit message to show next step and remove buttons
                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![])) // Remove buttons
                    .await
                {
                    // If edit fails, send new message
                    bot.send_message(msg.chat().id, msg_text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
            } else {
                bot.send_message(q.from.id, msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForApiKey {
                exchange: "binance".to_string(),
            })).await?;
        } else if data == "live_trading_setup_okx" {
            let exchange_name = "🟢 OKX";
            let callback_id = q.id.clone();
            bot.answer_callback_query(callback_id)
                .text(exchange_name)
                .await?;
            
            let msg_text = i18n::translate(locale, "live_trading_enter_api_key", Some(&[("exchange", exchange_name)]));
            if let Some(msg) = q.message {
                // Edit message to show next step and remove buttons
                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![])) // Remove buttons
                    .await
                {
                    // If edit fails, send new message
                    bot.send_message(msg.chat().id, msg_text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
            } else {
                bot.send_message(q.from.id, msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForApiKey {
                exchange: "okx".to_string(),
            })).await?;
        } else if data == "live_trading_show_strategies" {
            let callback_id = q.id.clone();
            let selection_text = i18n::translate(locale, "live_trading_selecting_strategy", None);
            bot.answer_callback_query(callback_id)
                .text(&selection_text)
                .await?;
            
            // Get user's strategies
            let strategies_list = state.strategy_service.get_user_strategies(telegram_id).await?;
            
            if strategies_list.is_empty() {
                let msg_text = i18n::translate(locale, "trading_no_strategies", None);
                if let Some(msg) = q.message {
                    // Edit message to show error and remove buttons
                    if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &msg_text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![]))
                        .await
                    {
                        bot.send_message(msg.chat().id, msg_text).await?;
                    }
                } else {
                    bot.send_message(q.from.id, msg_text).await?;
                }
                return Ok(());
            }
            
            // Create buttons for strategies
            let mut strategy_buttons = Vec::new();
            for strategy in &strategies_list {
                let button_text = strategy.name.as_ref()
                    .map(|n| n.clone())
                    .unwrap_or_else(|| format!("Strategy #{}", strategy.id));
                strategy_buttons.push(vec![
                    InlineKeyboardButton::callback(
                        button_text,
                        format!("live_trading_strategy_{}", strategy.id)
                    )
                ]);
            }
            
            strategy_buttons.push(vec![
                InlineKeyboardButton::callback(
                    i18n::get_button_text(locale, "trading_cancel"),
                    "cancel_live_trading"
                )
            ]);
            
            let msg_text = i18n::translate(locale, "live_trading_select_strategy", None);
            if let Some(msg) = q.message {
                // Edit message to show strategy selection
                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new(strategy_buttons.clone()))
                    .await
                {
                    // If edit fails, send new message
                    bot.send_message(msg.chat().id, msg_text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(strategy_buttons))
                        .await?;
                }
            } else {
                bot.send_message(q.from.id, msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new(strategy_buttons))
                    .await?;
            }
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForStrategy)).await?;
        } else if data.starts_with("live_trading_strategy_") {
            let strategy_id_str = data.trim_start_matches("live_trading_strategy_");
            if let Ok(strategy_id) = strategy_id_str.parse::<u64>() {
                // Get strategy name first for feedback
                let strategy_name = if let Some(strategy) = state.strategy_service.get_strategy_by_id(strategy_id).await? {
                    strategy.name.as_ref()
                        .map(|n| n.clone())
                        .unwrap_or_else(|| format!("Strategy #{}", strategy_id))
                } else {
                    format!("Strategy #{}", strategy_id)
                };
                
                // Answer callback with user's selection
                let callback_id = q.id.clone();
                bot.answer_callback_query(callback_id)
                    .text(&strategy_name)
                    .await?;
                
                // Get user's active tokens to select exchange
                let exchange_tokens_list = exchange_tokens::Entity::find()
                    .filter(exchange_tokens::Column::UserId.eq(telegram_id))
                    .filter(exchange_tokens::Column::IsActive.eq(1))
                    .all(state.db.as_ref())
                    .await?;
                
                if exchange_tokens_list.is_empty() {
                    let error_msg = i18n::translate(locale, "live_trading_no_tokens", None);
                    if let Some(msg) = q.message {
                        // Edit message to show error and remove buttons
                        if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &error_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![]))
                            .await
                        {
                            bot.send_message(msg.chat().id, error_msg).await?;
                        }
                    } else {
                        bot.send_message(q.from.id, error_msg).await?;
                    }
                    return Ok(());
                }
                
                // Create buttons for exchanges
                let mut exchange_buttons = Vec::new();
                for token in &exchange_tokens_list {
                    let exchange_name = match token.exchange.as_str() {
                        "binance" => "🔵 Binance",
                        "okx" => "🟢 OKX",
                        _ => &token.exchange,
                    };
                    exchange_buttons.push(vec![
                        InlineKeyboardButton::callback(
                            exchange_name.to_string(),
                            format!("live_trading_exchange_{}_{}", token.exchange, strategy_id)
                        )
                    ]);
                }
                
                exchange_buttons.push(vec![
                    InlineKeyboardButton::callback(
                        i18n::get_button_text(locale, "trading_cancel"),
                        "cancel_live_trading"
                    )
                ]);
                
                let msg_text = i18n::translate(locale, "live_trading_select_exchange", None);
                if let Some(msg) = q.message {
                    // Edit message to show exchange selection
                    if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &msg_text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(exchange_buttons.clone()))
                        .await
                    {
                        // If edit fails, send new message
                        bot.send_message(msg.chat().id, msg_text)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(exchange_buttons))
                            .await?;
                    }
                } else {
                    bot.send_message(q.from.id, msg_text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(exchange_buttons))
                        .await?;
                }
                
                dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForExchange {
                    strategy_id,
                })).await?;
            }
        } else if data.starts_with("live_trading_exchange_") {
            // Format: live_trading_exchange_{exchange}_{strategy_id}
            let parts: Vec<&str> = data.trim_start_matches("live_trading_exchange_").split('_').collect();
            if parts.len() >= 2 {
                let exchange = parts[0];
                if let Ok(strategy_id) = parts[1].parse::<u64>() {
                    // Get exchange name for feedback
                    let exchange_name = match exchange {
                        "binance" => "🔵 Binance",
                        "okx" => "🟢 OKX",
                        _ => exchange,
                    };
                    
                    // Answer callback with user's selection
                    let callback_id = q.id.clone();
                    bot.answer_callback_query(callback_id)
                        .text(exchange_name)
                        .await?;
                    
                    // Get token for this exchange
                    let token = exchange_tokens::Entity::find()
                        .filter(exchange_tokens::Column::UserId.eq(telegram_id))
                        .filter(exchange_tokens::Column::Exchange.eq(exchange))
                        .filter(exchange_tokens::Column::IsActive.eq(1))
                        .one(state.db.as_ref())
                        .await?;
                    
                    if let Some(token) = token {
                        // Get strategy config
                        if let Some(_strategy) = state.strategy_service.get_strategy_by_id(strategy_id).await? {
                            match state.strategy_service.strategy_to_config(&_strategy) {
                                Ok(config) => {
                                    // Clone config fields before moving config
                                    let pair = config.pair.clone();
                                    let timeframe = config.timeframe.clone();
                                    
                                    // Store message info before moving
                                    let msg_info = q.message.as_ref().map(|m| (m.chat().id, m.id()));
                                    
                                    // Show "Starting..." message and remove buttons
                                    let starting_msg = i18n::translate(locale, "live_trading_starting", Some(&[("exchange", exchange_name)]));
                                    
                                    if let Some((chat_id, msg_id)) = msg_info {
                                        // Edit message to show starting status and remove buttons
                                        if let Err(e) = bot.edit_message_text(chat_id, msg_id, &starting_msg)
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![]))
                                            .await
                                        {
                                            // If edit fails, send new message
                                            bot.send_message(chat_id, &starting_msg)
                                                .parse_mode(teloxide::types::ParseMode::Html)
                                                .await?;
                                        }
                                    }
                                    
                                    // Start live trading with exchange API
                                    // Get user chat ID from callback query
                                    let user_chat_id = q.from.id.0 as i64;
                                    
                                    match start_live_trading_with_exchange(
                                        state.clone(),
                                        bot.clone(),
                                        telegram_id,
                                        user_chat_id,
                                        &token,
                                        config,
                                        Some(strategy_id), // Pass strategy_id
                                    ).await {
                                        Ok(_) => {
                                            let strategy_name = _strategy.name.as_ref()
                                                .unwrap_or(&format!("Strategy #{}", strategy_id))
                                                .clone();
                                            let exchange_name = match exchange {
                                                "binance" => "🔵 Binance",
                                                "okx" => "🟢 OKX",
                                                _ => exchange,
                                            };
                                            
                                            let success_msg = i18n::translate(locale, "live_trading_started", Some(&[
                                                ("exchange", exchange_name),
                                                ("strategy", &strategy_name),
                                                ("pair", &pair),
                                                ("timeframe", &timeframe),
                                            ]));
                                            
                                            if let Some((chat_id, msg_id)) = msg_info {
                                                // Edit message to show success
                                                if let Err(e) = bot.edit_message_text(chat_id, msg_id, &success_msg)
                                                    .parse_mode(teloxide::types::ParseMode::Html)
                                                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![]))
                                                    .await
                                                {
                                                    // If edit fails, send new message
                                                    bot.send_message(chat_id, &success_msg)
                                                        .parse_mode(teloxide::types::ParseMode::Html)
                                                        .await?;
                                                }
                                            } else {
                                                bot.send_message(q.from.id, success_msg)
                                                    .parse_mode(teloxide::types::ParseMode::Html)
                                                    .await?;
                                            }
                                            
                                            // Exit dialogue after successful start
                                            dialogue.exit().await?;
                                        }
                                        Err(e) => {
                                            let error_msg = i18n::translate(locale, "live_trading_start_error", Some(&[
                                                ("error", &e.to_string()),
                                            ]));
                                            
                                            if let Some((chat_id, msg_id)) = msg_info {
                                                // Edit message to show error
                                                if let Err(e) = bot.edit_message_text(chat_id, msg_id, &error_msg)
                                                    .parse_mode(teloxide::types::ParseMode::Html)
                                                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![]))
                                                    .await
                                                {
                                                    // If edit fails, send new message
                                                    bot.send_message(chat_id, &error_msg)
                                                        .parse_mode(teloxide::types::ParseMode::Html)
                                                        .await?;
                                                }
                                            } else {
                                                bot.send_message(q.from.id, error_msg)
                                                    .parse_mode(teloxide::types::ParseMode::Html)
                                                    .await?;
                                            }
                                            
                                            // Exit dialogue even on error
                                            dialogue.exit().await?;
                                        }
                                    }
                                }
                                Err(e) => {
                                    let error_msg = i18n::translate(locale, "trading_config_error", Some(&[
                                        ("error", &e.to_string()),
                                    ]));
                                    
                                    if let Some(msg) = q.message {
                                        // Edit message to show error and remove buttons
                                        if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &error_msg)
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![]))
                                            .await
                                        {
                                            bot.send_message(msg.chat().id, error_msg).await?;
                                        }
                                    } else {
                                        bot.send_message(q.from.id, error_msg).await?;
                                    }
                                    
                                    // Exit dialogue on config error
                                    dialogue.exit().await?;
                                }
                            }
                        } else {
                            // Strategy not found - exit dialogue
                            let error_msg = i18n::translate(locale, "live_trading_strategy_not_found", None);
                            
                            if let Some(msg) = q.message {
                                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &error_msg)
                                    .parse_mode(teloxide::types::ParseMode::Html)
                                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![]))
                                    .await
                                {
                                    bot.send_message(msg.chat().id, &error_msg).await?;
                                }
                            }
                            dialogue.exit().await?;
                        }
                    } else {
                        // Token not found - exit dialogue
                        let error_msg = i18n::translate(locale, "live_trading_token_not_found", None);
                        
                        if let Some(msg) = q.message {
                            if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &error_msg)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![]))
                                .await
                            {
                                bot.send_message(msg.chat().id, &error_msg).await?;
                            }
                        }
                        dialogue.exit().await?;
                    }
                }
            }
        } else if data == "cancel_live_trading" {
            let callback_id = q.id.clone();
            let cancel_text = i18n::get_button_text(locale, "live_trading_cancelled");
            bot.answer_callback_query(callback_id)
                .text(&cancel_text)
                .await?;
            
            dialogue.exit().await?;
            
            let cancel_msg = i18n::translate(locale, "live_trading_cancelled", None);
            if let Some(msg) = q.message {
                // Edit message to show cancellation and remove buttons
                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &cancel_msg)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![]))
                    .await
                {
                    // If edit fails, send new message
                    bot.send_message(msg.chat().id, cancel_msg).await?;
                }
            } else {
                bot.send_message(q.from.id, cancel_msg).await?;
            }
        }
    }
    
    Ok(())
}

/// Handler for live trading input (API key, API secret)
pub async fn handle_live_trading_input(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let telegram_id = msg.from.as_ref().map(|f| f.id.0 as i64).unwrap_or(0);
    
    // Get text first before moving msg
    let text = msg.text().map(|t| t.to_string());
    
    // Get user locale
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    if let Ok(Some(BotState::LiveTrading(LiveTradingState::WaitingForApiKey { exchange }))) = dialogue.get().await {
        if let Some(text) = text {
            let api_key = text.trim().to_string();
            
            if api_key.is_empty() {
                let error_msg = i18n::translate(locale, "live_trading_invalid_api_key", None);
                bot.send_message(msg.chat.id, error_msg).await?;
                return Ok(());
            }
            
            let exchange_name = match exchange.as_str() {
                "binance" => "🔵 Binance",
                "okx" => "🟢 OKX",
                _ => &exchange,
            };
            
            let msg_text = i18n::translate(locale, "live_trading_enter_api_secret", Some(&[("exchange", exchange_name)]));
            bot.send_message(msg.chat.id, msg_text)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForApiSecret {
                exchange,
                api_key,
            })).await?;
        }
    } else if let Ok(Some(BotState::LiveTrading(LiveTradingState::WaitingForApiSecret { exchange, api_key }))) = dialogue.get().await {
        if let Some(text) = text {
            let api_secret = text.trim().to_string();
            
            if api_secret.is_empty() {
                let error_msg = i18n::translate(locale, "live_trading_invalid_api_secret", None);
                bot.send_message(msg.chat.id, error_msg).await?;
                return Ok(());
            }
            
            // Validate token by testing connection (optional, can be done later)
            // For now, just save it
            
            // Check if token already exists for this user and exchange
            let existing = exchange_tokens::Entity::find()
                .filter(exchange_tokens::Column::UserId.eq(telegram_id))
                .filter(exchange_tokens::Column::Exchange.eq(&exchange))
                .one(state.db.as_ref())
                .await?;
            
            if let Some(existing_token) = existing {
                // Update existing token
                let mut token: exchange_tokens::ActiveModel = existing_token.into();
                token.api_key = ActiveValue::Set(api_key);
                token.api_secret = ActiveValue::Set(api_secret);
                token.is_active = ActiveValue::Set(1);
                token.updated_at = ActiveValue::Set(Some(Utc::now()));
                
                exchange_tokens::Entity::update(token).exec(state.db.as_ref()).await?;
            } else {
                // Create new token
                let new_token = exchange_tokens::ActiveModel {
                    user_id: ActiveValue::Set(telegram_id),
                    exchange: ActiveValue::Set(exchange.clone()),
                    api_key: ActiveValue::Set(api_key),
                    api_secret: ActiveValue::Set(api_secret),
                    is_active: ActiveValue::Set(1),
                    created_at: ActiveValue::Set(Some(Utc::now())),
                    updated_at: ActiveValue::Set(Some(Utc::now())),
                    ..Default::default()
                };
                
                exchange_tokens::Entity::insert(new_token).exec(state.db.as_ref()).await?;
            }
            
            let exchange_name = match exchange.as_str() {
                "binance" => "🔵 Binance",
                "okx" => "🟢 OKX",
                _ => &exchange,
            };
            
            let success_msg = i18n::translate(locale, "live_trading_token_saved", Some(&[("exchange", exchange_name)]));
            bot.send_message(msg.chat.id, success_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            
            // After saving token, show the setup menu again so user can setup more or start trading
            let exchange_tokens_list = exchange_tokens::Entity::find()
                .filter(exchange_tokens::Column::UserId.eq(telegram_id))
                .all(state.db.as_ref())
                .await?;
            
            let active_tokens: Vec<_> = exchange_tokens_list.iter()
                .filter(|t| t.is_active == 1)
                .collect();
            
            let has_binance = exchange_tokens_list.iter().any(|t| t.exchange == "binance");
            let has_okx = exchange_tokens_list.iter().any(|t| t.exchange == "okx");
            
            let mut setup_buttons = Vec::new();
            
            // Binance button - on its own row
            let binance_text = if has_binance {
                format!("{} {}", 
                    i18n::get_button_text(locale, "live_trading_setup_binance"),
                    if active_tokens.iter().any(|t| t.exchange == "binance") {
                        "✅"
                    } else {
                        "⚠️"
                    }
                )
            } else {
                i18n::get_button_text(locale, "live_trading_setup_binance").to_string()
            };
            setup_buttons.push(vec![InlineKeyboardButton::callback(
                binance_text,
                "live_trading_setup_binance"
            )]);
            
            // OKX button - on its own row
            let okx_text = if has_okx {
                format!("{} {}", 
                    i18n::get_button_text(locale, "live_trading_setup_okx"),
                    if active_tokens.iter().any(|t| t.exchange == "okx") {
                        "✅"
                    } else {
                        "⚠️"
                    }
                )
            } else {
                i18n::get_button_text(locale, "live_trading_setup_okx").to_string()
            };
            setup_buttons.push(vec![InlineKeyboardButton::callback(
                okx_text,
                "live_trading_setup_okx"
            )]);
            
            if !active_tokens.is_empty() {
                setup_buttons.push(vec![
                    InlineKeyboardButton::callback(
                        i18n::get_button_text(locale, "live_trading_start_trading"),
                        "live_trading_show_strategies"
                    )
                ]);
            }
            
            setup_buttons.push(vec![
                InlineKeyboardButton::callback(
                    i18n::get_button_text(locale, "trading_cancel"),
                    "cancel_live_trading"
                )
            ]);
            
            let exchanges_list: Vec<String> = active_tokens.iter()
                .map(|t| {
                    match t.exchange.as_str() {
                        "binance" => "🔵 Binance".to_string(),
                        "okx" => "🟢 OKX".to_string(),
                        _ => t.exchange.clone(),
                    }
                })
                .collect();
            
            let status_msg = if active_tokens.is_empty() {
                i18n::translate(locale, "live_trading_no_tokens", None)
            } else {
                i18n::translate(locale, "live_trading_tokens_configured", Some(&[
                    ("exchanges", &exchanges_list.join(", ")),
                    ("count", &active_tokens.len().to_string()),
                ]))
            };
            
            bot.send_message(msg.chat.id, status_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(teloxide::types::InlineKeyboardMarkup::new(setup_buttons))
                .await?;
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForExchangeSetup)).await?;
        }
    }
    
    Ok(())
}

/// Start live trading with exchange API
async fn start_live_trading_with_exchange(
    state: Arc<AppState>,
    bot: Bot,
    user_id: i64,
    user_chat_id: i64, // Telegram chat ID to send signals to
    token: &exchange_tokens::Model,
    strategy_config: crate::services::strategy_engine::StrategyConfig,
    strategy_id: Option<u64>, // Strategy ID from database
) -> Result<()> {
    tracing::info!(
        "Starting live trading for user {} (chat: {}) on {} with strategy {}",
        user_id,
        user_chat_id,
        token.exchange,
        strategy_config.strategy_type
    );
    
    // Start strategy executor (registers user's strategy)
    state.strategy_executor.start_trading(user_id, strategy_config.clone(), Some(token.exchange.clone())).await?;
    
    // Save live trading session to database
    let session = live_trading_sessions::ActiveModel {
        user_id: ActiveValue::Set(user_id),
        strategy_id: ActiveValue::Set(strategy_id),
        strategy_name: ActiveValue::Set(Some(strategy_config.strategy_type.clone())),
        exchange: ActiveValue::Set(token.exchange.clone()),
        pair: ActiveValue::Set(strategy_config.pair.clone()),
        timeframe: ActiveValue::Set(Some(strategy_config.timeframe.clone())),
        status: ActiveValue::Set("active".to_string()),
        started_at: ActiveValue::Set(Some(Utc::now())),
        stopped_at: ActiveValue::NotSet,
        created_at: ActiveValue::Set(Some(Utc::now())),
        updated_at: ActiveValue::Set(Some(Utc::now())),
        ..Default::default()
    };
    
    let session_result = live_trading_sessions::Entity::insert(session)
        .exec(state.db.as_ref())
        .await?;
    
    tracing::info!("✅ Created live trading session {} for user {} with strategy {}", 
        session_result.last_insert_id, user_id, 
        strategy_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string()));
    
    // Start user-specific trading service (monitors market and sends signals)
    use crate::services::trading_signal::start_user_trading_service;
    
    start_user_trading_service(
        state,
        bot,
        user_id,
        user_chat_id,
        strategy_config.clone(),
        token.exchange.clone(),
        strategy_config.pair.clone(),
    );
    Ok(())
}

/// Handler for /mytrading command to view current live trading status
pub async fn handle_my_trading(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let from = msg.from.unwrap();
    let telegram_id = from.id.0 as i64;
    
    // Get user locale
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    // Get all active live trading sessions from database
    let active_sessions = live_trading_sessions::Entity::find()
        .filter(live_trading_sessions::Column::UserId.eq(telegram_id))
        .filter(live_trading_sessions::Column::Status.eq("active"))
        .order_by(live_trading_sessions::Column::StartedAt, Order::Desc)
        .all(state.db.as_ref())
        .await?;
    
    if !active_sessions.is_empty() {
        // Get the first (most recent) session for display
        let session = &active_sessions[0];
        let exchange_name = match session.exchange.as_str() {
            "binance" => "🔵 Binance",
            "okx" => "🟢 OKX",
            _ => &session.exchange,
        };
        
        let strategy_name = session.strategy_name.as_ref()
            .unwrap_or(&format!("Strategy #{}", session.strategy_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string())))
            .clone();
        
        let pair = session.pair.clone();
        let timeframe = session.timeframe.as_ref()
            .map(|t| t.clone())
            .unwrap_or_else(|| "N/A".to_string());
        
        let started_at = session.started_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        
        let status_msg = format!(
            "{}\n\n{}\n\n{}\n{}\n{}\n{}\n{}\n\n{}",
            i18n::translate(locale, "mytrading_status_title", None),
            i18n::translate(locale, "mytrading_status_running", None),
            i18n::translate(locale, "mytrading_strategy", Some(&[("strategy", &strategy_name)])),
            i18n::translate(locale, "mytrading_pair", Some(&[("pair", &pair)])),
            i18n::translate(locale, "mytrading_timeframe", Some(&[("timeframe", &timeframe)])),
            i18n::translate(locale, "mytrading_exchange", Some(&[("exchange", exchange_name)])),
            i18n::translate(locale, "mytrading_started", Some(&[("started_at", &started_at)])),
            i18n::translate(locale, "mytrading_monitoring", None),
        );
        
        // Create stop button - will show session list if multiple sessions
        let stop_button = InlineKeyboardButton::callback(
            i18n::get_button_text(locale, "mytrading_stop_button"),
            format!("stop_live_trading_{}", telegram_id)
        );
        
        let buttons = vec![vec![stop_button]];
        
        bot.send_message(msg.chat.id, status_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
            .await?;
    } else {
        // No active session found
        let msg_text = i18n::translate(locale, "mytrading_no_active", None);
        
        bot.send_message(msg.chat.id, msg_text)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }
    
    Ok(())
}

/// Handler for stop trading callback
pub async fn handle_stop_trading_callback(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    tracing::info!("🔍 handle_stop_trading_callback called with data: {:?}", q.data);
    
    if let Some(data) = q.data {
        let callback_user_id = q.from.id.0 as i64;
        
        tracing::info!("🔍 Processing callback data: '{}' for user {}", data, callback_user_id);
        
        // Get user locale
        let user = users::Entity::find_by_id(callback_user_id)
            .one(state.db.as_ref())
            .await?;
        let locale = user
            .as_ref()
            .and_then(|u| u.language.as_ref())
            .map(|l| i18n::get_user_language(Some(l)))
            .unwrap_or("en");
        
        if data.starts_with("stop_live_trading_") {
            tracing::info!("🔍 Matched stop_live_trading_ pattern");
            // Step 1: Show list of active sessions to choose from
            let user_id_str = data.trim_start_matches("stop_live_trading_");
            if let Ok(user_id) = user_id_str.parse::<i64>() {
                // Verify this is the user's own trading
                if user_id != callback_user_id {
                    let callback_id = q.id.clone();
                    bot.answer_callback_query(callback_id)
                        .text(i18n::translate(locale, "stop_trading_not_yours", None))
                        .await?;
                    return Ok(());
                }
                
                // Get all active sessions first
                let active_sessions = live_trading_sessions::Entity::find()
                    .filter(live_trading_sessions::Column::UserId.eq(user_id))
                    .filter(live_trading_sessions::Column::Status.eq("active"))
                    .order_by(live_trading_sessions::Column::StartedAt, Order::Desc)
                    .all(state.db.as_ref())
                    .await?;
                
                if active_sessions.is_empty() {
                    let callback_id = q.id.clone();
                    let error_text = i18n::translate(locale, "stop_trading_not_found", None);
                    bot.answer_callback_query(callback_id)
                        .text(&error_text)
                        .await?;
                    return Ok(());
                }
                
                // If only one session, go directly to confirmation
                if active_sessions.len() == 1 {
                    let session = &active_sessions[0];
                    let session_id = session.id;
                    
                    let exchange_name = match session.exchange.as_str() {
                        "binance" => "🔵 Binance",
                        "okx" => "🟢 OKX",
                        _ => &session.exchange,
                    };
                    
                    let strategy_name = session.strategy_name.as_ref()
                        .unwrap_or(&format!("Strategy #{}", session.strategy_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string())))
                        .clone();
                    
                    // Answer callback with user's selection
                    let callback_id = q.id.clone();
                    let selection_text = format!("{} - {}", strategy_name, session.pair);
                    bot.answer_callback_query(callback_id)
                        .text(&selection_text)
                        .await?;
                    
                    // Show confirmation dialog
                    if let Some(msg) = q.message {
                        let confirm_msg = format!(
                            "{}\n\n{}\n{}\n{}\n\n{}",
                            i18n::translate(locale, "stop_trading_confirm_title", None),
                            i18n::translate(locale, "mytrading_strategy", Some(&[("strategy", &strategy_name)])),
                            i18n::translate(locale, "mytrading_pair", Some(&[("pair", &session.pair)])),
                            i18n::translate(locale, "mytrading_exchange", Some(&[("exchange", exchange_name)])),
                            i18n::translate(locale, "stop_trading_confirm_message", None),
                        );
                        
                        let confirm_buttons = vec![
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "stop_trading_confirm_yes"),
                                    format!("stop_confirm_{}", session_id)
                                ),
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "stop_trading_confirm_no"),
                                    "stop_cancel"
                                ),
                            ],
                        ];
                        
                        // Edit message, ignore "message is not modified" error
                        if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &confirm_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(confirm_buttons.clone()))
                            .await
                        {
                            // Ignore "message is not modified" error - message already has correct content
                            let error_str = e.to_string();
                            if !error_str.contains("message is not modified") {
                                tracing::warn!("Failed to edit message: {}", e);
                            }
                        }
                    }
                } else {
                    // Multiple sessions - show list to choose from
                    let callback_id = q.id.clone();
                    let selection_text = i18n::translate(locale, "stop_trading_select_session", None);
                    bot.answer_callback_query(callback_id)
                        .text(&selection_text)
                        .await?;
                    
                    if let Some(msg) = q.message {
                        let select_msg = i18n::translate(locale, "stop_trading_select_session", None);
                        
                        let mut session_buttons = Vec::new();
                        for session in &active_sessions {
                            let exchange_name = match session.exchange.as_str() {
                                "binance" => "🔵 Binance",
                                "okx" => "🟢 OKX",
                                _ => &session.exchange,
                            };
                            
                            let strategy_name = session.strategy_name.as_ref()
                                .unwrap_or(&format!("Strategy #{}", session.strategy_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string())))
                                .clone();
                            
                            let button_text = format!("{} - {} ({})", strategy_name, session.pair, exchange_name);
                            session_buttons.push(vec![
                                InlineKeyboardButton::callback(
                                    button_text,
                                    format!("stop_session_{}", session.id)
                                )
                            ]);
                        }
                        
                        session_buttons.push(vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(locale, "trading_cancel"),
                                "stop_cancel"
                            )
                        ]);
                        
                        // Edit message, ignore "message is not modified" error
                        if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &select_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(session_buttons.clone()))
                            .await
                        {
                            // Ignore "message is not modified" error - message already has correct content
                            let error_str = e.to_string();
                            if !error_str.contains("message is not modified") {
                                tracing::warn!("Failed to edit message: {}", e);
                            }
                        }
                    }
                }
            }
        } else if data.starts_with("stop_session_") {
            // Step 2: Show confirmation for selected session
            let session_id_str = data.trim_start_matches("stop_session_");
            if let Ok(session_id) = session_id_str.parse::<u64>() {
                // Get session details first
                let session = live_trading_sessions::Entity::find_by_id(session_id)
                    .one(state.db.as_ref())
                    .await?;
                
                if let Some(session) = session {
                    // Verify session belongs to user
                    if session.user_id != callback_user_id {
                        let callback_id = q.id.clone();
                        let error_text = i18n::translate(locale, "stop_trading_not_yours", None);
                        bot.answer_callback_query(callback_id)
                            .text(&error_text)
                            .await?;
                        return Ok(());
                    }
                    
                    // Verify session is active
                    if session.status != "active" {
                        let callback_id = q.id.clone();
                        let info_text = i18n::translate(locale, "stop_trading_session_not_active", None);
                        bot.answer_callback_query(callback_id)
                            .text(&info_text)
                            .await?;
                        return Ok(());
                    }
                    
                    let exchange_name = match session.exchange.as_str() {
                        "binance" => "🔵 Binance",
                        "okx" => "🟢 OKX",
                        _ => &session.exchange,
                    };
                    
                    let strategy_name = session.strategy_name.as_ref()
                        .unwrap_or(&format!("Strategy #{}", session.strategy_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string())))
                        .clone();
                    
                    // Answer callback with user's selection
                    let callback_id = q.id.clone();
                    let selection_text = format!("{} - {}", strategy_name, session.pair);
                    bot.answer_callback_query(callback_id)
                        .text(&selection_text)
                        .await?;
                    
                    // Show confirmation dialog
                    if let Some(msg) = q.message {
                        let confirm_msg = format!(
                            "{}\n\n{}\n{}\n{}\n\n{}",
                            i18n::translate(locale, "stop_trading_confirm_title", None),
                            i18n::translate(locale, "mytrading_strategy", Some(&[("strategy", &strategy_name)])),
                            i18n::translate(locale, "mytrading_pair", Some(&[("pair", &session.pair)])),
                            i18n::translate(locale, "mytrading_exchange", Some(&[("exchange", exchange_name)])),
                            i18n::translate(locale, "stop_trading_confirm_message", None),
                        );
                        
                        let confirm_buttons = vec![
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "stop_trading_confirm_yes"),
                                    format!("stop_confirm_{}", session_id)
                                ),
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "stop_trading_confirm_no"),
                                    "stop_cancel"
                                ),
                            ],
                        ];
                        
                        // Edit message, ignore "message is not modified" error
                        if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &confirm_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(confirm_buttons.clone()))
                            .await
                        {
                            // Ignore "message is not modified" error - message already has correct content
                            let error_str = e.to_string();
                            if !error_str.contains("message is not modified") {
                                tracing::warn!("Failed to edit message: {}", e);
                            }
                        }
                    }
                } else {
                    let callback_id = q.id.clone();
                    let error_text = i18n::translate(locale, "stop_trading_session_not_found", None);
                    bot.answer_callback_query(callback_id)
                        .text(&error_text)
                        .await?;
                }
            }
        } else if data.starts_with("stop_confirm_") {
            // Step 3: Actually stop the trading
            let session_id_str = data.trim_start_matches("stop_confirm_");
            if let Ok(session_id) = session_id_str.parse::<u64>() {
                // Get session details first
                let session = live_trading_sessions::Entity::find_by_id(session_id)
                    .one(state.db.as_ref())
                    .await?;
                
                if let Some(session) = session {
                    // Verify session belongs to user
                    if session.user_id != callback_user_id {
                        let callback_id = q.id.clone();
                        let error_text = i18n::translate(locale, "stop_trading_not_yours", None);
                        bot.answer_callback_query(callback_id)
                            .text(&error_text)
                            .await?;
                        return Ok(());
                    }
                    
                    // Verify session is active
                    if session.status != "active" {
                        let callback_id = q.id.clone();
                        let info_text = i18n::translate(locale, "stop_trading_session_not_active", None);
                        bot.answer_callback_query(callback_id)
                            .text(&info_text)
                            .await?;
                        return Ok(());
                    }
                    
                    let strategy_name = session.strategy_name.as_ref()
                        .unwrap_or(&format!("Strategy #{}", session.strategy_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string())))
                        .clone();
                    
                    // Answer callback with confirmation
                    let callback_id = q.id.clone();
                    let confirm_text = i18n::translate(locale, "stop_trading_confirmed", Some(&[("strategy", &strategy_name)]));
                    bot.answer_callback_query(callback_id)
                        .text(&confirm_text)
                        .await?;
                    
                    let user_id = session.user_id;
                    
                    // Stop trading and unsubscribe from stream
                    match state.strategy_executor.stop_trading(user_id).await {
                        Ok(Some((exchange_stream, pair_stream))) => {
                            // Unsubscribe from stream
                            state.stream_manager.unsubscribe(&exchange_stream, &pair_stream, user_id).await;
                            
                            // Update live trading session status to stopped
                            let mut session_update: live_trading_sessions::ActiveModel = session.into();
                            session_update.status = ActiveValue::Set("stopped".to_string());
                            session_update.stopped_at = ActiveValue::Set(Some(Utc::now()));
                            session_update.updated_at = ActiveValue::Set(Some(Utc::now()));
                            
                            live_trading_sessions::Entity::update(session_update)
                                .exec(state.db.as_ref())
                                .await?;
                            
                            tracing::info!("✅ Updated live trading session {} status to stopped for user {}", session_id, user_id);
                            
                            // Update message and remove buttons to prevent further clicks
                            if let Some(msg) = q.message {
                                let success_msg = i18n::translate(locale, "stop_trading_success", None);
                                
                                // Edit message and remove buttons
                                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &success_msg)
                                    .parse_mode(teloxide::types::ParseMode::Html)
                                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![])) // Empty buttons
                                    .await
                                {
                                    // Ignore "message is not modified" error - message already has correct content
                                    let error_str = e.to_string();
                                    if !error_str.contains("message is not modified") {
                                        tracing::warn!("Failed to edit message: {}", e);
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            // User was not trading in executor, but session exists - just update session
                            let mut session_update: live_trading_sessions::ActiveModel = session.into();
                            session_update.status = ActiveValue::Set("stopped".to_string());
                            session_update.stopped_at = ActiveValue::Set(Some(Utc::now()));
                            session_update.updated_at = ActiveValue::Set(Some(Utc::now()));
                            
                            live_trading_sessions::Entity::update(session_update)
                                .exec(state.db.as_ref())
                                .await?;
                            
                            // Update message and remove buttons
                            if let Some(msg) = q.message {
                                let success_msg = i18n::translate(locale, "stop_trading_success", None);
                                
                                // Edit message and remove buttons
                                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &success_msg)
                                    .parse_mode(teloxide::types::ParseMode::Html)
                                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![])) // Empty buttons
                                    .await
                                {
                                    // Ignore "message is not modified" error - message already has correct content
                                    let error_str = e.to_string();
                                    if !error_str.contains("message is not modified") {
                                        tracing::warn!("Failed to edit message: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = i18n::translate(locale, "stop_trading_error", Some(&[("error", &e.to_string())]));
                            
                            // Show error in message and remove buttons
                            if let Some(msg) = q.message {
                                // Edit message with error and remove buttons
                                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &error_msg)
                                    .parse_mode(teloxide::types::ParseMode::Html)
                                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![])) // Empty buttons
                                    .await
                                {
                                    // If edit fails, send new message
                                    bot.send_message(msg.chat().id, &error_msg)
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .await?;
                                }
                            }
                        }
                    }
                } else {
                    let callback_id = q.id.clone();
                    let error_text = i18n::translate(locale, "stop_trading_session_not_found", None);
                    bot.answer_callback_query(callback_id)
                        .text(&error_text)
                        .await?;
                }
            }
        } else if data == "stop_cancel" {
            // Cancel stop operation
            let callback_id = q.id.clone();
            let cancel_text = i18n::get_button_text(locale, "live_trading_cancelled");
            bot.answer_callback_query(callback_id)
                .text(&cancel_text)
                .await?;
            
            if let Some(msg) = q.message {
                let cancel_msg = i18n::translate(locale, "stop_trading_cancelled", None);
                
                // Edit message and remove buttons to prevent further clicks
                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &cancel_msg)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new::<Vec<Vec<teloxide::types::InlineKeyboardButton>>>(vec![])) // Empty buttons
                    .await
                {
                    // Ignore "message is not modified" error - message already has correct content
                    let error_str = e.to_string();
                    if !error_str.contains("message is not modified") {
                        tracing::warn!("Failed to edit message: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(())
}
