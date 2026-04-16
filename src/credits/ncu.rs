//! NCU credit computation per FR-050 (T046-T047).
//!
//! NCU = Normalized Compute Unit: 1 TFLOP/s FP32-second on a reference
//! platform, normalized multidimensionally (compute, memory, storage, network)
//! with DRF dominant-dimension accounting.

use crate::credits::caliber::CaliberClass;
use crate::types::NcuAmount;

/// Compute NCU earned for a given work duration on a given caliber class.
/// Uses the caliber-class NCU/hr rate as the base, then scales by the
/// actual resource utilization fraction.
pub fn compute_ncu_earned(
    caliber: CaliberClass,
    duration_seconds: u64,
    utilization_fraction: f64,
) -> NcuAmount {
    let ncu_per_hour = caliber.ncu_per_hour();
    let hours = duration_seconds as f64 / 3600.0;
    let earned = ncu_per_hour * hours * utilization_fraction.clamp(0.0, 1.0);
    NcuAmount::from_ncu(earned)
}

/// Compute the S_ncu priority signal from a donor's NCU balance.
/// Formula: S_ncu = 1 - exp(-α·balance)
/// α is tuned so that the median donor balance yields S_ncu ≈ 0.7.
/// Per FR-032 and research/08-priority-redesign.md.
pub fn compute_priority_s_ncu(balance: NcuAmount, alpha: f64) -> f64 {
    let b = balance.as_ncu();
    (1.0 - (-alpha * b).exp()).clamp(0.0, 1.0)
}

/// Default alpha for S_ncu computation.
/// Tuned so that ~10 NCU (a few hours of C1 laptop donation) gives S_ncu ≈ 0.7.
pub const DEFAULT_ALPHA: f64 = 0.12;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_balance_gives_zero_priority() {
        let s = compute_priority_s_ncu(NcuAmount::ZERO, DEFAULT_ALPHA);
        assert!(s < 0.01);
    }

    #[test]
    fn high_balance_saturates_near_one() {
        let balance = NcuAmount::from_ncu(100.0);
        let s = compute_priority_s_ncu(balance, DEFAULT_ALPHA);
        assert!(s > 0.99, "100 NCU should saturate near 1.0, got {s}");
    }

    #[test]
    fn median_balance_gives_moderate_priority() {
        let balance = NcuAmount::from_ncu(10.0);
        let s = compute_priority_s_ncu(balance, DEFAULT_ALPHA);
        assert!(s > 0.5 && s < 0.9, "10 NCU should give ~0.7, got {s}");
    }

    #[test]
    fn ncu_earned_scales_with_caliber() {
        let c0 = compute_ncu_earned(CaliberClass::C0, 3600, 1.0);
        let c4 = compute_ncu_earned(CaliberClass::C4, 3600, 1.0);
        assert!(c4.as_ncu() > c0.as_ncu() * 1000.0);
    }

    #[test]
    fn ncu_earned_scales_with_duration() {
        let short = compute_ncu_earned(CaliberClass::C1, 60, 1.0);
        let long = compute_ncu_earned(CaliberClass::C1, 3600, 1.0);
        let ratio = long.as_ncu() / short.as_ncu();
        assert!((ratio - 60.0).abs() < 0.1);
    }

    #[test]
    fn zero_utilization_earns_nothing() {
        let earned = compute_ncu_earned(CaliberClass::C2, 3600, 0.0);
        assert_eq!(earned.as_micro_ncu(), 0);
    }
}
