//! Indicator Configurations for Freqtrade Template Generation
//! 
//! This module provides a modular way to define how each indicator maps to Freqtrade template.
//! Each indicator implements IndicatorConfig trait to define its own mapping logic.

use serde_json::Value;
use std::collections::HashMap;

/// Configuration for a single indicator in Freqtrade template
#[derive(Debug, Clone)]
pub struct IndicatorTemplateConfig {
    /// Whether this indicator should be used
    pub enabled: bool,
    /// Parameters for the indicator (period, etc.)
    pub parameters: HashMap<String, i32>,
    /// Entry condition enabled
    pub entry_enabled: bool,
    /// Entry threshold value
    pub entry_threshold: Option<i32>,
    /// Exit condition enabled
    pub exit_enabled: bool,
    /// Exit threshold value
    pub exit_threshold: Option<i32>,
    /// Python code snippet for populate_indicators
    pub indicator_code: Option<String>,
    /// Python code snippet for populate_entry_trend
    pub entry_code: Option<String>,
    /// Python code snippet for populate_exit_trend
    pub exit_code: Option<String>,
}

/// Trait for indicator configuration
/// Each indicator implements this to define how it maps to Freqtrade template
pub trait IndicatorConfig: Send + Sync {
    /// Get the name of the indicator (must match algorithm name)
    fn name(&self) -> &str;
    
    /// Check if this indicator should be enabled based on algorithm name
    fn is_enabled(&self, algorithm: &str) -> bool;
    
    /// Extract parameters from config JSON
    fn extract_parameters(&self, params: &Value) -> HashMap<String, i32>;
    
    /// Parse entry condition from buy_condition string
    fn parse_entry_condition(&self, buy_condition: &str) -> (bool, Option<i32>);
    
    /// Parse exit condition from sell_condition string
    fn parse_exit_condition(&self, sell_condition: &str) -> (bool, Option<i32>);
    
    /// Generate Python code for populate_indicators
    fn generate_indicator_code(&self, params: &HashMap<String, i32>) -> String;
    
    /// Generate Python code for populate_entry_trend
    fn generate_entry_code(&self, threshold: Option<i32>) -> Option<String>;
    
    /// Generate Python code for populate_exit_trend
    fn generate_exit_code(&self, threshold: Option<i32>) -> Option<String>;
}

/// RSI Indicator Config
pub struct RsiConfig;

impl IndicatorConfig for RsiConfig {
    fn name(&self) -> &str {
        "RSI"
    }
    
    fn is_enabled(&self, algorithm: &str) -> bool {
        algorithm.to_uppercase() == "RSI"
    }
    
    fn extract_parameters(&self, params: &Value) -> HashMap<String, i32> {
        let mut map = HashMap::new();
        let period = params
            .get("period")
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
            .unwrap_or(14) as i32;
        map.insert("period".to_string(), period);
        map
    }
    
    fn parse_entry_condition(&self, buy_condition: &str) -> (bool, Option<i32>) {
        let enabled = buy_condition.to_uppercase().contains("RSI") && buy_condition.contains("<");
        let threshold = if enabled {
            extract_threshold(buy_condition, "RSI").or(Some(30))
        } else {
            None
        };
        (enabled, threshold)
    }
    
    fn parse_exit_condition(&self, sell_condition: &str) -> (bool, Option<i32>) {
        let enabled = sell_condition.to_uppercase().contains("RSI") && sell_condition.contains(">");
        let threshold = if enabled {
            extract_threshold(sell_condition, "RSI").or(Some(70))
        } else {
            None
        };
        (enabled, threshold)
    }
    
    fn generate_indicator_code(&self, params: &HashMap<String, i32>) -> String {
        let period = params.get("period").copied().unwrap_or(14);
        format!("dataframe['rsi'] = ta.RSI(dataframe, period={})", period)
    }
    
    fn generate_entry_code(&self, threshold: Option<i32>) -> Option<String> {
        threshold.map(|t| format!("dataframe['rsi'] < {}", t))
    }
    
    fn generate_exit_code(&self, threshold: Option<i32>) -> Option<String> {
        threshold.map(|t| format!("dataframe['rsi'] > {}", t))
    }
}

/// MACD Indicator Config
pub struct MacdConfig;

impl IndicatorConfig for MacdConfig {
    fn name(&self) -> &str {
        "MACD"
    }
    
    fn is_enabled(&self, algorithm: &str) -> bool {
        algorithm.to_uppercase() == "MACD"
    }
    
    fn extract_parameters(&self, params: &Value) -> HashMap<String, i32> {
        let mut map = HashMap::new();
        map.insert("fast".to_string(), params.get("fast").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(12) as i32);
        map.insert("slow".to_string(), params.get("slow").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(26) as i32);
        map.insert("signal".to_string(), params.get("signal").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(9) as i32);
        map
    }
    
    fn parse_entry_condition(&self, buy_condition: &str) -> (bool, Option<i32>) {
        (buy_condition.to_uppercase().contains("MACD"), None)
    }
    
    fn parse_exit_condition(&self, sell_condition: &str) -> (bool, Option<i32>) {
        (sell_condition.to_uppercase().contains("MACD"), None)
    }
    
    fn generate_indicator_code(&self, params: &HashMap<String, i32>) -> String {
        let fast = params.get("fast").copied().unwrap_or(12);
        let slow = params.get("slow").copied().unwrap_or(26);
        let signal = params.get("signal").copied().unwrap_or(9);
        format!(
            "macd = ta.MACD(dataframe, fastperiod={}, slowperiod={}, signalperiod={})\ndataframe['macd'] = macd['macd']\ndataframe['macdsignal'] = macd['macdsignal']\ndataframe['macdhist'] = macd['macdhist']",
            fast, slow, signal
        )
    }
    
    fn generate_entry_code(&self, _threshold: Option<i32>) -> Option<String> {
        Some("(dataframe['macd'] > dataframe['macdsignal']) | (dataframe['macdhist'] > 0)".to_string())
    }
    
    fn generate_exit_code(&self, _threshold: Option<i32>) -> Option<String> {
        Some("(dataframe['macd'] < dataframe['macdsignal']) | (dataframe['macdhist'] < 0)".to_string())
    }
}

/// EMA Indicator Config
pub struct EmaConfig;

impl IndicatorConfig for EmaConfig {
    fn name(&self) -> &str {
        "EMA"
    }
    
    fn is_enabled(&self, algorithm: &str) -> bool {
        let algo_upper = algorithm.to_uppercase();
        algo_upper == "EMA" || algo_upper == "MA" || algo_upper == "SMA"
    }
    
    fn extract_parameters(&self, params: &Value) -> HashMap<String, i32> {
        let mut map = HashMap::new();
        let params_obj = params.as_object();
        if let Some(obj) = params_obj {
            if obj.contains_key("period") && !obj.contains_key("fast") {
                let period = params.get("period").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(20) as i32;
                map.insert("fast".to_string(), period);
                map.insert("slow".to_string(), period);
            } else {
                map.insert("fast".to_string(), params.get("fast").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(12) as i32);
                map.insert("slow".to_string(), params.get("slow").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(26) as i32);
            }
        } else {
            map.insert("fast".to_string(), 12);
            map.insert("slow".to_string(), 26);
        }
        map
    }
    
    fn parse_entry_condition(&self, buy_condition: &str) -> (bool, Option<i32>) {
        (buy_condition.to_uppercase().contains("EMA") || buy_condition.to_uppercase().contains("MA"), None)
    }
    
    fn parse_exit_condition(&self, sell_condition: &str) -> (bool, Option<i32>) {
        (sell_condition.to_uppercase().contains("EMA") || sell_condition.to_uppercase().contains("MA"), None)
    }
    
    fn generate_indicator_code(&self, params: &HashMap<String, i32>) -> String {
        let fast = params.get("fast").copied().unwrap_or(12);
        let slow = params.get("slow").copied().unwrap_or(26);
        format!(
            "dataframe['ema_fast'] = ta.EMA(dataframe, timeperiod={})\ndataframe['ema_slow'] = ta.EMA(dataframe, timeperiod={})",
            fast, slow
        )
    }
    
    fn generate_entry_code(&self, _threshold: Option<i32>) -> Option<String> {
        Some("dataframe['ema_fast'] > dataframe['ema_slow']".to_string())
    }
    
    fn generate_exit_code(&self, _threshold: Option<i32>) -> Option<String> {
        Some("dataframe['ema_fast'] < dataframe['ema_slow']".to_string())
    }
}

/// Bollinger Bands Indicator Config
pub struct BollingerConfig;

impl IndicatorConfig for BollingerConfig {
    fn name(&self) -> &str {
        "Bollinger"
    }
    
    fn is_enabled(&self, algorithm: &str) -> bool {
        let algo_upper = algorithm.to_uppercase();
        algo_upper == "BOLLINGER" || algo_upper == "BOLLINGER BANDS" || algo_upper == "BB"
    }
    
    fn extract_parameters(&self, params: &Value) -> HashMap<String, i32> {
        let mut map = HashMap::new();
        let period = params.get("period").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(20) as i32;
        map.insert("period".to_string(), period);
        map
    }
    
    fn parse_entry_condition(&self, buy_condition: &str) -> (bool, Option<i32>) {
        (buy_condition.to_uppercase().contains("BOLLINGER") || buy_condition.to_uppercase().contains("LOWERBAND"), None)
    }
    
    fn parse_exit_condition(&self, sell_condition: &str) -> (bool, Option<i32>) {
        (sell_condition.to_uppercase().contains("BOLLINGER") || sell_condition.to_uppercase().contains("UPPERBAND"), None)
    }
    
    fn generate_indicator_code(&self, params: &HashMap<String, i32>) -> String {
        let period = params.get("period").copied().unwrap_or(20);
        format!(
            "bollinger = ta.BBANDS(dataframe, timeperiod={}, nbdevup=2, nbdevdn=2)\ndataframe['bb_upper'] = bollinger['upperband']\ndataframe['bb_middle'] = bollinger['middleband']\ndataframe['bb_lower'] = bollinger['lowerband']\ndataframe['bb_percent'] = (dataframe['close'] - dataframe['bb_lower']) / (dataframe['bb_upper'] - dataframe['bb_lower'])",
            period
        )
    }
    
    fn generate_entry_code(&self, _threshold: Option<i32>) -> Option<String> {
        Some("dataframe['bb_percent'] < 0.2".to_string())
    }
    
    fn generate_exit_code(&self, _threshold: Option<i32>) -> Option<String> {
        Some("dataframe['bb_percent'] > 0.8".to_string())
    }
}

/// Stochastic Indicator Config
pub struct StochasticConfig;

impl IndicatorConfig for StochasticConfig {
    fn name(&self) -> &str {
        "Stochastic"
    }
    
    fn is_enabled(&self, algorithm: &str) -> bool {
        algorithm.to_uppercase() == "STOCHASTIC"
    }
    
    fn extract_parameters(&self, params: &Value) -> HashMap<String, i32> {
        let mut map = HashMap::new();
        map.insert("period".to_string(), params.get("period").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(14) as i32);
        map.insert("smooth_k".to_string(), params.get("smooth_k").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(3) as i32);
        map.insert("smooth_d".to_string(), params.get("smooth_d").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(3) as i32);
        map
    }
    
    fn parse_entry_condition(&self, buy_condition: &str) -> (bool, Option<i32>) {
        let enabled = buy_condition.to_uppercase().contains("STOCHASTIC") && buy_condition.contains("<");
        let threshold = if enabled {
            extract_threshold(buy_condition, "Stochastic").or(Some(20))
        } else {
            None
        };
        (enabled, threshold)
    }
    
    fn parse_exit_condition(&self, sell_condition: &str) -> (bool, Option<i32>) {
        let enabled = sell_condition.to_uppercase().contains("STOCHASTIC") && sell_condition.contains(">");
        let threshold = if enabled {
            extract_threshold(sell_condition, "Stochastic").or(Some(80))
        } else {
            None
        };
        (enabled, threshold)
    }
    
    fn generate_indicator_code(&self, params: &HashMap<String, i32>) -> String {
        let period = params.get("period").copied().unwrap_or(14);
        let smooth_k = params.get("smooth_k").copied().unwrap_or(3);
        let smooth_d = params.get("smooth_d").copied().unwrap_or(3);
        format!(
            "stochastic = ta.STOCH(dataframe, fastk_period={}, slowk_period={}, slowd_period={})\ndataframe['stoch_k'] = stochastic['slowk']\ndataframe['stoch_d'] = stochastic['slowd']",
            period, smooth_k, smooth_d
        )
    }
    
    fn generate_entry_code(&self, threshold: Option<i32>) -> Option<String> {
        threshold.map(|t| format!("dataframe['stoch_k'] < {}", t))
    }
    
    fn generate_exit_code(&self, threshold: Option<i32>) -> Option<String> {
        threshold.map(|t| format!("dataframe['stoch_k'] > {}", t))
    }
}

/// ADX Indicator Config
pub struct AdxConfig;

impl IndicatorConfig for AdxConfig {
    fn name(&self) -> &str {
        "ADX"
    }
    
    fn is_enabled(&self, algorithm: &str) -> bool {
        algorithm.to_uppercase() == "ADX"
    }
    
    fn extract_parameters(&self, params: &Value) -> HashMap<String, i32> {
        let mut map = HashMap::new();
        let period = params.get("period").and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64))).unwrap_or(14) as i32;
        map.insert("period".to_string(), period);
        map
    }
    
    fn parse_entry_condition(&self, buy_condition: &str) -> (bool, Option<i32>) {
        let enabled = buy_condition.to_uppercase().contains("ADX") && buy_condition.contains(">");
        let threshold = if enabled {
            extract_threshold(buy_condition, "ADX").or(Some(25))
        } else {
            None
        };
        (enabled, threshold)
    }
    
    fn parse_exit_condition(&self, _sell_condition: &str) -> (bool, Option<i32>) {
        // ADX typically only used for entry (trend strength)
        (false, None)
    }
    
    fn generate_indicator_code(&self, params: &HashMap<String, i32>) -> String {
        let period = params.get("period").copied().unwrap_or(14);
        format!("dataframe['adx'] = ta.ADX(dataframe, timeperiod={})", period)
    }
    
    fn generate_entry_code(&self, threshold: Option<i32>) -> Option<String> {
        threshold.map(|t| format!("dataframe['adx'] > {}", t))
    }
    
    fn generate_exit_code(&self, _threshold: Option<i32>) -> Option<String> {
        None
    }
}

/// Helper function to extract threshold from condition string
fn extract_threshold(condition: &str, indicator: &str) -> Option<i32> {
    if condition.to_uppercase().contains(&indicator.to_uppercase()) {
        for part in condition.split_whitespace() {
            let cleaned = part.trim_matches(&['<', '>', '='][..]);
            if let Ok(num) = cleaned.parse::<i32>() {
                return Some(num);
            }
        }
    }
    None
}

/// Registry of all indicator configs
pub struct IndicatorConfigRegistry {
    configs: Vec<Box<dyn IndicatorConfig>>,
}

impl IndicatorConfigRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            configs: Vec::new(),
        };
        
        // Register all indicator configs
        registry.register(Box::new(RsiConfig));
        registry.register(Box::new(MacdConfig));
        registry.register(Box::new(EmaConfig));
        registry.register(Box::new(BollingerConfig));
        registry.register(Box::new(StochasticConfig));
        registry.register(Box::new(AdxConfig));
        
        registry
    }
    
    /// Register a new indicator config
    pub fn register(&mut self, config: Box<dyn IndicatorConfig>) {
        self.configs.push(config);
    }
    
    /// Get config for a specific algorithm
    pub fn get_config(&self, algorithm: &str) -> Option<&dyn IndicatorConfig> {
        self.configs.iter()
            .find(|c| c.is_enabled(algorithm))
            .map(|c| c.as_ref())
    }
    
    /// Get all configs
    pub fn all_configs(&self) -> &[Box<dyn IndicatorConfig>] {
        &self.configs
    }
}

impl Default for IndicatorConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

