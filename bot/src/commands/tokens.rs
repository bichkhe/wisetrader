//! Tokens Command - manage OAuth tokens for exchanges

use std::sync::Arc;
use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter, ActiveValue};
use crate::state::AppState;
use crate::i18n;
use shared::entity::{users, exchange_tokens};
use chrono::Utc;

/// Handler for /tokens command
pub async fn handle_tokens(
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
    
    // Get user's tokens
    let tokens = exchange_tokens::Entity::find()
        .filter(exchange_tokens::Column::UserId.eq(telegram_id))
        .all(state.db.as_ref())
        .await?;
    
    if tokens.is_empty() {
        let msg_text = i18n::translate(locale, "tokens_no_tokens", None);
        bot.send_message(msg.chat.id, msg_text).await?;
        return Ok(());
    }
    
    // Show tokens list
    let mut buttons = Vec::new();
    for token in &tokens {
        let exchange_name = match token.exchange.as_str() {
            "binance" => "🔵 Binance",
            "okx" => "🟢 OKX",
            _ => &token.exchange,
        };
        let status = if token.is_active == 1 { "✅" } else { "❌" };
        let button_text = format!("{} {} {}", status, exchange_name, 
            if token.is_active == 1 { "(Active)" } else { "(Inactive)" });
        
        buttons.push(vec![
            InlineKeyboardButton::callback(
                button_text,
                format!("tokens_show_{}", token.id)
            )
        ]);
    }
    
    buttons.push(vec![
        InlineKeyboardButton::callback(
            i18n::get_button_text(locale, "tokens_revoke_all"),
            "tokens_revoke_all"
        )
    ]);
    
    let msg_text = i18n::translate(locale, "tokens_list", Some(&[("count", &tokens.len().to_string())]));
    bot.send_message(msg.chat.id, msg_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
        .await?;
    
    Ok(())
}

/// Handler for tokens callbacks
pub async fn handle_tokens_callback(
    bot: Bot,
    q: CallbackQuery,
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
        
        if data.starts_with("tokens_show_") {
            bot.answer_callback_query(q.id).await?;
            let token_id_str = data.strip_prefix("tokens_show_").unwrap();
            if let Ok(token_id) = token_id_str.parse::<u64>() {
                if let Some(token) = exchange_tokens::Entity::find_by_id(token_id)
                    .one(state.db.as_ref())
                    .await? {
                    
                    // Verify ownership
                    if token.user_id != telegram_id {
                        let error_msg = i18n::translate(locale, "tokens_unauthorized", None);
                        bot.send_message(q.message.as_ref().unwrap().chat().id, error_msg).await?;
                        return Ok(());
                    }
                    
                    // Show token details (masked)
                    let masked_key = mask_string(&token.api_key, 4);
                    let masked_secret = mask_string(&token.api_secret, 4);
                    
                    let exchange_name = match token.exchange.as_str() {
                        "binance" => "🔵 Binance",
                        "okx" => "🟢 OKX",
                        _ => &token.exchange,
                    };
                    
                    let status = if token.is_active == 1 { "✅ Active" } else { "❌ Inactive" };
                    
                    let msg_text = format!(
                        "📋 <b>Token Details</b>\n\n\
                        <b>Exchange:</b> {}\n\
                        <b>Status:</b> {}\n\
                        <b>API Key:</b> <code>{}</code>\n\
                        <b>API Secret:</b> <code>{}</code>\n\
                        <b>Created:</b> {}\n\
                        <b>Updated:</b> {}",
                        exchange_name,
                        status,
                        masked_key,
                        masked_secret,
                        token.created_at.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|| "N/A".to_string()),
                        token.updated_at.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|| "N/A".to_string())
                    );
                    
                    let buttons = vec![
                        vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(locale, "tokens_revoke"),
                                format!("revoke_token_{}", token.id)
                            )
                        ],
                        vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(locale, "tokens_back"),
                                "tokens_back"
                            )
                        ]
                    ];
                    
                    if let Some(msg_ref) = q.message {
                        bot.edit_message_text(msg_ref.chat().id, msg_ref.id(), msg_text)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
                            .await?;
                    }
                }
            }
        } else if data.starts_with("revoke_token_") {
            bot.answer_callback_query(q.id).await?;
            let token_id_str = data.strip_prefix("revoke_token_").unwrap();
            if let Ok(token_id) = token_id_str.parse::<u64>() {
                if let Some(token) = exchange_tokens::Entity::find_by_id(token_id)
                    .one(state.db.as_ref())
                    .await? {
                    
                    // Verify ownership
                    if token.user_id != telegram_id {
                        let error_msg = i18n::translate(locale, "tokens_unauthorized", None);
                        bot.send_message(q.message.as_ref().unwrap().chat().id, error_msg).await?;
                        return Ok(());
                    }
                    
                    // Deactivate token
                    let exchange_name = token.exchange.clone();
                    let mut token: exchange_tokens::ActiveModel = token.into();
                    token.is_active = ActiveValue::Set(0);
                    token.updated_at = ActiveValue::Set(Some(Utc::now()));
                    
                    exchange_tokens::Entity::update(token).exec(state.db.as_ref()).await?;
                    
                    let success_msg = i18n::translate(locale, "tokens_revoked", Some(&[("exchange", &exchange_name)]));
                    bot.send_message(q.message.as_ref().unwrap().chat().id, success_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
            }
        } else if data == "tokens_revoke_all" {
            bot.answer_callback_query(q.id).await?;
            
            // Deactivate all tokens for this user
            let tokens = exchange_tokens::Entity::find()
                .filter(exchange_tokens::Column::UserId.eq(telegram_id))
                .filter(exchange_tokens::Column::IsActive.eq(1))
                .all(state.db.as_ref())
                .await?;
            
            for token in tokens {
                let mut token: exchange_tokens::ActiveModel = token.into();
                token.is_active = ActiveValue::Set(0);
                token.updated_at = ActiveValue::Set(Some(Utc::now()));
                exchange_tokens::Entity::update(token).exec(state.db.as_ref()).await?;
            }
            
            let success_msg = i18n::translate(locale, "tokens_revoked_all", None);
            bot.send_message(q.message.as_ref().unwrap().chat().id, success_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        } else if data == "tokens_back" {
            bot.answer_callback_query(q.id).await?;
            // Re-show tokens list by editing the message
            if let Some(msg_ref) = q.message {
                let chat_id = msg_ref.chat().id;
                // Get tokens again
                let tokens = exchange_tokens::Entity::find()
                    .filter(exchange_tokens::Column::UserId.eq(telegram_id))
                    .all(state.db.as_ref())
                    .await?;
                
                if tokens.is_empty() {
                    let msg_text = i18n::translate(locale, "tokens_no_tokens", None);
                    bot.edit_message_text(chat_id, msg_ref.id(), msg_text).await?;
                    return Ok(());
                }
                
                let mut buttons = Vec::new();
                for token in &tokens {
                    let exchange_name = match token.exchange.as_str() {
                        "binance" => "🔵 Binance",
                        "okx" => "🟢 OKX",
                        _ => &token.exchange,
                    };
                    let status = if token.is_active == 1 { "✅" } else { "❌" };
                    let button_text = format!("{} {} {}", status, exchange_name, 
                        if token.is_active == 1 { "(Active)" } else { "(Inactive)" });
                    
                    buttons.push(vec![
                        InlineKeyboardButton::callback(
                            button_text,
                            format!("tokens_show_{}", token.id)
                        )
                    ]);
                }
                
                buttons.push(vec![
                    InlineKeyboardButton::callback(
                        i18n::get_button_text(locale, "tokens_revoke_all"),
                        "tokens_revoke_all"
                    )
                ]);
                
                let msg_text = i18n::translate(locale, "tokens_list", Some(&[("count", &tokens.len().to_string())]));
                bot.edit_message_text(chat_id, msg_ref.id(), msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
                    .await?;
            }
        }
    }
    
    Ok(())
}

/// Mask a string, showing only first and last N characters
fn mask_string(s: &str, visible_chars: usize) -> String {
    if s.len() <= visible_chars * 2 {
        return "*".repeat(s.len());
    }
    format!("{}...{}", &s[..visible_chars], &s[s.len() - visible_chars..])
}

