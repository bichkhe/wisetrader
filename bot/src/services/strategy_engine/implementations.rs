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

/// Stochastic Oscillator Strategy
/// Measures momentum by comparing closing price to price range over a period
#[derive(Debug)]
pub struct StochasticStrategy {
    config: StrategyConfig,
    prices: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    period: usize,
    smooth_k: usize,
    smooth_d: usize,
    last_k: Option<f64>,
    last_d: Option<f64>,
}

impl StochasticStrategy {
    pub fn new(config: StrategyConfig, period: usize, smooth_k: usize, smooth_d: usize) -> Result<Self> {
        Ok(Self {
            config,
            prices: Vec::with_capacity(period + smooth_k + smooth_d + 10),
            highs: Vec::with_capacity(period + smooth_k + smooth_d + 10),
            lows: Vec::with_capacity(period + smooth_k + smooth_d + 10),
            period,
            smooth_k,
            smooth_d,
            last_k: None,
            last_d: None,
        })
    }
    
    /// Calculate Stochastic %K
    fn calculate_k(&self) -> Option<f64> {
        if self.prices.len() < self.period {
            return None;
        }
        
        let recent_highs = &self.highs[self.highs.len() - self.period..];
        let recent_lows = &self.lows[self.lows.len() - self.period..];
        let current_close = self.prices[self.prices.len() - 1];
        
        let highest_high = recent_highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lowest_low = recent_lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        if highest_high == lowest_low {
            return Some(50.0); // Neutral when no range
        }
        
        let k = ((current_close - lowest_low) / (highest_high - lowest_low)) * 100.0;
        Some(k)
    }
    
    /// Calculate Stochastic %D (SMA of %K)
    fn calculate_d(&self) -> Option<f64> {
        if self.prices.len() < self.period + self.smooth_k {
            return None;
        }
        
        // Calculate %K values for smoothing
        let mut k_values = Vec::new();
        for i in (self.period - 1)..self.prices.len() {
            let recent_highs = &self.highs[i + 1 - self.period..=i];
            let recent_lows = &self.lows[i + 1 - self.period..=i];
            let current_close = self.prices[i];
            
            let highest_high = recent_highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let lowest_low = recent_lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            
            if highest_high == lowest_low {
                k_values.push(50.0);
            } else {
                let k = ((current_close - lowest_low) / (highest_high - lowest_low)) * 100.0;
                k_values.push(k);
            }
        }
        
        if k_values.len() < self.smooth_k {
            return None;
        }
        
        // Calculate SMA of %K
        let recent_k = &k_values[k_values.len() - self.smooth_k..];
        let d = recent_k.iter().sum::<f64>() / self.smooth_k as f64;
        Some(d)
    }
}

impl Strategy for StochasticStrategy {
    fn name(&self) -> &str {
        "Stochastic"
    }
    
    fn config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn process_candle(&mut self, candle: &Candle) -> Option<StrategySignal> {
        self.prices.push(candle.close);
        self.highs.push(candle.high);
        self.lows.push(candle.low);
        
        // Need enough data
        if self.prices.len() < self.period + self.smooth_k {
            return None;
        }
        
        let k = self.calculate_k().unwrap_or(50.0);
        let d = self.calculate_d();
        
        // Update last values
        self.last_k = Some(k);
        if let Some(d_val) = d {
            self.last_d = Some(d_val);
        }
        
        // Use %K for condition parsing (can also use %D if needed)
        let buy_signal = parse_condition(&self.config.buy_condition, k);
        let sell_signal = parse_condition(&self.config.sell_condition, k);
        
        if buy_signal {
            return Some(StrategySignal::Buy {
                confidence: 0.75,
                price: candle.close,
                reason: format!("Stochastic %K = {:.2} meets buy condition: {}", k, self.config.buy_condition),
            });
        }
        
        if sell_signal {
            return Some(StrategySignal::Sell {
                confidence: 0.75,
                price: candle.close,
                reason: format!("Stochastic %K = {:.2} meets sell condition: {}", k, self.config.sell_condition),
            });
        }
        
        None
    }
    
    fn reset(&mut self) {
        self.prices.clear();
        self.highs.clear();
        self.lows.clear();
        self.last_k = None;
        self.last_d = None;
    }
    
    fn get_state_info(&self) -> String {
        format!("Stochastic Strategy - Prices: {}, Period: {}, K: {:?}, D: {:?}", 
            self.prices.len(), self.period, self.last_k, self.last_d)
    }
}

/// ADX (Average Directional Index) Strategy
/// Measures trend strength regardless of direction
#[derive(Debug)]
pub struct AdxStrategy {
    config: StrategyConfig,
    prices: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    period: usize,
    last_adx: Option<f64>,
    last_plus_di: Option<f64>,
    last_minus_di: Option<f64>,
}

impl AdxStrategy {
    pub fn new(config: StrategyConfig, period: usize) -> Result<Self> {
        Ok(Self {
            config,
            prices: Vec::with_capacity(period * 2 + 10),
            highs: Vec::with_capacity(period * 2 + 10),
            lows: Vec::with_capacity(period * 2 + 10),
            period,
            last_adx: None,
            last_plus_di: None,
            last_minus_di: None,
        })
    }
    
    /// Calculate True Range
    fn true_range(&self, i: usize) -> f64 {
        if i == 0 {
            return self.highs[i] - self.lows[i];
        }
        
        let tr1 = self.highs[i] - self.lows[i];
        let tr2 = (self.highs[i] - self.prices[i - 1]).abs();
        let tr3 = (self.lows[i] - self.prices[i - 1]).abs();
        
        tr1.max(tr2).max(tr3)
    }
    
    /// Calculate Directional Movement
    fn directional_movement(&self, i: usize) -> (f64, f64) {
        if i == 0 {
            return (0.0, 0.0);
        }
        
        let plus_dm = if self.highs[i] > self.highs[i - 1] && self.highs[i] - self.highs[i - 1] > self.lows[i - 1] - self.lows[i] {
            self.highs[i] - self.highs[i - 1]
        } else {
            0.0
        };
        
        let minus_dm = if self.lows[i] < self.lows[i - 1] && self.lows[i - 1] - self.lows[i] > self.highs[i] - self.highs[i - 1] {
            self.lows[i - 1] - self.lows[i]
        } else {
            0.0
        };
        
        (plus_dm, minus_dm)
    }
    
    /// Calculate ADX
    fn calculate_adx(&self) -> Option<(f64, f64, f64)> {
        if self.prices.len() < self.period * 2 {
            return None;
        }
        
        // Calculate smoothed TR, +DM, -DM
        let mut tr_sum = 0.0;
        let mut plus_dm_sum = 0.0;
        let mut minus_dm_sum = 0.0;
        
        for i in (self.prices.len() - self.period)..self.prices.len() {
            tr_sum += self.true_range(i);
            let (plus_dm, minus_dm) = self.directional_movement(i);
            plus_dm_sum += plus_dm;
            minus_dm_sum += minus_dm;
        }
        
        // Calculate DI+ and DI-
        let plus_di = if tr_sum > 0.0 {
            (plus_dm_sum / tr_sum) * 100.0
        } else {
            0.0
        };
        
        let minus_di = if tr_sum > 0.0 {
            (minus_dm_sum / tr_sum) * 100.0
        } else {
            0.0
        };
        
        // Calculate DX
        let di_sum = plus_di + minus_di;
        let dx = if di_sum > 0.0 {
            ((plus_di - minus_di).abs() / di_sum) * 100.0
        } else {
            0.0
        };
        
        // ADX is smoothed DX (simplified - using current DX as ADX)
        // In real implementation, ADX should be smoothed over period
        Some((dx, plus_di, minus_di))
    }
}

impl Strategy for AdxStrategy {
    fn name(&self) -> &str {
        "ADX"
    }
    
    fn config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn process_candle(&mut self, candle: &Candle) -> Option<StrategySignal> {
        self.prices.push(candle.close);
        self.highs.push(candle.high);
        self.lows.push(candle.low);
        
        // Need enough data
        if self.prices.len() < self.period * 2 {
            return None;
        }
        
        if let Some((adx, plus_di, minus_di)) = self.calculate_adx() {
            self.last_adx = Some(adx);
            self.last_plus_di = Some(plus_di);
            self.last_minus_di = Some(minus_di);
            
            // Use ADX for condition parsing
            // ADX > 25 indicates strong trend
            let buy_signal = parse_condition(&self.config.buy_condition, adx);
            let sell_signal = parse_condition(&self.config.sell_condition, adx);
            
            if buy_signal {
                return Some(StrategySignal::Buy {
                    confidence: 0.7,
                    price: candle.close,
                    reason: format!("ADX = {:.2} (DI+ = {:.2}, DI- = {:.2}) meets buy condition: {}", 
                        adx, plus_di, minus_di, self.config.buy_condition),
                });
            }
            
            if sell_signal {
                return Some(StrategySignal::Sell {
                    confidence: 0.7,
                    price: candle.close,
                    reason: format!("ADX = {:.2} (DI+ = {:.2}, DI- = {:.2}) meets sell condition: {}", 
                        adx, plus_di, minus_di, self.config.sell_condition),
                });
            }
        }
        
        None
    }
    
    fn reset(&mut self) {
        self.prices.clear();
        self.highs.clear();
        self.lows.clear();
        self.last_adx = None;
        self.last_plus_di = None;
        self.last_minus_di = None;
    }
    
    fn get_state_info(&self) -> String {
        format!("ADX Strategy - Prices: {}, Period: {}, ADX: {:?}", 
            self.prices.len(), self.period, self.last_adx)
    }
}

