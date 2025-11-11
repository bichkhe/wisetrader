# Bot Examples

This directory contains example implementations demonstrating various trading strategies and integrations.

## RSI BNBUSDT Trading Strategies

### 1. Basic RSI Strategy (Manual Implementation)

**File:** `rsi_bnbusdt.rs`

### Description

This example demonstrates a basic RSI (Relative Strength Index) trading strategy for BNB/USDT pair using **simulated** price data:

- **Buy Signal:** When RSI < 30 (oversold condition)
- **Sell Signal:** When RSI > 70 (overbought condition)

### 2. Paper Trading with barter-rs (Recommended) ⭐

**File:** `rsi_bnbusdt_barter.rs`

### Description

This example demonstrates **Paper Trading** with **live market data** from Binance using barter-rs framework:

- ✅ **Live Market Data:** Real-time BNB/USDT trades from Binance via `barter-data`
- ✅ **RSI Calculation:** Using `ta-rs` library for technical analysis
- ✅ **Paper Trading:** Mock execution with virtual balance tracking
- ✅ **Strategy:** Buy when RSI < 30, Sell when RSI > 70

### Features

- ✅ RSI indicator calculation (14-period standard)
- ✅ Real-time price monitoring simulation
- ✅ Buy/Sell signal detection
- ✅ Position tracking with profit/loss calculation
- ✅ Logging for all trading actions

### Running the Examples

**Basic Example (Simulated Data):**
```bash
# From the bot directory
cargo run --example rsi_bnbusdt

# Or with logging
RUST_LOG=info cargo run --example rsi_bnbusdt
```

**Paper Trading with Live Data (Recommended):**
```bash
# From the bot directory
cargo run --example rsi_bnbusdt_barter

# Or with detailed logging
RUST_LOG=info cargo run --example rsi_bnbusdt_barter
```

Press `Ctrl+C` to stop and see trading summary.

## Gemini API Integration Test

### 3. Test Gemini API

**File:** `test_gemini.rs`

### Description

This example demonstrates how to test the Gemini API integration for backtest analysis:

- ✅ **Environment Variable:** Loads `GEMINI_API_KEY` from `.env` file or environment
- ✅ **GeminiService:** Creates and uses GeminiService to call Gemini API
- ✅ **Backtest Analysis:** Tests `analyze_backtest` method with sample data
- ✅ **Error Handling:** Proper error handling and logging

### Features

- ✅ Loads API key from environment variables
- ✅ Tests simple backtest analysis request
- ✅ Tests detailed backtest analysis with tables
- ✅ Displays AI-generated analysis in Vietnamese
- ✅ Comprehensive error handling

### Running the Example

**Prerequisites:**
1. Set `GEMINI_API_KEY` in your `.env` file or environment:
   ```bash
   export GEMINI_API_KEY="your-api-key-here"
   ```

**Run the test:**
```bash
# From the bot directory
cargo run --example test_gemini

# Or with detailed logging
RUST_LOG=info cargo run --example test_gemini
```

### Implementation Details

**Gemini Test Example (`test_gemini.rs`):**
- Standalone implementation of GeminiService for testing
- Tests two scenarios:
  1. Simple analysis with minimal data
  2. Detailed analysis with trade tables
- Uses Vietnamese prompts for analysis
- Handles API errors gracefully

## Implementation Details

**Basic Example (`rsi_bnbusdt.rs`):**
- This is a **simulated** implementation that generates random price data
- Good for testing strategy logic without network dependency 

**Paper Trading Example (`rsi_bnbusdt_barter.rs`):**
- Uses **barter-data** to stream live trades from Binance WebSocket
- Uses **ta-rs** library for RSI calculation
- Implements paper trading with virtual balance tracking
- Shows profit/loss calculations
- Handles reconnection automatically
- Press `Ctrl+C` to view trading summary

### For Production/Live Trading

To convert to live trading, you would need to:
1. Replace mock execution with `barter-execution` for real order execution
2. Add API keys and authentication
3. Implement proper risk management (stop-loss, take-profit, position sizing)
4. Add order validation and error handling
5. Consider adding position limits and portfolio management

### Dependencies Used

The `rsi_bnbusdt_barter.rs` example uses:

```toml
[dependencies]
barter-data = "0.10.2"      # For WebSocket data streams from Binance
barter-instrument = "0.3.1" # For market instruments
ta = "0.5"                  # For technical analysis indicators (RSI)
futures = "0.3"             # For async stream utilities
tokio = { version = "1.45", features = ["full"] }
tracing = "0.1"             # For logging
```

**Note:** See the example for full dependency list.

### Next Steps

1. **Connect to Binance WebSocket** - Replace price simulation with real WebSocket stream
2. **Add barter-rs integration** - Use `barter-data` for normalized market data
3. **Implement order execution** - Use `barter-execution` or direct Binance API
4. **Add risk management** - Implement stop-loss, take-profit, position limits
5. **Backtesting** - Test strategy on historical data before live trading

### Disclaimer

⚠️ **This is for educational purposes only. Trading cryptocurrencies involves significant risk. Always test strategies thoroughly before using real money.**

