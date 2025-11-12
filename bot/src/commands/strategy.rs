use std::sync::Arc;
use teloxide::{prelude::*, types::InlineKeyboardButton};
use sea_orm::{EntityTrait, ActiveValue};
use shared::entity::{users, strategies};
use tracing;
use sea_orm::{QueryFilter, QueryOrder};
use crate::state::{AppState, BotState, CreateStrategyState, MyDialogue};
use crate::i18n;
use crate::services::preset_strategies;

// Helper function to HTML escape (must escape & first!)
fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
}

// Helper function to strip HTML tags for plain text display (e.g., in callback query alerts)
fn strip_html_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    
    // Clean up common HTML entities
    result.replace("&lt;", "<")
          .replace("&gt;", ">")
          .replace("&amp;", "&")
          .replace("&quot;", "\"")
          .replace("&#x27;", "'")
          .replace("&nbsp;", " ")
          .trim()
          .to_string()
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
                "strategy_type_custom" => {
                    bot.answer_callback_query(q.id).await?;
                    // Show algorithm selection buttons
                    let algorithm_buttons = vec![
                        vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "algorithm_rsi"),
                                "algorithm_rsi"
                            ),
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "algorithm_bollinger"),
                                "algorithm_bollinger"
                            ),
                        ],
                        vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "algorithm_ema"),
                                "algorithm_ema"
                            ),
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "algorithm_macd"),
                                "algorithm_macd"
                            ),
                        ],
                        vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "algorithm_ma"),
                                "algorithm_ma"
                            ),
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "algorithm_stochastic"),
                                "algorithm_stochastic"
                            ),
                        ],
                        vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "algorithm_adx"),
                                "algorithm_adx"
                            ),
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "strategy_cancel_button"),
                                "cancel_strategy"
                            ),
                        ],
                    ];
                    let custom_msg = i18n::translate(&locale, "strategy_choose_algorithm", None);
                    bot.edit_message_text(chat_id, message_id, custom_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(algorithm_buttons))
                        .await?;
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForAlgorithm)).await?;
                }
                "strategy_type_preset" => {
                    bot.answer_callback_query(q.id).await?;
                    // Show preset strategy selection buttons
                    let preset_list = preset_strategies::get_preset_strategy_list();
                    let mut preset_buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
                    
                    // Create buttons in pairs (2 per row)
                    for chunk in preset_list.chunks(2) {
                        let row: Vec<InlineKeyboardButton> = chunk
                            .iter()
                            .map(|(name, display)| {
                                InlineKeyboardButton::callback(
                                    display.to_string(),
                                    format!("preset_{}", name)
                                )
                            })
                            .collect();
                        preset_buttons.push(row);
                    }
                    
                    // Add cancel button
                    preset_buttons.push(vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(&locale, "strategy_cancel_button"),
                            "cancel_strategy"
                        )
                    ]);
                    
                    let preset_msg = i18n::translate(&locale, "strategy_choose_preset", None);
                    bot.edit_message_text(chat_id, message_id, preset_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(preset_buttons))
                        .await?;
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForPresetSelection)).await?;
                }
                "strategy_type_custom_mix" => {
                    bot.answer_callback_query(q.id).await?;
                    
                    // Get user's strategies
                    let telegram_id = q.from.id.0.to_string();
                    use sea_orm::ColumnTrait;
                    let user_strategies = strategies::Entity::find()
                        .filter(strategies::Column::TelegramId.eq(telegram_id.clone()))
                        .order_by_desc(strategies::Column::CreatedAt)
                        .all(state.db.as_ref())
                        .await?;
                    
                    if user_strategies.is_empty() {
                        let error_msg = i18n::translate(&locale, "strategy_mix_no_strategies", None);
                        bot.edit_message_text(chat_id, message_id, error_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                        dialogue.exit().await?;
                        return Ok(());
                    }
                    
                    // Create buttons for each strategy (with toggle functionality)
                    let mut strategy_buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
                    
                    // Create buttons in pairs (2 per row)
                    for chunk in user_strategies.chunks(2) {
                        let row: Vec<InlineKeyboardButton> = chunk
                            .iter()
                            .map(|strategy| {
                                let name = strategy.name.as_ref()
                                    .map(|n| n.as_str())
                                    .unwrap_or("Unnamed");
                                let display = if name.len() > 20 {
                                    format!("{}...", &name[..20])
                                } else {
                                    name.to_string()
                                };
                                InlineKeyboardButton::callback(
                                    format!("☐ {}", display),
                                    format!("mix_select_{}", strategy.id)
                                )
                            })
                            .collect();
                        strategy_buttons.push(row);
                    }
                    
                    // Add "Done" and "Cancel" buttons
                    strategy_buttons.push(vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(&locale, "strategy_mix_done"),
                            "mix_done"
                        ),
                    ]);
                    strategy_buttons.push(vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(&locale, "strategy_cancel_button"),
                            "cancel_strategy"
                        ),
                    ]);
                    
                    let mix_msg = i18n::translate(&locale, "strategy_choose_mix", None);
                    bot.edit_message_text(chat_id, message_id, mix_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(strategy_buttons))
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForMixStrategySelection {
                        selected_strategy_ids: Vec::new(),
                    })).await?;
                }
                _ if data.starts_with("preset_") => {
                    bot.answer_callback_query(q.id).await?;
                    let strategy_name = data.replace("preset_", "");
                    
                    // Show loading message
                    let loading_msg = i18n::translate(&locale, "strategy_loading_preset", Some(&[("name", &strategy_name)]));
                    bot.edit_message_text(chat_id, message_id, loading_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    // Fetch preset strategy from local filesystem
                    match preset_strategies::load_strategy_from_local(&strategy_name).await {
                        Ok(preset) => {
                            // Auto-fill strategy details and ask for strategy name
                            let algorithm = preset.indicators.first().unwrap_or(&"RSI".to_string()).clone();
                            let buy_condition = preset.buy_condition.clone();
                            let sell_condition = preset.sell_condition.clone();
                            let timeframe = preset.timeframe.unwrap_or_else(|| "1h".to_string());
                            
                            // Show summary and ask for strategy name
                            // Escape HTML in conditions before passing to translate
                            let buy_condition_escaped = escape_html(&buy_condition);
                            let sell_condition_escaped = escape_html(&sell_condition);
                            let summary_msg = i18n::translate(&locale, "strategy_preset_loaded", Some(&[
                                ("name", &escape_html(&preset.display_name)),
                                ("algorithm", &escape_html(&algorithm)),
                                ("buy_condition", &buy_condition_escaped),
                                ("sell_condition", &sell_condition_escaped),
                                ("timeframe", &escape_html(&timeframe)),
                            ]));
                            let name_msg = i18n::translate(&locale, "strategy_enter_name", None);
                            let full_msg = format!("{}\n\n{}", summary_msg, name_msg);
                            
                            bot.edit_message_text(chat_id, message_id, full_msg)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                            
                            dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForPresetName {
                                algorithm,
                                buy_condition,
                                sell_condition,
                                timeframe,
                            })).await?;
                        }
                        Err(e) => {
                            let error_msg = i18n::translate(&locale, "strategy_preset_load_error", Some(&[
                                ("name", &strategy_name),
                                ("error", &e.to_string()),
                            ]));
                            bot.edit_message_text(chat_id, message_id, error_msg)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                            dialogue.exit().await?;
                        }
                    }
                }
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
                "algorithm_stochastic" => {
                    bot.answer_callback_query(q.id).await?;
                    let algorithm_msg = i18n::translate(&locale, "strategy_algorithm_selected", Some(&[("algorithm", "Stochastic")]));
                    let info_msg = i18n::translate(&locale, "strategy_algorithm_stochastic_info", None);
                    let step2_msg = i18n::translate(&locale, "strategy_step2_enter_buy", Some(&[("example", "Stochastic < 20")]));
                    let instruction = format!("{}\n\n{}\n\n{}", algorithm_msg, info_msg, step2_msg);
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "Stochastic".to_string(),
                    })).await?;
                }
                "algorithm_adx" => {
                    bot.answer_callback_query(q.id).await?;
                    let algorithm_msg = i18n::translate(&locale, "strategy_algorithm_selected", Some(&[("algorithm", "ADX")]));
                    let info_msg = i18n::translate(&locale, "strategy_algorithm_adx_info", None);
                    let step2_msg = i18n::translate(&locale, "strategy_step2_enter_buy", Some(&[("example", "ADX > 25")]));
                    let instruction = format!("{}\n\n{}\n\n{}", algorithm_msg, info_msg, step2_msg);
                    
                    bot.edit_message_text(chat_id, message_id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
                        algorithm: "ADX".to_string(),
                    })).await?;
                }
                _ if data.starts_with("mix_select_") => {
                    let callback_query_id = q.id.clone();
                    bot.answer_callback_query(callback_query_id).await?;
                    
                    // Extract strategy ID from callback data
                    let strategy_id_str = data.replace("mix_select_", "");
                    let strategy_id: u64 = match strategy_id_str.parse() {
                        Ok(id) => id,
                        Err(_) => {
                            let error_msg = i18n::translate(&locale, "error_invalid_selection", None);
                            bot.answer_callback_query(q.id)
                                .text(&error_msg)
                                .await?;
                            return Ok(());
                        }
                    };
                    
                    // Get current state
                    if let Ok(Some(BotState::CreateStrategy(CreateStrategyState::WaitingForMixStrategySelection { selected_strategy_ids }))) = dialogue.get().await {
                        let mut new_selected = selected_strategy_ids.clone();
                        
                        // Toggle selection
                        if new_selected.contains(&strategy_id) {
                            new_selected.retain(|&id| id != strategy_id);
                        } else {
                            new_selected.push(strategy_id);
                        }
                        
                        // Get user's strategies to rebuild buttons
                        let telegram_id = q.from.id.0.to_string();
                        use sea_orm::ColumnTrait;
                        let user_strategies = strategies::Entity::find()
                            .filter(strategies::Column::TelegramId.eq(telegram_id.clone()))
                            .order_by_desc(strategies::Column::CreatedAt)
                            .all(state.db.as_ref())
                            .await?;
                        
                        // Rebuild buttons with updated selection
                        let mut strategy_buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();
                        
                        for chunk in user_strategies.chunks(2) {
                            let row: Vec<InlineKeyboardButton> = chunk
                                .iter()
                                .map(|strategy| {
                                    let name = strategy.name.as_ref()
                                        .map(|n| n.as_str())
                                        .unwrap_or("Unnamed");
                                    let display = if name.len() > 20 {
                                        format!("{}...", &name[..20])
                                    } else {
                                        name.to_string()
                                    };
                                    let is_selected = new_selected.contains(&strategy.id);
                                    let prefix = if is_selected { "☑" } else { "☐" };
                                    InlineKeyboardButton::callback(
                                        format!("{} {}", prefix, display),
                                        format!("mix_select_{}", strategy.id)
                                    )
                                })
                                .collect();
                            strategy_buttons.push(row);
                        }
                        
                        // Add "Done" and "Cancel" buttons
                        strategy_buttons.push(vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "strategy_mix_done"),
                                "mix_done"
                            ),
                        ]);
                        strategy_buttons.push(vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(&locale, "strategy_cancel_button"),
                                "cancel_strategy"
                            ),
                        ]);
                        
                        // Update message with new selection count
                        let count_msg = if new_selected.is_empty() {
                            i18n::translate(&locale, "strategy_choose_mix", None)
                        } else {
                            i18n::translate(&locale, "strategy_mix_selected", Some(&[("count", &new_selected.len().to_string())]))
                        };
                        
                        bot.edit_message_text(chat_id, message_id, count_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(strategy_buttons))
                            .await?;
                        
                        // Update state with new selection
                        dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForMixStrategySelection {
                            selected_strategy_ids: new_selected,
                        })).await?;
                    }
                }
                "mix_done" => {
                    let callback_query_id = q.id.clone();
                    bot.answer_callback_query(callback_query_id).await?;
                    
                    // Get current state
                    if let Ok(Some(BotState::CreateStrategy(CreateStrategyState::WaitingForMixStrategySelection { selected_strategy_ids }))) = dialogue.get().await {
                        if selected_strategy_ids.is_empty() {
                            let error_msg = i18n::translate(&locale, "strategy_mix_no_selection", None);
                            bot.answer_callback_query(q.id)
                                .text(&error_msg)
                                .show_alert(true)
                                .await?;
                            return Ok(());
                        }
                        
                        // Ask for strategy name
                        let name_msg = i18n::translate(&locale, "strategy_enter_name", None);
                        bot.edit_message_text(chat_id, message_id, name_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                        
                        // Update state to wait for name
                        dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForMixStrategyName {
                            selected_strategy_ids: selected_strategy_ids.clone(),
                        })).await?;
                    }
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
                                InlineKeyboardButton::callback(i18n::get_button_text(&locale, "pair_btc_usdt"), "pair_BTCUSDT"),
                                InlineKeyboardButton::callback(i18n::get_button_text(&locale, "pair_eth_usdt"), "pair_ETHUSDT"),
                            ],
                            vec![
                                InlineKeyboardButton::callback(i18n::get_button_text(&locale, "pair_bnb_usdt"), "pair_BNBUSDT"),
                                InlineKeyboardButton::callback(i18n::get_button_text(&locale, "pair_ada_usdt"), "pair_ADAUSDT"),
                            ],
                            vec![
                                InlineKeyboardButton::callback(i18n::get_button_text(&locale, "pair_sol_usdt"), "pair_SOLUSDT"),
                                InlineKeyboardButton::callback(i18n::get_button_text(&locale, "pair_dot_usdt"), "pair_DOTUSDT"),
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
                            strategy_name: String::new(), // Empty for custom flow
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
                        if let Ok(Some(BotState::CreateStrategy(CreateStrategyState::WaitingForPair { algorithm, buy_condition, sell_condition, timeframe, strategy_name }))) = dialogue.get().await {
                            // Get telegram_id from callback query
                            let telegram_id = q.from.id.0.to_string();
                            // Use strategy_name from state if provided, otherwise generate from algorithm, timeframe, pair
                            let final_strategy_name = if strategy_name.is_empty() {
                                format!("{}_{}_{}", algorithm, timeframe, pair)
                            } else {
                                strategy_name.clone()
                            };
                            
                            // Create StrategyConfig for content field
                            use crate::services::strategy_engine::StrategyConfig;
                            let strategy_service = crate::services::strategy_service::StrategyService::new(state.db.clone());
                            let parameters = strategy_service.extract_parameters(&algorithm, &buy_condition, &sell_condition);
                            let config = StrategyConfig {
                                strategy_type: algorithm.clone(),
                                parameters,
                                pair: pair.clone(),
                                timeframe: timeframe.clone(),
                                buy_condition: buy_condition.clone(),
                                sell_condition: sell_condition.clone(),
                            };
                            let content_json = serde_json::to_string(&config)
                                .unwrap_or_else(|_| "{}".to_string());
                            
                            let new_strategy = strategies::ActiveModel {
                                telegram_id: ActiveValue::Set(telegram_id.clone()),
                                name: ActiveValue::Set(Some(final_strategy_name.clone())),
                                description: ActiveValue::Set(Some(format!(
                                    "Algorithm: {}\nBuy: {}\nSell: {}\nTimeframe: {}\nPair: {}",
                                    algorithm, buy_condition, sell_condition, timeframe, pair
                                ))),
                                content: ActiveValue::Set(Some(content_json)),
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
                                        ("strategy_name", &final_strategy_name),
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
                                    let error_msg = i18n::translate(&locale, "strategy_saved_error", Some(&[
                                        ("error", &e.to_string())
                                    ]));
                                    bot.send_message(chat_id, error_msg)
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .await?;
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
    
    let from = msg.from.unwrap();
    let telegram_id = from.id.0 as i64;
    let username = from.username.unwrap_or("Unknown".to_string());

    tracing::info!(
        "Handling /createstrategy command from user: {} (id: {})",
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

    // First, ask user to choose between Custom, Preset, or Custom Mix strategy
    let strategy_type_buttons = vec![
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "strategy_type_custom"),
                "strategy_type_custom"
            ),
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "strategy_type_preset"),
                "strategy_type_preset"
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "strategy_type_custom_mix"),
                "strategy_type_custom_mix"
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "strategy_cancel_button"),
                "cancel_strategy"
            ),
        ],
    ];

    let welcome_msg = i18n::translate(locale, "strategy_welcome", None);

    bot.send_message(msg.chat.id, welcome_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(strategy_type_buttons))
        .await?;

    // Update dialogue state to wait for strategy type selection
    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForStrategyType)).await?;

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
            BotState::CreateStrategy(CreateStrategyState::WaitingForPresetName { algorithm, buy_condition, sell_condition, timeframe }) => {
                if let Some(text) = msg.text() {
                    let strategy_name = text.trim().to_string();
                    
                    // Validate strategy name is not empty
                    if strategy_name.is_empty() {
                        let error_msg = i18n::translate(locale, "strategy_name_empty", None);
                        bot.send_message(msg.chat.id, error_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                        return Ok(());
                    }
                    
                    // Show pair selection buttons
                    let mut pair_buttons = vec![
                        vec![
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "pair_btc_usdt"), "pair_BTCUSDT"),
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "pair_eth_usdt"), "pair_ETHUSDT"),
                        ],
                        vec![
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "pair_bnb_usdt"), "pair_BNBUSDT"),
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "pair_ada_usdt"), "pair_ADAUSDT"),
                        ],
                        vec![
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "pair_sol_usdt"), "pair_SOLUSDT"),
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "pair_dot_usdt"), "pair_DOTUSDT"),
                        ],
                        vec![
                            InlineKeyboardButton::callback(i18n::get_button_text(locale, "pair_manual"), "pair_manual"),
                        ],
                    ];
                    pair_buttons.push(vec![
                        InlineKeyboardButton::callback(i18n::get_button_text(locale, "strategy_cancel_button"), "cancel_strategy"),
                    ]);
                    
                    let name_confirm_msg = i18n::translate(locale, "strategy_name_set", Some(&[
                        ("name", &escape_html(&strategy_name)),
                    ]));
                    let pair_msg = i18n::translate(locale, "strategy_step5_choose_pair", None);
                    let instruction = format!("{}\n\n{}", name_confirm_msg, pair_msg);
                    
                    bot.send_message(msg.chat.id, instruction)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(pair_buttons))
                        .await?;
                    
                    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForPair {
                        algorithm,
                        buy_condition,
                        sell_condition,
                        timeframe,
                        strategy_name,
                    })).await?;
                }
            }
            BotState::CreateStrategy(CreateStrategyState::WaitingForMixStrategyName { selected_strategy_ids }) => {
                if let Some(text) = msg.text() {
                    let strategy_name = text.trim().to_string();
                    
                    // Validate strategy name is not empty
                    if strategy_name.is_empty() {
                        let error_msg = i18n::translate(locale, "strategy_name_empty", None);
                        bot.send_message(msg.chat.id, error_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                        return Ok(());
                    }
                    
                    // Load selected strategies from database
                    let mut mixed_strategies: Vec<serde_json::Value> = Vec::new();
                    let mut strategy_summary = String::new();
                    
                    for strategy_id in &selected_strategy_ids {
                        if let Ok(Some(strategy)) = strategies::Entity::find_by_id(*strategy_id)
                            .one(state.db.as_ref())
                            .await
                        {
                            // Try to parse strategy config from content field
                            let strategy_service = crate::services::strategy_service::StrategyService::new(state.db.clone());
                            if let Ok(config) = strategy_service.strategy_to_config(&strategy) {
                                let strategy_json = serde_json::json!({
                                    "id": strategy.id,
                                    "name": strategy.name,
                                    "algorithm": config.strategy_type,
                                    "buy_condition": config.buy_condition,
                                    "sell_condition": config.sell_condition,
                                    "timeframe": config.timeframe,
                                    "pair": config.pair,
                                    "parameters": config.parameters,
                                });
                                mixed_strategies.push(strategy_json);
                                
                                // Build summary
                                let name = strategy.name.as_ref()
                                    .map(|n| n.as_str())
                                    .unwrap_or("Unnamed");
                                strategy_summary.push_str(&format!("\n• {} ({})", escape_html(name), config.strategy_type));
                            }
                        }
                    }
                    
                    if mixed_strategies.is_empty() {
                        let error_msg = i18n::translate(locale, "strategy_mix_load_error", None);
                        bot.send_message(msg.chat.id, error_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                        dialogue.exit().await?;
                        return Ok(());
                    }
                    
                    // Create mixed strategy config
                    use serde_json::json;
                    
                    // For mixed strategy, we'll store all strategies in the content field
                    let mixed_config = json!({
                        "type": "mixed",
                        "strategies": mixed_strategies,
                        "mix_mode": "all" // Could be "all", "any", "majority" etc.
                    });
                    
                    // Create a combined description
                    let description = format!(
                        "Mixed Strategy combining {} strategies:{}",
                        mixed_strategies.len(),
                        strategy_summary
                    );
                    
                    // Save mixed strategy to database
                    let telegram_id = msg.from.as_ref().unwrap().id.0 as i64;
                    let new_strategy = strategies::ActiveModel {
                        name: ActiveValue::Set(Some(strategy_name.clone())),
                        description: ActiveValue::Set(Some(description)),
                        content: ActiveValue::Set(Some(serde_json::to_string(&mixed_config).unwrap_or_default())),
                        telegram_id: ActiveValue::Set(telegram_id.to_string()),
                        created_at: ActiveValue::Set(Some(chrono::Utc::now())),
                        ..Default::default()
                    };
                    
                    match strategies::Entity::insert(new_strategy)
                        .exec(state.db.as_ref())
                        .await
                    {
                        Ok(_) => {
                            let success_msg = i18n::translate(locale, "strategy_mix_saved", Some(&[
                                ("name", &escape_html(&strategy_name)),
                                ("count", &mixed_strategies.len().to_string()),
                            ]));
                            bot.send_message(msg.chat.id, success_msg)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                            dialogue.exit().await?;
                        }
                        Err(e) => {
                            let error_msg = i18n::translate(locale, "strategy_saved_error", Some(&[
                                ("error", &e.to_string())
                            ]));
                            bot.send_message(msg.chat.id, error_msg)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                            dialogue.exit().await?;
                        }
                    }
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
            BotState::CreateStrategy(CreateStrategyState::WaitingForPair { algorithm, buy_condition, sell_condition, timeframe, strategy_name }) => {
                if let Some(text) = msg.text() {
                    let pair = text.trim().to_uppercase();
                    // Save strategy to database
                    let telegram_id = msg.from.as_ref().unwrap().id.0.to_string();
                    // Use strategy_name from state if provided, otherwise generate from algorithm, timeframe, pair
                    let final_strategy_name = if strategy_name.is_empty() {
                        format!("{}_{}_{}", algorithm, timeframe, pair)
                    } else {
                        strategy_name.clone()
                    };
                    
                    // Create StrategyConfig for content field
                    use crate::services::strategy_engine::StrategyConfig;
                    let strategy_service = crate::services::strategy_service::StrategyService::new(state.db.clone());
                    let parameters = strategy_service.extract_parameters(&algorithm, &buy_condition, &sell_condition);
                    let config = StrategyConfig {
                        strategy_type: algorithm.clone(),
                        parameters,
                        pair: pair.clone(),
                        timeframe: timeframe.clone(),
                        buy_condition: buy_condition.clone(),
                        sell_condition: sell_condition.clone(),
                    };
                    let content_json = serde_json::to_string(&config)
                        .unwrap_or_else(|_| "{}".to_string());
                    
                    let new_strategy = strategies::ActiveModel {
                        telegram_id: ActiveValue::Set(telegram_id.clone()),
                        name: ActiveValue::Set(Some(final_strategy_name.clone())),
                        description: ActiveValue::Set(Some(format!(
                            "Algorithm: {}\nBuy: {}\nSell: {}\nTimeframe: {}\nPair: {}",
                            algorithm, buy_condition, sell_condition, timeframe, pair
                        ))),
                        content: ActiveValue::Set(Some(content_json)),
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

    // Build inline keyboard with only "Delete Strategy" and "Back" buttons
    let buttons = vec![
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "strategy_delete_button"),
                "show_delete_strategies"
            )
        ],
        vec![
            InlineKeyboardButton::callback(
                i18n::get_button_text(locale, "button_back"),
                "back_to_my_strategies"
            )
        ],
    ];

    let send_msg = bot.send_message(msg.chat.id, msg_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons));
    
    send_msg.await?;

    Ok(())
}

/// Handler for delete strategy callback
pub async fn handle_delete_strategy_callback(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    tracing::info!("handle_delete_strategy_callback called with data: {:?}", q.data);
    
    // Get user locale
    let user_id = q.from.id.0 as i64;
    let user = users::Entity::find_by_id(user_id)
        .one(state.db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    if let Some(data) = &q.data {
        // Handle confirmation callbacks (delete_confirm_<strategy_id>)
        if data.as_str().starts_with("delete_confirm_") {
            let strategy_id_str = data.as_str().replace("delete_confirm_", "");
            let strategy_id: u64 = match strategy_id_str.parse() {
                Ok(id) => id,
                Err(_) => {
                    let error_msg = i18n::translate(locale, "error_invalid_strategy_id", None);
                    bot.answer_callback_query(q.id)
                        .text(&error_msg)
                        .await?;
                    return Ok(());
                }
            };

            // Get user ID and locale
            let user_id = q.from.id.0.to_string();
            let db = state.db.clone();
            
            // Get user locale
            let user = users::Entity::find_by_id(user_id.parse::<i64>().unwrap_or(0))
                .one(db.as_ref())
                .await?;
            let locale = user
                .as_ref()
                .and_then(|u| u.language.as_ref())
                .map(|l| i18n::get_user_language(Some(l)))
                .unwrap_or("en");

            // Check if strategy exists and belongs to user before deleting
            use sea_orm::ColumnTrait;
            let strategy = strategies::Entity::find_by_id(strategy_id)
                .filter(strategies::Column::TelegramId.eq(user_id.clone()))
                .one(db.as_ref())
                .await?;

            if let Some(ref strategy_model) = strategy {
                let strategy_name = strategy_model.name.as_ref()
                    .map(|n| n.clone())
                    .unwrap_or_else(|| format!("Strategy #{}", strategy_id));

                // Proceed with delete
                match strategies::Entity::delete_by_id(strategy_id)
                    .exec(db.as_ref())
                    .await
                {
                    Ok(_) => {
                        // Success
                        let success_msg = i18n::translate(locale, "strategy_delete_success", Some(&[
                            ("strategy_name", &strategy_name),
                        ]));
                        
                        // Strip HTML tags for callback query alert (doesn't support HTML)
                        let plain_text = strip_html_tags(&success_msg);
                        
                        bot.answer_callback_query(q.id)
                            .text(&plain_text)
                            .show_alert(true)
                            .await?;

                        if let Some(msg) = q.message {
                            bot.edit_message_text(msg.chat().id, msg.id(), &success_msg)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                        }
                    }
                    Err(e) => {
                        // Error deleting
                        let error_msg = i18n::translate(locale, "strategy_delete_error", None);
                        bot.answer_callback_query(q.id)
                            .text(&format!("{}: {}", error_msg, e))
                            .show_alert(true)
                            .await?;
                    }
                }
            } else {
                // Strategy not found or doesn't belong to user
                let error_msg = i18n::translate(locale, "strategy_delete_not_found", None);
                bot.answer_callback_query(q.id)
                    .text(&error_msg)
                    .show_alert(true)
                    .await?;
            }
        }
        // Handle initial delete button click (delete_strategy_<strategy_id>)
        else if data.starts_with("delete_strategy_") {
            let strategy_id_str = data.replace("delete_strategy_", "");
            let strategy_id: u64 = match strategy_id_str.parse() {
                Ok(id) => id,
                Err(_) => {
                    let error_msg = i18n::translate(locale, "error_invalid_strategy_id", None);
                    bot.answer_callback_query(q.id)
                        .text(&error_msg)
                        .await?;
                    return Ok(());
                }
            };

            // Get user ID and locale
            let user_id = q.from.id.0.to_string();
            let db = state.db.clone();
            
            // Get user locale
            let user = users::Entity::find_by_id(user_id.parse::<i64>().unwrap_or(0))
                .one(db.as_ref())
                .await?;
            let locale = user
                .as_ref()
                .and_then(|u| u.language.as_ref())
                .map(|l| i18n::get_user_language(Some(l)))
                .unwrap_or("en");

            // Check if strategy exists and belongs to user
            use sea_orm::ColumnTrait;
            let strategy = strategies::Entity::find_by_id(strategy_id)
                .filter(strategies::Column::TelegramId.eq(user_id.clone()))
                .one(db.as_ref())
                .await?;

            if let Some(ref strategy_model) = strategy {
                let strategy_name = strategy_model.name.as_ref()
                    .map(|n| n.clone())
                    .unwrap_or_else(|| format!("Strategy #{}", strategy_id));

                // Show confirmation dialog
                bot.answer_callback_query(q.id).await?;

                if let Some(msg) = q.message {
                    let confirm_msg = i18n::translate(locale, "strategy_delete_confirm", Some(&[
                        ("strategy_name", &strategy_name),
                    ]));
                    
                    let confirm_buttons = vec![
                        vec![
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(locale, "strategy_delete_confirm_yes"),
                                format!("delete_confirm_{}", strategy_id)
                            ),
                            InlineKeyboardButton::callback(
                                i18n::get_button_text(locale, "strategy_delete_confirm_no"),
                                "delete_cancel"
                            ),
                        ],
                    ];

                    bot.edit_message_text(msg.chat().id, msg.id(), confirm_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(confirm_buttons))
                        .await?;
                }
            } else {
                // Strategy not found or doesn't belong to user
                let error_msg = i18n::translate(locale, "strategy_delete_not_found", None);
                bot.answer_callback_query(q.id)
                    .text(&error_msg)
                    .show_alert(true)
                    .await?;
            }
        }
        // Handle cancel deletion (delete_cancel)
        else if data == "delete_cancel" {
            bot.answer_callback_query(q.id).await?;
            
            if let Some(msg) = q.message {
                let user_id = q.from.id.0.to_string();
                let db = state.db.clone();
                
                // Get user locale
                let user = users::Entity::find_by_id(user_id.parse::<i64>().unwrap_or(0))
                    .one(db.as_ref())
                    .await?;
                let locale = user
                    .as_ref()
                    .and_then(|u| u.language.as_ref())
                    .map(|l| i18n::get_user_language(Some(l)))
                    .unwrap_or("en");
                
                // Send cancellation message
                let refresh_msg = i18n::translate(locale, "strategy_delete_cancelled", None);
                bot.edit_message_text(msg.chat().id, msg.id(), refresh_msg)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
        }
        // Handle show delete strategies list (show_delete_strategies)
        else if data == "show_delete_strategies" {
            bot.answer_callback_query(q.id).await?;
            
            if let Some(msg) = q.message {
                let user_id = q.from.id.0.to_string();
                let db = state.db.clone();
                
                // Get user locale
                let user = users::Entity::find_by_id(user_id.parse::<i64>().unwrap_or(0))
                    .one(db.as_ref())
                    .await?;
                let locale = user
                    .as_ref()
                    .and_then(|u| u.language.as_ref())
                    .map(|l| i18n::get_user_language(Some(l)))
                    .unwrap_or("en");
                
                // Query strategies filtered by telegram_id
                use sea_orm::ColumnTrait;
                let user_strategies = strategies::Entity::find()
                    .filter(strategies::Column::TelegramId.eq(user_id.clone()))
                    .order_by_desc(strategies::Column::CreatedAt)
                    .all(db.as_ref())
                    .await?;
                
                if user_strategies.is_empty() {
                    let empty_msg = i18n::translate(locale, "strategy_my_strategies_empty", None);
                    bot.edit_message_text(msg.chat().id, msg.id(), empty_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                    return Ok(());
                }
                
                // Build message with delete instructions
                let delete_title = i18n::translate(locale, "strategy_delete_list_title", None);
                let delete_msg = delete_title + "\n\n";
                
                let unnamed_str = "Unnamed".to_string();
                
                // Build inline keyboard buttons for each strategy to delete
                let mut buttons = Vec::new();
                for strategy in &user_strategies {
                    let strategy_name = strategy.name.as_ref().unwrap_or(&unnamed_str);
                    // Truncate name if too long
                    let display_name = if strategy_name.chars().count() > 30 {
                        let truncated: String = strategy_name.chars().take(27).collect();
                        format!("{}...", truncated)
                    } else {
                        strategy_name.clone()
                    };
                    
                    // Create button text with strategy name
                    let delete_prefix = i18n::get_button_text(locale, "strategy_delete_with_name");
                    let delete_text = format!("{}: {}", delete_prefix, display_name);
                    
                    buttons.push(vec![
                        InlineKeyboardButton::callback(
                            delete_text,
                            format!("delete_strategy_{}", strategy.id)
                        )
                    ]);
                }
                
                // Add Back button
                buttons.push(vec![
                    InlineKeyboardButton::callback(
                        i18n::get_button_text(locale, "button_back"),
                        "back_to_my_strategies"
                    )
                ]);
                
                bot.edit_message_text(msg.chat().id, msg.id(), delete_msg)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
                    .await?;
            }
        }
        // Handle back to my strategies (back_to_my_strategies)
        else if data == "back_to_my_strategies" {
            bot.answer_callback_query(q.id).await?;
            
            if let Some(msg) = q.message {
                let user_id = q.from.id.0.to_string();
                let db = state.db.clone();
                
                // Get user locale
                let user = users::Entity::find_by_id(user_id.parse::<i64>().unwrap_or(0))
                    .one(db.as_ref())
                    .await?;
                let locale = user
                    .as_ref()
                    .and_then(|u| u.language.as_ref())
                    .map(|l| i18n::get_user_language(Some(l)))
                    .unwrap_or("en");
                
                // Query strategies filtered by telegram_id
                use sea_orm::ColumnTrait;
                let user_strategies = strategies::Entity::find()
                    .filter(strategies::Column::TelegramId.eq(user_id.clone()))
                    .order_by_desc(strategies::Column::CreatedAt)
                    .all(db.as_ref())
                    .await?;
                
                if user_strategies.is_empty() {
                    let empty_msg = i18n::translate(locale, "strategy_my_strategies_empty", None);
                    // Ignore "message is not modified" error
                    if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &empty_msg)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await
                    {
                        let error_str = format!("{}", e);
                        if !error_str.contains("message is not modified") {
                            tracing::warn!("Failed to edit message (empty strategies in back_to_my_strategies): {}", e);
                        }
                    }
                    return Ok(());
                }
                
                // Rebuild the original my strategies message
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
                
                // Build inline keyboard with only "Delete Strategy" and "Back" buttons
                let buttons = vec![
                    vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "strategy_delete_button"),
                            "show_delete_strategies"
                        )
                    ],
                    vec![
                        InlineKeyboardButton::callback(
                            i18n::get_button_text(locale, "button_back"),
                            "back_to_my_strategies"
                        )
                    ],
                ];
                
                // Edit message, but ignore "message is not modified" error
                if let Err(e) = bot.edit_message_text(msg.chat().id, msg.id(), &msg_text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
                    .await
                {
                    // Ignore "message is not modified" error - it means the message is already in the desired state
                    let error_str = format!("{}", e);
                    if !error_str.contains("message is not modified") {
                        // Log other errors but don't fail
                        tracing::warn!("Failed to edit message in back_to_my_strategies: {}", e);
                    }
                }
            }
        }
    }
    Ok(())
}