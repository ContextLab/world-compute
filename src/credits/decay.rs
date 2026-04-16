//! Credit decay per FR-051 — exponential half-life with earn-rate floor.

use crate::types::NcuAmount;

/// Configuration for credit decay.
#[derive(Debug, Clone)]
pub struct CreditDecayConfig {
    /// Half-life in days: balance halves every `half_life_days` days.
    pub half_life_days: f64,
    /// Minimum floor expressed as N days of trailing earn rate.
    pub min_floor_multiplier: f64,
}

impl Default for CreditDecayConfig {
    fn default() -> Self {
        Self { half_life_days: 45.0, min_floor_multiplier: 30.0 }
    }
}

/// Apply exponential credit decay to `balance` over `days_elapsed` days.
///
/// Formula:
///   decayed = balance × 0.5^(days_elapsed / half_life_days)
///   floor   = trailing_earn_rate × min_floor_multiplier
///   result  = max(floor, decayed)
pub fn apply_decay(
    balance: NcuAmount,
    days_elapsed: f64,
    trailing_earn_rate: NcuAmount,
    config: &CreditDecayConfig,
) -> NcuAmount {
    let decayed = balance.as_ncu() * (0.5f64).powf(days_elapsed / config.half_life_days);
    let floor = trailing_earn_rate.as_ncu() * config.min_floor_multiplier;
    NcuAmount::from_ncu(decayed.max(floor))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> CreditDecayConfig {
        CreditDecayConfig::default()
    }

    #[test]
    fn no_decay_at_day_zero() {
        let balance = NcuAmount::from_ncu(100.0);
        let rate = NcuAmount::from_ncu(0.1);
        let result = apply_decay(balance, 0.0, rate, &cfg());
        // 0.5^0 = 1.0, so result should equal balance (floor = 3.0 NCU, below 100)
        assert!((result.as_ncu() - 100.0).abs() < 0.001, "got {}", result.as_ncu());
    }

    #[test]
    fn half_balance_at_45_days() {
        let balance = NcuAmount::from_ncu(100.0);
        // Zero earn rate means zero floor, so pure half-life applies
        let result = apply_decay(balance, 45.0, NcuAmount::ZERO, &cfg());
        assert!((result.as_ncu() - 50.0).abs() < 0.001, "got {}", result.as_ncu());
    }

    #[test]
    fn floor_protects_minimum() {
        let balance = NcuAmount::from_ncu(1.0);
        // High earn rate: floor = 0.5 NCU/day × 30 = 15 NCU > decayed 0.5 NCU
        let earn_rate = NcuAmount::from_ncu(0.5);
        let result = apply_decay(balance, 45.0, earn_rate, &cfg());
        let floor = 0.5 * 30.0;
        assert!(
            result.as_ncu() >= floor - 0.001,
            "floor not respected: {} < {}",
            result.as_ncu(),
            floor
        );
    }

    #[test]
    fn zero_earn_rate_means_zero_floor() {
        let balance = NcuAmount::from_ncu(100.0);
        let result = apply_decay(balance, 45.0, NcuAmount::ZERO, &cfg());
        // Floor is 0, decay should give exactly 50 NCU
        assert!((result.as_ncu() - 50.0).abs() < 0.001, "got {}", result.as_ncu());
    }
}
