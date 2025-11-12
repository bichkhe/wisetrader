use reqwest::Client;
use serde_json::json;
use teloxide::{
    Bot,
    payloads::SetMyCommandsSetters,
    prelude::Requester,
    types::{BotCommand, BotCommandScope},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();
    let token_dev = "8222061674:AAGU2JiydFyAA4AhX1g6vqueSmNhAs3psLs"; // -- dev: wisetrader
    let token_prod = "-"; // -- prod: wisetrader
    let url = format!("https://api.telegram.org/bot{}/setMyCommands", token_dev);

    let commands = json!({
        "commands": [
        { "command": "start", "description": "🚀 Start the bot and register" },
        { "command": "help", "description": "ℹ️ Show this help message" },
        { "command": "version", "description": "🆚 Show bot version information" },
        { "command": "me", "description": "👤 Show your user profile" },
        { "command": "createstrategy", "description": "🛠️ Create a new trading strategy" },
        { "command": "mystrategies", "description": "📋 List all your strategies" },
        // { "command": "starttrading", "description": "💹 Start trading with a selected strategy" },
        { "command": "backtest", "description": "🔎 Run backtest on a strategy" },
        { "command": "back", "description": "🔙 Exit current dialogue and return to normal state" },
        { "command": "deposit", "description": "➕ Deposit points to your account" },
        { "command": "balance", "description": "💰 View your current balance" },
        { "command": "livetrading", "description": "💹 Start live trading with exchange" },
        { "command": "tokens", "description": "🔑 Manage OAuth tokens for exchanges" },
        { "command": "ai", "description": "🤖 Ask AI (Gemini) any question" },
        { "command": "mytrading", "description": "📊 View your live trading status" },
        { "command": "pnl", "description": "💰 View profit and loss statistics" },
        ],
        "scope": { "type": "default" }
    });

    let admin_commands = json!({
        "commands": [
            { "command": "start", "description": "🚀 Start the bot and register" },
        { "command": "help", "description": "ℹ️ Show this help message" },
        { "command": "version", "description": "🆚 Show bot version information" },
        { "command": "me", "description": "👤 Show your user profile" },
        { "command": "createstrategy", "description": "🛠️ Create a new trading strategy" },
        { "command": "mystrategies", "description": "📋 List all your strategies" },
        // { "command": "starttrading", "description": "💹 Start trading with a selected strategy" },
        { "command": "backtest", "description": "🔎 Run backtest on a strategy" },
        { "command": "back", "description": "🔙 Exit current dialogue and return to normal state" },
        { "command": "deposit", "description": "➕ Deposit points to your account" },
        { "command": "balance", "description": "💰 View your current balance" },
        { "command": "livetrading", "description": "💹 Start live trading with exchange" },
        { "command": "tokens", "description": "🔑 Manage OAuth tokens for exchanges" },
        { "command": "ai", "description": "🤖 Ask AI (Gemini) any question" },
        { "command": "mytrading", "description": "📊 View your live trading status" },
        { "command": "pnl", "description": "💰 View profit and loss statistics" },
        { "command": "streams", "description": "📡 View active market data streams" },
        ],
        "scope": { "type": "default" }
    });

    let client = Client::new();
    let resp = client.post(&url).json(&commands).send().await?;

    println!("{:?}", resp.text().await?);
    Ok(())
}