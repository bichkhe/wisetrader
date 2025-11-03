use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use sea_orm::{EntityTrait, ActiveValue, ColumnTrait, QueryFilter};
use shared::entity::users;
use chrono::Utc;

use crate::state::{AppState, MyDialogue};
use crate::i18n;

/// Handler for /deposit command to add points
pub async fn handle_deposit(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    use crate::state::BotState;
    
    // Get user locale
    let telegram_id = msg.from.as_ref().map(|f| f.id.0 as i64).unwrap_or(0);
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");

    if user.is_none() {
        let error_msg = i18n::translate(locale, "payment_user_not_found", None);
        bot.send_message(msg.chat.id, error_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }

    // Show deposit options
    let deposit_msg = i18n::translate(locale, "payment_deposit_welcome", None);
    
    let deposit_buttons = vec![
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "payment_deposit_100"),
                "deposit_100"
            ),
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "payment_deposit_500"),
                "deposit_500"
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "payment_deposit_1000"),
                "deposit_1000"
            ),
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "payment_deposit_5000"),
                "deposit_5000"
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "payment_deposit_custom"),
                "deposit_custom"
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "payment_cancel"),
                "deposit_cancel"
            ),
        ],
    ];

    bot.send_message(msg.chat.id, deposit_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(deposit_buttons))
        .await?;

    Ok(())
}

/// Handler for /balance command to show current balance
pub async fn handle_balance(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let telegram_id = msg.from.as_ref().map(|f| f.id.0 as i64).unwrap_or(0);
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");

    if let Some(ref user_model) = user {
        let points = user_model.points;
        let points_unit = i18n::translate(locale, "profile_points_unit", None);
        
        let balance_msg = i18n::translate(locale, "payment_balance_info", Some(&[
            ("points", &points.to_string()),
            ("points_unit", &points_unit),
        ]));

        let buttons = vec![
            vec![
                InlineKeyboardButton::callback(
                    i18n::get_button_text(locale, "payment_deposit_button"),
                    "deposit_start"
                ),
            ],
        ];

        bot.send_message(msg.chat.id, balance_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
            .await?;
    } else {
        let error_msg = i18n::translate(locale, "payment_user_not_found", None);
        bot.send_message(msg.chat.id, error_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }

    Ok(())
}

/// Handler for deposit callback queries
pub async fn handle_deposit_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    use crate::state::BotState;
    
    if let Some(data) = q.data {
        let user_id = q.from.id.0 as i64;
        
        // Get user locale
        let user = users::Entity::find_by_id(user_id)
            .one(state.db.as_ref())
            .await?;
        let locale = user
            .as_ref()
            .and_then(|u| u.language.as_ref())
            .map(|l| i18n::get_user_language(Some(l)))
            .unwrap_or("en");

        match data.as_str() {
            "deposit_cancel" => {
                bot.answer_callback_query(q.id).await?;
                let cancel_msg = i18n::translate(locale, "payment_deposit_cancelled", None);
                if let Some(msg) = q.message {
                    bot.edit_message_text(msg.chat().id, msg.id(), cancel_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
            }
            "deposit_custom" => {
                bot.answer_callback_query(q.id).await?;
                let custom_msg = i18n::translate(locale, "payment_deposit_custom_prompt", None);
                if let Some(msg) = q.message {
                    bot.edit_message_text(msg.chat().id, msg.id(), custom_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
                // Could set state to WaitingForAmount here if needed
            }
            "deposit_start" => {
                // Redirect to deposit wizard
                bot.answer_callback_query(q.id).await?;
                let deposit_msg = i18n::translate(locale, "payment_deposit_welcome", None);
                
                let deposit_buttons = vec![
                    vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "payment_deposit_100"),
                            "deposit_100"
                        ),
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "payment_deposit_500"),
                            "deposit_500"
                        ),
                    ],
                    vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "payment_deposit_1000"),
                            "deposit_1000"
                        ),
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "payment_deposit_5000"),
                            "deposit_5000"
                        ),
                    ],
                    vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "payment_deposit_custom"),
                            "deposit_custom"
                        ),
                    ],
                    vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "payment_cancel"),
                            "deposit_cancel"
                        ),
                    ],
                ];

                if let Some(msg) = q.message {
                    bot.edit_message_text(msg.chat().id, msg.id(), deposit_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(deposit_buttons))
                        .await?;
                }
            }
            amount if amount.starts_with("deposit_") => {
                // Extract amount from callback data
                let amount_str = amount.replace("deposit_", "");
                
                if let Ok(points_to_add) = amount_str.parse::<u64>() {
                    // Update user points
                    if let Some(ref user_model) = user {
                        let current_points = user_model.points;
                        let new_points = current_points + points_to_add;
                        
                        // Create ActiveModel from the user
                        let mut user_active: users::ActiveModel = user_model.clone().into();
                        user_active.points = ActiveValue::Set(new_points);
                        
                        match users::Entity::update(user_active).exec(state.db.as_ref()).await {
                            Ok(_) => {
                                let success_msg = i18n::translate(locale, "payment_deposit_success", Some(&[
                                    ("amount", &points_to_add.to_string()),
                                    ("total", &new_points.to_string()),
                                ]));
                                
                                bot.answer_callback_query(q.id)
                                    .text(&i18n::translate(locale, "payment_deposit_success_short", Some(&[
                                        ("amount", &points_to_add.to_string()),
                                    ])))
                                    .show_alert(true)
                                    .await?;
                                
                                if let Some(msg) = q.message {
                                    bot.edit_message_text(msg.chat().id, msg.id(), success_msg)
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .await?;
                                }
                            }
                            Err(e) => {
                                let error_msg = i18n::translate(locale, "payment_deposit_error", None);
                                bot.answer_callback_query(q.id)
                                    .text(&format!("{}: {}", error_msg, e))
                                    .show_alert(true)
                                    .await?;
                            }
                        }
                    }
                } else {
                    let error_msg = i18n::translate(locale, "error_invalid_amount", None);
                    bot.answer_callback_query(q.id)
                        .text(&error_msg)
                        .await?;
                }
            }
            _ => {
                bot.answer_callback_query(q.id).await?;
            }
        }
    }

    Ok(())
}

