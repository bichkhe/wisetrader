//! RSI (Relative Strength Index) indicator

use crate::indicators::Indicator;
use ta::indicators::RelativeStrengthIndex;
use ta::Next;

/// RSI indicator wrapper
#[derive(Debug)]
pub struct RSI {
    inner: RelativeStrengthIndex,
    period: usize,
    update_count: usize,
    last_value: Option<f64>,
}

impl RSI {
    /// Create new RSI indicator
    pub fn new(period: usize) -> Self {
        Self {
            inner: RelativeStrengthIndex::new(period).unwrap(),
            period,
            update_count: 0,
            last_value: None,
        }
    }

    /// Get RSI period
    pub fn period(&self) -> usize {
        self.period
    }
}

impl Indicator for RSI {
    fn name(&self) -> &str {
        "RSI"
    }

    fn update(&mut self, value: f64) {
        let rsi_value = self.inner.next(value);
        self.update_count += 1;
        if self.update_count > self.period {
            self.last_value = Some(rsi_value);
        }
    }

    fn value(&self) -> Option<f64> {
        self.last_value
    }

    fn is_ready(&self) -> bool {
        // ta RSI needs period+1 values
        self.update_count > self.period
    }
}

/// Calculate RSI from a series of values
pub fn calculate_rsi(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut rsi = RSI::new(period);
    let mut results = Vec::new();
    
    for &value in values {
        rsi.update(value);
        results.push(rsi.value());
    }
    
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi() {
        let mut rsi = RSI::new(14);
        let values = vec![100.0, 102.0, 101.0, 103.0, 105.0, 104.0, 106.0];
        
        for value in values {
            rsi.update(value);
        }
        
        // RSI needs at least period+1 values to be ready
        // For 14 period, need at least 15 values
        assert!(!rsi.is_ready());
    }
}

