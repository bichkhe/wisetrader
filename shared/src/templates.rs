use askama::Template;

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

