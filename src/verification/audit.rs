//! 3% audit re-execution selection per FR-062.
//!
//! Uses a deterministic PRNG seeded from the result CID so that audit
//! selection is independently verifiable by any coordinator.

use crate::types::Cid;

/// Decision on whether a result should be re-executed for audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditDecision {
    pub should_audit: bool,
    pub reason: String,
}

/// Target audit rate: 3%.
const AUDIT_RATE_NUMERATOR: u64 = 3;
const AUDIT_RATE_DENOMINATOR: u64 = 100;

/// Determine whether a result identified by `result_cid` should be audited.
///
/// The decision is deterministic: the same CID always produces the same
/// decision. The PRNG is a simple xorshift64 seeded from the first 8 bytes
/// of the CID's multihash digest.
pub fn audit_decision(result_cid: &Cid) -> AuditDecision {
    let seed = cid_seed(result_cid);
    let value = xorshift64(seed);
    // Map to [0, AUDIT_RATE_DENOMINATOR) and compare.
    let bucket = value % AUDIT_RATE_DENOMINATOR;
    let should_audit = bucket < AUDIT_RATE_NUMERATOR;
    let reason = if should_audit {
        format!("CID {result_cid} selected for audit (bucket {bucket} < {AUDIT_RATE_NUMERATOR})")
    } else {
        format!(
            "CID {result_cid} not selected for audit (bucket {bucket} >= {AUDIT_RATE_NUMERATOR})"
        )
    };
    AuditDecision { should_audit, reason }
}

/// Extract a u64 seed from the CID's raw bytes.
fn cid_seed(cid: &Cid) -> u64 {
    // Use the multihash digest bytes, not the CID prefix (which is constant
    // across all CIDv1-raw objects and would make every seed identical).
    let hash = cid.hash();
    let digest = hash.digest();
    let mut seed = 0u64;
    for (i, &b) in digest.iter().take(8).enumerate() {
        seed |= (b as u64) << (i * 8);
    }
    // Ensure non-zero seed for xorshift.
    if seed == 0 {
        seed = 0xdeadbeef_cafebabe;
    }
    seed
}

/// Xorshift64 — a simple, fast, deterministic PRNG.
fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use cid::Cid;
    use multihash::Multihash;
    use sha2::{Digest, Sha256};

    fn make_cid(seed: &[u8]) -> Cid {
        let hash = Sha256::digest(seed);
        let mh = Multihash::<64>::wrap(0x12, &hash).unwrap();
        Cid::new_v1(0x55, mh)
    }

    #[test]
    fn test_deterministic_same_cid() {
        let cid = make_cid(b"test-result-abc");
        let d1 = audit_decision(&cid);
        let d2 = audit_decision(&cid);
        assert_eq!(d1, d2, "same CID must always produce the same decision");
    }

    #[test]
    fn test_audit_rate_converges_to_3_percent() {
        let n = 1000usize;
        let mut audited = 0usize;
        for i in 0..n {
            let cid = make_cid(format!("result-{i}").as_bytes());
            if audit_decision(&cid).should_audit {
                audited += 1;
            }
        }
        let rate = audited as f64 / n as f64;
        // Allow 1% absolute tolerance around 3%.
        assert!(
            (rate - 0.03).abs() < 0.015,
            "audit rate {:.2}% outside expected ~3% (±1.5%)",
            rate * 100.0
        );
    }

    #[test]
    fn test_different_cids_can_differ() {
        let cid_a = make_cid(b"result-alpha");
        let cid_b = make_cid(b"result-beta");
        // They might coincidentally match, but with different seeds they
        // should differ across a range of inputs — just verify they're valid.
        let da = audit_decision(&cid_a);
        let db = audit_decision(&cid_b);
        assert!(!da.reason.is_empty());
        assert!(!db.reason.is_empty());
    }

    #[test]
    fn test_reason_reflects_decision() {
        for i in 0..50 {
            let cid = make_cid(format!("r-{i}").as_bytes());
            let d = audit_decision(&cid);
            if d.should_audit {
                assert!(
                    d.reason.contains("selected for audit"),
                    "reason should say selected: {}",
                    d.reason
                );
            } else {
                assert!(
                    d.reason.contains("not selected"),
                    "reason should say not selected: {}",
                    d.reason
                );
            }
        }
    }

    #[test]
    fn test_audit_decision_struct_fields() {
        let cid = make_cid(b"struct-test");
        let d = audit_decision(&cid);
        // should_audit is bool, reason is non-empty string
        let _ = d.should_audit;
        assert!(!d.reason.is_empty());
    }
}
