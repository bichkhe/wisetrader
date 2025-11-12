//! Streams Command - shows active market data streams

use std::sync::Arc;
use teloxide::prelude::*;
use sea_orm::EntityTrait;
use crate::state::AppState;
use crate::i18n;
use shared::entity::users;

/// Handler for /streams command to view active market data streams
pub async fn handle_streams(
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
    
    // Get all active streams from StreamManager
    let active_streams = state.stream_manager.get_active_streams().await;
    
    if active_streams.is_empty() {
        let msg_text = i18n::translate(locale, "streams_no_active", None);
        bot.send_message(msg.chat.id, msg_text)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }
    
    // Build message with stream information
    let mut msg_text = i18n::translate(locale, "streams_title", None);
    msg_text.push_str("\n━━━━━━━━━━\n\n");
    
    for (idx, (exchange, pair, subscriber_count, subscriber_ids)) in active_streams.iter().enumerate() {
        let exchange_name = match exchange.as_str() {
            "binance" => "🔵 Binance",
            "okx" => "🟢 OKX",
            _ => exchange,
        };
        
        msg_text.push_str(&format!(
            "<b>{}. {} - {}</b>\n",
            idx + 1,
            exchange_name,
            pair
        ));
        
        msg_text.push_str(&i18n::translate(
            locale,
            "streams_subscribers",
            Some(&[
                ("count", &subscriber_count.to_string()),
            ]),
        ));
        msg_text.push_str("\n");
        
        // Show subscriber IDs (first 5, then "... and X more" if more)
        if !subscriber_ids.is_empty() {
            let display_count = subscriber_ids.len().min(5);
            let subscriber_list: Vec<String> = subscriber_ids.iter()
                .take(display_count)
                .map(|id| format!("<code>{}</code>", id))
                .collect();
            
            msg_text.push_str(&format!("  👥 {}", subscriber_list.join(", ")));
            
            if subscriber_ids.len() > display_count {
                let remaining = subscriber_ids.len() - display_count;
                msg_text.push_str(&i18n::translate(
                    locale,
                    "streams_subscribers_more",
                    Some(&[("count", &remaining.to_string())]),
                ));
            }
            msg_text.push_str("\n");
        }
        
        msg_text.push_str("\n");
    }
    
    msg_text.push_str("━━━━━━━━━━\n\n");
    msg_text.push_str(&i18n::translate(locale, "streams_footer", None));
    
    bot.send_message(msg.chat.id, msg_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    
    Ok(())
}

