//! EMA (Exponential Moving Average) indicator

use crate::indicators::Indicator;
use ta::indicators::ExponentialMovingAverage;
use ta::Next;

/// EMA indicator wrapper
#[derive(Debug)]
pub struct EMA {
    inner: ExponentialMovingAverage,
    period: usize,
    update_count: usize,
    last_value: Option<f64>,
}

impl EMA {
    /// Create new EMA indicator
    pub fn new(period: usize) -> Self {
        Self {
            inner: ExponentialMovingAverage::new(period).unwrap(),
            period,
            update_count: 0,
            last_value: None,
        }
    }

    /// Get EMA period
    pub fn period(&self) -> usize {
        self.period
    }
}

impl Indicator for EMA {
    fn name(&self) -> &str {
        "EMA"
    }

    fn update(&mut self, value: f64) {
        let ema_value = self.inner.next(value);
        self.update_count += 1;
        if self.update_count >= self.period {
            self.last_value = Some(ema_value);
        }
    }

    fn value(&self) -> Option<f64> {
        self.last_value
    }

    fn is_ready(&self) -> bool {
        self.update_count >= self.period
    }
}

/// Calculate EMA from a series of values
pub fn calculate_ema(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut ema = EMA::new(period);
    let mut results = Vec::new();
    
    for &value in values {
        ema.update(value);
        results.push(ema.value());
    }
    
    results
}

