use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;
use sea_orm::EntityTrait;
use shared::entity::users;

use crate::state::{AppState, MyDialogue};

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

    if existing_user.is_none() {
        // Tạo user mới
        use sea_orm::ActiveValue::Set;
        let new_user = users::ActiveModel {
            id: Set(telegram_id),
            username: Set(Some(username.clone())),
            language: Set(Some("vi".to_string())),
            subscription_tier: Set(Some("free_trial".to_string())),
            subscription_expires: Set(None), // Hoặc set ngày nếu cần
            live_trading_enabled: Set(Some(0)),
            created_at: Set(Some(chrono::Utc::now())),
            telegram_id: Set(Some(telegram_id.to_string())),
            fullname: Set(fullname.clone().into()),
            points: Set(0u64),
        };
        state.user_service.create_user(new_user).await?;
        bot.send_message(
            msg.chat.id,
            format!(
                "🆔 User ID <b>{}</b> chưa tồn tại trong hệ thống.\n\nVui lòng sử dụng lệnh /start để đăng ký tài khoản mới.",
                telegram_id
            ),
        )
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
        return Ok(());
    }

    if let Some(ref user) = existing_user {
        let status = if user.live_trading_enabled.unwrap_or(0) == 1 {
            "Đang Hoạt động"
        } else {
            "Chưa được kích hoạt"
        };

        let info = format!(
            "✅👤 <b>Thông tin tài khoản</b>\n\n\
        👤 Tên: <b>{}</b>\n\
        👤 Username: @{}\n\
        🆔 UserID: {}\n\
        🔢 Số điểm: <b>{} điểm</b>\n\
        📋 Gói dịch vụ: {}\n\
        📅 Ngày đăng ký: {}\n\
        ⏰ Hết hạn: {}\n\
        ℹ️ Trạng thái: <b>{}</b>\n\n\
        💡 Sử dụng lệnh /help để xem các lệnh khả dụng.",
            user.fullname.as_ref().unwrap_or(&fullname),
            username,
            telegram_id,
            user.points,
            user.subscription_tier.as_ref().unwrap_or(&"N/A".to_string()),
            user.created_at.as_ref().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "N/A".to_string()),
            user.subscription_expires.as_ref().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "Không giới hạn".to_string()),
            status
        );
        
        bot.send_message(msg.chat.id, info)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }

    Ok(())
}
