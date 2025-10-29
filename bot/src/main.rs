use anyhow::Result;
use std::sync::Arc;
use teloxide::{dispatching::{UpdateHandler, dialogue}, prelude::*};
use teloxide::{dispatching::dialogue::InMemStorage};
mod commands;
mod state;
mod services;
mod repositories;

use crate::{commands::{handle_invalid, handle_invalid_callback, handle_version,
    handle_me,handle_help, Command},  state::AppState};
use state::BotState;

fn schema() -> UpdateHandler<anyhow::Error> {
    use dptree::case;
    let command_handler = teloxide::filter_command::<Command, _>().branch(
        case![BotState::Normal]
            .branch(case![Command::Version].endpoint(handle_version))
            .branch(case![Command::Me].endpoint(handle_me))
            .branch(case![Command::Help].endpoint(handle_help))
    );

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![BotState::Subscription].endpoint(handle_invalid))
        .branch(dptree::endpoint(handle_invalid));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![BotState::Subscription].endpoint(handle_invalid_callback));
        

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
