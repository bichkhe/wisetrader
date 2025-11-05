//! SMA (Simple Moving Average) indicator

use crate::indicators::Indicator;
use ta::indicators::SimpleMovingAverage;
use ta::Next;

/// SMA indicator wrapper
#[derive(Debug)]
pub struct SMA {
    inner: SimpleMovingAverage,
    period: usize,
    update_count: usize,
    last_value: Option<f64>,
}

impl SMA {
    /// Create new SMA indicator
    pub fn new(period: usize) -> Self {
        Self {
            inner: SimpleMovingAverage::new(period).unwrap(),
            period,
            update_count: 0,
            last_value: None,
        }
    }

    /// Get SMA period
    pub fn period(&self) -> usize {
        self.period
    }
}

impl Indicator for SMA {
    fn name(&self) -> &str {
        "SMA"
    }

    fn update(&mut self, value: f64) {
        let sma_value = self.inner.next(value);
        self.update_count += 1;
        if self.update_count >= self.period {
            self.last_value = Some(sma_value);
        }
    }

    fn value(&self) -> Option<f64> {
        self.last_value
    }

    fn is_ready(&self) -> bool {
        self.update_count >= self.period
    }
}

/// Calculate SMA from a series of values
pub fn calculate_sma(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut sma = SMA::new(period);
    let mut results = Vec::new();
    
    for &value in values {
        sma.update(value);
        results.push(sma.value());
    }
    
    results
}

