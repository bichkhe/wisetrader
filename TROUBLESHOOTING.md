# Troubleshooting Guide

## Current Compilation Issues

### Issue 1: Teloxide Feature Configuration

**Problem**: Teloxide feature flags need to be properly configured

**Solution**: Update workspace Cargo.toml:
```toml
teloxide = { version = "0.12", features = ["macros"] }
```

Then update bot/Cargo.toml:
```toml
teloxide = { workspace = true }
```

### Issue 2: BigDecimal Type

**Problem**: BigDecimal doesn't integrate well with sqlx for MySQL DECIMAL types

**Solution**: Already fixed - using String types instead
- Price fields now use `String` instead of `BigDecimal`
- This is acceptable for initial MVP

### Issue 3: Handler Function Return Types

**Problem**: Handler functions need to return `ResponseResult<()>` properly

**Current state**: Functions return `Result<()>` but teloxide expects proper error conversion

**Fix needed**: Wrap errors properly or use `.map_err()` conversions

## Quick Fix Steps

1. **Update Cargo.toml**:
   ```bash
   # In Cargo.toml (workspace), update teloxide to:
   teloxide = { version = "0.12", features = ["macros"] }
   ```

2. **Fix handlers.rs error conversions**:
   ```rust
   // Change from:
   match cmd {
       BotCommand::Start => handle_start(bot, msg, pool).await?,
       ...
   }
   
   // To:
   match cmd {
       BotCommand::Start => handle_start(bot, msg, pool).await?,
       ...
   }
   // Make sure handle_start returns Result that can be converted
   ```

3. **Clean and rebuild**:
   ```bash
   cargo clean
   cargo update
   cargo check
   ```

## Alternative Approach: Simplify Handler Errors

If compilation issues persist, simplify error handling:

```rust
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
    pool: MySqlPool,
) -> Result<()> {
    match cmd {
        BotCommand::Start => handle_start(bot, msg, pool).await,
        BotCommand::Help => handle_help(bot, msg).await,
        BotCommand::Subscription => handle_subscription(bot, msg, pool).await,
        BotCommand::Strategies => handle_strategies(bot, msg, pool).await,
        BotCommand::MyStrategies => handle_my_strategies(bot, msg, pool).await,
    }?;
    Ok(())
}
```

## Testing Infrastructure

### Check if Docker services are running:
```bash
docker-compose ps
```

### Check MySQL logs:
```bash
docker-compose logs mysql
```

### Check database connection:
```bash
docker exec -it wisetrader_mysql mysql -u wisetrader -pwisetraderpass wisetrader -e "SHOW TABLES;"
```

### Test Redis:
```bash
docker exec -it wisetrader_redis redis-cli ping
```

## Current Working State

✅ Docker containers started successfully
✅ MySQL and Redis are up
✅ Database schema initialized
✅ Basic project structure complete

⚠️ Compilation errors in Rust code
❌ Bot not yet runnable

## Next Actions

1. Fix teloxide compilation errors
2. Test bot compilation
3. Test database queries
4. Run bot and test /start command

## Getting Help

- Check teloxide documentation: https://docs.rs/teloxide/
- Check teloxide examples: https://github.com/teloxide/teloxide/tree/master/examples
- Cargo version issues: Run `cargo update`

