//! Integration tests for trust score computation (T111).

use worldcompute::verification::trust_score::{
    classify_trust_tier, compute_trust_score, TrustScoreInputs, TrustTier,
};

#[test]
fn new_node_capped_at_half() {
    let inputs = TrustScoreInputs {
        result_consistency: 1.0,
        attestation_score: 1.0,
        age_days: 3.0,
        recent_failure_rate: 0.0,
    };
    let score = compute_trust_score(&inputs);
    assert!(
        score.as_f64() <= 0.501,
        "New node (3 days) must be capped at 0.5, got {}",
        score.as_f64()
    );
}

#[test]
fn mature_node_reaches_full_score() {
    let inputs = TrustScoreInputs {
        result_consistency: 1.0,
        attestation_score: 1.0,
        age_days: 60.0,
        recent_failure_rate: 0.0,
    };
    let score = compute_trust_score(&inputs);
    assert!(score.as_f64() > 0.99, "Mature perfect node should be ~1.0, got {}", score.as_f64());
}

#[test]
fn failure_penalty_reduces_score() {
    let base = TrustScoreInputs {
        result_consistency: 0.9,
        attestation_score: 0.8,
        age_days: 30.0,
        recent_failure_rate: 0.0,
    };
    let penalized = TrustScoreInputs { recent_failure_rate: 0.5, ..base };
    let s_base = compute_trust_score(&base);
    let s_penalized = compute_trust_score(&penalized);
    assert!(
        s_penalized.as_f64() < s_base.as_f64(),
        "Failure penalty should reduce score: {} < {}",
        s_penalized.as_f64(),
        s_base.as_f64()
    );
}

#[test]
fn trust_tier_min_replicas() {
    assert_eq!(TrustTier::T0.min_replicas(), 5);
    assert_eq!(TrustTier::T1.min_replicas(), 3);
    assert_eq!(TrustTier::T3.min_replicas(), 1);
}

#[test]
fn trust_tier_confidential_support() {
    assert!(!TrustTier::T0.supports_confidential());
    assert!(!TrustTier::T2.supports_confidential());
    assert!(TrustTier::T3.supports_confidential());
    assert!(TrustTier::T4.supports_confidential());
}

#[test]
fn classify_wasm_only_as_t0() {
    let tier = classify_trust_tier(false, false, false, false, false, true);
    assert_eq!(tier, TrustTier::T0);
}
