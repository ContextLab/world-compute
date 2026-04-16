//! Trust Score computation per FR-052 and data-model §3.16.
//!
//! T = clamp(0,1, 0.5·R_consistency + 0.3·R_attestation + 0.2·R_age)
//!     × (1 − P_recent_failures)
//! Capped at 0.5 for first 7 days, ramps to 1.0 after 30 days.

use crate::types::TrustScore;
use serde::{Deserialize, Serialize};

/// Trust tier classification per data-model §3.16.
/// Determines maximum workload sensitivity and replication factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrustTier {
    /// T0: Browser/WASM donors. R≥5, public-data only.
    T0 = 0,
    /// T1: TPM-attested CPU VM. R=3, standard workloads.
    T1 = 1,
    /// T2: TPM-attested + GPU. R=3, GPU workloads.
    T2 = 2,
    /// T3: SEV-SNP or TDX confidential compute. R=1 allowed.
    T3 = 3,
    /// T4: H100 Confidential Compute. R=1, including confidential GPU training.
    T4 = 4,
}

impl TrustTier {
    /// Minimum replication factor for this trust tier.
    pub fn min_replicas(self) -> u32 {
        match self {
            Self::T0 => 5,
            Self::T1 | Self::T2 => 3,
            Self::T3 | Self::T4 => 1,
        }
    }

    /// Whether this tier can run confidential workloads.
    pub fn supports_confidential(self) -> bool {
        matches!(self, Self::T3 | Self::T4)
    }
}

/// Inputs to the Trust Score computation.
pub struct TrustScoreInputs {
    /// Fraction of results that agreed with quorum [0.0, 1.0]
    pub result_consistency: f64,
    /// Attestation quality score [0.0, 1.0] (1.0 = hardware TEE, 0.0 = soft)
    pub attestation_score: f64,
    /// Node age in days
    pub age_days: f64,
    /// Recent failure rate [0.0, 1.0]
    pub recent_failure_rate: f64,
}

/// Compute the Trust Score from inputs.
pub fn compute_trust_score(inputs: &TrustScoreInputs) -> TrustScore {
    let r_age = (inputs.age_days / 30.0).min(1.0);
    let raw = 0.5 * inputs.result_consistency + 0.3 * inputs.attestation_score + 0.2 * r_age;
    let penalized = raw * (1.0 - inputs.recent_failure_rate);
    let clamped = penalized.clamp(0.0, 1.0);

    // Cap at 0.5 for first 7 days
    let capped = if inputs.age_days < 7.0 { clamped.min(0.5) } else { clamped };

    TrustScore::from_f64(capped)
}

/// Determine trust tier from attestation type and hardware capabilities.
pub fn classify_trust_tier(
    has_tpm: bool,
    has_sev_snp: bool,
    has_tdx: bool,
    has_h100_cc: bool,
    has_gpu: bool,
    is_wasm_only: bool,
) -> TrustTier {
    if is_wasm_only {
        return TrustTier::T0;
    }
    if has_h100_cc {
        return TrustTier::T4;
    }
    if has_sev_snp || has_tdx {
        return TrustTier::T3;
    }
    if has_tpm && has_gpu {
        return TrustTier::T2;
    }
    if has_tpm {
        return TrustTier::T1;
    }
    TrustTier::T0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_node_capped_at_half() {
        let inputs = TrustScoreInputs {
            result_consistency: 1.0,
            attestation_score: 1.0,
            age_days: 3.0,
            recent_failure_rate: 0.0,
        };
        let score = compute_trust_score(&inputs);
        assert!(score.as_f64() <= 0.5001, "New node should be capped at 0.5");
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
        assert!(score.as_f64() > 0.99, "Mature perfect node should be ~1.0");
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
        let s1 = compute_trust_score(&base);
        let s2 = compute_trust_score(&penalized);
        assert!(s2.as_f64() < s1.as_f64() * 0.6);
    }

    #[test]
    fn trust_tier_classification() {
        assert_eq!(classify_trust_tier(false, false, false, false, false, true), TrustTier::T0);
        assert_eq!(classify_trust_tier(true, false, false, false, false, false), TrustTier::T1);
        assert_eq!(classify_trust_tier(true, false, false, false, true, false), TrustTier::T2);
        assert_eq!(classify_trust_tier(true, true, false, false, true, false), TrustTier::T3);
        assert_eq!(classify_trust_tier(true, false, false, true, true, false), TrustTier::T4);
    }
}
