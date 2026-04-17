//! Integration tests for NCU computation (T103).

use worldcompute::credits::caliber::CaliberClass;
use worldcompute::credits::ncu::{compute_ncu_earned, compute_priority_s_ncu, DEFAULT_ALPHA};
use worldcompute::types::NcuAmount;

#[test]
fn ncu_for_known_hardware_c1_laptop_1hr() {
    // C1 laptop at full utilization for 1 hour should earn ~0.1 NCU
    let earned = compute_ncu_earned(CaliberClass::C1, 3600, 1.0);
    let ncu = earned.as_ncu();
    assert!((ncu - 0.1).abs() < 0.001, "C1 for 1 hour should earn ~0.1 NCU, got {ncu}");
}

#[test]
fn caliber_class_assignment_ordering() {
    // C0 < C1 < C2 < C3 < C4 in NCU/hr
    let rates: Vec<f64> =
        [CaliberClass::C0, CaliberClass::C1, CaliberClass::C2, CaliberClass::C3, CaliberClass::C4]
            .iter()
            .map(|c| c.ncu_per_hour())
            .collect();

    for i in 0..rates.len() - 1 {
        assert!(rates[i] < rates[i + 1], "Caliber class NCU rates must be strictly increasing");
    }
}

#[test]
fn drf_dominant_dimension_priority() {
    // Higher NCU balance gives higher priority signal
    let low_balance = NcuAmount::from_ncu(1.0);
    let high_balance = NcuAmount::from_ncu(50.0);
    let s_low = compute_priority_s_ncu(low_balance, DEFAULT_ALPHA);
    let s_high = compute_priority_s_ncu(high_balance, DEFAULT_ALPHA);
    assert!(s_high > s_low, "Higher balance should give higher priority: {s_high} > {s_low}");
}

#[test]
fn ncu_amount_arithmetic() {
    let a = NcuAmount::from_ncu(5.0);
    let b = NcuAmount::from_ncu(3.0);
    let sum = a.saturating_add(b);
    let diff = a.saturating_sub(b);
    assert!((sum.as_ncu() - 8.0).abs() < 0.001);
    assert!((diff.as_ncu() - 2.0).abs() < 0.001);
}
