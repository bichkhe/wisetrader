//! Backtest Template Generation Module
//! 
//! This module provides a cleaner way to generate Freqtrade strategy templates
//! using the indicator config registry pattern.

use serde_json::Value;
use crate::services::strategy_engine::indicator_configs::IndicatorConfigRegistry;

/// Template data for generating Python strategy file
#[derive(Debug, Clone)]
pub struct StrategyTemplateData {
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
    
    // Dynamic indicator code (generated from configs)
    pub indicator_code_blocks: Vec<String>,
    pub entry_conditions: Vec<String>,
    pub exit_conditions: Vec<String>,
}

impl StrategyTemplateData {
    /// Create template data from algorithm and conditions using indicator config registry
    pub fn from_config(
        algorithm: &str,
        buy_condition: &str,
        sell_condition: &str,
        timeframe: &str,
        parameters: &Value,
        strategy_name: &str,
    ) -> Self {
        let registry = IndicatorConfigRegistry::new();
        
        // Find the indicator config for this algorithm
        let indicator_config = registry.get_config(algorithm);
        
        let mut indicator_code_blocks = Vec::new();
        let mut entry_conditions = Vec::new();
        let mut exit_conditions = Vec::new();
        
        if let Some(config) = indicator_config {
            // Extract parameters
            let params = config.extract_parameters(parameters);
            
            // Generate indicator code
            let indicator_code = config.generate_indicator_code(&params);
            indicator_code_blocks.push(indicator_code);
            
            // Parse and generate entry condition
            let (entry_enabled, entry_threshold) = config.parse_entry_condition(buy_condition);
            if entry_enabled {
                if let Some(entry_code) = config.generate_entry_code(entry_threshold) {
                    entry_conditions.push(entry_code);
                }
            }
            
            // Parse and generate exit condition
            let (exit_enabled, exit_threshold) = config.parse_exit_condition(sell_condition);
            if exit_enabled {
                if let Some(exit_code) = config.generate_exit_code(exit_threshold) {
                    exit_conditions.push(exit_code);
                }
            }
        }
        
        Self {
            strategy_name: strategy_name.to_string(),
            minimal_roi_60: "0.05".to_string(),
            minimal_roi_30: "0.03".to_string(),
            minimal_roi_0: "0.01".to_string(),
            stoploss: "-0.10".to_string(),
            trailing_stop: false,
            trailing_stop_positive: "0.02".to_string(),
            trailing_stop_offset: "0.01".to_string(),
            timeframe: timeframe.to_string(),
            startup_candle_count: 200,
            indicator_code_blocks,
            entry_conditions,
            exit_conditions,
        }
    }
    
    /// Generate Python code from template data
    pub fn generate_python_code(&self) -> String {
        let indicator_code = self.indicator_code_blocks.join("\n        ");
        let entry_conditions_code = if self.entry_conditions.is_empty() {
            String::new()
        } else {
            self.entry_conditions.iter()
                .map(|c| format!("        conditions.append({})", c))
                .collect::<Vec<_>>()
                .join("\n")
        };
        
        let exit_code = if self.exit_conditions.is_empty() {
            String::new()
        } else {
            self.exit_conditions.iter()
                .map(|c| format!("        dataframe.loc[\n            ({}),\n            'exit_long'\n        ] = 1", c))
                .collect::<Vec<_>>()
                .join("\n")
        };
        
        format!(
            r#"import talib.abstract as ta
import pandas as pd
from functools import reduce
from pandas import DataFrame
from freqtrade.strategy import IStrategy

class {}(IStrategy):
    INTERFACE_VERSION: int = 3

    minimal_roi = {{
        "60": {},
        "30": {},
        "0": {}
    }}

    stoploss = {}

    trailing_stop = {}
    trailing_stop_positive = {}
    trailing_stop_positive_offset = {}
    trailing_only_offset_is_reached = True

    timeframe = '{}'

    startup_candle_count: int = {}

    def informative_pairs(self):
        return []

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        {}
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        conditions = []
{}
        if conditions:
            dataframe.loc[
                reduce(lambda x, y: x & y, conditions),
                'enter_long'
            ] = 1

        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
{}
        return dataframe
"#,
            self.strategy_name,
            self.minimal_roi_60,
            self.minimal_roi_30,
            self.minimal_roi_0,
            self.stoploss,
            if self.trailing_stop { "True" } else { "False" },
            self.trailing_stop_positive,
            self.trailing_stop_offset,
            self.timeframe,
            self.startup_candle_count,
            indicator_code,
            entry_conditions_code,
            exit_code
        )
    }
}

