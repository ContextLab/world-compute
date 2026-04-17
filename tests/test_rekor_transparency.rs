//! T056: Integration test for Rekor transparency log — offline graceful handling.
//!
//! Verifies that record_artifact_signature() and record_policy_decision()
//! return TransparencyLogResult::Unavailable when Rekor is unreachable,
//! rather than panicking.

use worldcompute::registry::transparency::{
    record_artifact_signature, record_policy_decision, ProvenanceAttestation, TransparencyLogResult,
};
use worldcompute::types::Timestamp;

#[test]
fn artifact_signature_returns_unavailable_when_rekor_offline() {
    // Point at a non-routable address so the request fails fast.
    std::env::set_var("REKOR_URL", "http://127.0.0.1:1");

    let provenance = ProvenanceAttestation {
        build_source: "github.com/test/repo@abc123".into(),
        build_pipeline: "ci-run-001".into(),
        build_timestamp: Timestamp::now(),
        reproducible: false,
    };

    let result = record_artifact_signature("bafytest123", &[0xDE, 0xAD], &provenance);
    match result {
        TransparencyLogResult::Unavailable(msg) => {
            assert!(!msg.is_empty(), "Unavailable message should be descriptive");
        }
        TransparencyLogResult::Recorded { .. } => {
            panic!("Should not record when Rekor is unreachable");
        }
    }
}

#[test]
fn policy_decision_returns_unavailable_when_rekor_offline() {
    std::env::set_var("REKOR_URL", "http://127.0.0.1:1");

    let result = record_policy_decision("decision-001", "approved", "policy-v1.0");
    match result {
        TransparencyLogResult::Unavailable(msg) => {
            assert!(!msg.is_empty(), "Unavailable message should be descriptive");
        }
        TransparencyLogResult::Recorded { .. } => {
            panic!("Should not record when Rekor is unreachable");
        }
    }
}
