//! Backtest performance metrics

use crate::backtest::BacktestResult;

/// Calculate additional metrics from backtest result
pub struct MetricsCalculator;

impl MetricsCalculator {
    /// Calculate profit factor
    pub fn profit_factor(result: &BacktestResult) -> f64 {
        if result.avg_loss == 0.0 {
            return 0.0;
        }
        (result.avg_profit * result.winning_trades as f64).abs()
            / (result.avg_loss * result.losing_trades as f64).abs()
    }

    /// Calculate expectancy
    pub fn expectancy(result: &BacktestResult) -> f64 {
        if result.num_trades == 0 {
            return 0.0;
        }
        (result.win_rate / 100.0 * result.avg_profit)
            - ((100.0 - result.win_rate) / 100.0 * result.avg_loss.abs())
    }

    /// Calculate return on investment
    pub fn roi(result: &BacktestResult) -> f64 {
        result.total_return_percent
    }

    /// Calculate average trade duration (placeholder - would need trade timestamps)
    pub fn avg_trade_duration(_result: &BacktestResult) -> f64 {
        // TODO: Calculate from trade timestamps
        0.0
    }
}

