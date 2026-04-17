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

/// Record of a single credit decay application for audit/replay.
#[derive(Debug, Clone)]
pub struct CreditDecayEvent {
    /// The account whose balance was decayed.
    pub account_id: crate::types::PeerId,
    /// Balance before decay was applied.
    pub balance_before: crate::types::NcuAmount,
    /// Balance after decay was applied.
    pub balance_after: crate::types::NcuAmount,
    /// The decay rate used (derived from half-life).
    pub decay_rate: f64,
    /// The floor that was enforced.
    pub floor: crate::types::NcuAmount,
    /// When the decay was applied.
    pub timestamp: crate::types::Timestamp,
}

/// Compute a credit decay event for a given account, applying floor protection
/// and anti-hoarding acceleration.
///
/// - `account_id`: the peer whose balance is being decayed.
/// - `balance`: current balance before decay.
/// - `days_elapsed`: number of days since last decay application.
/// - `trailing_earn_rate`: average daily NCU earn rate over trailing 30 days.
/// - `trailing_redemption`: average daily NCU redemption over trailing period.
/// - `config`: decay configuration (half-life, floor multiplier).
///
/// Anti-hoarding (T125): if balance > 1.1 * trailing_redemption, the effective
/// half-life is reduced by a factor of 1.5 (decay accelerated).
///
/// Floor protection (T124): the decayed balance will not fall below
/// `trailing_earn_rate * config.min_floor_multiplier`.
pub fn compute_decay_event(
    account_id: crate::types::PeerId,
    balance: NcuAmount,
    days_elapsed: f64,
    trailing_earn_rate: NcuAmount,
    trailing_redemption: NcuAmount,
    config: &CreditDecayConfig,
) -> CreditDecayEvent {
    // T125: Anti-hoarding — if balance > 1.1 * trailing redemption, accelerate decay
    let effective_half_life = if trailing_redemption.as_ncu() > 0.0
        && balance.as_ncu() > 1.1 * trailing_redemption.as_ncu()
    {
        config.half_life_days / 1.5
    } else {
        config.half_life_days
    };

    let effective_config = CreditDecayConfig { half_life_days: effective_half_life, ..*config };

    // T123 + T124: apply decay with floor protection
    let balance_after = apply_decay(balance, days_elapsed, trailing_earn_rate, &effective_config);

    let decay_rate = (0.5f64).powf(days_elapsed / effective_half_life);
    let floor = NcuAmount::from_ncu(trailing_earn_rate.as_ncu() * config.min_floor_multiplier);

    CreditDecayEvent {
        account_id,
        balance_before: balance,
        balance_after,
        decay_rate,
        floor,
        timestamp: crate::types::Timestamp::now(),
    }
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
