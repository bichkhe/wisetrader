//! Backtesting engine

use crate::data::{Candle, CandleSeries};
use crate::strategy::{Strategy, Signal, SignalType};
use crate::portfolio::{Balance, Position, PositionSide};
use crate::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Backtest result
#[derive(Debug, Clone)]
pub struct BacktestResult {
    /// Starting balance
    pub start_balance: f64,
    /// Ending balance
    pub end_balance: f64,
    /// Total return
    pub total_return: f64,
    /// Total return percentage
    pub total_return_percent: f64,
    /// Number of trades
    pub num_trades: usize,
    /// Winning trades
    pub winning_trades: usize,
    /// Losing trades
    pub losing_trades: usize,
    /// Win rate
    pub win_rate: f64,
    /// Average profit
    pub avg_profit: f64,
    /// Average loss
    pub avg_loss: f64,
    /// Maximum drawdown
    pub max_drawdown: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
}

/// Backtesting engine
pub struct BacktestEngine {
    /// Initial balance
    pub initial_balance: f64,
    balance: Balance,
    positions: Vec<Position>,
    trades: Vec<Trade>,
}

/// Trade record
#[derive(Debug, Clone)]
pub struct Trade {
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub symbol: String,
    pub side: PositionSide,
    pub entry_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub pnl: f64,
    pub pnl_percent: f64,
}

impl BacktestEngine {
    /// Create new backtest engine
    pub fn new(initial_balance: f64) -> Self {
        Self {
            initial_balance,
            balance: Balance::new(initial_balance),
            positions: Vec::new(),
            trades: Vec::new(),
        }
    }

    /// Run backtest
    pub fn run<T: Strategy>(
        &mut self,
        strategy: &mut T,
        candles: &CandleSeries,
    ) -> Result<BacktestResult> {
        // Initialize strategy
        strategy.initialize(candles.candles())?;

        // Process each candle
        for candle in candles.candles() {
            if !strategy.is_ready() {
                continue;
            }

            // Update existing positions
            self.update_positions(candle);

            // Generate signal
            let signal = strategy.process(candle)?;

            // Execute signal
            self.execute_signal(&signal, candle)?;
        }

        // Close all remaining positions
        self.close_all_positions(candles.candles().last().unwrap());

        // Calculate results
        self.calculate_results()
    }

    /// Update positions with new candle price
    fn update_positions(&mut self, candle: &Candle) {
        for position in &mut self.positions {
            position.update_price(candle.close);
        }
    }

    /// Execute trading signal
    fn execute_signal(&mut self, signal: &Signal, candle: &Candle) -> Result<()> {
        match signal.signal_type {
            SignalType::Buy => {
                // Close short positions first
                self.close_positions_for_symbol(&candle.symbol, PositionSide::Short, candle);

                // Open long position
                if let Some(entry_price) = signal.entry_price {
                    let quantity = self.calculate_position_size(entry_price);
                    if quantity > 0.0 {
                        let mut position = Position::new(
                            Uuid::new_v4().to_string(),
                            candle.symbol.clone(),
                            PositionSide::Long,
                            entry_price,
                            quantity,
                        );
                        position.set_stop_loss(signal.stop_loss.unwrap_or(entry_price * 0.95));
                        position.set_take_profit(signal.take_profit.unwrap_or(entry_price * 1.05));

                        self.positions.push(position);
                        self.balance.in_positions += entry_price * quantity;
                    }
                }
            }
            SignalType::Sell => {
                // Close long positions
                self.close_positions_for_symbol(&candle.symbol, PositionSide::Long, candle);
            }
            SignalType::Hold => {
                // Do nothing
            }
        }

        Ok(())
    }

    /// Close positions for symbol and side
    fn close_positions_for_symbol(
        &mut self,
        symbol: &str,
        side: PositionSide,
        candle: &Candle,
    ) {
        let positions_to_close: Vec<_> = self
            .positions
            .iter()
            .enumerate()
            .filter(|(_, p)| p.symbol == symbol && p.side == side)
            .map(|(i, _)| i)
            .collect();

        for &index in positions_to_close.iter().rev() {
            let position = self.positions.remove(index);
            let trade = Trade {
                entry_time: position.entry_time,
                exit_time: candle.timestamp,
                symbol: position.symbol.clone(),
                side: position.side,
                entry_price: position.entry_price,
                exit_price: candle.close,
                quantity: position.quantity,
                pnl: position.unrealized_pnl,
                pnl_percent: position.unrealized_pnl_percent,
            };

            self.trades.push(trade);
            self.balance.total += position.unrealized_pnl;
            self.balance.in_positions -= position.entry_value();
        }
    }

    /// Close all positions
    fn close_all_positions(&mut self, candle: &Candle) {
        let symbols: Vec<_> = self.positions.iter().map(|p| p.symbol.clone()).collect();
        for symbol in symbols {
            self.close_positions_for_symbol(&symbol, PositionSide::Long, candle);
            self.close_positions_for_symbol(&symbol, PositionSide::Short, candle);
        }
    }

    /// Calculate position size
    fn calculate_position_size(&self, entry_price: f64) -> f64 {
        let max_position_value = self.balance.available * 0.1; // 10% of available
        max_position_value / entry_price
    }

    /// Calculate backtest results
    fn calculate_results(&self) -> Result<BacktestResult> {
        let total_return = self.balance.total - self.initial_balance;
        let total_return_percent = (total_return / self.initial_balance) * 100.0;

        let winning_trades = self.trades.iter().filter(|t| t.pnl > 0.0).count();
        let losing_trades = self.trades.iter().filter(|t| t.pnl < 0.0).count();
        let win_rate = if self.trades.is_empty() {
            0.0
        } else {
            (winning_trades as f64 / self.trades.len() as f64) * 100.0
        };

        let avg_profit = if winning_trades > 0 {
            self.trades
                .iter()
                .filter(|t| t.pnl > 0.0)
                .map(|t| t.pnl)
                .sum::<f64>()
                / winning_trades as f64
        } else {
            0.0
        };

        let avg_loss = if losing_trades > 0 {
            self.trades
                .iter()
                .filter(|t| t.pnl < 0.0)
                .map(|t| t.pnl)
                .sum::<f64>()
                / losing_trades as f64
        } else {
            0.0
        };

        // Calculate max drawdown
        let mut max_drawdown = 0.0;
        let mut peak = self.initial_balance;
        let mut current = self.initial_balance;

        for trade in &self.trades {
            current += trade.pnl;
            if current > peak {
                peak = current;
            }
            let drawdown = (peak - current) / peak;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }

        // Calculate Sharpe ratio (simplified)
        let sharpe_ratio = if self.trades.len() > 1 {
            let returns: Vec<f64> = self.trades.iter().map(|t| t.pnl_percent / 100.0).collect();
            let mean = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance = returns
                .iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>()
                / returns.len() as f64;
            let std_dev = variance.sqrt();
            if std_dev > 0.0 {
                mean / std_dev
            } else {
                0.0
            }
        } else {
            0.0
        };

        Ok(BacktestResult {
            start_balance: self.initial_balance,
            end_balance: self.balance.total,
            total_return,
            total_return_percent,
            num_trades: self.trades.len(),
            winning_trades,
            losing_trades,
            win_rate,
            avg_profit,
            avg_loss,
            max_drawdown,
            sharpe_ratio,
        })
    }
}

