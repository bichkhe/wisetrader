use anyhow::Result;
use sea_orm::{EntityTrait};
use teloxide::utils::command::BotCommands;
use teloxide::{ prelude::*};
use teloxide::types::Message;
use shared::entity::{users, strategies};
use std::sync::Arc;
use std::time::Instant;
use crate::state::{AppState, MyDialogue};
pub mod admin;
pub mod me;
pub mod trading;
pub mod strategy;
pub mod backtest;
pub mod start;

pub use admin::handle_version;
pub use me::handle_me;
pub use strategy::{handle_create_strategy, handle_strategy_callback, handle_strategy_input_callback, handle_my_strategies, handle_delete_strategy_callback};
pub use backtest::{handle_backtest as handle_backtest_wizard, handle_backtest_callback};
pub use start::{handle_start, handle_language_selection, handle_language_callback};
pub use me::handle_profile_callback;
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
    /// Các indicators
   ///  Xem các chiến thuật hiện có
   Strategies,
   /// Xem các chiến thuật đã tạo của bạn
   MyStrategies,
   /// Tạo chiến thuật mới
   CreateStrategy,

   /// Xem kết quả backtest
   Backtest(String),
}


// handle_start moved to start.rs module

pub async fn handle_help(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
) -> Result<()> {
    use crate::i18n;
    let start_time = Instant::now();
    
    let from = msg.from.unwrap();
    let fullname = from.full_name();
    let telegram_id = from.id.0 as i64;
    let username = from.username.unwrap_or("Không có".to_string());
    
    // Get user language
    let user = shared::entity::users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    tracing::info!(
        "Handling /help command for user: {} (id: {}, username: {}, locale: {})",
        fullname,
        telegram_id,
        username,
        locale
    );
    
    // Build help message with translations
    let mut help_text = i18n::translate(locale, "cmd_help_title", None);
    
    // Add command descriptions using translations
    help_text.push_str(&format!("/start - {}\n", i18n::translate(locale, "cmd_help_start", None)));
    help_text.push_str(&format!("/help - {}\n", i18n::translate(locale, "cmd_help_help", None)));
    help_text.push_str(&format!("/version - {}\n", i18n::translate(locale, "cmd_help_version", None)));
    help_text.push_str(&format!("/me - {}\n", i18n::translate(locale, "cmd_help_me", None)));
    help_text.push_str(&format!("/createstrategy - {}\n", i18n::translate(locale, "cmd_help_create_strategy", None)));
    help_text.push_str(&format!("/mystrategies - {}\n", i18n::translate(locale, "cmd_help_mystrategies", None)));
    help_text.push_str(&format!("/backtest - {}\n", i18n::translate(locale, "cmd_help_backtest", None)));
    
    help_text.push_str(&i18n::translate(locale, "cmd_help_footer", None));
    
    bot.send_message(msg.chat.id, help_text)
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




pub async fn handle_invalid(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> anyhow::Result<()>  {
    if let Ok(state) = dialogue.get().await {
        let state_text = format!("Current dialogue state: {:?}", state);
        bot.send_message(msg.chat.id, state_text).await?;
    }

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