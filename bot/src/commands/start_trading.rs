//! Start Trading Command - allows users to select and start a strategy

use std::sync::Arc;
use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use crate::state::{AppState, MyDialogue};
use crate::services::strategy_service::StrategyService;
use crate::i18n;
use shared::entity::users;
use sea_orm::EntityTrait;

pub async fn handle_start_trading(
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
    let strategies = state.strategy_service.get_user_strategies(telegram_id).await?;
    
    if strategies.is_empty() {
        let msg_text = i18n::translate(locale, "trading_no_strategies", None);
        bot.send_message(msg.chat.id, msg_text).await?;
        return Ok(());
    }
    
    // Create inline buttons for strategies
    let mut buttons = Vec::new();
    for strategy in &strategies {
        let button_text = strategy.name.as_ref()
            .map(|n| n.clone())
            .unwrap_or_else(|| format!("Strategy #{}", strategy.id));
        buttons.push(vec![
            InlineKeyboardButton::callback(
                button_text,
                format!("start_trading_{}", strategy.id)
            )
        ]);
    }
    
    buttons.push(vec![
        InlineKeyboardButton::callback(
            i18n::get_button_text(locale, "trading_cancel"),
            "cancel_start_trading"
        )
    ]);
    
    let msg_text = i18n::translate(locale, "trading_select_strategy", None);
    bot.send_message(msg.chat.id, msg_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
        .await?;
    
    Ok(())
}

pub async fn handle_start_trading_callback(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    if let Some(data) = q.data {
        if data.starts_with("start_trading_") {
            let strategy_id_str = data.strip_prefix("start_trading_").unwrap();
            if let Ok(strategy_id) = strategy_id_str.parse::<u64>() {
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
                
                // Get strategy from database
                if let Some(strategy) = state.strategy_service.get_strategy_by_id(strategy_id).await? {
                    // Convert to StrategyConfig
                    match state.strategy_service.strategy_to_config(&strategy) {
                        Ok(config) => {
                            // Start trading
                            if let Err(e) = state.strategy_executor.start_trading(telegram_id, config).await {
                                let error_msg = i18n::translate(locale, "trading_start_error", Some(&[
                                    ("error", &e.to_string())
                                ]));
                                bot.answer_callback_query(q.id)
                                    .text(&error_msg)
                                    .await?;
                            } else {
                                let success_msg = i18n::translate(locale, "trading_started", None);
                                bot.answer_callback_query(q.id)
                                    .text(&success_msg)
                                    .await?;
                                
                                // Send confirmation message
                                let strategy_name_str = strategy.name.as_ref()
                                    .map(|n| n.clone())
                                    .unwrap_or_else(|| format!("Strategy #{}", strategy.id));
                                // translate() will automatically escape the argument
                                let msg = i18n::translate(locale, "trading_started_message", Some(&[
                                    ("strategy_name", &strategy_name_str)
                                ]));
                                if let Some(msg_ref) = q.message {
                                    let chat_id = msg_ref.chat().id;
                                    bot.send_message(chat_id, msg)
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .await?;
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = i18n::translate(locale, "trading_config_error", Some(&[
                                ("error", &e.to_string())
                            ]));
                            bot.answer_callback_query(q.id)
                                .text(&error_msg)
                                .await?;
                        }
                    }
                } else {
                    let error_msg = i18n::translate(locale, "trading_strategy_not_found", None);
                    bot.answer_callback_query(q.id)
                        .text(&error_msg)
                        .await?;
                }
            }
        } else if data == "cancel_start_trading" {
            bot.answer_callback_query(q.id).await?;
        }
    }
    
    Ok(())
}

