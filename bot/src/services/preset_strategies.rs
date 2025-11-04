use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

/// Preset strategy information parsed from GitHub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetStrategy {
    pub name: String,              // e.g., "Strategy001"
    pub display_name: String,      // e.g., "Strategy 001"
    pub description: String,        // Description from GitHub
    pub indicators: Vec<String>,    // e.g., ["RSI", "EMA"]
    pub buy_condition: String,      // Parsed buy condition
    pub sell_condition: String,     // Parsed sell condition
    pub timeframe: Option<String>, // Default timeframe if specified
    pub github_url: String,        // URL to raw file
    pub backtest_stats: Option<BacktestStats>, // Stats from README
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestStats {
    pub buy_count: Option<u32>,
    pub avg_profit: Option<f64>,
    pub total_profit: Option<f64>,
    pub avg_duration: Option<f64>,
}

/// List of known Freqtrade strategies from local files
/// Located in: docker/freqtrade/strategies/
pub const PRESET_STRATEGIES: &[(&str, &str, &str)] = &[
    ("MACDStrategy_crossed", "MACD Strategy Crossed", "MACD crosses signal with CCI filter"),
    ("ElliotV8_original_ichiv2", "Elliot V8 (Ichiv2)", "Elliot Wave with RSI and EMA indicators"),
];

use std::path::PathBuf;
use tokio::fs;

/// Load strategy file from local filesystem and parse it
pub async fn load_strategy_from_local(strategy_name: &str) -> Result<PresetStrategy> {
    // Get strategy path from env var or use defaults
    let base_path = std::env::var("STRATEGIES_PATH")
        .unwrap_or_else(|_| "/app/strategies".to_string());
    
    // Preset strategies path (read-only from host)
    let presets_path = std::env::var("STRATEGIES_PRESETS_PATH")
        .unwrap_or_else(|_| "/app/strategies_presets".to_string());
    
    // Try multiple possible paths
    // Priority: 1) Generated strategies (read-write volume), 2) Preset strategies (read-only), 3) Local paths
    let possible_paths = vec![
        format!("{}/{}.py", base_path, strategy_name),
        format!("{}/{}.py", presets_path, strategy_name),
        format!("./docker/freqtrade/strategies/{}.py", strategy_name),
        format!("docker/freqtrade/strategies/{}.py", strategy_name),
        format!("../docker/freqtrade/strategies/{}.py", strategy_name),
    ];
    
    let mut content = String::new();
    let mut found = false;
    
    for path_str in possible_paths {
        let path = PathBuf::from(&path_str);
        if path.exists() {
            content = fs::read_to_string(&path)
                .await
                .with_context(|| format!("Failed to read strategy file: {}", path_str))?;
            found = true;
            break;
        }
    }
    
    if !found {
        anyhow::bail!("Strategy file not found: {}.py. Searched in docker/freqtrade/strategies/", strategy_name);
    }
    
    let strategy = parse_strategy_file(&content, strategy_name)?;
    
    Ok(strategy)
}

/// Fetch strategy file from GitHub and parse it (kept for backward compatibility)
pub async fn fetch_strategy_from_github(strategy_name: &str) -> Result<PresetStrategy> {
    load_strategy_from_local(strategy_name).await
}

/// Parse Python strategy file to extract relevant information
fn parse_strategy_file(content: &str, strategy_name: &str) -> Result<PresetStrategy> {
    
    // Find display name
    let display_name = strategy_name
        .chars()
        .enumerate()
        .fold(String::new(), |mut acc, (i, c)| {
            if i > 0 && c.is_uppercase() {
                acc.push(' ');
            }
            acc.push(c);
            acc
        });
    
    // Extract indicators used
    let mut indicators = Vec::new();
    if content.contains("RSI") || content.contains("rsi(") || content.contains("rsi ") {
        indicators.push("RSI".to_string());
    }
    if content.contains("EMA") || content.contains("ema(") || content.contains("ema ") {
        indicators.push("EMA".to_string());
    }
    if content.contains("SMA") || content.contains("sma(") || content.contains("sma ") {
        indicators.push("MA".to_string());
    }
    if content.contains("MACD") || content.contains("macd(") || content.contains("macd ") {
        indicators.push("MACD".to_string());
    }
    if content.contains("bollinger") || content.contains("BB") || content.contains("bb_") {
        indicators.push("Bollinger Bands".to_string());
    }
    
    // Default to RSI if no indicators found
    if indicators.is_empty() {
        indicators.push("RSI".to_string());
    }
    
    // Extract buy condition from populate_buy_trend or similar
    let buy_condition = extract_buy_condition(content).unwrap_or_else(|| {
        // Default based on primary indicator
        if indicators.contains(&"RSI".to_string()) {
            "RSI < 30".to_string()
        } else if indicators.contains(&"MACD".to_string()) {
            "MACD > Signal".to_string()
        } else if indicators.contains(&"EMA".to_string()) {
            "EMA(12) > EMA(26)".to_string()
        } else {
            format!("{} < Lower", indicators[0])
        }
    });
    
    // Extract sell condition from populate_sell_trend or similar
    let sell_condition = extract_sell_condition(content).unwrap_or_else(|| {
        // Default based on primary indicator
        if indicators.contains(&"RSI".to_string()) {
            "RSI > 70".to_string()
        } else if indicators.contains(&"MACD".to_string()) {
            "MACD < Signal".to_string()
        } else if indicators.contains(&"EMA".to_string()) {
            "EMA(12) < EMA(26)".to_string()
        } else {
            format!("{} > Upper", indicators[0])
        }
    });
    
    // Extract timeframe (look for timeframe = '5m', '1h', etc.)
    let timeframe = extract_timeframe(content);
    
    // Build description
    let description = format!(
        "Preset strategy from freqtrade-strategies repo. Uses: {}",
        indicators.join(", ")
    );
    
    Ok(PresetStrategy {
        name: strategy_name.to_string(),
        display_name,
        description,
        indicators,
        buy_condition,
        sell_condition,
        timeframe,
        github_url: format!(
            "docker/freqtrade/strategies/{}.py",
            strategy_name
        ),
        backtest_stats: None, // Could be parsed from README if needed
    })
}

/// Extract buy condition from Python code
fn extract_buy_condition(content: &str) -> Option<String> {
    use regex::Regex;
    // Look for populate_buy_trend function
    let buy_regex = Regex::new(r"(?s)def populate_buy_trend.*?return dataframe").ok()?;
    if let Some(cap) = buy_regex.find(content) {
        let func_body = cap.as_str();
        
        // Try to extract common patterns
        if func_body.contains("rsi") && func_body.contains("<") {
            use regex::Regex;
            if let Some(rsi_match) = Regex::new(r"rsi\s*[<>]\s*(\d+)").unwrap().find(func_body) {
                let threshold = rsi_match.as_str();
                if threshold.contains("<") {
                    return Some(format!("RSI < {}", extract_number(threshold)));
                }
            }
        }
        
        // Try to extract dataframe conditions for RSI
        if func_body.contains("dataframe['rsi']") || func_body.contains("dataframe[\"rsi\"]") {
            use regex::Regex;
            let rsi_pattern = Regex::new(r#"dataframe\[['"]rsi['"]\]\s*[<>=!]+\s*(\d+)"#).ok()?;
            if let Some(m) = rsi_pattern.find(func_body) {
                let full_match = m.as_str();
                if full_match.contains("<") {
                    return Some(format!("RSI < {}", extract_number(full_match)));
                } else if full_match.contains(">") {
                    return Some(format!("RSI > {}", extract_number(full_match)));
                }
            }
        }
        
        // Try to extract MACD cross conditions
        if func_body.contains("crossed_above") && func_body.contains("macd") {
            return Some("MACD crosses above Signal".to_string());
        }
        if func_body.contains("crossed_below") && func_body.contains("macd") {
            return Some("MACD crosses below Signal".to_string());
        }
        
        // Try to extract CCI conditions
        if func_body.contains("cci") && (func_body.contains("<=") || func_body.contains("< -")) {
            use regex::Regex;
            let cci_pattern = Regex::new(r"cci.*?[<=]+\s*([-]?\d+\.?\d*)").ok()?;
            if let Some(m) = cci_pattern.find(func_body) {
                let threshold = extract_number(m.as_str());
                return Some(format!("CCI <= {}", threshold));
            }
        }
        
        // Generic fallback: return simplified condition based on indicators found
        if func_body.contains("rsi") {
            return Some("RSI < 30".to_string());
        }
        if func_body.contains("macd") {
            return Some("MACD > Signal".to_string());
        }
    }
    
    None
}

/// Extract sell condition from Python code
fn extract_sell_condition(content: &str) -> Option<String> {
    use regex::Regex;
    // Look for populate_sell_trend or populate_exit_trend function
    let sell_regex = Regex::new(r"(?s)def (populate_sell_trend|populate_exit_trend).*?return dataframe").ok()?;
    if let Some(cap) = sell_regex.find(content) {
        let func_body = cap.as_str();
        
        // Try to extract common patterns
        if func_body.contains("rsi") && func_body.contains(">") {
            use regex::Regex;
            if let Some(rsi_match) = Regex::new(r"rsi\s*[<>]\s*(\d+)").unwrap().find(func_body) {
                let threshold = rsi_match.as_str();
                if threshold.contains(">") {
                    return Some(format!("RSI > {}", extract_number(threshold)));
                }
            }
        }
        
        // Try to extract dataframe conditions for RSI
        if func_body.contains("dataframe['rsi']") || func_body.contains("dataframe[\"rsi\"]") {
            use regex::Regex;
            let rsi_pattern = Regex::new(r#"dataframe\[['"]rsi['"]\]\s*[<>=!]+\s*(\d+)"#).ok()?;
            if let Some(m) = rsi_pattern.find(func_body) {
                let full_match = m.as_str();
                if full_match.contains(">") {
                    return Some(format!("RSI > {}", extract_number(full_match)));
                } else if full_match.contains("<") {
                    return Some(format!("RSI < {}", extract_number(full_match)));
                }
            }
        }
        
        // Try to extract MACD cross conditions
        if func_body.contains("crossed_below") && func_body.contains("macd") {
            return Some("MACD crosses below Signal".to_string());
        }
        if func_body.contains("crossed_above") && func_body.contains("macd") {
            return Some("MACD crosses above Signal".to_string());
        }
        
        // Try to extract CCI conditions
        if func_body.contains("cci") && func_body.contains(">=") {
            use regex::Regex;
            let cci_pattern = Regex::new(r"cci.*?[>=]+\s*(\d+\.?\d*)").ok()?;
            if let Some(m) = cci_pattern.find(func_body) {
                let threshold = extract_number(m.as_str());
                return Some(format!("CCI >= {}", threshold));
            }
        }
        
        // Generic fallback
        if func_body.contains("rsi") {
            return Some("RSI > 70".to_string());
        }
        if func_body.contains("macd") {
            return Some("MACD < Signal".to_string());
        }
    }
    
    None
}

/// Extract timeframe from Python code
fn extract_timeframe(content: &str) -> Option<String> {
    use regex::Regex;
    // Look for timeframe = '5m', '1h', etc.
    let timeframe_regex = Regex::new(r#"timeframe\s*=\s*['"]([0-9]+[mhd])['"]"#).ok()?;
    if let Some(caps) = timeframe_regex.captures(content) {
        if let Some(m) = caps.get(1) {
            return Some(m.as_str().to_string());
        }
    }
    
    // Look in informative_pairs
    let informative_regex = Regex::new(r#"\(['"].*?['"],\s*['"]([0-9]+[mhd])['"]"#).ok()?;
    if let Some(caps) = informative_regex.captures(content) {
        if let Some(m) = caps.get(1) {
            return Some(m.as_str().to_string());
        }
    }
    
    None
}

/// Extract number from string
fn extract_number(s: &str) -> String {
    use regex::Regex;
    Regex::new(r"\d+")
        .unwrap()
        .find(s)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "30".to_string())
}

/// Get list of all preset strategies
pub fn get_preset_strategy_list() -> Vec<(&'static str, &'static str)> {
    PRESET_STRATEGIES
        .iter()
        .map(|(name, display, _)| (*name, *display))
        .collect()
}

