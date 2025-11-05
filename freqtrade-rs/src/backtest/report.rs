//! Backtest report generation

use crate::backtest::BacktestResult;
use crate::backtest::MetricsCalculator;

/// Backtest report
#[derive(Debug)]
pub struct BacktestReport {
    result: BacktestResult,
    profit_factor: f64,
    expectancy: f64,
}

impl BacktestReport {
    /// Create new report from result
    pub fn new(result: BacktestResult) -> Self {
        let profit_factor = MetricsCalculator::profit_factor(&result);
        let expectancy = MetricsCalculator::expectancy(&result);

        Self {
            result,
            profit_factor,
            expectancy,
        }
    }

    /// Format report as string
    pub fn format(&self) -> String {
        format!(
            r#"
Backtest Results
================
Starting Balance: ${:.2}
Ending Balance: ${:.2}
Total Return: ${:.2} ({:.2}%)
Number of Trades: {}
Winning Trades: {}
Losing Trades: {}
Win Rate: {:.2}%
Average Profit: ${:.2}
Average Loss: ${:.2}
Profit Factor: {:.2}
Expectancy: ${:.2}
Maximum Drawdown: {:.2}%
Sharpe Ratio: {:.2}
"#,
            self.result.start_balance,
            self.result.end_balance,
            self.result.total_return,
            self.result.total_return_percent,
            self.result.num_trades,
            self.result.winning_trades,
            self.result.losing_trades,
            self.result.win_rate,
            self.result.avg_profit,
            self.result.avg_loss,
            self.profit_factor,
            self.expectancy,
            self.result.max_drawdown * 100.0,
            self.result.sharpe_ratio,
        )
    }

    /// Get result reference
    pub fn result(&self) -> &BacktestResult {
        &self.result
    }
}

