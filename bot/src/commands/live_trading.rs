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
            bot.answer_callback_query(q.id).await?;
            
            let exchange_name = "🔵 Binance";
            let msg_text = i18n::translate(locale, "live_trading_enter_api_key", Some(&[("exchange", exchange_name)]));
            bot.send_message(q.message.as_ref().unwrap().chat().id, msg_text)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForApiKey {
                exchange: "binance".to_string(),
            })).await?;
        } else if data == "live_trading_setup_okx" {
            bot.answer_callback_query(q.id).await?;
            
            let exchange_name = "🟢 OKX";
            let msg_text = i18n::translate(locale, "live_trading_enter_api_key", Some(&[("exchange", exchange_name)]));
            bot.send_message(q.message.as_ref().unwrap().chat().id, msg_text)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForApiKey {
                exchange: "okx".to_string(),
            })).await?;
        } else if data == "live_trading_show_strategies" {
            bot.answer_callback_query(q.id).await?;
            
            // Get user's strategies
            let strategies_list = state.strategy_service.get_user_strategies(telegram_id).await?;
            
            if strategies_list.is_empty() {
                let msg_text = i18n::translate(locale, "trading_no_strategies", None);
                bot.send_message(q.message.as_ref().unwrap().chat().id, msg_text).await?;
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
            bot.send_message(q.message.as_ref().unwrap().chat().id, msg_text)
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(teloxide::types::InlineKeyboardMarkup::new(strategy_buttons))
                .await?;
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForStrategy)).await?;
        } else if data.starts_with("live_trading_strategy_") {
            let strategy_id_str = data.trim_start_matches("live_trading_strategy_");
            if let Ok(strategy_id) = strategy_id_str.parse::<u64>() {
                bot.answer_callback_query(q.id).await?;
                
                // Get user's active tokens to select exchange
                let exchange_tokens_list = exchange_tokens::Entity::find()
                    .filter(exchange_tokens::Column::UserId.eq(telegram_id))
                    .filter(exchange_tokens::Column::IsActive.eq(1))
                    .all(state.db.as_ref())
                    .await?;
                
                if exchange_tokens_list.is_empty() {
                    let error_msg = i18n::translate(locale, "live_trading_no_tokens", None);
                    bot.send_message(q.message.as_ref().unwrap().chat().id, error_msg).await?;
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
                bot.send_message(q.message.as_ref().unwrap().chat().id, msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new(exchange_buttons))
                    .await?;
                
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
                    bot.answer_callback_query(q.id).await?;
                    
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
                                            let success_msg = i18n::translate(locale, "live_trading_started", Some(&[
                                                ("exchange", exchange),
                                                ("strategy", &_strategy.name.as_ref().unwrap_or(&format!("Strategy #{}", strategy_id))),
                                            ]));
                                            bot.send_message(q.message.as_ref().unwrap().chat().id, success_msg)
                                                .parse_mode(teloxide::types::ParseMode::Html)
                                                .await?;
                                            
                                            // Exit dialogue after successful start
                                            dialogue.exit().await?;
                                        }
                                        Err(e) => {
                                            let error_msg = i18n::translate(locale, "live_trading_start_error", Some(&[
                                                ("error", &e.to_string()),
                                            ]));
                                            bot.send_message(q.message.as_ref().unwrap().chat().id, error_msg)
                                                .parse_mode(teloxide::types::ParseMode::Html)
                                                .await?;
                                            
                                            // Exit dialogue even on error
                                            dialogue.exit().await?;
                                        }
                                    }
                                }
                                Err(e) => {
                                    let error_msg = i18n::translate(locale, "trading_config_error", Some(&[
                                        ("error", &e.to_string()),
                                    ]));
                                    bot.send_message(q.message.as_ref().unwrap().chat().id, error_msg).await?;
                                    
                                    // Exit dialogue on config error
                                    dialogue.exit().await?;
                                }
                            }
                        } else {
                            // Strategy not found - exit dialogue
                            dialogue.exit().await?;
                        }
                    } else {
                        // Token not found - exit dialogue
                        dialogue.exit().await?;
                    }
                }
            }
        } else if data == "cancel_live_trading" {
            bot.answer_callback_query(q.id).await?;
            dialogue.exit().await?;
            
            let cancel_msg = i18n::translate(locale, "trading_cancelled", None);
            bot.send_message(q.message.as_ref().unwrap().chat().id, cancel_msg).await?;
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
    
    // Get active live trading session from database
    let active_session = live_trading_sessions::Entity::find()
        .filter(live_trading_sessions::Column::UserId.eq(telegram_id))
        .filter(live_trading_sessions::Column::Status.eq("active"))
        .order_by(live_trading_sessions::Column::StartedAt, Order::Desc)
        .one(state.db.as_ref())
        .await?;
    
    if let Some(session) = active_session {
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
        
        let status_msg = if locale == "vi" {
            format!(
                "📊 <b>Live Trading Status</b>\n\n\
                ✅ <b>Trạng thái:</b> Đang chạy\n\n\
                📈 <b>Strategy:</b> {}\n\
                💱 <b>Pair:</b> {}\n\
                ⏰ <b>Timeframe:</b> {}\n\
                🌐 <b>Exchange:</b> {}\n\
                🕐 <b>Bắt đầu:</b> {}\n\n\
                ⚠️ <i>Live trading đang monitor thị trường và sẽ gửi signals khi có tín hiệu.</i>",
                strategy_name, pair, timeframe, exchange_name, started_at
            )
        } else {
            format!(
                "📊 <b>Live Trading Status</b>\n\n\
                ✅ <b>Status:</b> Running\n\n\
                📈 <b>Strategy:</b> {}\n\
                💱 <b>Pair:</b> {}\n\
                ⏰ <b>Timeframe:</b> {}\n\
                🌐 <b>Exchange:</b> {}\n\
                🕐 <b>Started:</b> {}\n\n\
                ⚠️ <i>Live trading is monitoring the market and will send signals when detected.</i>",
                strategy_name, pair, timeframe, exchange_name, started_at
            )
        };
        
        // Create stop button
        let stop_button = InlineKeyboardButton::callback(
            if locale == "vi" {
                "🛑 Dừng Live Trading"
            } else {
                "🛑 Stop Live Trading"
            },
            format!("stop_live_trading_{}", telegram_id)
        );
        
        let buttons = vec![vec![stop_button]];
        
        bot.send_message(msg.chat.id, status_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
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
    if let Some(data) = q.data {
        if data.starts_with("stop_live_trading_") {
            let user_id_str = data.trim_start_matches("stop_live_trading_");
            if let Ok(user_id) = user_id_str.parse::<i64>() {
                // Verify this is the user's own trading
                let callback_user_id = q.from.id.0 as i64;
                if user_id != callback_user_id {
                    // Get user locale
                    let user = users::Entity::find_by_id(callback_user_id)
                        .one(state.db.as_ref())
                        .await?;
                    let locale = user
                        .as_ref()
                        .and_then(|u| u.language.as_ref())
                        .map(|l| i18n::get_user_language(Some(l)))
                        .unwrap_or("en");
                    
                    bot.answer_callback_query(q.id)
                        .text(if locale == "vi" {
                            "❌ Bạn chỉ có thể dừng trading của chính mình."
                        } else {
                            "❌ You can only stop your own trading."
                        })
                        .await?;
                    return Ok(());
                }
                
                // Get user locale
                let user = users::Entity::find_by_id(user_id)
                    .one(state.db.as_ref())
                    .await?;
                let locale = user
                    .as_ref()
                    .and_then(|u| u.language.as_ref())
                    .map(|l| i18n::get_user_language(Some(l)))
                    .unwrap_or("en");
                
                // Stop trading and unsubscribe from stream
                match state.strategy_executor.stop_trading(user_id).await {
                    Ok(Some((exchange, pair))) => {
                        // Unsubscribe from stream
                        state.stream_manager.unsubscribe(&exchange, &pair, user_id).await;
                        
                        // Update live trading session status to stopped
                        let active_session = live_trading_sessions::Entity::find()
                            .filter(live_trading_sessions::Column::UserId.eq(user_id))
                            .filter(live_trading_sessions::Column::Status.eq("active"))
                            .order_by(live_trading_sessions::Column::StartedAt, Order::Desc)
                            .one(state.db.as_ref())
                            .await?;
                        
                        if let Some(session) = active_session {
                            let mut session_update: live_trading_sessions::ActiveModel = session.into();
                            session_update.status = ActiveValue::Set("stopped".to_string());
                            session_update.stopped_at = ActiveValue::Set(Some(Utc::now()));
                            session_update.updated_at = ActiveValue::Set(Some(Utc::now()));
                            
                            live_trading_sessions::Entity::update(session_update)
                                .exec(state.db.as_ref())
                                .await?;
                            
                            tracing::info!("✅ Updated live trading session status to stopped for user {}", user_id);
                        }
                        
                        bot.answer_callback_query(q.id)
                            .text(if locale == "vi" {
                                "✅ Đã dừng live trading"
                            } else {
                                "✅ Live trading stopped"
                            })
                            .await?;
                        
                        // Update message
                        if let Some(msg) = q.message {
                            let success_msg = if locale == "vi" {
                                "✅ <b>Live Trading đã được dừng</b>\n\n\
                                Bạn có thể bắt đầu lại bằng lệnh /livetrading"
                            } else {
                                "✅ <b>Live Trading Stopped</b>\n\n\
                                You can start again using /livetrading"
                            };
                            
                            bot.edit_message_text(msg.chat().id, msg.id(), success_msg)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                        }
                    }
                    Ok(None) => {
                        // User was not trading
                        bot.answer_callback_query(q.id)
                            .text(if locale == "vi" {
                                "ℹ️ Bạn không có live trading đang chạy"
                            } else {
                                "ℹ️ You are not currently trading"
                            })
                            .await?;
                    }
                    Err(e) => {
                        let error_msg = if locale == "vi" {
                            format!("❌ Lỗi khi dừng trading: {}", e)
                        } else {
                            format!("❌ Error stopping trading: {}", e)
                        };
                        
                        bot.answer_callback_query(q.id)
                            .text(&error_msg)
                            .await?;
                    }
                }
            }
        }
    }
    
    Ok(())
}
