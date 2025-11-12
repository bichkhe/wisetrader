# Logging Configuration Guide

## Overview

WiseTrader bot uses `tracing` for logging and supports configurable log levels via the `RUST_LOG` environment variable.

## Configuration

### Environment Variable: `RUST_LOG`

The bot reads the `RUST_LOG` environment variable to determine which logs to display. If not set, it defaults to `info` level.

### Log Levels

- `error` - Only error messages
- `warn` - Warnings and errors
- `info` - Info, warnings, and errors (default)
- `debug` - Debug, info, warnings, and errors
- `trace` - All logs (most verbose)

### Examples

#### Show all debug logs
```bash
RUST_LOG=debug cargo run --bin bot
```

#### Show debug logs only for bot crate
```bash
RUST_LOG=bot=debug cargo run --bin bot
```

#### Show debug logs for specific module
```bash
RUST_LOG=bot::services::trading_signal=debug cargo run --bin bot
```

#### Show debug for multiple modules
```bash
RUST_LOG=bot::services::trading_signal=debug,bot::services::strategy_engine=debug cargo run --bin bot
```

#### Show info for most modules, but debug for trading_signal
```bash
RUST_LOG=info,bot::services::trading_signal=debug cargo run --bin bot
```

### Docker Compose

When running with Docker Compose, set `RUST_LOG` in your `.env` file or in `docker-compose.yml`:

```yaml
environment:
  - RUST_LOG=${RUST_LOG:-info}  # Defaults to 'info' if not set
```

Then in your `.env` file:
```bash
RUST_LOG=debug
```

Or override when running:
```bash
RUST_LOG=debug docker-compose up bot
```

### Common Use Cases

#### Debug Live Trading Issues
```bash
RUST_LOG=bot::services::trading_signal=debug,bot::services::strategy_engine=debug
```

#### Debug RSI Strategy
```bash
RUST_LOG=bot::services::strategy_engine::implementations=debug
```

#### Debug Stream Management
```bash
RUST_LOG=bot::services::trading_signal=debug
```

#### Full Debug Mode (Very Verbose)
```bash
RUST_LOG=debug
```

### Log Format

Logs are formatted with:
- Timestamp
- Log level
- Module path
- Message

Example:
```
2024-01-15T10:30:45.123Z INFO bot::services::trading_signal: Starting User Trading Service for user 12345
```

### Tips

1. **Start with `info`** - Most logs are at info level, which is usually sufficient
2. **Use module-specific debug** - Instead of `RUST_LOG=debug`, target specific modules to reduce noise
3. **Production** - Use `RUST_LOG=warn` or `RUST_LOG=error` in production to reduce log volume
4. **Development** - Use `RUST_LOG=debug` when developing or troubleshooting

### Available Log Points

Key modules that emit logs:
- `bot::services::trading_signal` - Market data streams, trading signals
- `bot::services::strategy_engine` - Strategy execution and evaluation
- `bot::services::strategy_engine::implementations` - RSI, MACD, etc. strategy implementations
- `bot::commands::live_trading` - Live trading command handlers
- `bot::commands::backtest` - Backtest command handlers

