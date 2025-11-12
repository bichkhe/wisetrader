use askama::Template;
use chrono::Utc;
use base64::{Engine as _, engine::general_purpose};
use pulldown_cmark::{Parser, Options, html};

/// Convert markdown text to HTML
pub fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    
    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    html_output
}

/// Load logo and convert to base64 data URI
/// Tries to load from filesystem first, falls back to embedded logo if available
fn load_logo_base64() -> String {
    // Try multiple possible paths for logo (relative to workspace root, current directory, or Docker paths)
    let possible_paths = vec![
        // Docker container paths (first priority)
        "/app/shared/templates/logo.png",
        "/app/templates/logo.png",
        // Local development paths
        "shared/templates/logo.png",
        "./shared/templates/logo.png",
        "../shared/templates/logo.png",
        "../../shared/templates/logo.png",
        "../../../shared/templates/logo.png",
        "templates/logo.png",
        "./templates/logo.png",
        // Relative to current working directory
        "./logo.png",
    ];
    
    for path_str in &possible_paths {
        match std::fs::read(path_str) {
            Ok(bytes) if !bytes.is_empty() => {
                let base64 = general_purpose::STANDARD.encode(&bytes);
                tracing::info!("✅ Logo loaded successfully from: {}", path_str);
                return format!("data:image/png;base64,{}", base64);
            }
            Ok(_) => {
                tracing::warn!("Logo file exists but is empty: {}", path_str);
            }
            Err(e) => {
                tracing::debug!("Could not read logo from {}: {}", path_str, e);
            }
        }
    }
    
    // Try embedded logo as fallback (if we compile with include_bytes!)
    // For now, we'll just log a warning
    tracing::warn!("⚠️ Logo file not found in any of the expected paths. Searched: {:?}", possible_paths);
    
    // Return empty string if logo not found
    String::new()
}

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
    pub use_stochastic: bool,
    pub stochastic_period: i32,
    pub stochastic_smooth_k: i32,
    pub stochastic_smooth_d: i32,
    pub use_adx: bool,
    pub adx_period: i32,
    
    // Entry conditions
    pub entry_condition_rsi: bool,
    pub rsi_oversold: i32,
    pub entry_condition_macd: bool,
    pub entry_condition_ema: bool,
    pub entry_condition_bb: bool,
    pub entry_condition_stochastic: bool,
    pub stochastic_oversold: i32,
    pub entry_condition_adx: bool,
    pub adx_threshold: i32,
    
    // Exit conditions
    pub exit_condition_rsi: bool,
    pub rsi_overbought: i32,
    pub exit_condition_stochastic: bool,
    pub stochastic_overbought: i32,
}

#[derive(Template)]
#[template(path = "ai_analysis.html.jinja", escape = "none")]
struct AiAnalysisTemplate {
    ai_analysis_html: String,
}

#[derive(Template)]
#[template(path = "ai_analysis_section.html.jinja", escape = "none")]
struct AiAnalysisSectionTemplate {
    ai_analysis_html: String,
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
    pub user_fullname: Option<String>,
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
    pub logo_base64: String,
    pub ai_analysis: Option<String>,
    pub ai_analysis_html: Option<String>,
    pub ai_analysis_section: Option<String>, // Pre-rendered HTML section (raw HTML, no escaping)
}

impl BacktestReportTemplate {
    /// Custom render that handles AI analysis section without escaping
    pub fn render_with_ai_analysis(&self) -> Result<String, askama::Error> {
        // Render the main template without AI analysis section
        let mut html = self.render()?;
        
        // If we have AI analysis section, insert it at the comment marker
        if let Some(ref section_html) = self.ai_analysis_section {
            // Find the comment marker where we should insert the AI analysis
            if let Some(pos) = html.find("<!-- AI Analysis section will be inserted here by render_with_ai_analysis() -->") {
                // Insert the section HTML at the comment position, replacing the comment
                let comment_len = "<!-- AI Analysis section will be inserted here by render_with_ai_analysis() -->".len();
                html.replace_range(pos..pos + comment_len, section_html);
            } else {
                // Fallback: append before footer
                if let Some(pos) = html.find("<div class=\"footer\">") {
                    html.insert_str(pos, section_html);
                } else {
                    // Last resort: append at end
                    html.push_str(section_html);
                }
            }
        }
        
        Ok(html)
    }
}

impl BacktestReportTemplate {
    pub fn new(
        strategy_name: String,
        exchange: String,
        pair: String,
        timeframe: String,
        timerange: String,
        user_fullname: Option<String>,
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
        ai_analysis: Option<String>,
    ) -> Self {
        // Convert markdown to HTML if ai_analysis is provided
        let ai_analysis_html = ai_analysis.as_ref()
            .map(|md| {
                let html = markdown_to_html(md);
                // Render through template with escape = "none" to get properly formatted HTML
                let html_clone = html.clone();
                AiAnalysisTemplate { ai_analysis_html: html }
                    .render()
                    .unwrap_or_else(|_| html_clone) // Fallback to raw HTML if template render fails
            });
        
        // Pre-render AI analysis section with escape = "none" to avoid double escaping
        // This section is already HTML, so we render it separately and embed as raw HTML
        let ai_analysis_section = ai_analysis_html.as_ref()
            .map(|html| {
                // Render the section template with escape = "none"
                AiAnalysisSectionTemplate { ai_analysis_html: html.clone() }
                    .render()
                    .unwrap_or_else(|_| String::new())
            });
        
        Self {
            strategy_name,
            exchange,
            pair,
            timeframe,
            timerange,
            created_at: Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            user_fullname,
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
            logo_base64: load_logo_base64(),
            ai_analysis,
            ai_analysis_html,
            ai_analysis_section,
        }
    }
}
