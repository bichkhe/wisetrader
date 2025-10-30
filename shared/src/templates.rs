use askama::Template;
use chrono::Utc;

#[derive(Template)]
#[template(path = "strategy_template.py", escape = "none")]
pub struct StrategyTemplate {
    pub strategy_name: String,
    pub minimal_roi_60: String,
    pub minimal_roi_30: String,
    pub minimal_roi_0: String,
    pub stoploss: String,
    pub trailing_stop: bool,
    pub trailing_stop_positive: String,
    pub trailing_stop_offset: String,
    pub timeframe: String,
    pub startup_candle_count: i32,
    
    // Indicator flags
    pub use_rsi: bool,
    pub rsi_period: i32,
    pub use_macd: bool,
    pub macd_fast: i32,
    pub macd_slow: i32,
    pub macd_signal: i32,
    pub use_ema: bool,
    pub ema_fast: i32,
    pub ema_slow: i32,
    pub use_bb: bool,
    pub bb_period: i32,
    
    // Entry conditions
    pub entry_condition_rsi: bool,
    pub rsi_oversold: i32,
    pub entry_condition_macd: bool,
    pub entry_condition_ema: bool,
    pub entry_condition_bb: bool,
    
    // Exit conditions
    pub exit_condition_rsi: bool,
    pub rsi_overbought: i32,
}

#[derive(Template)]
#[template(path = "backtest_report.html.jinja", escape = "html")]
pub struct BacktestReportTemplate {
    pub strategy_name: String,
    pub exchange: String,
    pub pair: String,
    pub timeframe: String,
    pub timerange: String,
    pub created_at: String,
    pub trades: i32,
    pub profit_pct: f64,
    pub win_rate: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub starting_balance: Option<f64>,
    pub final_balance: Option<f64>,
    pub download_time_secs: Option<u64>,
    pub backtest_time_secs: u64,
    pub tables: Vec<(String, String)>,
    pub raw_output: Option<String>,
}

impl BacktestReportTemplate {
    pub fn new(
        strategy_name: String,
        exchange: String,
        pair: String,
        timeframe: String,
        timerange: String,
        trades: i32,
        profit_pct: f64,
        win_rate: Option<f64>,
        max_drawdown: Option<f64>,
        starting_balance: Option<f64>,
        final_balance: Option<f64>,
        download_time_secs: Option<u64>,
        backtest_time_secs: u64,
        tables: Vec<(String, String)>,
        raw_output: Option<String>,
    ) -> Self {
        Self {
            strategy_name,
            exchange,
            pair,
            timeframe,
            timerange,
            created_at: Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            trades,
            profit_pct,
            win_rate,
            max_drawdown,
            starting_balance,
            final_balance,
            download_time_secs,
            backtest_time_secs,
            tables,
            raw_output,
        }
    }
}
