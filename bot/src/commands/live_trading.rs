//! Live Trading Command - allows users to trade with real exchanges using OAuth tokens

use std::sync::Arc;
use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use sea_orm::{EntityTrait, ActiveValue, ColumnTrait, QueryFilter};
use crate::state::{AppState, MyDialogue, BotState, LiveTradingState};
use crate::i18n;
use shared::entity::{users, exchange_tokens};
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
    
    // Check if user already has a strategy running
    if state.strategy_executor.is_user_trading(telegram_id).await {
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
    
    // Build setup buttons with status - each button on its own row
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
    
    // If user has at least one active token, show strategy selection option
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
    
    // Build message text
    let msg_text = if active_tokens.is_empty() {
        i18n::translate(locale, "live_trading_no_tokens", None)
    } else {
        let exchanges_list: Vec<String> = active_tokens.iter()
            .map(|t| {
                match t.exchange.as_str() {
                    "binance" => "🔵 Binance".to_string(),
                    "okx" => "🟢 OKX".to_string(),
                    _ => t.exchange.clone(),
                }
            })
            .collect();
        i18n::translate(locale, "live_trading_tokens_configured", Some(&[
            ("exchanges", &exchanges_list.join(", ")),
            ("count", &active_tokens.len().to_string()),
        ]))
    };
    
    bot.send_message(msg.chat.id, msg_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(setup_buttons))
        .await?;
    
    dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForExchangeSetup)).await?;
    Ok(())
}

/// Handler for live trading callbacks
pub async fn handle_live_trading_callback(
    bot: Bot,
    q: CallbackQuery,
    dialogue: MyDialogue,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    if let Some(data) = q.data {
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
        
        if data.starts_with("live_trading_setup_") {
            bot.answer_callback_query(q.id).await?;
            let exchange = data.replace("live_trading_setup_", "");
            
            // Check if token already exists for this exchange
            let existing_token = exchange_tokens::Entity::find()
                .filter(exchange_tokens::Column::UserId.eq(telegram_id))
                .filter(exchange_tokens::Column::Exchange.eq(&exchange))
                .one(state.db.as_ref())
                .await?;
            
            let exchange_name = match exchange.as_str() {
                "binance" => "🔵 Binance",
                "okx" => "🟢 OKX",
                _ => &exchange,
            };
            
            let msg_text = if existing_token.is_some() {
                i18n::translate(locale, "live_trading_update_token", Some(&[
                    ("exchange", exchange_name),
                ]))
            } else {
                i18n::translate(locale, "live_trading_enter_api_key", Some(&[("exchange", exchange_name)]))
            };
            
            // Edit the existing message instead of sending a new one to avoid duplicates
            if let Some(msg_ref) = q.message {
                bot.edit_message_text(msg_ref.chat().id, msg_ref.id(), msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            } else {
                bot.send_message(q.message.as_ref().unwrap().chat().id, msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForApiKey {
                exchange: exchange.clone(),
            })).await?;
        } else if data == "live_trading_show_strategies" {
            bot.answer_callback_query(q.id).await?;
            
            // Show strategy selection
            let strategies = state.strategy_service.get_user_strategies(telegram_id).await?;
            
            if strategies.is_empty() {
                let msg_text = i18n::translate(locale, "trading_no_strategies", None);
                bot.send_message(q.message.as_ref().unwrap().chat().id, msg_text).await?;
                return Ok(());
            }
            
            let mut buttons = Vec::new();
            for strategy in &strategies {
                let button_text = strategy.name.as_ref()
                    .map(|n| n.clone())
                    .unwrap_or_else(|| format!("Strategy #{}", strategy.id));
                buttons.push(vec![
                    InlineKeyboardButton::callback(
                        button_text,
                        format!("live_trading_{}", strategy.id)
                    )
                ]);
            }
            
            buttons.push(vec![
                InlineKeyboardButton::callback(
                    i18n::get_button_text(locale, "trading_cancel"),
                    "cancel_live_trading"
                )
            ]);
            
            let msg_text = i18n::translate(locale, "live_trading_select_strategy", None);
            if let Some(msg_ref) = q.message {
                bot.edit_message_text(msg_ref.chat().id, msg_ref.id(), msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
                    .await?;
            }
            
            dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForStrategy)).await?;
        } else if data.starts_with("live_trading_") {
            let strategy_id_str = data.strip_prefix("live_trading_").unwrap();
            if let Ok(strategy_id) = strategy_id_str.parse::<u64>() {
                bot.answer_callback_query(q.id).await?;
                
                // Get strategy from database
                if let Some(_strategy) = state.strategy_service.get_strategy_by_id(strategy_id).await? {
                    // Get user's exchange tokens
                    let tokens = exchange_tokens::Entity::find()
                        .filter(exchange_tokens::Column::UserId.eq(telegram_id))
                        .filter(exchange_tokens::Column::IsActive.eq(1))
                        .all(state.db.as_ref())
                        .await?;
                    
                    if tokens.is_empty() {
                        let error_msg = i18n::translate(locale, "live_trading_no_tokens", None);
                        bot.send_message(q.message.as_ref().unwrap().chat().id, error_msg).await?;
                        return Ok(());
                    }
                    
                    // Show exchange selection
                    let mut buttons = Vec::new();
                    for token in &tokens {
                        let exchange_name = match token.exchange.as_str() {
                            "binance" => "🔵 Binance",
                            "okx" => "🟢 OKX",
                            _ => &token.exchange,
                        };
                        buttons.push(vec![
                            InlineKeyboardButton::callback(
                                exchange_name.to_string(),
                                format!("live_trading_exchange_{}_{}", token.exchange, strategy_id)
                            )
                        ]);
                    }
                    buttons.push(vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "trading_cancel"),
                            "cancel_live_trading"
                        )
                    ]);
                    
                    let msg_text = i18n::translate(locale, "live_trading_select_exchange", None);
                    if let Some(msg_ref) = q.message {
                        bot.edit_message_text(msg_ref.chat().id, msg_ref.id(), msg_text)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
                            .await?;
                    }
                    
                    dialogue.update(BotState::LiveTrading(LiveTradingState::WaitingForExchange {
                        strategy_id,
                    })).await?;
                }
            }
        } else if data.starts_with("live_trading_exchange_") {
            bot.answer_callback_query(q.id).await?;
            let parts: Vec<&str> = data.strip_prefix("live_trading_exchange_").unwrap().split('_').collect();
            if parts.len() >= 2 {
                let exchange = parts[0];
                let strategy_id = parts[1].parse::<u64>().ok();
                
                if let Some(strategy_id) = strategy_id {
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
                                    match start_live_trading_with_exchange(
                                        &state,
                                        telegram_id,
                                        &token,
                                        config,
                                    ).await {
                                        Ok(_) => {
                                            let success_msg = i18n::translate(locale, "live_trading_started", Some(&[
                                                ("exchange", exchange),
                                                ("strategy", &_strategy.name.as_ref().unwrap_or(&format!("Strategy #{}", strategy_id))),
                                            ]));
                                            bot.send_message(q.message.as_ref().unwrap().chat().id, success_msg)
                                                .parse_mode(teloxide::types::ParseMode::Html)
                                                .await?;
                                        }
                                        Err(e) => {
                                            let error_msg = i18n::translate(locale, "live_trading_start_error", Some(&[
                                                ("error", &e.to_string()),
                                            ]));
                                            bot.send_message(q.message.as_ref().unwrap().chat().id, error_msg)
                                                .parse_mode(teloxide::types::ParseMode::Html)
                                                .await?;
                                        }
                                    }
                                }
                                Err(e) => {
                                    let error_msg = i18n::translate(locale, "trading_config_error", Some(&[
                                        ("error", &e.to_string()),
                                    ]));
                                    bot.send_message(q.message.as_ref().unwrap().chat().id, error_msg).await?;
                                }
                            }
                        }
                    }
                }
            }
        } else if data == "cancel_live_trading" {
            bot.answer_callback_query(q.id).await?;
            dialogue.exit().await?;
        }
    }
    
    Ok(())
}

/// Handler for text input during live trading setup
pub async fn handle_live_trading_input(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let telegram_id = msg.from.as_ref().unwrap().id.0 as i64;
    
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
        if let Some(text) = msg.text() {
            // Store API key temporarily and ask for API secret
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
        if let Some(text) = msg.text() {
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
    state: &AppState,
    user_id: i64,
    token: &exchange_tokens::Model,
    strategy_config: crate::services::strategy_engine::StrategyConfig,
) -> Result<()> {
    // TODO: Integrate with Binance/OKX API to execute trades
    // For now, just log and start the strategy executor
    tracing::info!(
        "Starting live trading for user {} on {} with strategy {}",
        user_id,
        token.exchange,
        strategy_config.strategy_type
    );
    
    // Start strategy executor (will be enhanced to use exchange API)
    state.strategy_executor.start_trading(user_id, strategy_config).await?;
    
    Ok(())
}

