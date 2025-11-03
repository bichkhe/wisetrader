use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use sea_orm::{EntityTrait, ActiveValue::Set};
use shared::entity::{users, strategies};
use chrono::{Utc, Duration};
use tracing::info;
use crate::state::{AppState, MyDialogue, BotState};
use crate::i18n;

/// Handler for /start command with language selection
pub async fn handle_start(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let user_id = msg.from.as_ref().unwrap().id.0 as i64;
    let username = msg.from.as_ref().unwrap().username.clone();
    let db = state.db.clone();
    
    info!("Processing /start command from user {}", user_id);

    // Check if user already exists
    let existing_user = users::Entity::find_by_id(user_id)
        .one(db.as_ref())
        .await?;

    // If user exists and has language set, show welcome back message
    if let Some(ref user) = existing_user {
        if let Some(ref lang) = user.language {
            let locale = i18n::get_user_language(Some(lang));
            let welcome_back = i18n::translate(locale, "welcome_back", None);
            
            bot.send_message(msg.chat.id, welcome_back)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            dialogue.exit().await?;
            return Ok(());
        }
        // User exists but no language - show language selection
    } else {
        // New user - create user record without language initially
        let new_user = users::ActiveModel {
            id: Set(user_id),
            username: Set(username.clone()),
            language: Set(None), // No language set yet
            subscription_tier: Set(Some("free_trial".to_string())),
            subscription_expires: Set(Some(Utc::now() + Duration::days(7))),
            live_trading_enabled: Set(Some(0)),
            created_at: Set(Some(Utc::now())),
            telegram_id: Set(Some(user_id.to_string())),
            fullname: Set(username.clone().unwrap_or_else(|| "".to_string()).into()),
            points: Set(0u64),
        };

        state.user_service.create_user(new_user).await?;
    }

    // Show language selection buttons (using English for initial message)
    let lang_buttons = vec![
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text("en", "lang_selection_button_vi"),
                "lang_select_vi"
            ),
            InlineKeyboardButton::callback(
                i18n::get_button_text("en", "lang_selection_button_en"),
                "lang_select_en"
            ),
        ],
    ];

    let selection_msg = i18n::translate("en", "lang_selection_title", None);
    
    bot.send_message(msg.chat.id, selection_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(lang_buttons))
        .await?;
    
    // Set dialogue state to waiting for language
    dialogue.update(BotState::WaitingForLanguage).await?;

    Ok(())
}

/// Handler for language selection callback (from WaitingForLanguage state)
pub async fn handle_language_selection(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    handle_language_callback_internal(bot, dialogue, q, state, true).await
}

/// Handler for language callback (can be called from any state)
pub async fn handle_language_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    handle_language_callback_internal(bot, dialogue, q, state, false).await
}

/// Internal handler for language selection callbacks
async fn handle_language_callback_internal(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
    is_new_user: bool,
) -> Result<(), anyhow::Error> {
    if let Some(data) = q.data {
        let user_id = q.from.id.0 as i64;
        
        // Extract language from callback data
        let (lang, lang_code) = if data == "lang_select_vi" {
            ("vi", "vi")
        } else if data == "lang_select_en" {
            ("en", "en")
        } else {
            // Get user locale for error message
            let telegram_id = q.from.id.0 as i64;
            let user = users::Entity::find_by_id(telegram_id)
                .one(state.db.as_ref())
                .await
                .ok()
                .flatten();
            let locale = user
                .as_ref()
                .and_then(|u| u.language.as_ref())
                .map(|l| i18n::get_user_language(Some(l)))
                .unwrap_or("en");
            
            let error_msg = i18n::translate(locale, "error_invalid_selection", None);
            bot.answer_callback_query(q.id)
                .text(&error_msg)
                .await?;
            return Ok(());
        };

        // Update user language in database
        let db = state.db.clone();
        let user = users::Entity::find_by_id(user_id)
            .one(db.as_ref())
            .await?;

        if let Some(user) = user {
            let mut user: users::ActiveModel = user.into();
            user.language = Set(Some(lang_code.to_string()));
            
            use sea_orm::EntityTrait;
            users::Entity::update(user)
                .exec(db.as_ref())
                .await?;
        }

        // Answer callback query
        let confirm_msg = i18n::translate(lang, "lang_selected", None);
        bot.answer_callback_query(q.id)
            .text(&confirm_msg)
            .show_alert(true)
            .await?;

        // Edit the message to remove buttons
        if let Some(msg) = q.message {
            let chat_id = msg.chat().id;
            let message_id = msg.id();
            
            let edit_msg = i18n::translate(lang, "lang_selected", None);
            bot.edit_message_text(chat_id, message_id, edit_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            
            // Show welcome message for new users, or confirmation for existing users changing language
            if is_new_user {
                // Create default RSI strategy for new user
                create_default_rsi_strategy(user_id, lang, &state).await?;
                
                let welcome_msg = i18n::translate(lang, "welcome_new_user", None);
                bot.send_message(chat_id, welcome_msg)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            } else {
                // User changed language from profile
                let updated_msg = format!("✅ {}\n\n{}", 
                    i18n::translate(lang, "lang_updated_success", None),
                    i18n::translate(lang, "lang_updated_notice", None));
                bot.send_message(chat_id, updated_msg)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
        }

        // Exit to Normal state so user can use commands
        dialogue.update(BotState::Normal).await?;
    }

    Ok(())
}

/// Create a default RSI strategy for new users
async fn create_default_rsi_strategy(
    user_id: i64,
    locale: &str,
    state: &Arc<AppState>,
) -> Result<(), anyhow::Error> {
    use sea_orm::ActiveValue;
    
    // Get localized strategy name
    let strategy_name = match locale {
        "vi" => "Chiến Lược RSI Mặc Định",
        _ => "Default RSI Strategy",
    };
    
    // Create strategy description
    let description = format!(
        "Algorithm: RSI\nBuy: RSI < 30\nSell: RSI > 70\nTimeframe: 1h\nPair: BTCUSDT"
    );
    
    // Create strategy record (id is auto-generated by database)
    let new_strategy = strategies::ActiveModel {
        name: ActiveValue::Set(Some(strategy_name.to_string())),
        description: ActiveValue::Set(Some(description)),
        repo_ref: ActiveValue::Set(None),
        created_at: ActiveValue::Set(Some(Utc::now())),
        telegram_id: ActiveValue::Set(user_id.to_string()),
        ..Default::default()
    };
    
    strategies::Entity::insert(new_strategy)
        .exec(state.db.as_ref())
        .await?;
    
    info!("Created default RSI strategy for new user {}", user_id);
    
    Ok(())
}

