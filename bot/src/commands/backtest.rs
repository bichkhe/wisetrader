use std::sync::Arc;
use std::path::{Path, PathBuf};
use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButton;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
use shared::entity::strategies;
use shared::FreqtradeApiClient;
use shared::{BacktestReportTemplate, Config};
use std::fs;
use askama::Template;
use chrono::{Utc, Duration};
use crate::state::{AppState, BotState, BacktestState, MyDialogue};
use crate::i18n;

/// Helper function to HTML escape
fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
}

/// Split text into chunks at character boundaries (safe for UTF-8)
fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_pos = 0;
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();
    
    while current_pos < total_chars {
        let end_pos = std::cmp::min(current_pos + max_chars, total_chars);
        let chunk: String = chars[current_pos..end_pos].iter().collect();
        chunks.push(chunk);
        current_pos = end_pos;
    }
    
    chunks
}

/// Convert table with box-drawing characters to mobile-friendly format
/// Parses table rows and converts to simple list format
/// Get icon for metric based on its name
fn get_metric_icon(metric_name: &str) -> &'static str {
    let lower = metric_name.to_lowercase();
    if lower.contains("profit") || lower.contains("pnl") {
        "💰"
    } else if lower.contains("return") || lower.contains("roi") {
        "📈"
    } else if lower.contains("win") || lower.contains("winrate") {
        "✅"
    } else if lower.contains("loss") {
        "❌"
    } else if lower.contains("trade") || lower.contains("trades") {
        "🔄"
    } else if lower.contains("duration") || lower.contains("time") {
        "⏱️"
    } else if lower.contains("drawdown") || lower.contains("dd") {
        "📉"
    } else if lower.contains("sharpe") {
        "📊"
    } else if lower.contains("expectancy") {
        "🎯"
    } else if lower.contains("avg") || lower.contains("average") {
        "📊"
    } else if lower.contains("total") {
        "📦"
    } else if lower.contains("max") {
        "⬆️"
    } else if lower.contains("min") {
        "⬇️"
    } else if lower.contains("best") {
        "🏆"
    } else if lower.contains("worst") {
        "⚠️"
    } else if lower.contains("count") || lower.contains("number") {
        "🔢"
    } else if lower.contains("ratio") || lower.contains("percent") || lower.contains("%") {
        "📊"
    } else {
        "📌"
    }
}

// All box-drawing characters to skip
const BOX_CHARS: &[char] = &[
    '┃', '│', '┼', '━', '═', '─', '┡', '┏', '┗', '┳', '┻', '┣', '┫',
    '┨', '┧', '┠', '┯', '┷', '┿', '╂', '╋', '╁', '╀', '╇', '╈', '╉',
    '╊', '╃', '╄', '╅', '╆', '┩', '┪', '┬', '┭', '┮',
    '┯', '┰', '┱', '┲', '┴', '┵', '┶', '┸', '┹', '┺',
    '┽', '┾', '╌', '╍', '╎', '╏', '║', '╒', '╓', '╔', '╕',
    '╖', '╗', '╘', '╙', '╚', '╛', '╜', '╝', '╞', '╟', '╠', '╡', '╢',
    '╣', '╤', '╥', '╦', '╧', '╨', '╩', '╪', '╫', '╬'
];

fn format_table_mobile_friendly(table_content: &str) -> String {
    let lines: Vec<&str> = table_content.lines().collect();
    let mut formatted = String::new();
    let mut headers: Vec<String> = Vec::new();
    let mut data_rows: Vec<Vec<String>> = Vec::new();
    
    for line in lines.iter() {
        let trimmed = line.trim();
        
        // Skip separator lines (box-drawing only, dashes, or empty)
        if trimmed.is_empty() {
            continue;
        }
        
        // Skip lines that are only separators (dashes, underscores, box-drawing chars, pipes)
        if trimmed.chars().all(|c| BOX_CHARS.contains(&c) || 
                                  c == ' ' || c == '-' || c == '_' ||
                                  c == '=' || c == '|') {
            continue;
        }
        
        // Skip lines with pattern like |----|----| or |----|----|----| (pipe + dashes pattern)
        if trimmed.contains('|') && trimmed.chars().filter(|c| *c == '|' || *c == '-' || *c == '_' || *c == '=').count() as f64 / trimmed.len() as f64 > 0.8 {
            continue;
        }
        
        // Skip lines that are mostly dashes/separators (e.g., "------", "━━━━━━")
        let separator_chars = trimmed.chars().filter(|c| BOX_CHARS.contains(&c) ||
                                                           *c == '-' || *c == '_' || *c == '=' ||
                                                           *c == '|').count();
        if separator_chars as f64 / trimmed.len() as f64 > 0.7 {
            continue;
        }
        
        // Parse table row - split by box-drawing characters and clean them
        let mut cleaned_line = trimmed.to_string();
        // Remove all box-drawing characters
        for &box_char in BOX_CHARS {
            cleaned_line = cleaned_line.replace(box_char, "|");
        }
        
        let cells: Vec<String> = cleaned_line
            .split('|')
            .map(|s| {
                // Remove any remaining box-drawing chars and clean
                s.trim()
                    .chars()
                    .filter(|c| !BOX_CHARS.contains(c))
                    .collect::<String>()
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();
        
        if cells.is_empty() {
            continue;
        }
        
        // First non-separator row is usually headers
        if headers.is_empty() && cells.len() > 1 {
            headers = cells.clone();
        } else if !headers.is_empty() && cells.len() == headers.len() {
            // Data row with same number of columns as headers
            data_rows.push(cells);
        } else if !headers.is_empty() && cells.len() > 0 {
            // Row with different column count - might be summary or special row
            // Format as single line
            let row_text: String = cells.join(" | ");
            if !row_text.trim().is_empty() {
                formatted.push_str(&format!("• {}\n", row_text));
            }
        }
    }
    
    // Format table rows with headers
    if !headers.is_empty() && !data_rows.is_empty() {
        // Check if this is a summary/metrics table (usually has 2 columns: Metrics/Key and Value)
        let is_summary_table = headers.len() == 2 && 
            (headers[0].to_lowercase().contains("metric") || 
             headers[0].to_lowercase().contains("key") ||
             headers[1].to_lowercase().contains("value"));
        
        // Limit rows for mobile readability
        let max_rows = 20;
        let rows_to_show = std::cmp::min(data_rows.len(), max_rows);
        
        for row in data_rows.iter().take(rows_to_show) {
            if is_summary_table && row.len() == 2 {
                // Simple format for summary: "Icon Key: Value"
                let key = row[0].trim();
                let value = row[1].trim();
                if !key.is_empty() && !value.is_empty() {
                    let icon = get_metric_icon(key);
                    formatted.push_str(&format!("{} {}: {}\n", 
                        icon, escape_html(key), escape_html(value)));
                }
            } else {
                // For other tables, use simple format: "Icon Key: Value" for each column
                for (col_idx, cell) in row.iter().enumerate() {
                    if col_idx < headers.len() && !cell.trim().is_empty() {
                        let header = &headers[col_idx];
                        let icon = get_metric_icon(header);
                        formatted.push_str(&format!("{} {}: {}\n", 
                            icon, escape_html(header), escape_html(cell)));
                    }
                }
            }
        }
        
        if data_rows.len() > max_rows {
            formatted.push_str(&format!("\n... ({} more rows)\n", data_rows.len() - max_rows));
        }
    } else if formatted.is_empty() {
        // Fallback: just clean up the table format
        for line in lines.iter() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            
            // Skip separator lines
            // Skip lines with pattern like |----|----| or |----|----|----| (pipe + dashes pattern)
            if trimmed.contains('|') && trimmed.chars().filter(|c| *c == '|' || *c == '-' || *c == '_' || *c == '=').count() as f64 / trimmed.len() as f64 > 0.8 {
                continue;
            }
            
            let separator_chars = trimmed.chars().filter(|c| *c == '-' || *c == '_' || *c == '=' || 
                                                               *c == '━' || *c == '═' || *c == '─' ||
                                                               *c == '┃' || *c == '│' || *c == '|').count();
            if separator_chars as f64 / trimmed.len() as f64 > 0.7 {
                continue;
            }
            
            // Check if line contains any non-separator content
            let has_content = trimmed.chars().any(|c| !BOX_CHARS.contains(&c) && 
                                                   c != ' ' && c != '-' && 
                                                   c != '_' && c != '=' && c != '|');
            
            if has_content {
                // Remove all box-drawing chars, keep content
                let mut cleaned = trimmed
                    .chars()
                    .filter(|c| !BOX_CHARS.contains(c))
                    .collect::<String>()
                    .trim()
                    .to_string();
                
                // Remove leading/trailing dashes and separators
                cleaned = cleaned.trim_start_matches(|c| c == '-' || c == '_' || c == '=' || 
                                                              c == '━' || c == '═' || c == '─' ||
                                                              c == '|')
                                     .trim_end_matches(|c| c == '-' || c == '_' || c == '=' || 
                                                            c == '━' || c == '═' || c == '─' ||
                                                            c == '|')
                                     .trim()
                                     .to_string();
                
                if !cleaned.is_empty() {
                    formatted.push_str(&format!("• {}\n", cleaned));
                }
            }
        }
    }
    
    // Clean up formatted string: remove leading/trailing dashes and empty lines
    let cleaned: String = formatted
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return false;
            }
            // Skip lines that are mostly separators
            // Skip lines with pattern like |----|----| or |----|----|----| (pipe + dashes pattern)
            if trimmed.contains('|') && trimmed.chars().filter(|c| *c == '|' || *c == '-' || *c == '_' || *c == '=').count() as f64 / trimmed.len() as f64 > 0.8 {
                return false;
            }
            
            // Check for box-drawing characters
            let box_char_count = trimmed.chars().filter(|c| BOX_CHARS.contains(c)).count();
            let separator_chars = trimmed.chars().filter(|c| BOX_CHARS.contains(c) ||
                                                               *c == '-' || *c == '_' || *c == '=' ||
                                                               *c == '━' || *c == '═' || *c == '─' ||
                                                               *c == '|').count();
            let ratio = separator_chars as f64 / trimmed.len() as f64;
            let box_ratio = box_char_count as f64 / trimmed.len() as f64;
            // Skip if mostly separators or if it's mostly box-drawing chars
            ratio < 0.7 && box_ratio < 0.8
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    // Remove leading dashes/separators from the entire string
    cleaned.trim_start_matches(|c| c == '-' || c == '_' || c == '=' || 
                                    c == '━' || c == '═' || c == '─' ||
                                    c == '\n' || c == ' ')
           .trim()
           .to_string()
}

/// Extract all tables from freqtrade output, returns vector of (title, content)
fn extract_all_tables(stdout: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = stdout.lines().collect();
    let mut tables: Vec<(String, String)> = Vec::new();
    let mut current_table_title = String::new();
    let mut current_table_lines: Vec<String> = Vec::new();
    let mut in_table = false;
    
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Detect table titles (centered text with REPORT/STATS/METRICS/SUMMARY, no box-drawing chars)
        let is_title = trimmed.len() > 10 && trimmed.len() < 100 && 
                       (trimmed.contains("REPORT") || 
                        trimmed.contains("STATS") || 
                        trimmed.contains("METRICS") ||
                        trimmed.contains("SUMMARY")) &&
                       !trimmed.contains("┃") && !trimmed.contains("│") && 
                       !trimmed.contains("┼") && !trimmed.contains("┡") &&
                       !trimmed.contains("━") && !trimmed.contains("═");
        
        // Detect table lines (box-drawing characters)
        let is_table_line = trimmed.contains("┃") || trimmed.contains("│") || 
                           trimmed.contains("┡") || trimmed.contains("┼") ||
                           trimmed.contains("┏") || trimmed.contains("┗") ||
                           (trimmed.contains("━━") && trimmed.len() > 20) ||
                           (trimmed.contains("══") && trimmed.len() > 20);
        
        if is_title {
            // Save previous table if exists
            if in_table && !current_table_lines.is_empty() {
                tables.push((current_table_title.clone(), current_table_lines.join("\n")));
                current_table_lines.clear();
            }
            // Start new table
            current_table_title = trimmed.to_string();
            in_table = true;
        } else if in_table {
            // Skip separator lines with pattern |----|----| or similar
            let is_separator_line = trimmed.contains('|') && 
                trimmed.chars().filter(|c| *c == '|' || *c == '-' || *c == '_' || *c == '=').count() as f64 / trimmed.len() as f64 > 0.8;
            
            if is_separator_line {
                // Skip separator lines
                continue;
            } else if is_table_line {
                current_table_lines.push(line.to_string());
            } else if trimmed.is_empty() {
                // Empty line within table (separator)
                if current_table_lines.len() > 0 {
                    current_table_lines.push(line.to_string());
                }
            } else {
                // Check if next line is still part of table
                let next_is_table = lines.get(idx + 1)
                    .map(|l| {
                        let t = l.trim();
                        t.contains("┃") || t.contains("│") || t.contains("┡") || 
                        t.contains("┼") || t.contains("┏") || t.contains("┗") ||
                        t.is_empty() || t.contains("━━") || t.contains("══")
                    })
                    .unwrap_or(false);
                
                if !next_is_table && current_table_lines.len() > 5 {
                    // End of table, save it
                    tables.push((current_table_title.clone(), current_table_lines.join("\n")));
                    current_table_lines.clear();
                    current_table_title.clear();
                    in_table = false;
                }
            }
        }
    }
    
    // Save last table if exists
    if !current_table_lines.is_empty() {
        tables.push((current_table_title, current_table_lines.join("\n")));
    }
    
    tables
}

/// Extract summary table section from freqtrade output (legacy, kept for compatibility)
fn extract_summary_table(stdout: &str) -> String {
    let lines: Vec<&str> = stdout.lines().collect();
    let mut summary_lines: Vec<String> = Vec::new();
    let mut in_summary = false;
    let mut table_lines = 0;
    
    for line in lines.iter() {
        let trimmed = line.trim();
        
        // Detect summary section
        if trimmed.contains("SUMMARY") || trimmed.contains("BACKTEST RESULT") || trimmed.contains("===================") {
            in_summary = true;
            if !trimmed.contains("========") {
                summary_lines.push(line.to_string());
            }
            continue;
        }
        
        if in_summary {
            // Collect table lines (usually contain | or multiple spaces)
            if trimmed.contains("|") || (trimmed.len() > 20 && trimmed.chars().filter(|c| c.is_whitespace()).count() > 5) {
                summary_lines.push(line.to_string());
                table_lines += 1;
                // Limit table size to avoid message too long
                if table_lines > 30 {
                    summary_lines.push("... (table truncated)".to_string());
                    break;
                }
            } else if trimmed.is_empty() {
                if summary_lines.len() > 5 {
                    summary_lines.push(line.to_string());
                }
            } else if table_lines > 5 && !trimmed.contains("=") && !trimmed.contains("-") {
                // End of summary section
                break;
            }
        }
    }
    
    // If no summary found, return key metrics lines
    if summary_lines.is_empty() || summary_lines.len() < 3 {
        for line in lines.iter() {
            let trimmed = line.trim();
            if trimmed.contains("Total") || trimmed.contains("Profit") || trimmed.contains("Win") || 
               trimmed.contains("Drawdown") || trimmed.contains("Trades") {
                summary_lines.push(line.to_string());
            }
            if summary_lines.len() > 15 {
                break;
            }
        }
    }
    
    summary_lines.join("\n")
}

/// Generate HTML report from backtest results
async fn generate_html_report(
    config: &Config,
    strategy_name: &str,
    exchange: &str,
    pair: &str,
    timeframe: &str,
    timerange: &str,
    user_fullname: Option<String>,
    result: &shared::BacktestResult,
    tables: &[(String, String)],
    ai_analysis: Option<String>,
) -> Result<Option<(String, std::path::PathBuf)>, anyhow::Error> {
    // Create reports directory if it doesn't exist
    fs::create_dir_all(&config.html_reports_dir)?;
    
    // Generate unique filename
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("backtest_{}_{}.html", 
        strategy_name.replace(" ", "_").replace("/", "_"),
        timestamp
    );
    let filepath = Path::new(&config.html_reports_dir).join(&filename);
    
    // Create template
    let template = BacktestReportTemplate::new(
        strategy_name.to_string(),
        exchange.to_string(),
        pair.to_string(),
        timeframe.to_string(),
        timerange.to_string(),
        user_fullname,
        result.trades,
        result.profit_pct,
        result.win_rate,
        result.max_drawdown,
        result.starting_balance,
        result.final_balance,
        result.download_time_secs,
        result.backtest_time_secs,
        tables.to_vec(),
        result.stdout.clone(),
        ai_analysis,
    );
    
    // Render template with AI analysis (if available)
    let html_content = template.render_with_ai_analysis()?;
    
    // Write to file
    fs::write(&filepath, html_content)?;
    
    tracing::info!("HTML report saved to: {}", filepath.display());
    
    // Return URL - use API server if available, otherwise use file:// or custom base URL
    let url = if let Some(ref base_url) = config.html_reports_base_url {
        format!("{}/{}", base_url.trim_end_matches('/'), filename)
    } else {
        // Use API server by default
        format!("{}/reports/{}", config.api_base_url.trim_end_matches('/'), filename)
    };
    
    Ok(Some((url, filepath)))
}

/// Update HTML report with AI analysis
async fn update_html_report_with_ai_analysis(
    filepath: &std::path::Path,
    strategy_name: &str,
    exchange: &str,
    pair: &str,
    timeframe: &str,
    timerange: &str,
    user_fullname: Option<String>,
    result: &shared::BacktestResult,
    tables: &[(String, String)],
    ai_analysis: String,
) -> Result<(), anyhow::Error> {
    tracing::info!("📝 Updating HTML report with AI analysis (length: {} chars) at: {}", ai_analysis.len(), filepath.display());
    
    // Re-create template with AI analysis
    let template = BacktestReportTemplate::new(
        strategy_name.to_string(),
        exchange.to_string(),
        pair.to_string(),
        timeframe.to_string(),
        timerange.to_string(),
        user_fullname,
        result.trades,
        result.profit_pct,
        result.win_rate,
        result.max_drawdown,
        result.starting_balance,
        result.final_balance,
        result.download_time_secs,
        result.backtest_time_secs,
        tables.to_vec(),
        result.stdout.clone(),
        Some(ai_analysis),
    );
    
    // Check if ai_analysis_html was generated
    if template.ai_analysis_html.is_none() {
        tracing::warn!("⚠️ ai_analysis_html is None after template creation!");
    } else {
        tracing::info!("✅ ai_analysis_html generated successfully (length: {} chars)", 
            template.ai_analysis_html.as_ref().map(|s| s.len()).unwrap_or(0));
    }
    
    // Render template with AI analysis (if available)
    let html_content = template.render_with_ai_analysis()?;
    
    // Write to file (overwrite)
    fs::write(filepath, html_content)?;
    
    tracing::info!("✅ HTML report updated with AI analysis: {}", filepath.display());
    
    Ok(())
}

/// Calculate timerange string for Freqtrade CLI (format: YYYYMMDD-)
fn calculate_timerange(range: &str) -> String {
    let now = Utc::now();
    let start_date = match range {
        "1day" => now - Duration::days(1),
        "1week" => now - Duration::days(7),
        "1month" => now - Duration::days(30),
        "3months" => now - Duration::days(90),
        "6months" => now - Duration::days(180),
        "9months" => now - Duration::days(270),
        "1year" => now - Duration::days(365),
        _ => now - Duration::days(7),
    };
    
    // Format as YYYYMMDD- for Freqtrade CLI
    format!("{}-", start_date.format("%Y%m%d"))
}

/// Parse strategy description to extract parameters (returns algorithm, buy, sell, timeframe, pair)
fn parse_strategy_description(description: &str) -> (String, String, String, String, String) {
    let mut algorithm = "RSI".to_string();
    let mut buy_condition = "RSI < 30".to_string();
    let mut sell_condition = "RSI > 70".to_string();
    let mut timeframe = "1h".to_string();
    let mut pair = "BTC/USDT".to_string();

    for line in description.lines() {
        if line.starts_with("Algorithm: ") {
            algorithm = line[11..].to_string();
        } else if line.starts_with("Buy: ") {
            buy_condition = line[5..].to_string();
        } else if line.starts_with("Sell: ") {
            sell_condition = line[6..].to_string();
        } else if line.starts_with("Timeframe: ") {
            timeframe = line[11..].to_string();
        } else if line.starts_with("Pair: ") {
            pair = line[6..].to_string();
        }
    }

    (algorithm, buy_condition, sell_condition, timeframe, pair)
}

/// Extract thresholds from conditions (e.g., "RSI < 30" -> 30)
fn extract_threshold(condition: &str, indicator: &str) -> Option<i32> {
    if condition.contains(indicator) {
        // Try to find number after <, >, <=, >= operators
        for part in condition.split_whitespace() {
            // Remove common operators and check if remaining is a number
            let cleaned = part.trim_matches(&['<', '>', '='][..]);
            if let Ok(num) = cleaned.parse::<i32>() {
                return Some(num);
            }
        }
    }
    None
}

/// Map StrategyConfig to Freqtrade template parameters
/// This ensures consistency between backtest (Freqtrade) and live trading
#[allow(dead_code)]
fn map_config_to_freqtrade_template(
    algorithm: &str,
    buy_condition: &str,
    sell_condition: &str,
    _timeframe: &str,
    parameters: &serde_json::Value,
) -> (
    bool, i32,  // use_rsi, rsi_period
    bool, i32, i32, i32,  // use_macd, macd_fast, macd_slow, macd_signal
    bool, i32, i32,  // use_ema, ema_fast, ema_slow
    bool, i32,  // use_bb, bb_period
    bool, i32, i32, i32,  // use_stochastic, stochastic_period, stochastic_smooth_k, stochastic_smooth_d
    bool, i32,  // use_adx, adx_period
    bool, i32,  // entry_condition_rsi, rsi_oversold
    bool, bool, bool,  // entry_condition_macd, entry_condition_ema, entry_condition_bb
    bool, i32,  // entry_condition_stochastic, stochastic_oversold
    bool, i32,  // entry_condition_adx, adx_threshold
    bool, i32,  // exit_condition_rsi, rsi_overbought
    bool, i32,  // exit_condition_stochastic, stochastic_overbought
) {
    let empty_map = serde_json::Map::new();
    let params = parameters.as_object().unwrap_or(&empty_map);
    
    // Determine indicators based on algorithm
    let (use_rsi, use_macd, use_ema, use_bb, use_stochastic, use_adx) = match algorithm.to_uppercase().as_str() {
        "RSI" => (true, false, false, false, false, false),
        "MACD" => (false, true, false, false, false, false),
        "EMA" => (false, false, true, false, false, false),
        "BOLLINGER" | "BOLLINGER BANDS" | "BB" => (false, false, false, true, false, false),
        "MA" | "SMA" => (false, false, true, false, false, false),
        "STOCHASTIC" => (false, false, false, false, true, false),
        "ADX" => (false, false, false, false, false, true),
        _ => (true, false, false, false, false, false),
    };

    // Parse buy/sell conditions to determine entry/exit conditions
    let entry_condition_rsi = buy_condition.to_uppercase().contains("RSI") && buy_condition.contains("<");
    let exit_condition_rsi = sell_condition.to_uppercase().contains("RSI") && sell_condition.contains(">");
    let entry_condition_macd = buy_condition.to_uppercase().contains("MACD");
    let entry_condition_ema = buy_condition.to_uppercase().contains("EMA");
    let entry_condition_bb = buy_condition.to_uppercase().contains("BOLLINGER") || buy_condition.to_uppercase().contains("LOWERBAND");
    let entry_condition_stochastic = buy_condition.to_uppercase().contains("STOCHASTIC") && buy_condition.contains("<");
    let exit_condition_stochastic = sell_condition.to_uppercase().contains("STOCHASTIC") && sell_condition.contains(">");
    let entry_condition_adx = buy_condition.to_uppercase().contains("ADX") && buy_condition.contains(">");

    // Extract RSI parameters from config
    let rsi_period = params
        .get("period")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(14) as i32;
    
    // Extract RSI thresholds from conditions
    let rsi_oversold = extract_threshold(buy_condition, "RSI").unwrap_or(30);
    let rsi_overbought = extract_threshold(sell_condition, "RSI").unwrap_or(70);

    // Extract MACD parameters from config
    let macd_fast = params
        .get("fast")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(12) as i32;
    let macd_slow = params
        .get("slow")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(26) as i32;
    let macd_signal = params
        .get("signal")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(9) as i32;

    // Extract EMA parameters from config
    let ema_fast = params
        .get("fast")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(12) as i32;
    let ema_slow = params
        .get("slow")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(26) as i32;
    
    // If EMA uses single period, use that for both
    let (ema_fast_final, ema_slow_final) = if params.contains_key("period") && !params.contains_key("fast") {
        let period = params
            .get("period")
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
            .unwrap_or(20) as i32;
        (period, period)
    } else {
        (ema_fast, ema_slow)
    };

    // Extract Bollinger Bands parameters from config
    let bb_period = params
        .get("period")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(20) as i32;

    // Extract Stochastic parameters from config
    let stochastic_period = params
        .get("period")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(14) as i32;
    let stochastic_smooth_k = params
        .get("smooth_k")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(3) as i32;
    let stochastic_smooth_d = params
        .get("smooth_d")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(3) as i32;
    
    // Extract Stochastic thresholds from conditions
    let stochastic_oversold = extract_threshold(buy_condition, "Stochastic").unwrap_or(20);
    let stochastic_overbought = extract_threshold(sell_condition, "Stochastic").unwrap_or(80);

    // Extract ADX parameters from config
    let adx_period = params
        .get("period")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
        .unwrap_or(14) as i32;
    
    // Extract ADX threshold from conditions
    let adx_threshold = extract_threshold(buy_condition, "ADX").unwrap_or(25);

    (
        use_rsi, rsi_period,
        use_macd, macd_fast, macd_slow, macd_signal,
        use_ema, ema_fast_final, ema_slow_final,
        use_bb, bb_period,
        use_stochastic, stochastic_period, stochastic_smooth_k, stochastic_smooth_d,
        use_adx, adx_period,
        entry_condition_rsi, rsi_oversold,
        entry_condition_macd, entry_condition_ema, entry_condition_bb,
        entry_condition_stochastic, stochastic_oversold,
        entry_condition_adx, adx_threshold,
        exit_condition_rsi, rsi_overbought,
        exit_condition_stochastic, stochastic_overbought,
    )
}

/// Generate Python strategy file from strategy data
/// Uses StrategyConfig to ensure consistency with live trading
fn generate_strategy_file(
    strategy_id: u64,
    strategy_name: &str,
    algorithm: &str,
    buy_condition: &str,
    sell_condition: &str,
    timeframe: &str,
    strategies_path: &Path,
    config: Option<&crate::services::strategy_engine::StrategyConfig>,
) -> Result<PathBuf, anyhow::Error> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    
    // Ensure strategies directory exists
    tracing::info!("Creating strategies directory: {:?}", strategies_path);
    fs::create_dir_all(strategies_path)
        .map_err(|e| anyhow::anyhow!("Failed to create strategies directory {:?}: {}", strategies_path, e))?;
    
    // Verify directory is writable
    let metadata = fs::metadata(strategies_path)
        .map_err(|e| anyhow::anyhow!("Failed to get metadata for {:?}: {}", strategies_path, e))?;
    tracing::info!("Strategies directory metadata: {:?}, permissions: {:o}", 
        metadata, metadata.permissions().mode());
    
    // Check write permission by attempting to create a test file
    let test_file = strategies_path.join(".write_test");
    match fs::File::create(&test_file) {
        Ok(_) => {
            fs::remove_file(&test_file)
                .map_err(|e| tracing::warn!("Failed to remove test file: {}", e)).ok();
            tracing::info!("✅ Write permission verified for {:?}", strategies_path);
        }
        Err(e) => {
            tracing::warn!("Write permission test failed: {}. Attempting to fix permissions...", e);
            
            // Try to fix permissions using the helper script
            let fix_script = "/fix-permissions.sh";
            if std::path::Path::new(fix_script).exists() {
                use std::process::Command;
                match Command::new("sudo")
                    .arg("-n") // Non-interactive
                    .arg(fix_script)
                    .output()
                {
                    Ok(output) => {
                        if output.status.success() {
                            tracing::info!("✅ Permissions fixed successfully via helper script");
                            // Try again after fix
                            match fs::File::create(&test_file) {
                                Ok(_) => {
                                    fs::remove_file(&test_file)
                                        .map_err(|e| tracing::warn!("Failed to remove test file: {}", e)).ok();
                                    tracing::info!("✅ Write permission verified after fix");
                                }
                                Err(e2) => {
                                    return Err(anyhow::anyhow!(
                                        "No write permission for strategies directory {:?}: {}. \
                                        Fix script ran but permission still denied. \
                                        Please restart container or check volume permissions manually. \
                                        Current user: {:?}, UID: {:?}",
                                        strategies_path, e2,
                                        std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
                                        std::env::var("UID").unwrap_or_else(|_| "unknown".to_string())
                                    ));
                                }
                            }
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            tracing::warn!("Fix script failed: {}", stderr);
                            return Err(anyhow::anyhow!(
                                "No write permission for strategies directory {:?}: {}. \
                                Attempted to fix but failed: {}. \
                                Please restart container or run: sudo /fix-permissions.sh \
                                Current user: {:?}, UID: {:?}",
                                strategies_path, e, stderr,
                                std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
                                std::env::var("UID").unwrap_or_else(|_| "unknown".to_string())
                            ));
                        }
                    }
                    Err(e2) => {
                        tracing::warn!("Could not run fix script: {}", e2);
                        return Err(anyhow::anyhow!(
                            "No write permission for strategies directory {:?}: {}. \
                            Could not run fix script: {}. \
                            Please restart container or check volume permissions. \
                            Current user: {:?}, UID: {:?}",
                            strategies_path, e, e2,
                            std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
                            std::env::var("UID").unwrap_or_else(|_| "unknown".to_string())
                        ));
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "No write permission for strategies directory {:?}: {}. \
                    Fix script not found. Please restart container to fix permissions. \
                    Current user: {:?}, UID: {:?}",
                    strategies_path, e,
                    std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
                    std::env::var("UID").unwrap_or_else(|_| "unknown".to_string())
                ));
            }
        }
    }

    // Use config parameters if available, otherwise extract from conditions
    let parameters = if let Some(cfg) = config {
        cfg.parameters.clone()
    } else {
        // Fallback: create empty parameters object
        serde_json::json!({})
    };

    // Generate filename first to use for class name
    let safe_name = strategy_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>();
    let filename = format!("{}_{}.py", safe_name, strategy_id);
    
    // Class name must match filename (without .py) for Freqtrade
    // Freqtrade expects: filename RSI_5m_BTCUSDT_3.py -> class RSI_5m_BTCUSDT_3
    let class_name = filename
        .trim_end_matches(".py")
        .to_string();
    
    // Use new modular template generation system
    use crate::commands::backtest_template::StrategyTemplateData;
    let template_data = StrategyTemplateData::from_config(
        algorithm,
        buy_condition,
        sell_condition,
        timeframe,
        &parameters,
        &class_name,
    );

    // Generate Python code directly (no need for Askama template)
    let code = template_data.generate_python_code();

    // Write file
    let filepath = strategies_path.join(&filename);
    tracing::info!("Writing strategy file to: {:?}", filepath);
    fs::write(&filepath, code)
        .map_err(|e| anyhow::anyhow!(
            "Failed to write strategy file to {:?}: {}. \
            Please check write permissions for strategies directory.",
            filepath, e
        ))?;
    
    tracing::info!("✅ Successfully created strategy file: {:?}", filepath);
    Ok(filepath)
}

/// Handler to start backtest wizard
pub async fn handle_backtest(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    dialogue.update(BotState::Backtest(BacktestState::Start)).await?;

    let telegram_id = msg.from.as_ref().unwrap().id.0 as i64;
    let db = state.db.clone();

    // Get user language
    let user = shared::entity::users::Entity::find_by_id(telegram_id)
        .one(state.db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");

    // Get user's strategies
    use sea_orm::QueryOrder;
    let user_strategies = strategies::Entity::find()
        .filter(strategies::Column::TelegramId.eq(telegram_id.to_string()))
        .order_by_desc(strategies::Column::CreatedAt)
        .all(db.as_ref())
        .await?;

    if user_strategies.is_empty() {
        let empty_msg = i18n::translate(locale, "error_no_strategies", None);
        bot.send_message(msg.chat.id, empty_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        dialogue.exit().await?;
        return Ok(());
    }

    // Create inline keyboard with strategies
    let mut buttons = Vec::new();
    for strategy in &user_strategies {
        let name = strategy.name.as_ref()
            .map(|n| escape_html(n))
            .unwrap_or_else(|| format!("Strategy #{}", strategy.id));
        // Use character-based truncation to avoid UTF-8 boundary errors
        let button_text = if name.chars().count() > 30 {
            let truncated: String = name.chars().take(27).collect();
            format!("{}...", truncated)
        } else {
            name
        };
        buttons.push(vec![
            InlineKeyboardButton::callback(
                button_text,
                format!("backtest_strategy_{}", strategy.id)
            )
        ]);
    }
    buttons.push(vec![
        InlineKeyboardButton::callback(
            i18n::get_button_text(locale, "backtest_button_cancel"),
            "backtest_cancel"
        )
    ]);

    let welcome_msg = i18n::translate(locale, "backtest_welcome", Some(&[("count", &user_strategies.len().to_string())]));

    bot.send_message(msg.chat.id, welcome_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(teloxide::types::InlineKeyboardMarkup::new(buttons))
        .await?;

    dialogue.update(BotState::Backtest(BacktestState::WaitingForStrategy)).await?;

    Ok(())
}

/// Handler for backtest callback queries
pub async fn handle_backtest_callback(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    // Get user locale
    let user_id = q.from.id.0 as i64;
    let user = shared::entity::users::Entity::find_by_id(user_id)
        .one(state.db.as_ref())
        .await?;
    let locale = user
        .as_ref()
        .and_then(|u| u.language.as_ref())
        .map(|l| i18n::get_user_language(Some(l)))
        .unwrap_or("en");
    
    if let Some(data) = q.data {
        if let Some(msg) = q.message {
            let chat_id = msg.chat().id;
            let message_id = msg.id();

            match data.as_str() {
                "backtest_cancel" => {
                    bot.answer_callback_query(q.id).await?;
                    let cancel_msg = i18n::translate(locale, "backtest_cancelled", None);
                    bot.edit_message_text(chat_id, message_id, cancel_msg)
                        .await?;
                    dialogue.exit().await?;
                    return Ok(());
                }
                _ if data.starts_with("backtest_strategy_") => {
                    bot.answer_callback_query(q.id).await?;
                    let strategy_id: u64 = data.replace("backtest_strategy_", "").parse()?;
                    
                    // Get strategy from database
                    let strategy = strategies::Entity::find_by_id(strategy_id)
                        .one(state.db.as_ref())
                        .await?;

                    if let Some(strategy) = strategy {
                        let strategy_name = strategy.name.as_ref()
                            .unwrap_or(&format!("Strategy{}", strategy_id))
                            .clone();

                        // Show exchange selection
                        let exchange_buttons = vec![
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "backtest_exchange_binance"),
                                    "backtest_exchange_binance"
                                ),
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "backtest_exchange_okx"),
                                    "backtest_exchange_okx"
                                ),
                            ],
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "backtest_button_cancel"),
                                    "backtest_cancel"
                                ),
                            ],
                        ];

                        let escaped_name = escape_html(&strategy_name);
                        let strategy_selected = i18n::translate(locale, "backtest_strategy_selected", Some(&[("strategy_name", &escaped_name)]));
                        bot.edit_message_text(chat_id, message_id, strategy_selected)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(exchange_buttons))
                            .await?;

                        dialogue.update(BotState::Backtest(BacktestState::WaitingForExchange {
                            strategy_id,
                            strategy_name,
                        })).await?;
                    }
                }
                _ if data.starts_with("backtest_exchange_") => {
                    bot.answer_callback_query(q.id).await?;
                    let exchange = data.replace("backtest_exchange_", "");

                    if let Ok(Some(BotState::Backtest(BacktestState::WaitingForExchange { strategy_id, strategy_name }))) = dialogue.get().await {
                        // Show time range selection
                        let timerange_buttons = vec![
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_1day"),
                                    "backtest_timerange_1day"
                                ),
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_1week"),
                                    "backtest_timerange_1week"
                                ),
                            ],
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_1month"),
                                    "backtest_timerange_1month"
                                ),
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_3months"),
                                    "backtest_timerange_3months"
                                ),
                            ],
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_6months"),
                                    "backtest_timerange_6months"
                                ),
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_9months"),
                                    "backtest_timerange_9months"
                                ),
                            ],
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "period_1year"),
                                    "backtest_timerange_1year"
                                ),
                            ],
                            vec![
                                InlineKeyboardButton::callback(
                                    i18n::get_button_text(locale, "backtest_button_cancel"),
                                    "backtest_cancel"
                                ),
                            ],
                        ];

                        let escaped_exchange = escape_html(&exchange);
                        let exchange_selected = i18n::translate(locale, "backtest_exchange_selected", Some(&[("exchange", &escaped_exchange)]));
                        bot.edit_message_text(chat_id, message_id, exchange_selected)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(teloxide::types::InlineKeyboardMarkup::new(timerange_buttons))
                            .await?;

                        dialogue.update(BotState::Backtest(BacktestState::WaitingForTimeRange {
                            strategy_id,
                            strategy_name,
                            exchange,
                        })).await?;
                    }
                }
                _ if data.starts_with("backtest_timerange_") => {
                    bot.answer_callback_query(q.id).await?;
                    let timerange = data.replace("backtest_timerange_", "");

                    if let Ok(Some(BotState::Backtest(BacktestState::WaitingForTimeRange { strategy_id, strategy_name, exchange }))) = dialogue.get().await {
                        // Get strategy details
                        let strategy = strategies::Entity::find_by_id(strategy_id)
                            .one(state.db.as_ref())
                            .await?;

                        // Try to get config from strategy service (supports both content JSON and description)
                        let (algorithm, buy_condition, sell_condition, timeframe, pair, strategy_config) = 
                            if let Some(strategy_model) = strategy.as_ref() {
                                match state.strategy_service.strategy_to_config(strategy_model) {
                                    Ok(config) => {
                                        // Use validated config from content field
                                        (
                                            config.strategy_type.clone(),
                                            config.buy_condition.clone(),
                                            config.sell_condition.clone(),
                                            config.timeframe.clone(),
                                            config.pair.clone(),
                                            Some(config),
                                        )
                                    }
                                    Err(e) => {
                                        // Fallback to parsing description
                                        tracing::warn!("Failed to parse strategy config: {}, falling back to description parsing", e);
                                        let strategy_desc = strategy_model.description.as_ref()
                                            .map(|s| s.as_str())
                                            .unwrap_or("");
                                        let (alg, buy, sell, tf, pr) = parse_strategy_description(strategy_desc);
                                        (alg, buy, sell, tf, pr, None)
                                    }
                                }
                            } else {
                                ("RSI".to_string(), "RSI < 30".to_string(), "RSI > 70".to_string(), "1h".to_string(), "BTC/USDT".to_string(), None)
                            };
                        
                        // Convert pair format if needed (BTCUSDT -> BTC/USDT)
                        let freqtrade_pair = if pair.contains('/') {
                            pair.clone()
                        } else {
                            // Convert BTCUSDT to BTC/USDT
                            if pair.len() > 4 && pair.ends_with("USDT") {
                                format!("{}/{}", &pair[..pair.len()-4], &pair[pair.len()-4..])
                            } else if pair.len() > 3 && pair.ends_with("BTC") {
                                format!("{}/{}", &pair[..pair.len()-3], &pair[pair.len()-3..])
                            } else {
                                format!("{}/USDT", pair) // Default to USDT pair
                            }
                        };

                        // Show processing message
                        let running_msg = i18n::translate(locale, "backtest_running", None);
                        bot.edit_message_text(chat_id, message_id, running_msg)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;

                        // Generate strategy file
                        // Use env var for Docker path, fallback to local path
                        let strategies_path_str = std::env::var("STRATEGIES_PATH")
                            .unwrap_or_else(|_| "./docker/freqtrade/strategies".to_string());
                        let strategies_path = Path::new(&strategies_path_str);
                        
                        match generate_strategy_file(
                            strategy_id,
                            &strategy_name,
                            &algorithm,
                            &buy_condition,
                            &sell_condition,
                            &timeframe,
                            strategies_path,
                            strategy_config.as_ref(),
                        ) {
                            Ok(filepath) => {
                                tracing::info!("Generated strategy file: {:?}", filepath);

                                // Get strategy name for Freqtrade
                                let default_name = format!("Strategy{}", strategy_id);
                                let freq_strategy_name = filepath.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(&default_name);

                                // Prepare timerange
                                let timerange_str = calculate_timerange(&timerange);

                                // Initialize Freqtrade client
                                let freq_client = FreqtradeApiClient::new(
                                    "http://127.0.0.1:9081".to_string(),
                                    "freqtrader".to_string(),
                                    "freqtraderpass".to_string(),
                                );

                                // Check if Freqtrade is running
                                // match freq_client.ping().await {
                                //     Ok(_) => {}
                                //     Err(e) => {
                                //         bot.edit_message_text(
                                //             chat_id,
                                //             message_id,
                                //             format!("❌ <b>Freqtrade Not Available</b>\n\nError: {}", e)
                                //         )
                                //             .parse_mode(teloxide::types::ParseMode::Html)
                                //             .await?;
                                //         dialogue.exit().await?;
                                //         return Ok(());
                                //     }
                                // }

                                // Update message - checking/downloading data
                                let checking_msg = i18n::translate(
                                    locale,
                                    "backtest_checking_data",
                                    Some(&[
                                        ("strategy", &escape_html(&strategy_name)),
                                        ("exchange", &escape_html(&exchange)),
                                        ("pair", &freqtrade_pair),
                                        ("timeframe", &timeframe),
                                        ("timerange", &timerange),
                                    ]),
                                );
                                bot.edit_message_text(chat_id, message_id, checking_msg)
                                    .parse_mode(teloxide::types::ParseMode::Html)
                                    .await?;

                                // Run backtest via CLI (with data download check)
                                let result = match freq_client.backtest_via_cli(
                                    "wisetrader_freqtrade",
                                    freq_strategy_name,
                                    &exchange,
                                    &freqtrade_pair,
                                    &timeframe,
                                    &timerange_str,
                                ).await {
                                    Ok(result) => Ok(result),
                                    Err(e) => {
                                        // Truncate error message to avoid Telegram MESSAGE_TOO_LONG error
                                        // Use character-based truncation to avoid UTF-8 boundary errors
                                        let error_str = e.to_string();
                                        let truncated_error = if error_str.chars().count() > 1500 {
                                            let truncated: String = error_str.chars().take(1500).collect();
                                            let truncated_msg = i18n::translate(locale, "backtest_error_truncated", None);
                                            format!("{}{}", truncated, truncated_msg)
                                        } else {
                                            error_str.clone()
                                        };
                                        
                                        let failed_msg = i18n::translate(
                                            locale,
                                            "backtest_failed",
                                            Some(&[
                                                ("error", &escape_html(&truncated_error)),
                                                ("filepath", &filepath.display().to_string()),
                                            ]),
                                        );
                                        bot.edit_message_text(chat_id, message_id, failed_msg)
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .await?;
                                        Err(e)
                                    }
                                };

                                match result {
                                    Ok(result) => {
                                        tracing::info!(
                                            "Backtest succeeded: strategy={} exchange={} trades={} profit_pct={:.2}",
                                            strategy_name,
                                            exchange,
                                            result.trades,
                                            result.profit_pct
                                        );
                                        
                                        // Build result message with detailed report table
                                        let mut result_msg = i18n::translate(
                                            locale,
                                            "backtest_result_header",
                                            Some(&[
                                                ("strategy", &escape_html(&strategy_name)),
                                                ("exchange", &escape_html(&exchange)),
                                                ("pair", &freqtrade_pair),
                                                ("timerange", &timerange),
                                                ("timeframe", &timeframe),
                                            ]),
                                        );
                                        
                                        // Add detailed results table
                                        result_msg.push_str(&i18n::translate(locale, "backtest_report_title", None));
                                        result_msg.push_str("\n");
                                        result_msg.push_str("━━━━━━━━━━\n");
                                        
                                        // Total trades
                                        result_msg.push_str(&i18n::translate(
                                            locale,
                                            "backtest_total_trades",
                                            Some(&[("count", &result.trades.to_string())]),
                                        ));
                                        result_msg.push_str("\n");
                                        
                                        // Profit with color indication
                                        let profit_key = if result.profit_pct >= 0.0 {
                                            "backtest_profit"
                                        } else {
                                            "backtest_profit_negative"
                                        };
                                        result_msg.push_str(&i18n::translate(
                                            locale,
                                            profit_key,
                                            Some(&[("profit", &format!("{:.2}", result.profit_pct))]),
                                        ));
                                        result_msg.push_str("\n");
                                        
                                        // Additional metrics if available
                                        if let Some(win_rate) = result.win_rate {
                                            result_msg.push_str(&i18n::translate(
                                                locale,
                                                "backtest_win_rate",
                                                Some(&[("rate", &format!("{:.2}", win_rate))]),
                                            ));
                                            result_msg.push_str("\n");
                                        }
                                        if let Some(drawdown) = result.max_drawdown {
                                            result_msg.push_str(&i18n::translate(
                                                locale,
                                                "backtest_max_drawdown",
                                                Some(&[("drawdown", &format!("{:.2}", drawdown))]),
                                            ));
                                            result_msg.push_str("\n");
                                        }
                                        if let (Some(start), Some(final_bal)) = (result.starting_balance, result.final_balance) {
                                            result_msg.push_str(&i18n::translate(
                                                locale,
                                                "backtest_balance",
                                                Some(&[
                                                    ("start", &format!("{:.2}", start)),
                                                    ("final", &format!("{:.2}", final_bal)),
                                                ]),
                                            ));
                                            result_msg.push_str("\n");
                                        }
                                        
                                        result_msg.push_str("━━━━━━━━━━━━━━━\n\n");
                                        
                                        // Add timing info
                                        result_msg.push_str(&i18n::translate(locale, "backtest_performance_title", None));
                                        result_msg.push_str("\n");
                                        if let Some(dl_time) = result.download_time_secs {
                                            result_msg.push_str(&i18n::translate(
                                                locale,
                                                "backtest_data_download",
                                                Some(&[("time", &dl_time.to_string())]),
                                            ));
                                        } else {
                                            result_msg.push_str(&i18n::translate(locale, "backtest_data_download_skipped", None));
                                        }
                                        result_msg.push_str("\n");
                                        result_msg.push_str(&i18n::translate(
                                            locale,
                                            "backtest_execution_time",
                                            Some(&[("time", &result.backtest_time_secs.to_string())]),
                                        ));
                                        result_msg.push_str("\n");
                                        
                                        let total_time = result.download_time_secs.unwrap_or(0) + result.backtest_time_secs;
                                        result_msg.push_str(&i18n::translate(
                                            locale,
                                            "backtest_total_time",
                                            Some(&[("time", &total_time.to_string())]),
                                        ));
                                        result_msg.push_str("\n\n");
                                        
                                        result_msg.push_str(&i18n::translate(
                                            locale,
                                            "backtest_strategy_file",
                                            Some(&[("filepath", &filepath.display().to_string())]),
                                        ));
                                        
                                        // Extract tables for HTML report and Telegram messages
                                        let tables = if let Some(ref stdout) = result.stdout {
                                            extract_all_tables(stdout)
                                        } else {
                                            Vec::new()
                                        };
                                        
                                        // Generate HTML report if enabled - use config from AppState
                                        let config = state.config.as_ref();
                                        
                                        // Get user fullname for HTML report
                                        let user_fullname = user.as_ref().and_then(|u| u.fullname.clone());
                                        
                                        // Generate HTML report immediately (without AI analysis)
                                        let (html_report_url, html_report_filepath) = if config.generate_html_reports {
                                            match generate_html_report(
                                                &config,
                                                &strategy_name,
                                                &exchange,
                                                &freqtrade_pair,
                                                &timeframe,
                                                &timerange,
                                                user_fullname.clone(),
                                                &result,
                                                &tables,
                                                None, // No AI analysis initially
                                            ).await {
                                                Ok(Some((url, filepath))) => {
                                                    tracing::info!("HTML report generated: {}", url);
                                                    (Some(url), Some(filepath))
                                                }
                                                Ok(None) => (None, None),
                                                Err(e) => {
                                                    tracing::error!("Failed to generate HTML report: {}", e);
                                                    (None, None)
                                                }
                                            }
                                        } else {
                                            (None, None)
                                        };
                                        
                                        // Spawn background task to generate AI analysis and update HTML report
                                        tracing::info!(
                                            "🔍 Checking Gemini analysis conditions: enable_gemini_analysis={}, html_report_filepath.is_some()={}, gemini_api_key.is_some()={}",
                                            config.enable_gemini_analysis,
                                            html_report_filepath.is_some(),
                                            config.gemini_api_key.is_some()
                                        );
                                        
                                        if config.enable_gemini_analysis && html_report_filepath.is_some() {
                                            if let Some(ref api_key) = config.gemini_api_key {
                                                tracing::info!("✅ Starting Gemini AI analysis background task...");
                                                use crate::services::gemini::GeminiService;
                                                
                                                // Determine language based on user locale (before moving into task)
                                                let locale = user.as_ref()
                                                    .and_then(|u| u.language.as_ref())
                                                    .map(|l| l.as_str())
                                                    .unwrap_or("en")
                                                    .to_string();
                                                
                                                // Clone data needed for background task
                                                let gemini = GeminiService::with_config(
                                                    api_key.clone(),
                                                    config.gemini_model_name.clone(),
                                                    config.gemini_base_url.clone(),
                                                    config.gemini_timeout_secs,
                                                );
                                                
                                                let filepath = html_report_filepath.clone().unwrap();
                                                let strategy_name_clone = strategy_name.clone();
                                                let exchange_clone = exchange.clone();
                                                let freqtrade_pair_clone = freqtrade_pair.clone();
                                                let timeframe_clone = timeframe.clone();
                                                let timerange_clone = timerange.clone();
                                                let user_fullname_clone = user_fullname.clone();
                                                let result_clone = result.clone();
                                                let tables_clone = tables.clone();
                                                
                                                // Spawn background task
                                                tokio::spawn(async move {
                                                    tracing::info!("🔄 Starting background Gemini AI analysis...");
                                                    
                                                    let analysis_result = if locale.as_str() == "vi" {
                                                    gemini.analyze_backtest(
                                                            &strategy_name_clone,
                                                            &exchange_clone,
                                                            &freqtrade_pair_clone,
                                                            &timeframe_clone,
                                                            &timerange_clone,
                                                            result_clone.trades,
                                                            result_clone.profit_pct,
                                                            result_clone.win_rate,
                                                            result_clone.max_drawdown,
                                                            result_clone.starting_balance,
                                                            result_clone.final_balance,
                                                            &tables_clone,
                                                            result_clone.stdout.as_deref(),
                                                    ).await
                                                } else {
                                                    gemini.analyze_backtest_en(
                                                            &strategy_name_clone,
                                                            &exchange_clone,
                                                            &freqtrade_pair_clone,
                                                            &timeframe_clone,
                                                            &timerange_clone,
                                                            result_clone.trades,
                                                            result_clone.profit_pct,
                                                            result_clone.win_rate,
                                                            result_clone.max_drawdown,
                                                            result_clone.starting_balance,
                                                            result_clone.final_balance,
                                                            &tables_clone,
                                                            result_clone.stdout.as_deref(),
                                                    ).await
                                                };
                                                
                                                match analysis_result {
                                                    Ok(analysis) => {
                                                            tracing::info!("✅ Gemini AI analysis generated successfully (length: {} chars)", analysis.len());
                                                            
                                                            // Update HTML report with AI analysis
                                                            if let Err(e) = update_html_report_with_ai_analysis(
                                                                &filepath,
                                                                &strategy_name_clone,
                                                                &exchange_clone,
                                                                &freqtrade_pair_clone,
                                                                &timeframe_clone,
                                                                &timerange_clone,
                                                                user_fullname_clone,
                                                                &result_clone,
                                                                &tables_clone,
                                                                analysis,
                                                            ).await {
                                                                tracing::error!("⚠️ Failed to update HTML report with AI analysis: {}", e);
                                                            } else {
                                                                tracing::info!("✅ HTML report successfully updated with AI analysis at: {}", filepath.display());
                                                            }
                                                    }
                                                    Err(e) => {
                                                        tracing::warn!("⚠️ Failed to generate Gemini AI analysis: {}", e);
                                                    }
                                                }
                                                });
                                            } else {
                                                tracing::warn!("⚠️ Gemini API key not configured, skipping AI analysis");
                                            }
                                        } else {
                                            if !config.enable_gemini_analysis {
                                                tracing::debug!("Gemini analysis is disabled in config");
                                            }
                                            if html_report_filepath.is_none() {
                                                tracing::debug!("HTML report filepath is None, skipping AI analysis");
                                            }
                                        }
                                        
                                        tracing::info!("html_report_url: {:?}", html_report_url);
                                        // Add HTML report link to summary message if available
                                        if let Some(ref html_url) = html_report_url {
                                            // Telegram HTML link format: <a href="URL">text</a>
                                            // Đảm bảo URL không có spaces và format đúng
                                            let clean_url = html_url.trim();
                                            
                                            result_msg.push_str("\n\n✅🌐 <b>View Full Report:</b>\n");
                                            result_msg.push_str(&format!("<code>{}</code>\n", clean_url));
                                            
                                            // Warning nếu URL là localhost
                                            if clean_url.contains("localhost") || clean_url.contains("127.0.0.1") {
                                                tracing::warn!("URL contains localhost, Telegram users won't be able to access it. URL: {}", clean_url);
                                                result_msg.push_str("\n⚠️ <i>Note: This is a localhost URL. Use a public domain for remote access.</i>");
                                            }
                                            
                                            // Debug: log full message để kiểm tra
                                            tracing::info!("Added HTML link to message. Full message length: {}, URL: {}", result_msg.len(), clean_url);
                                            tracing::debug!("Link HTML format: <a href=\"{}\">Open HTML Report</a>", clean_url);
                                        } else {
                                            tracing::warn!("html_report_url is None, not adding link to message");
                                        }
                                        
                                        bot.edit_message_text(
                                            chat_id,
                                            message_id,
                                            result_msg
                                        )
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .await?;
                                        
                                        // Log full output to console for debugging
                                        tracing::info!("=== Backtest Full Output ===");
                                        if let Some(ref stdout) = result.stdout {
                                            tracing::info!("STDOUT:\n{}", stdout);
                                        }
                                        if let Some(ref stderr) = result.stderr {
                                            if !stderr.is_empty() {
                                                tracing::info!("STDERR:\n{}", stderr);
                                            }
                                        }
                                        tracing::info!("=== End Backtest Output ===");
                                        
                                        // Extract and send all tables from backtest output
                                        if !tables.is_empty() {
                                            // Check if mobile-friendly format is enabled
                                            let use_mobile_format = config.mobile_friendly_tables;
                                            
                                            // Send each table as a separate message for better readability
                                            for (idx, (title, table_content)) in tables.iter().enumerate() {
                                                let table_num = idx + 1;
                                                let total_tables = tables.len();
                                                
                                                // Format title nicely with emoji based on content
                                                let emoji = if title.contains("SUMMARY") {
                                                    "📊"
                                                } else if title.contains("REPORT") {
                                                    "📈"
                                                } else if title.contains("STATS") {
                                                    "📉"
                                                } else {
                                                    "📋"
                                                };
                                                
                                                let formatted_title = format!(
                                                    "{} <b>{}</b> ({}/{})\n",
                                                    emoji,
                                                    escape_html(title),
                                                    table_num,
                                                    total_tables
                                                );
                                                
                                                // Format table content based on mobile-friendly flag
                                                let formatted_content = if use_mobile_format {
                                                    format_table_mobile_friendly(table_content)
                                                } else {
                                                    escape_html(table_content)
                                                };
                                                
                                                // Split table content into chunks if needed
                                                // Use larger chunk size for mobile format (it's more compact)
                                                let chunk_size = if use_mobile_format { 3500 } else { 3200 };
                                                let chunks = split_into_chunks(&formatted_content, chunk_size);
                                                
                                                for (chunk_idx, chunk) in chunks.iter().enumerate() {
                                                    let mut table_msg = if chunk_idx == 0 {
                                                        formatted_title.clone()
                                                    } else {
                                                        format!("{} <b>{} (cont.)</b>\n", emoji, escape_html(title))
                                                    };
                                                    
                                                    if use_mobile_format {
                                                        // Mobile-friendly format: no <pre> tag, just formatted text
                                                        table_msg.push_str(&chunk);
                                                    } else {
                                                        // Original format: use <pre> for monospace
                                                        table_msg.push_str("<pre>");
                                                        table_msg.push_str(chunk);
                                                        table_msg.push_str("</pre>");
                                                    }
                                                    
                                                    bot.send_message(chat_id, table_msg)
                                                        .parse_mode(teloxide::types::ParseMode::Html)
                                                        .await?;
                                                    
                                                    // Small delay between messages to avoid rate limiting
                                                    if chunk_idx < chunks.len() - 1 || idx < total_tables - 1 {
                                                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                                                    }
                                                }
                                            }
                                        } else if let Some(ref stdout) = result.stdout {
                                            // Fallback: if no tables found, send full output in chunks
                                            let chunks = split_into_chunks(stdout, 3500);
                                            let total_chunks = chunks.len();
                                            
                                            for (idx, chunk) in chunks.iter().enumerate() {
                                                let chunk_num = idx + 1;
                                                let mut chunk_msg = format!(
                                                    "📋 <b>Backtest Output ({}/{})</b>\n\n",
                                                    chunk_num,
                                                    total_chunks
                                                );
                                                chunk_msg.push_str("<pre>");
                                                chunk_msg.push_str(&escape_html(chunk));
                                                chunk_msg.push_str("</pre>");
                                                
                                                bot.send_message(chat_id, chunk_msg)
                                                    .parse_mode(teloxide::types::ParseMode::Html)
                                                    .await?;
                                                
                                                if idx < chunks.len() - 1 {
                                                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        // Error already handled above
                                    }
                                }

                                dialogue.exit().await?;
                            }
                            Err(e) => {
                                tracing::error!("Failed to generate strategy file: {}", e);
                                bot.edit_message_text(
                                    chat_id,
                                    message_id,
                                    format!("❌ <b>Failed to generate strategy file</b>\n\nError: {}", e)
                                )
                                    .parse_mode(teloxide::types::ParseMode::Html)
                                    .await?;
                                dialogue.exit().await?;
                            }
                        }
                    }
                }
                _ => {
                    bot.answer_callback_query(q.id).await?;
                }
            }
        }
    }

    Ok(())
}

