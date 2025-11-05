//! MACD (Moving Average Convergence Divergence) indicator

use crate::indicators::Indicator;
use ta::indicators::MovingAverageConvergenceDivergence;
use ta::Next;

/// MACD indicator wrapper
#[derive(Debug)]
pub struct MACD {
    inner: MovingAverageConvergenceDivergence,
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    update_count: usize,
    last_output: Option<ta::indicators::MovingAverageConvergenceDivergenceOutput>,
}

impl MACD {
    /// Create new MACD indicator
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            inner: MovingAverageConvergenceDivergence::new(fast_period, slow_period, signal_period).unwrap(),
            fast_period,
            slow_period,
            signal_period,
            update_count: 0,
            last_output: None,
        }
    }

    /// Get MACD line value
    pub fn macd(&self) -> Option<f64> {
        self.last_output.as_ref().map(|o| o.macd)
    }

    /// Get signal line value
    pub fn signal(&self) -> Option<f64> {
        self.last_output.as_ref().map(|o| o.signal)
    }

    /// Get histogram value (MACD - Signal)
    pub fn histogram(&self) -> Option<f64> {
        self.last_output.as_ref().map(|o| o.histogram)
    }
}

impl Indicator for MACD {
    fn name(&self) -> &str {
        "MACD"
    }

    fn update(&mut self, value: f64) {
        let output = self.inner.next(value);
        self.update_count += 1;
        if self.update_count > self.slow_period + self.signal_period {
            self.last_output = Some(output);
        }
    }

    fn value(&self) -> Option<f64> {
        self.macd()
    }

    fn is_ready(&self) -> bool {
        // MACD needs slow_period + signal_period values
        self.update_count > self.slow_period + self.signal_period
    }
}

/// MACD result structure
#[derive(Debug, Clone)]
pub struct MACDResult {
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}

