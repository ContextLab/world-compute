//! T056: Integration test for Rekor transparency log — offline graceful handling.
//!
//! Verifies that record_artifact_signature() and record_policy_decision()
//! return TransparencyLogResult::Unavailable when Rekor is unreachable,
//! rather than panicking.

use sha2::{Digest, Sha256};
use worldcompute::ledger::transparency::{verify_inclusion_proof, InclusionProof, SignedTreeHead};
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

// ─── T032: Inclusion proof with valid hashes verifies correctly ─────────

#[test]
fn inclusion_proof_valid_three_level_tree() {
    // Build a 3-level Merkle tree manually and verify the inclusion proof.
    let leaf = [0x42u8; 32];
    let sibling_1 = [0x01u8; 32];
    let sibling_2 = [0x02u8; 32];

    // Level 1: hash(0x01 || leaf || sibling_1)
    let mut h1 = Sha256::new();
    h1.update([0x01]);
    h1.update(leaf);
    h1.update(sibling_1);
    let level1: [u8; 32] = h1.finalize().into();

    // Level 2: hash(0x01 || level1 || sibling_2)
    let mut h2 = Sha256::new();
    h2.update([0x01]);
    h2.update(level1);
    h2.update(sibling_2);
    let root: [u8; 32] = h2.finalize().into();

    let proof = InclusionProof {
        leaf_hash: leaf,
        tree_size: 4,
        proof_hashes: vec![sibling_1, sibling_2],
        signed_tree_head: SignedTreeHead { tree_size: 4, root_hash: root, signature: vec![] },
    };

    let result = verify_inclusion_proof(&proof).expect("should not error");
    assert!(result, "Valid inclusion proof must verify successfully");

    // Now tamper with one proof hash and verify it fails.
    let mut tampered_sibling = sibling_1;
    tampered_sibling[0] ^= 0xFF;

    let tampered_proof = InclusionProof {
        leaf_hash: leaf,
        tree_size: 4,
        proof_hashes: vec![tampered_sibling, sibling_2],
        signed_tree_head: SignedTreeHead { tree_size: 4, root_hash: root, signature: vec![] },
    };

    let tampered_result = verify_inclusion_proof(&tampered_proof).expect("should not error");
    assert!(!tampered_result, "Tampered proof hash must cause verification failure");
}

// ─── T033: Deliberately wrong proof data fails verification ─────────────

#[test]
fn inclusion_proof_wrong_leaf_hash_fails() {
    let real_leaf = [0xAAu8; 32];
    let sibling = [0xBBu8; 32];

    // Compute correct root from real_leaf.
    let mut h = Sha256::new();
    h.update([0x01]);
    h.update(real_leaf);
    h.update(sibling);
    let root: [u8; 32] = h.finalize().into();

    // Use a different leaf hash -- proof should fail.
    let wrong_leaf = [0xCCu8; 32];
    let proof = InclusionProof {
        leaf_hash: wrong_leaf,
        tree_size: 2,
        proof_hashes: vec![sibling],
        signed_tree_head: SignedTreeHead { tree_size: 2, root_hash: root, signature: vec![] },
    };

    let result = verify_inclusion_proof(&proof).expect("should not error");
    assert!(!result, "Wrong leaf hash must cause proof failure");
}

#[test]
fn inclusion_proof_wrong_root_hash_fails() {
    let leaf = [0x11u8; 32];
    let sibling = [0x22u8; 32];
    let wrong_root = [0xFFu8; 32];

    let proof = InclusionProof {
        leaf_hash: leaf,
        tree_size: 2,
        proof_hashes: vec![sibling],
        signed_tree_head: SignedTreeHead { tree_size: 2, root_hash: wrong_root, signature: vec![] },
    };

    let result = verify_inclusion_proof(&proof).expect("should not error");
    assert!(!result, "Wrong root hash must cause proof failure");
}

#[test]
fn inclusion_proof_empty_proof_wrong_root_fails() {
    // Single-element tree: proof_hashes is empty, leaf_hash should equal root_hash.
    let leaf = [0x55u8; 32];
    let wrong_root = [0x66u8; 32];

    let proof = InclusionProof {
        leaf_hash: leaf,
        tree_size: 1,
        proof_hashes: vec![],
        signed_tree_head: SignedTreeHead { tree_size: 1, root_hash: wrong_root, signature: vec![] },
    };

    let result = verify_inclusion_proof(&proof).expect("should not error");
    assert!(!result, "Single-element proof with mismatched root must fail");
}
