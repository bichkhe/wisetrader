use anyhow::Result;
use std::sync::Arc;
use teloxide::{dispatching::{UpdateHandler, dialogue}, prelude::*};
use teloxide::{dispatching::dialogue::InMemStorage};
mod commands;
mod state;
mod services;
mod repositories;
mod i18n;

// Initialize i18n at crate root (required by rust-i18n)
rust_i18n::i18n!("locales", fallback = "en");


use crate::{commands::{handle_invalid, handle_version,
    handle_me,handle_help,handle_backtest_wizard, handle_backtest_callback,
    handle_create_strategy, handle_strategy_callback, 
    handle_strategy_input_callback, handle_my_strategies, 
    handle_start, handle_language_selection, handle_language_callback, handle_profile_callback, Command},  state::AppState};
use state::{BotState, BacktestState};

fn schema() -> UpdateHandler<anyhow::Error> {
    use dptree::case;
    // Start command can be used in ANY state, so handle it separately first
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(handle_start))
        .branch(
            case![BotState::Normal]
                .branch(case![Command::Version].endpoint(handle_version))
                .branch(case![Command::Me].endpoint(handle_me))
                .branch(case![Command::Help].endpoint(handle_help))
                .branch(case![Command::Backtest(pk)].endpoint(handle_backtest_wizard))
                .branch(case![Command::CreateStrategy].endpoint(handle_create_strategy))
                .branch(case![Command::MyStrategies].endpoint(handle_my_strategies))
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
    let app_state = Arc::new(AppState::new().await?);
    tracing::info!("AppState initialized");

    // Create bot
    let bot = Bot::new(&app_state.bot_token);
    tracing::info!("Bot created");

    // Spawn the dispatcher in a separate Tokio task (thread)
    let mut dispatcher = Dispatcher::builder(bot.clone(), schema())
        .dependencies(dptree::deps![
            InMemStorage::<BotState>::new(),
            app_state.clone()
        ])
        .enable_ctrlc_handler()
        .build();

    tracing::info!("Bot is running and waiting for updates...");
    dispatcher.dispatch().await;

    Ok(())
}
