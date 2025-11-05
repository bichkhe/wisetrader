//! Bollinger Bands indicator

use crate::indicators::Indicator;
use ta::Next;
use ta::indicators::BollingerBands as TaBollingerBands;

/// Bollinger Bands indicator wrapper
#[derive(Debug)]
pub struct BollingerBands {
    inner: TaBollingerBands,
    period: usize,
    std_dev: f64,
    update_count: usize,
    last_output: Option<ta::indicators::BollingerBandsOutput>,
}

impl BollingerBands {
    /// Create new Bollinger Bands indicator
    pub fn new(period: usize, std_dev: f64) -> Self {
        Self {
            inner: TaBollingerBands::new(period, std_dev).unwrap(),
            period,
            std_dev,
            update_count: 0,
            last_output: None,
        }
    }

    /// Get upper band
    pub fn upper(&self) -> Option<f64> {
        self.last_output.as_ref().map(|o| o.upper)
    }

    /// Get middle band (SMA)
    pub fn middle(&self) -> Option<f64> {
        self.last_output.as_ref().map(|o| o.average)
    }

    /// Get lower band
    pub fn lower(&self) -> Option<f64> {
        self.last_output.as_ref().map(|o| o.lower)
    }
}

impl Indicator for BollingerBands {
    fn name(&self) -> &str {
        "BollingerBands"
    }

    fn update(&mut self, value: f64) {
        let output = self.inner.next(value);
        self.update_count += 1;
        if self.update_count >= self.period {
            self.last_output = Some(output);
        }
    }

    fn value(&self) -> Option<f64> {
        self.middle()
    }

    fn is_ready(&self) -> bool {
        self.update_count >= self.period
    }
}

/// Bollinger Bands result structure
#[derive(Debug, Clone)]
pub struct BBResult {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

