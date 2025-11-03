use anyhow::Result;
use std::sync::Arc;
use teloxide::{dispatching::{UpdateHandler, dialogue}, prelude::*};
use teloxide::{dispatching::dialogue::InMemStorage};
use tracing::{info, warn, error};
use url::Url;
mod commands;
mod state;
mod services;
mod repositories;
mod i18n;

use services::trading_signal;

// Initialize i18n at crate root (required by rust-i18n)
rust_i18n::i18n!("locales", fallback = "en");


use crate::{commands::{handle_invalid, handle_version,
    handle_me,handle_help,handle_backtest_wizard, handle_backtest_callback,
    handle_create_strategy, handle_strategy_callback, 
    handle_strategy_input_callback, handle_my_strategies,
    handle_delete_strategy_callback,
    handle_start, handle_language_selection, handle_language_callback, handle_profile_callback,
    handle_start_trading, handle_start_trading_callback,
    handle_back, handle_deposit, handle_balance, handle_deposit_callback,
    Command},  state::AppState};
use state::{BotState, BacktestState};

fn schema() -> UpdateHandler<anyhow::Error> {
    use dptree::case;
    // Start and Back commands can be used in ANY state, so handle them separately first
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(handle_start))
        .branch(case![Command::Back].endpoint(handle_back))
        .branch(
            case![BotState::Normal]
                .branch(case![Command::Version].endpoint(handle_version))
                .branch(case![Command::Me].endpoint(handle_me))
                .branch(case![Command::Help].endpoint(handle_help))
                .branch(case![Command::Backtest(pk)].endpoint(handle_backtest_wizard))
                .branch(case![Command::CreateStrategy].endpoint(handle_create_strategy))
                .branch(case![Command::MyStrategies].endpoint(handle_my_strategies))
                .branch(case![Command::Deposit].endpoint(handle_deposit))
                .branch(case![Command::Balance].endpoint(handle_balance))
                .branch(case![Command::StartTrading].endpoint(handle_start_trading))
        );

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(
            case![BotState::CreateStrategy(pk)]
                .endpoint(handle_strategy_input_callback)
        )
        .branch(case![BotState::Normal].endpoint(handle_invalid))
        .branch(dptree::endpoint(handle_invalid));

    let callback_query_handler = Update::filter_callback_query()
        // Handle delete strategy callbacks from any state FIRST (before other handlers)
        .branch(
            dptree::filter(|q: CallbackQuery| {
                q.data.as_ref().map(|d| d.starts_with("delete_strategy_") || d.starts_with("delete_confirm_") || d == "delete_cancel").unwrap_or(false)
            })
            .endpoint(handle_delete_strategy_callback)
        )
        .branch(
            // Language selection can happen in WaitingForLanguage state
            case![BotState::WaitingForLanguage]
                .endpoint(handle_language_selection)
        )
        .branch(
            // Handle language selection callbacks (lang_select_vi, lang_select_en) from any state
            dptree::filter(|q: CallbackQuery| {
                q.data.as_ref().map(|d| d == "lang_select_vi" || d == "lang_select_en").unwrap_or(false)
            })
            .endpoint(handle_language_callback)
        )
        .branch(
            // Handle profile callbacks (like change language button) in Normal state
            case![BotState::Normal]
                .endpoint(handle_profile_callback)
        )
        .branch(
            case![BotState::CreateStrategy(pk)]
                .endpoint(handle_strategy_callback)
        )
        .branch(
            case![BotState::Backtest(pk)]
                .endpoint(handle_backtest_callback)
        )
        .branch(
            // Handle payment/deposit callbacks from any state
            dptree::filter(|q: CallbackQuery| {
                q.data.as_ref().map(|d| d.starts_with("deposit_") || d == "deposit_cancel" || d == "deposit_start").unwrap_or(false)
            })
            .endpoint(handle_deposit_callback)
        )
        .branch(
            // Handle start trading callbacks from any state
            dptree::filter(|q: CallbackQuery| {
                q.data.as_ref().map(|d| d.starts_with("start_trading_") || d == "cancel_start_trading").unwrap_or(false)
            })
            .endpoint(handle_start_trading_callback)
        );
        

    dialogue::enter::<Update, InMemStorage<BotState>, BotState, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting WiseTrader bot...");

    // Initialize AppState with configuration
    tracing::info!("Initializing AppState...");
    let app_state = match AppState::new().await {
        Ok(state) => {
            tracing::info!("AppState initialized successfully");
            Arc::new(state)
        }
        Err(e) => {
            tracing::error!("Failed to initialize AppState: {:?}", e);
            tracing::error!("Error details: {}", e);
            return Err(e);
        }
    };
    tracing::info!("AppState wrapped in Arc");

    // Create bot - teloxide will handle timeout internally
    // We'll configure error handling instead
    let bot = Bot::new(&app_state.bot_token);
    tracing::info!("Bot created");

    // Spawn the dispatcher with error handler for network timeout recovery
    let mut dispatcher = Dispatcher::builder(bot.clone(), schema())
        .dependencies(dptree::deps![
            InMemStorage::<BotState>::new(),
            app_state.clone()
        ])
        .enable_ctrlc_handler()
        .build();

    // Start trading signal service if channel ID is configured
    if let Ok(channel_id_str) = std::env::var("TRADING_SIGNAL_CHANNEL_ID") {
        if let Ok(channel_id) = channel_id_str.parse::<i64>() {
            let app_state_signal = app_state.clone();
            let bot_signal = bot.clone();
            trading_signal::start_trading_signal_service(
                app_state_signal,
                bot_signal,
                channel_id,
                "BNB/USDT".to_string(),
            );
            info!("✅ Trading Signal Service started for channel: {}", channel_id);
        } else {
            warn!("⚠️  Invalid TRADING_SIGNAL_CHANNEL_ID, trading signals disabled");
        }
    } else {
        info!("ℹ️  TRADING_SIGNAL_CHANNEL_ID not set, trading signals disabled");
    }

    // Check if webhook mode is enabled
    if let Some(webhook_url) = &app_state.config.webhook_url {
        // WEBHOOK MODE
        info!("🌐 Starting bot in WEBHOOK mode");
        info!("📡 Webhook URL: {}", webhook_url);
        info!("🔗 Webhook path: {}", app_state.config.webhook_path);
        info!("🔌 Listening on port: {}", app_state.config.webhook_port);
        
        // Delete old webhook if exists (cleanup)
        bot.delete_webhook().await?;
        info!("🧹 Old webhook deleted");
        
        // Set new webhook
        let webhook_url_full = format!("{}{}", webhook_url, app_state.config.webhook_path);
        let webhook_url_parsed = Url::parse(&webhook_url_full)?;
        bot.set_webhook(webhook_url_parsed).await?;
        info!("✅ Webhook set: {}", webhook_url_full);
        
        // Create webhook listener using teloxide's built-in webhook support
        use teloxide::update_listeners::webhooks;
        use std::net::SocketAddr;
        
        let addr = SocketAddr::from(([0, 0, 0, 0], app_state.config.webhook_port));
        let path = app_state.config.webhook_path.parse()?;
        
        info!("🚀 Starting webhook server on {}", addr);
        info!("📥 Webhook endpoint: {}", app_state.config.webhook_path);
        
        // Create webhook listener with Axum
        // axum_to_router returns (listener, server_future, stop_token)
        // The server_future needs to be spawned to run the HTTP server
        let (listener, server_future, _stop_token) = webhooks::axum_to_router(
            bot.clone(),
            webhooks::Options::new(addr, path),
        )
        .await?;
        
        // Start Axum server in background - this runs the HTTP server
        info!("🌐 Starting webhook HTTP server on {}", addr);
        tokio::spawn(async move {
            server_future.await;
        });
        
        // Start dispatcher with webhook listener
        // The listener already handles receiving updates from Telegram
        // Use teloxide's built-in IgnoringErrorHandlerSafe for Infallible errors
        use teloxide::error_handlers::IgnoringErrorHandlerSafe;
        use std::sync::Arc;
        dispatcher.dispatch_with_listener(listener, Arc::new(IgnoringErrorHandlerSafe)).await;
    } else {
        // POLLING MODE (fallback)
        warn!("📡 Webhook URL not set, using POLLING mode");
        warn!("💡 To use webhook mode, set WEBHOOK_URL environment variable");
        
        tracing::info!("Bot is running and waiting for updates...");
        
        // Note: The "TimedOut" errors from update listener are expected during network issues.
        // Teloxide automatically retries connections. These errors don't crash the bot.
        dispatcher.dispatch().await;
    }
    
    Ok(())
}
