# Freqtrade-RS

A high-performance Rust implementation of Freqtrade trading bot using [barter-rs](https://github.com/barter-rs/barter) for exchange integration and [ta-rs](https://github.com/greyblake/ta-rs) for technical analysis.

## Features

### ✅ Core Features (Planned)

1. **Data Management**
   - ✅ OHLCV candle data fetching and storage
   - ✅ Historical data management
   - ✅ Real-time data streaming
   - ✅ Data validation and cleaning

2. **Technical Indicators** (using `ta` crate)
   - ✅ RSI (Relative Strength Index)
   - ✅ MACD (Moving Average Convergence Divergence)
   - ✅ EMA (Exponential Moving Average)
   - ✅ SMA (Simple Moving Average)
   - ✅ BB (Bollinger Bands)
   - ✅ Stochastic Oscillator
   - ✅ ATR (Average True Range)

3. **Strategy Engine**
   - ✅ Strategy definition and validation
   - ✅ Entry/Exit signal generation
   - ✅ Position sizing
   - ✅ Risk management

4. **Backtesting Engine**
   - ✅ Historical backtesting
   - ✅ Performance metrics calculation
   - ✅ Trade analysis and reporting
   - ✅ Walk-forward optimization

5. **Portfolio Management**
   - ✅ Position tracking
   - ✅ Balance management
   - ✅ Risk calculation
   - ✅ Trade history

6. **Exchange Integration** (using `barter-rs`)
   - ✅ Binance Spot & Futures
   - ✅ Kraken
   - ✅ OKX
   - ✅ Order placement
   - ✅ Order management
   - ✅ Market data streaming

7. **Configuration & Runtime**
   - ✅ Configuration management
   - ✅ Strategy parameters
   - ✅ Risk parameters
   - ✅ Runtime state management

## Architecture

```
freqtrade-rs/
├── src/
│   ├── lib.rs                 # Library entry point
│   ├── data/                  # Data management
│   │   ├── mod.rs
│   │   ├── candle.rs          # OHLCV candle data
│   │   └── storage.rs          # Data storage
│   ├── indicators/            # Technical indicators
│   │   ├── mod.rs
│   │   ├── rsi.rs
│   │   ├── macd.rs
│   │   ├── ema.rs
│   │   ├── sma.rs
│   │   └── bb.rs
│   ├── strategy/              # Strategy engine
│   │   ├── mod.rs
│   │   ├── base.rs            # Base strategy trait
│   │   ├── signal.rs          # Entry/exit signals
│   │   └── validator.rs       # Strategy validation
│   ├── backtest/              # Backtesting engine
│   │   ├── mod.rs
│   │   ├── engine.rs          # Backtest engine
│   │   ├── metrics.rs         # Performance metrics
│   │   └── report.rs          # Report generation
│   ├── portfolio/             # Portfolio management
│   │   ├── mod.rs
│   │   ├── position.rs        # Position tracking
│   │   ├── balance.rs         # Balance management
│   │   └── risk.rs            # Risk management
│   ├── exchange/              # Exchange integration
│   │   ├── mod.rs
│   │   ├── client.rs          # Exchange client wrapper
│   │   └── order.rs           # Order management
│   └── config/                # Configuration
│       ├── mod.rs
│       ├── strategy.rs         # Strategy config
│       └── risk.rs             # Risk config
└── examples/
    ├── simple_strategy.rs
    └── backtest_example.rs
```

## Usage

### Basic Strategy Example

```rust
use freqtrade_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize exchange client
    let client = ExchangeClient::new("binance").await?;
    
    // Create strategy
    let strategy = RSIStrategy::new(
        RSIStrategyConfig {
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            timeframe: "5m".parse()?,
        }
    );
    
    // Run strategy
    let engine = TradingEngine::new(client, strategy);
    engine.run().await?;
    
    Ok(())
}
```

## Development Status

- [x] Project structure
- [ ] Data management module
- [ ] Technical indicators module
- [ ] Strategy engine module
- [ ] Backtesting engine module
- [ ] Portfolio management module
- [ ] Exchange integration module
- [ ] Configuration module

## License

MIT

