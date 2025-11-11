use std::sync::Arc;
use teloxide::prelude::*;
use sea_orm::EntityTrait;
use shared::entity::users;
use crate::state::AppState;
use crate::i18n;
use crate::services::gemini::GeminiService;

/// Handler for the /ai command to ask Gemini AI questions
pub async fn handle_ai(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let telegram_id = msg.from.as_ref().map(|f| f.id.0 as i64).unwrap_or(0);
    
    // Get question from command text first (before moving msg)
    let question = msg.text()
        .and_then(|text| {
            // Extract question after /ai command
            if text.starts_with("/ai") {
                let parts: Vec<&str> = text.splitn(2, ' ').collect();
                if parts.len() > 1 {
                    Some(parts[1].trim())
                } else {
                    None
                }
            } else {
                None
            }
        });
    
    // Get user from database
    let user = users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    
    // Get user language
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    // Check if Gemini is enabled and configured
    let config = state.config.as_ref();
    if !config.enable_gemini_analysis {
        let error_msg = if locale == "vi" {
            "❌ Tính năng AI chưa được kích hoạt."
        } else {
            "❌ AI feature is not enabled."
        };
        bot.send_message(msg.chat.id, error_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }
    
    let api_key = match &config.gemini_api_key {
        Some(key) => key.clone(),
        None => {
            let error_msg = if locale == "vi" {
                "❌ Gemini API key chưa được cấu hình."
            } else {
                "❌ Gemini API key is not configured."
            };
            bot.send_message(msg.chat.id, error_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            return Ok(());
        }
    };
    
    // Check if question is provided
    if question.is_none() || question.unwrap().is_empty() {
        let help_msg = if locale == "vi" {
            "🤖 <b>AI Assistant (Gemini)</b>\n\n\
            Sử dụng: <code>/ai [câu hỏi của bạn]</code>\n\n\
            Ví dụ:\n\
            • <code>/ai Giải thích RSI là gì?</code>\n\
            • <code>/ai Cách sử dụng MACD trong trading?</code>\n\
            • <code>/ai Phân tích xu hướng thị trường hiện tại</code>"
        } else {
            "🤖 <b>AI Assistant (Gemini)</b>\n\n\
            Usage: <code>/ai [your question]</code>\n\n\
            Examples:\n\
            • <code>/ai What is RSI?</code>\n\
            • <code>/ai How to use MACD in trading?</code>\n\
            • <code>/ai Analyze current market trends</code>"
        };
        
        bot.send_message(msg.chat.id, help_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }
    
    let question_text = question.unwrap();
    
    // Send "thinking" message
    let thinking_msg = if locale == "vi" {
        "🤔 Đang suy nghĩ..."
    } else {
        "🤔 Thinking..."
    };
    
    let sent_msg = bot.send_message(msg.chat.id, thinking_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    
    // Create Gemini service
    let gemini = GeminiService::with_config(
        api_key,
        config.gemini_model_name.clone(),
        config.gemini_base_url.clone(),
        config.gemini_timeout_secs,
    );
    
    // Build prompt based on user language
    let prompt = if locale == "vi" {
        format!(
            "Bạn là một chuyên gia tư vấn về trading và cryptocurrency. \
            Hãy trả lời câu hỏi sau một cách chi tiết, rõ ràng và dễ hiểu. \
            Sử dụng định dạng markdown để trình bày.\n\n\
            Câu hỏi: {}\n\n\
            Hãy trả lời bằng tiếng Việt.",
            question_text
        )
    } else {
        format!(
            "You are an expert advisor on trading and cryptocurrency. \
            Please answer the following question in detail, clearly and understandably. \
            Use markdown formatting for presentation.\n\n\
            Question: {}\n\n\
            Please answer in English.",
            question_text
        )
    };
    
    // Call Gemini API
    let response = gemini.ask_question(&prompt).await;
    
    match response {
        Ok(answer) => {
            // Edit the "thinking" message with the answer
            bot.edit_message_text(msg.chat.id, sent_msg.id, answer)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        Err(e) => {
            let error_msg = if locale == "vi" {
                format!("❌ Lỗi khi gọi AI: {}", e)
            } else {
                format!("❌ Error calling AI: {}", e)
            };
            
            bot.edit_message_text(msg.chat.id, sent_msg.id, error_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            
            tracing::error!("Failed to get AI response: {}", e);
        }
    }
    
    Ok(())
}

