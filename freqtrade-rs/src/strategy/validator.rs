//! Strategy validation

use crate::Result;
use crate::strategy::Strategy;

/// Strategy validator
pub struct StrategyValidator;

impl StrategyValidator {
    /// Validate strategy configuration
    pub fn validate<T: Strategy>(strategy: &T) -> Result<()> {
        // Check strategy name is not empty
        if strategy.name().is_empty() {
            return Err(anyhow::anyhow!("Strategy name cannot be empty"));
        }
        
        Ok(())
    }
}

