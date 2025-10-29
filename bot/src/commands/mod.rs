use anyhow::Result;
use sea_orm::EntityTrait;
use teloxide::utils::command::BotCommands;
use teloxide::{ prelude::*};
use teloxide::types::Message;
use shared::entity::{users, strategies};
use chrono::{Utc, Duration};
use tracing::info;
use std::sync::Arc;
use std::time::Instant;
use crate::state::{AppState, MyDialogue};
pub mod admin;
pub mod me;

pub use admin::handle_version;
pub use me::handle_me;

/// ✅🤖 <b>WiseTrader</b> 🧠 — Bạn có thể chọn một trong các lệnh sau
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// ✨  Các lệnh trợ giúp
    Help,
    /// Quay trở lại menu chính
    Cancel,
    /// ❓ Bắt đầu sử dụng BOT.
    Start,
    /// Xem thông tin của bạn
    Me,
    /// ℹ️  Thông tin tài khoản của khách hàng
    Info(String),
    /// Nạp điểm vào hệ thống
    Deposit,
    /// Nhắn tin toàn hệ thống
    Broadcast(String),
    /// Get server ip
    Ip(String),
    /// What is the current version ?
    /// 
    Version,
    /// Kích hoạt người dùng
    Unlock(String),

    /// Xem thông tin subscription của bạn
    Subscription,
   ///  Xem các chiến thuật hiện có
   Strategies
}


async fn handle_start(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
        let user_id = msg.from.as_ref().unwrap().id.0 as i64;
        let username = msg.from.as_ref().unwrap().username.clone();

    let db = state.db.clone();
    info!("Processing /start command from user {}", user_id);

    // Check if user already exists using Sea-ORM
    let existing_user = users::Entity::find_by_id(user_id)
        .one(db.as_ref())
        .await?;

    if existing_user.is_some() {
        bot.send_message(msg.chat.id, "Welcome back! Use /help to see available commands.")
            .await?;
        return Ok(());
    }

    // Register new user with free trial
    let expires_at = Utc::now() + Duration::days(7);
    
    use sea_orm::ActiveValue::Set;

    let new_user = users::ActiveModel {
        id: Set(user_id),
        username: Set(username.clone()),
        language: Set(Some("en".to_string())),
        subscription_tier: Set(Some("free_trial".to_string())),
        subscription_expires: Set(Some(expires_at)),
        live_trading_enabled: Set(Some(0)),
        created_at: Set(Some(Utc::now())),
        telegram_id: Set(Some(user_id.to_string())),
        fullname: Set(username.unwrap_or_else(|| "".to_string()).into()),
        points: Set(0u64),
    };

    state.user_service.create_user(new_user).await.unwrap();

    let welcome_msg = format!(
        "<b>Welcome to WiseTrader! 🚀</b>\n\n\
        You've been registered with a <b>7-day Free Trial</b>. 🆓🗓️<br><br>\
        <b>Features available:</b><br>\
        ⭐ Delayed trading signals<br>\
        🧪 1 backtest job<br>\
        📚 Access to strategy library<br><br>\
        <b>Use</b> <code>/help</code> <b>to see all commands.</b> ℹ️<br>\
        <b>Use</b> <code>/upgrade</code> <b>to see subscription plans.</b> 💎<br><br>\
        <i>Note: ⚠️ This is a trading bot. Trading cryptocurrencies involves risk.</i>"
    );

    bot.send_message(msg.chat.id, welcome_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}

pub async fn handle_help(bot: Bot, msg: Message) -> Result<()> {
    let start_time = Instant::now();
    
    let from = msg.from.unwrap();
    let fullname = from.full_name();
    let telegram_id = from.id.0 as i64;
    let username = from.username.unwrap_or("Không có".to_string());
    tracing::info!(
        "Handling /help command for user: {} (id: {}, username: {})",
        fullname,
        telegram_id,
        username
    );
    // Filter out some commands from the help message
    let  descriptions = Command::descriptions().to_string();
    bot.send_message(msg.chat.id, descriptions)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    let duration = start_time.elapsed();
    tracing::info!("Time taken to handle /help command: {:?}", duration);
    Ok(())
}

// async fn handle_subscription(bot: Bot, msg: Message, state: AppState) -> Result<()> {
//     let user_id = msg.from().unwrap().id.0 as i64;
//     let db = state.db.clone();

//     let user = users::Entity::find_by_id(user_id)
//         .one(db.as_ref())
//         .await?;

//     let user = match user {
//         Some(u) => u,
//         None => {
//             bot.send_message(msg.chat.id, "Please register first with /start").await?;
//             return Ok(());
//         }
//     };

//     let plan = billing_plan::Entity::find_by_id(user.subscription_tier.clone())
//         .one(db.as_ref())
//         .await?;

//     let plan = match plan {
//         Some(p) => p,
//         None => {
//             bot.send_message(msg.chat.id, "Plan not found").await?;
//             return Ok(());
//         }
//     };

//     // Parse features JSON
//     let features_str = plan.features.clone();
//     let features: Vec<String> = serde_json::from_str(&features_str)
//         .unwrap_or_else(|_| vec![]);
    
//     let status_msg = format!(
//         "📋 **Your Subscription**\n\n\
//         **Plan:** {}\n\
//         **Price:** ${}/month\n\
//         **Expires:** {}\n\n\
//         Use /upgrade to view available plans.",
//         plan.name,
//         plan.price_monthly_usd,
//         user.subscription_expires
//             .map(|d| d.format("%Y-%m-%d").to_string())
//             .unwrap_or_else(|| "Never".to_string())
//     );

//     bot.send_message(msg.chat.id, status_msg)
//         .parse_mode(teloxide::types::ParseMode::Markdown)
//         .await?;

//     Ok(())
// }

async fn handle_strategies(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    let db = state.db.clone();
    let strategies = strategies::Entity::find()
        .all(db.as_ref())
        .await?;

    if strategies.is_empty() {
        bot.send_message(msg.chat.id, "No strategies available yet.").await?;
        return Ok(());
    }

    let mut msg_text = "📊 **Available Strategies**\n\n".to_string();
    
    for strategy in strategies {
        msg_text.push_str(&format!(
            "**{}. {}**\n{}\n\n",
            strategy.id,
            strategy.name.unwrap_or_else(|| "No name".to_string()).to_string(),
            strategy.description.unwrap_or_else(|| "No description".to_string())
        ));
    }

    msg_text.push_str("Use /add_strategy <id> to subscribe to a strategy.");

    bot.send_message(msg.chat.id, msg_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}

// async fn handle_my_strategies(bot: Bot, msg: Message, db: Arc<DatabaseConnection>) -> Result<()> {
//     let user_id = msg.from().unwrap().id.0 as i64;

//     let user_strategies = user_strategy::Entity::find()
//         .filter(user_strategy::Column::UserId.eq(user_id))
//         .filter(user_strategy::Column::Active.eq(true))
//         .all(db.as_ref())
//         .await?;

//     if user_strategies.is_empty() {
//         bot.send_message(msg.chat.id, "You have no active strategies.\nUse /strategies to see available strategies.")
//             .await?;
//         return Ok(());
//     }

//     let mut msg_text = "📋 **Your Active Strategies**\n\n".to_string();
    
//     for us in user_strategies {
//         let strategy = strategy::Entity::find_by_id(us.strategy_id)
//             .one(db.as_ref())
//             .await?;

//         if let Some(s) = strategy {
//             msg_text.push_str(&format!(
//                 "**{}. {}**\n{}\n\n",
//                 s.id,
//                 s.name,
//                 s.description.unwrap_or_else(|| "No description".to_string())
//             ));
//         }
//     }

//     bot.send_message(msg.chat.id, msg_text)
//         .parse_mode(teloxide::types::ParseMode::Markdown)
//         .await?;

//     Ok(())
// }


pub async fn handle_invalid(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
) -> anyhow::Result<()>  {
    bot.send_message(
        msg.chat.id, 
        "❌ Invalid command. Please use /help to see available commands."
    ).await?;
    Ok(())
}

pub async fn handle_invalid_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> anyhow::Result<()>  {
    bot.send_message(dialogue.chat_id(), format!(" Select network"))
        .await?;
    Ok(())
}