//! Signal generation and validation

use crate::strategy::{Signal, SignalType};

/// Signal validator
pub struct SignalValidator;

impl SignalValidator {
    /// Validate signal
    pub fn validate(signal: &Signal) -> bool {
        match signal.signal_type {
            SignalType::Buy | SignalType::Sell => {
                signal.entry_price.is_some()
                    && signal.confidence >= 0.0
                    && signal.confidence <= 1.0
            }
            SignalType::Hold => true,
        }
    }

    /// Check if signal is actionable
    pub fn is_actionable(signal: &Signal, min_confidence: f64) -> bool {
        Self::validate(signal)
            && signal.confidence >= min_confidence
            && signal.signal_type != SignalType::Hold
    }
}

