# WiseTrader - Next Steps to Fix Compilation

## Current Status

✅ **Infrastructure Running:**
- MySQL container running on port 3306
- Redis container running on port 6379
- Database schema initialized with tables and seed data

⚠️ **Compilation Issues:**
- Teloxide BotCommands trait not deriving properly
- Handler error conversions need fixing
- Bot command descriptions missing

## Required Fixes

### 1. Fix Teloxide Imports

Update `bot/src/handlers.rs` to properly import BotCommands:

```rust
use teloxide::utils::command::BotCommands;
```

### 2. Fix Handler Return Types

The main issue is that handler functions return `Result<()>` but teloxide expects them to properly integrate with ResponseResult.

Simplest fix - wrap handlers properly:

```rust
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
    pool: MySqlPool,
) -> ResponseResult<()> {
    let result = match cmd {
        BotCommand::Start => handle_start(bot, msg, pool).await,
        BotCommand::Help => handle_help(bot, msg).await,
        BotCommand::Subscription => handle_subscription(bot, msg, pool).await,
        BotCommand::Strategies => handle_strategies(bot, msg, pool).await,
        BotCommand::MyStrategies => handle_my_strategies(bot, msg, pool).await,
    };
    
    result.map_err(|e| {
        tracing::error!("Handler error: {}", e);
        teloxide::RequestError::Other(e.into())
    })
}
```

### 3. Fix BotCommand::descriptions()

In `handle_help`, replace:
```rust
let help_text = BotCommand::descriptions();
```

With:
```rust
let help_text = BotCommand::descriptions().to_string();
```

## Alternative: Simplify Error Handling

If the above doesn't work, convert all handler errors properly:

```rust
// In each handler
.map_err(|e| teloxide::RequestError::Other(Box::new(e)))
```

## Testing After Fixes

1. **Compile**: `cargo build --bin bot`
2. **Run**: Create `.env` with `BOT_TOKEN=your_token` and run `cargo run --bin bot`
3. **Test Commands**: ep in Telegram and send `/start`

## Files to Update

1. `bot/src/handlers.rs` - Fix imports and error conversions
2. `bot/Cargo.toml` - Ensure teloxide features are correct (already done)

## Current Project Structure

```
✅ docker-compose.yml - Running
✅ docker/mysql/init.sql - Database ready
✅ bot/src/main.rs - Created
✅ bot/src/handlers.rs - Needs fixes
✅ shared/src/ - Models, DB, Redis ready
✅ api/src/main.rs - API skeleton ready
⚠️  Compilation errors to resolve
```

## Commands to Run

```bash
# Start infrastructure (already running)
docker-compose up -d

# After fixes
cargo build --bin bot
cargo run --bin bot

# Test with Telegram
# Send /start to your bot
```

## Quick Win Path

If stuck, create a minimal working bot first:

```rust
// Minimal bot/main.rs
use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    let bot = Bot::from_env();
    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        bot.send_message(msg.chat.id, "Hello!").await?;
        Ok(())
    })
    .await;
}
```

Then gradually add complexity.

