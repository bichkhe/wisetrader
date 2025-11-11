use anyhow::Result;
use sea_orm::{EntityTrait};
use teloxide::utils::command::BotCommands;
use teloxide::{ prelude::*};
use teloxide::types::Message;
use std::sync::Arc;
use std::time::Instant;
use crate::state::{AppState, MyDialogue};
pub mod admin;
pub mod me;
pub mod trading;
pub mod strategy;
pub mod backtest;
pub mod backtest_template;
pub mod start;
pub mod payment;

pub use admin::handle_version;
pub use me::handle_me;
pub use strategy::{handle_create_strategy, handle_strategy_callback, handle_strategy_input_callback, handle_my_strategies, handle_delete_strategy_callback};
pub use backtest::{handle_backtest as handle_backtest_wizard, handle_backtest_callback};
pub use start::{handle_start, handle_language_selection, handle_language_callback};
pub use me::handle_profile_callback;
pub use payment::{handle_deposit, handle_balance, handle_deposit_callback};
pub mod start_trading;
pub use start_trading::{handle_start_trading, handle_start_trading_callback};
pub mod live_trading;
pub use live_trading::{handle_live_trading, handle_live_trading_callback, handle_live_trading_input};
pub mod tokens;
pub use tokens::{handle_tokens, handle_tokens_callback};
pub mod ai;
pub use ai::handle_ai;
/// ✅🤖 <b>WiseTrader</b> 🧠 — Bạn có thể chọn một trong các lệnh sau
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// ✨  Các lệnh trợ giúp
    Help,
    /// Quay lại trạng thái bình thường (thoát dialogue)
    Back,
    /// ❓ Bắt đầu sử dụng BOT.
    Start,
    /// Xem thông tin của bạn
    Me,
    /// ℹ️  Thông tin tài khoản của khách hàng
    Info(String),
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
   /// Nạp tiền/điểm vào tài khoản
   Deposit,
   /// Xem số dư hiện tại
   Balance,
   /// Bắt đầu giao dịch với chiến lược đã chọn (deprecated, use LiveTrading)
   StartTrading,
   /// Bắt đầu giao dịch trực tiếp với exchange (Binance/OKX)
   LiveTrading,
   /// Quản lý OAuth tokens cho exchanges
   Tokens,
   /// Hỏi AI (Gemini) bất kỳ câu hỏi nào
   Ai(String),
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
    help_text.push_str(&format!("/starttrading - {}\n", i18n::translate(locale, "cmd_help_start_trading", None)));
    help_text.push_str(&format!("/livetrading - {}\n", i18n::translate(locale, "cmd_help_live_trading", None)));
    help_text.push_str(&format!("/tokens - {}\n", i18n::translate(locale, "cmd_help_tokens", None)));
    help_text.push_str(&format!("/backtest - {}\n", i18n::translate(locale, "cmd_help_backtest", None)));
    help_text.push_str(&format!("/back - {}\n", i18n::translate(locale, "cmd_help_back", None)));
    help_text.push_str(&format!("/deposit - {}\n", i18n::translate(locale, "cmd_help_deposit", None)));
    help_text.push_str(&format!("/balance - {}\n", i18n::translate(locale, "cmd_help_balance", None)));
    help_text.push_str(&format!("/ai - {}\n", i18n::translate(locale, "cmd_help_ai", None)));
    
    help_text.push_str(&i18n::translate(locale, "cmd_help_footer", None));
    
    bot.send_message(msg.chat.id, help_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    let duration = start_time.elapsed();
    tracing::info!("Time taken to handle /help command: {:?}", duration);
    Ok(())
}


pub async fn handle_invalid(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> anyhow::Result<()> {
    use crate::state::{BotState, CreateStrategyState, BacktestState, TradingState};
    use crate::i18n;
    use shared::entity::users;
    
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
    
    // Get dialogue state and provide context-aware message
    if let Ok(dialogue_state) = dialogue.get().await {
        if let Some(bot_state) = dialogue_state {
            let error_msg = match bot_state {
                BotState::WaitingForLanguage => {
                    i18n::translate(locale, "error_state_waiting_language", None)
                }
                BotState::CreateStrategy(CreateStrategyState::WaitingForAlgorithm) => {
                    i18n::translate(locale, "error_state_strategy_waiting_algorithm", None)
                }
                BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition { algorithm }) => {
                    i18n::translate(locale, "error_state_strategy_waiting_buy", Some(&[("algorithm", &algorithm)]))
                }
                BotState::CreateStrategy(CreateStrategyState::WaitingForSellCondition { algorithm, buy_condition }) => {
                    i18n::translate(locale, "error_state_strategy_waiting_sell", Some(&[
                        ("algorithm", &algorithm),
                        ("buy_condition", &buy_condition),
                    ]))
                }
                BotState::CreateStrategy(CreateStrategyState::WaitingForTimeframe { algorithm, buy_condition, sell_condition }) => {
                    i18n::translate(locale, "error_state_strategy_waiting_timeframe", None)
                }
                BotState::CreateStrategy(CreateStrategyState::WaitingForPair { algorithm, buy_condition, sell_condition, timeframe, strategy_name }) => {
                    i18n::translate(locale, "error_state_strategy_waiting_pair", None)
                }
                BotState::Backtest(BacktestState::WaitingForStrategy) => {
                    i18n::translate(locale, "error_state_backtest_waiting_strategy", None)
                }
                BotState::Backtest(BacktestState::WaitingForExchange { strategy_name, .. }) => {
                    i18n::translate(locale, "error_state_backtest_waiting_exchange", Some(&[("strategy_name", &strategy_name)]))
                }
                BotState::Backtest(BacktestState::WaitingForTimeRange { strategy_name, exchange, .. }) => {
                    i18n::translate(locale, "error_state_backtest_waiting_timerange", Some(&[
                        ("strategy_name", &strategy_name),
                        ("exchange", &exchange),
                    ]))
                }
                BotState::Trading(TradingState::WaitingForPair) => {
                    i18n::translate(locale, "error_state_trading_waiting_pair", None)
                }
                BotState::Trading(TradingState::WaitingForAmount) => {
                    i18n::translate(locale, "error_state_trading_waiting_amount", None)
                }
                BotState::Trading(TradingState::WaitingForConfirmation) => {
                    i18n::translate(locale, "error_state_trading_waiting_confirmation", None)
                }
                BotState::Normal => {
                    i18n::translate(locale, "error_invalid_command", None)
                }
                _ => {
                    i18n::translate(locale, "error_invalid_command", None)
                }
            };
            
            bot.send_message(msg.chat.id, error_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        } else {
            // No state, show default error
            let error_msg = i18n::translate(locale, "error_invalid_command", None);
            bot.send_message(msg.chat.id, error_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
    } else {
        // Error getting state, show default error
        let error_msg = i18n::translate(locale, "error_invalid_command", None);
        bot.send_message(msg.chat.id, error_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }
    
    Ok(())
}

/// Handler for /back command to exit dialogue and return to Normal state
pub async fn handle_back(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> anyhow::Result<()> {
    use crate::i18n;
    use crate::state::BotState;
    use shared::entity::users;
    
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
    
    // Check current state
    let current_state = dialogue.get().await?;
    
    // If already in Normal state, just send a message
    if let Some(BotState::Normal) = current_state {
        let msg_text = i18n::translate(locale, "back_already_normal", None);
        bot.send_message(msg.chat.id, msg_text)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }
    
    // Exit dialogue to Normal state
    dialogue.exit().await?;
    
    // Send confirmation message
    let msg_text = i18n::translate(locale, "back_success", None);
    bot.send_message(msg.chat.id, msg_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    
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