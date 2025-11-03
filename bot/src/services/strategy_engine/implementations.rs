//! Strategy implementations

use anyhow::Result;
use ta::{indicators::*, Next};
use crate::services::strategy_engine::{
    Strategy, StrategyConfig, StrategySignal, Candle, parse_condition,
};

/// RSI Strategy
#[derive(Debug)]
pub struct RsiStrategy {
    config: StrategyConfig,
    rsi: RelativeStrengthIndex,
    prices: Vec<f64>,
    period: usize,
}

impl RsiStrategy {
    pub fn new(config: StrategyConfig, period: usize) -> Result<Self> {
        Ok(Self {
            config,
            rsi: RelativeStrengthIndex::new(period)
                .map_err(|e| anyhow::anyhow!("Failed to create RSI: {}", e))?,
            prices: Vec::with_capacity(period + 10),
            period,
        })
    }
}

impl Strategy for RsiStrategy {
    fn name(&self) -> &str {
        "RSI"
    }
    
    fn config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn process_candle(&mut self, candle: &Candle) -> Option<StrategySignal> {
        self.prices.push(candle.close);
        
        // Need enough prices for RSI calculation
        if self.prices.len() < self.period + 1 {
            return None;
        }
        
        let rsi_value = self.rsi.next(candle.close);
        
        // Parse buy/sell conditions
        let buy_signal = parse_condition(&self.config.buy_condition, rsi_value);
        let sell_signal = parse_condition(&self.config.sell_condition, rsi_value);
        
        if buy_signal {
            return Some(StrategySignal::Buy {
                confidence: 0.8,
                price: candle.close,
                reason: format!("RSI = {:.2} meets buy condition: {}", rsi_value, self.config.buy_condition),
            });
        }
        
        if sell_signal {
            return Some(StrategySignal::Sell {
                confidence: 0.8,
                price: candle.close,
                reason: format!("RSI = {:.2} meets sell condition: {}", rsi_value, self.config.sell_condition),
            });
        }
        
        None
    }
    
    fn reset(&mut self) {
        self.prices.clear();
        // RSI indicator maintains its own state, but we can reinitialize if needed
    }
    
    fn get_state_info(&self) -> String {
        format!("RSI Strategy - Prices: {}, Period: {}", 
            self.prices.len(), self.period)
    }
}

/// MACD Strategy
#[derive(Debug)]
pub struct MacdStrategy {
    config: StrategyConfig,
    macd: MovingAverageConvergenceDivergence,
    prices: Vec<f64>,
}

impl MacdStrategy {
    pub fn new(config: StrategyConfig, fast: usize, slow: usize, signal: usize) -> Result<Self> {
        Ok(Self {
            config,
            macd: MovingAverageConvergenceDivergence::new(fast, slow, signal)
                .map_err(|e| anyhow::anyhow!("Failed to create MACD: {}", e))?,
            prices: Vec::new(),
        })
    }
}

impl Strategy for MacdStrategy {
    fn name(&self) -> &str {
        "MACD"
    }
    
    fn config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn process_candle(&mut self, candle: &Candle) -> Option<StrategySignal> {
        self.prices.push(candle.close);
        
        if self.prices.len() < 30 {
            return None;
        }
        
        let macd_output = self.macd.next(candle.close);
        
        // MACD cross detection - using histogram value
        // Buy when histogram crosses above zero, sell when crosses below
        let buy_signal = macd_output.histogram > 0.0 
            && self.prices.len() > 1 && self.prices[self.prices.len() - 2] > 0.0; // Simplified
        let sell_signal = macd_output.histogram < 0.0
            && self.prices.len() > 1 && self.prices[self.prices.len() - 2] < 0.0; // Simplified
        
        if buy_signal || macd_output.histogram > 0.0 {
            return Some(StrategySignal::Buy {
                confidence: 0.75,
                price: candle.close,
                reason: format!("MACD histogram = {:.4}", macd_output.histogram),
            });
        }
        
        if sell_signal || macd_output.histogram < 0.0 {
            return Some(StrategySignal::Sell {
                confidence: 0.75,
                price: candle.close,
                reason: format!("MACD histogram = {:.4}", macd_output.histogram),
            });
        }
        
        None
    }
    
    fn reset(&mut self) {
        self.prices.clear();
    }
    
    fn get_state_info(&self) -> String {
        format!("MACD Strategy - Prices: {}", self.prices.len())
    }
}

/// Bollinger Bands Strategy
#[derive(Debug)]
pub struct BollingerStrategy {
    config: StrategyConfig,
    bb: BollingerBands,
    prices: Vec<f64>,
    period: usize,
}

impl BollingerStrategy {
    pub fn new(config: StrategyConfig, period: usize, std_dev: f64) -> Result<Self> {
        Ok(Self {
            config,
            bb: BollingerBands::new(period, std_dev)
                .map_err(|e| anyhow::anyhow!("Failed to create Bollinger Bands: {}", e))?,
            prices: Vec::with_capacity(period + 10),
            period,
        })
    }
}

impl Strategy for BollingerStrategy {
    fn name(&self) -> &str {
        "Bollinger Bands"
    }
    
    fn config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn process_candle(&mut self, candle: &Candle) -> Option<StrategySignal> {
        self.prices.push(candle.close);
        
        if self.prices.len() < self.period {
            return None;
        }
        
        let bb_output = self.bb.next(candle.close);
        
        // Buy when price touches lower band, sell when touches upper band
        let buy_signal = candle.close <= bb_output.lower;
        let sell_signal = candle.close >= bb_output.upper;
        
        if buy_signal {
            return Some(StrategySignal::Buy {
                confidence: 0.7,
                price: candle.close,
                reason: format!("Price {} <= Lower Band {}", candle.close, bb_output.lower),
            });
        }
        
        if sell_signal {
            return Some(StrategySignal::Sell {
                confidence: 0.7,
                price: candle.close,
                reason: format!("Price {} >= Upper Band {}", candle.close, bb_output.upper),
            });
        }
        
        None
    }
    
    fn reset(&mut self) {
        self.prices.clear();
    }
    
    fn get_state_info(&self) -> String {
        format!("Bollinger Strategy - Prices: {}, Period: {}", 
            self.prices.len(), self.period)
    }
}

/// EMA Strategy
#[derive(Debug)]
pub struct EmaStrategy {
    config: StrategyConfig,
    ema: ExponentialMovingAverage,
    prices: Vec<f64>,
    last_price: Option<f64>,
    period: usize,
}

impl EmaStrategy {
    pub fn new(config: StrategyConfig, period: usize) -> Result<Self> {
        Ok(Self {
            config,
            ema: ExponentialMovingAverage::new(period)
                .map_err(|e| anyhow::anyhow!("Failed to create EMA: {}", e))?,
            prices: Vec::new(),
            last_price: None,
            period,
        })
    }
}

impl Strategy for EmaStrategy {
    fn name(&self) -> &str {
        "EMA"
    }
    
    fn config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn process_candle(&mut self, candle: &Candle) -> Option<StrategySignal> {
        self.prices.push(candle.close);
        
        if self.prices.len() < self.period {
            return None;
        }
        
        let ema_value = self.ema.next(candle.close);
        
        if let Some(last_price) = self.last_price {
            // Buy when price crosses above EMA, sell when crosses below
            let buy_signal = last_price <= ema_value && candle.close > ema_value;
            let sell_signal = last_price >= ema_value && candle.close < ema_value;
            
            if buy_signal {
                self.last_price = Some(candle.close);
                return Some(StrategySignal::Buy {
                    confidence: 0.7,
                    price: candle.close,
                    reason: format!("Price {} crossed above EMA {}", candle.close, ema_value),
                });
            }
            
            if sell_signal {
                self.last_price = Some(candle.close);
                return Some(StrategySignal::Sell {
                    confidence: 0.7,
                    price: candle.close,
                    reason: format!("Price {} crossed below EMA {}", candle.close, ema_value),
                });
            }
        }
        
        self.last_price = Some(candle.close);
        None
    }
    
    fn reset(&mut self) {
        self.prices.clear();
        self.last_price = None;
    }
    
    fn get_state_info(&self) -> String {
        format!("EMA Strategy - Prices: {}, Period: {}", 
            self.prices.len(), self.period)
    }
}

/// MA (Simple Moving Average) Strategy
#[derive(Debug)]
pub struct MaStrategy {
    config: StrategyConfig,
    ma: SimpleMovingAverage,
    prices: Vec<f64>,
    last_price: Option<f64>,
    period: usize,
}

impl MaStrategy {
    pub fn new(config: StrategyConfig, period: usize) -> Result<Self> {
        Ok(Self {
            config,
            ma: SimpleMovingAverage::new(period)
                .map_err(|e| anyhow::anyhow!("Failed to create MA: {}", e))?,
            prices: Vec::new(),
            last_price: None,
            period,
        })
    }
}

impl Strategy for MaStrategy {
    fn name(&self) -> &str {
        "MA"
    }
    
    fn config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn process_candle(&mut self, candle: &Candle) -> Option<StrategySignal> {
        self.prices.push(candle.close);
        
        if self.prices.len() < self.period {
            return None;
        }
        
        let ma_value = self.ma.next(candle.close);
        
        if let Some(last_price) = self.last_price {
            // Buy when price crosses above MA, sell when crosses below
            let buy_signal = last_price <= ma_value && candle.close > ma_value;
            let sell_signal = last_price >= ma_value && candle.close < ma_value;
            
            if buy_signal {
                self.last_price = Some(candle.close);
                return Some(StrategySignal::Buy {
                    confidence: 0.65,
                    price: candle.close,
                    reason: format!("Price {} crossed above MA {}", candle.close, ma_value),
                });
            }
            
            if sell_signal {
                self.last_price = Some(candle.close);
                return Some(StrategySignal::Sell {
                    confidence: 0.65,
                    price: candle.close,
                    reason: format!("Price {} crossed below MA {}", candle.close, ma_value),
                });
            }
        }
        
        self.last_price = Some(candle.close);
        None
    }
    
    fn reset(&mut self) {
        self.prices.clear();
        self.last_price = None;
    }
    
    fn get_state_info(&self) -> String {
        format!("MA Strategy - Prices: {}, Period: {}", 
            self.prices.len(), self.period)
    }
}

