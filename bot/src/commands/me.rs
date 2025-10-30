use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;
use sea_orm::EntityTrait;
use shared::entity::users;

use crate::state::{AppState, MyDialogue, BotState};
use crate::i18n;
use teloxide::types::InlineKeyboardButton;

/// Handler for the /me command to show user profile information
pub async fn handle_me(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let start_time = Instant::now();
    
    let from = msg.from.unwrap();
    let fullname = from.full_name();
    let telegram_id = from.id.0 as i64;
    let username = from.username.unwrap_or("Không có".to_string());

    tracing::info!(
        "Handling /me command for user: {} (id: {}, username: {})",
        fullname,
        telegram_id,
        username
    );

    // Get user from database
    let existing_user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    
    // Get user language
    let locale = existing_user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");

    if existing_user.is_none() {
        // User not found - use translation
        let error_msg = i18n::translate(
            locale,
            "profile_not_registered",
            Some(&[("user_id", &telegram_id.to_string())])
        );
        bot.send_message(msg.chat.id, error_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }

    if let Some(ref user) = existing_user {
        let status = if user.live_trading_enabled.unwrap_or(0) == 1 {
            i18n::translate(locale, "profile_status_active", None)
        } else {
            i18n::translate(locale, "profile_status_inactive", None)
        };

        let points_unit = i18n::translate(locale, "profile_points_unit", None);
        
        // Get current language display name
        let current_lang_display = match user.language.as_deref() {
            Some("vi") => "🇻🇳 Tiếng Việt",
            Some("en") => "🇬🇧 English",
            _ => "Not set",
        };
        
        let info = format!(
            "{}\n\n\
        {} <b>{}</b>\n\
        {} @{}\n\
        {} <b>{}</b>\n\
        {} <b>{}</b>\n\
        {} <b>{} {}</b>\n\
        {} <b>{}</b>\n\
        {} <b>{}</b>\n\
        {} <b>{}</b>\n\
        {} <b>{}</b>\n\n\
        {}",
            i18n::translate(locale, "profile_title", None),
            i18n::translate(locale, "profile_name", None),
            user.fullname.as_ref().unwrap_or(&fullname),
            i18n::translate(locale, "profile_username", None),
            username,
            i18n::translate(locale, "profile_userid", None),
            telegram_id,
            i18n::translate(locale, "profile_language_current", None),
            current_lang_display,
            i18n::translate(locale, "profile_points", None),
            user.points,
            points_unit,
            i18n::translate(locale, "profile_subscription", None),
            user.subscription_tier.as_ref().unwrap_or(&"N/A".to_string()),
            i18n::translate(locale, "profile_created", None),
            user.created_at.as_ref().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "N/A".to_string()),
            i18n::translate(locale, "profile_expires", None),
            user.subscription_expires.as_ref().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "N/A".to_string()),
            i18n::translate(locale, "profile_status_label", None),
            status,
            i18n::translate(locale, "profile_tip", None)
        );
        
        // Add button to change language
        // Get translated text for button label
        let change_lang_text = match locale {
            "vi" => "🌐 Đổi Ngôn Ngữ",
            "en" => "🌐 Change Language",
            _ => "🌐 Change Language",
        };
        
        let change_lang_button = InlineKeyboardButton::callback(
            change_lang_text.to_string(),
            "profile_change_language"
        );
        let buttons = vec![vec![change_lang_button]];
        
        bot.send_message(msg.chat.id, info)
            .parse_mode(teloxide::types::ParseMode::Html)
            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
            .await?;
    }

    Ok(())
}

/// Handler for profile callback queries (like change language button)
pub async fn handle_profile_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    if let Some(data) = q.data {
        match data.as_str() {
            "profile_change_language" => {
                // Show language selection buttons
                let user_id = q.from.id.0 as i64;
                let user = shared::entity::users::Entity::find_by_id(user_id)
                    .one(state.db.as_ref())
                    .await?;
                let locale = user
                    .as_ref()
                    .and_then(|u| u.language.as_ref())
                    .map(|l| i18n::get_user_language(Some(l)))
                    .unwrap_or("en");
                
                let lang_buttons = vec![
                    vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "lang_selection_button_vi"),
                            "lang_select_vi"
                        ),
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "lang_selection_button_en"),
                            "lang_select_en"
                        ),
                    ],
                ];
                
                let selection_msg = i18n::translate(locale, "lang_selection_title", None);
                
                bot.answer_callback_query(q.id).await?;
                
                if let Some(msg) = q.message {
                    bot.edit_message_text(msg.chat().id, msg.id(), selection_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(lang_buttons))
                        .await?;
                } else {
                    // If no message (shouldn't happen), send new message
                    bot.send_message(q.from.id, selection_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(lang_buttons))
                        .await?;
                }
                
                // Set dialogue state to waiting for language
                dialogue.update(BotState::WaitingForLanguage).await?;
            }
            _ => {
                bot.answer_callback_query(q.id).await?;
            }
        }
    }
    
    Ok(())
}
