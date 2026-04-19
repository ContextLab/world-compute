//! Adversarial test: byzantine donor returning wrong results.
//!
//! T081: byzantine_data_corruption — one corrupted replica in a 3-replica quorum
//! T082: byzantine_quorum_bypass — 2 colluding nodes vs 1 honest node

use worldcompute::data_plane::cid_store::compute_cid;
use worldcompute::verification::audit::audit_decision;
use worldcompute::verification::quorum::{evaluate_quorum, ReplicaResult};
use worldcompute::verification::trust_score::{compute_trust_score, TrustScoreInputs};

/// T081: Verify that 1 corrupted replica out of 3 is detected and flagged.
///
/// Creates 3 replicas of a task result where 2 return the correct hash
/// and 1 returns a corrupted hash. Runs quorum verification and asserts:
/// 1. The 2-of-3 majority is accepted.
/// 2. The dissenting node is flagged.
/// 3. The dissenting node's trust score drops with high failure rate.
#[test]
fn byzantine_data_corruption() {
    // Compute the "correct" result CID (honest nodes agree on this)
    let correct_data = b"correct computation result: sha256(input_data) = 0xabcdef...";
    let correct_cid = compute_cid(correct_data).unwrap();

    // Compute the "corrupted" result CID (byzantine node XOR'd its output)
    let corrupted_data = b"corrupted computation result: XOR(0xFF) applied to output";
    let corrupted_cid = compute_cid(corrupted_data).unwrap();

    // Verify the CIDs are different
    assert_ne!(correct_cid, corrupted_cid, "Correct and corrupted CIDs must differ");

    // Create 3 replica results: 2 honest, 1 byzantine
    let results = vec![
        ReplicaResult {
            node_id: "honest-node-A".into(),
            result_cid: correct_cid,
            execution_ms: 1500,
        },
        ReplicaResult {
            node_id: "honest-node-B".into(),
            result_cid: correct_cid,
            execution_ms: 1600,
        },
        ReplicaResult {
            node_id: "byzantine-node-C".into(),
            result_cid: corrupted_cid,
            execution_ms: 1400,
        },
    ];

    // Run quorum verification (min_replicas = 3)
    let outcome = evaluate_quorum(&results, 3).expect("Quorum should succeed with 2-of-3 majority");

    // 1. The correct CID is accepted
    assert!(outcome.quorum_reached, "Quorum must be reached with 2-of-3 agreement");
    assert_eq!(outcome.accepted_cid, correct_cid, "The honest majority's CID must be accepted");

    // 2. The honest nodes are in the agreeing set
    assert_eq!(outcome.agreeing_nodes.len(), 2, "Two honest nodes should agree");
    assert!(
        outcome.agreeing_nodes.contains(&"honest-node-A".to_string()),
        "Honest node A must be in agreeing set"
    );
    assert!(
        outcome.agreeing_nodes.contains(&"honest-node-B".to_string()),
        "Honest node B must be in agreeing set"
    );

    // 3. The byzantine node is flagged as a dissenter
    assert_eq!(outcome.dissenting_nodes.len(), 1, "One byzantine node should dissent");
    assert_eq!(outcome.dissenting_nodes[0], "byzantine-node-C", "Byzantine node C must be flagged");

    // 4. Verify trust score impact: a node with high failure rate gets penalized
    let honest_inputs = TrustScoreInputs {
        result_consistency: 1.0,
        attestation_score: 0.8,
        age_days: 30.0,
        recent_failure_rate: 0.0,
    };
    let byzantine_inputs = TrustScoreInputs {
        result_consistency: 0.0, // Failed quorum check
        attestation_score: 0.8,
        age_days: 30.0,
        recent_failure_rate: 1.0, // 100% recent failure
    };

    let honest_score = compute_trust_score(&honest_inputs);
    let byzantine_score = compute_trust_score(&byzantine_inputs);

    assert!(
        byzantine_score.as_f64() < 0.1,
        "Byzantine node trust score ({}) should drop below 0.1",
        byzantine_score.as_f64()
    );
    assert!(
        honest_score.as_f64() > byzantine_score.as_f64(),
        "Honest score ({}) must exceed byzantine score ({})",
        honest_score.as_f64(),
        byzantine_score.as_f64()
    );
}

/// T082: Verify behavior when 2 colluding nodes return a wrong hash against 1 honest node.
///
/// With 2 colluding nodes and 1 honest node, the wrong hash wins by majority.
/// This is expected (BFT requires > 2/3 honest). We verify:
/// 1. The wrong hash is "accepted" by majority vote (this is correct behavior).
/// 2. The honest node is incorrectly flagged as dissenting.
/// 3. The 3% audit mechanism would eventually catch this via re-execution.
#[test]
fn byzantine_quorum_bypass() {
    let honest_data = b"genuine computation output";
    let honest_cid = compute_cid(honest_data).unwrap();

    let colluding_data = b"colluding nodes' fabricated output";
    let colluding_cid = compute_cid(colluding_data).unwrap();

    assert_ne!(honest_cid, colluding_cid, "Honest and colluding CIDs must differ");

    // 2 colluding nodes + 1 honest node
    let results = vec![
        ReplicaResult {
            node_id: "colluder-X".into(),
            result_cid: colluding_cid,
            execution_ms: 1000,
        },
        ReplicaResult {
            node_id: "colluder-Y".into(),
            result_cid: colluding_cid,
            execution_ms: 1050,
        },
        ReplicaResult { node_id: "honest-Z".into(), result_cid: honest_cid, execution_ms: 1500 },
    ];

    // Quorum accepts the colluding majority (this is expected — not a bug)
    let outcome = evaluate_quorum(&results, 3)
        .expect("Quorum should succeed with 2-of-3 majority (even if wrong)");

    // 1. The colluding CID wins by majority
    assert!(outcome.quorum_reached, "Quorum reached (2-of-3)");
    assert_eq!(
        outcome.accepted_cid, colluding_cid,
        "Colluding majority wins (expected BFT limitation with < 2/3 honest)"
    );

    // 2. The honest node is incorrectly flagged as dissenting
    assert_eq!(outcome.dissenting_nodes.len(), 1);
    assert_eq!(
        outcome.dissenting_nodes[0], "honest-Z",
        "Honest node is incorrectly flagged (BFT limitation)"
    );

    // 3. Verify the 3% audit mechanism exists and would flag this over time
    // The audit_decision function deterministically selects ~3% of results for
    // re-execution. Over many colluded results, roughly 3% will be audited.
    let mut audited_count = 0;
    let total_simulated = 1000;
    for i in 0..total_simulated {
        // Simulate many colluded results with different CIDs
        let fake_data = format!("colluded-result-{i}");
        let fake_cid = compute_cid(fake_data.as_bytes()).unwrap();
        let decision = audit_decision(&fake_cid);
        if decision.should_audit {
            audited_count += 1;
        }
    }

    // Verify audit rate converges to ~3% (with tolerance)
    let audit_rate = audited_count as f64 / total_simulated as f64;
    assert!(
        audit_rate > 0.01 && audit_rate < 0.06,
        "Audit rate ({:.1}%) should be approximately 3% (between 1% and 6%)",
        audit_rate * 100.0,
    );

    // 4. Specifically check that the colluding result CID has a defined audit decision
    let colluding_audit = audit_decision(&colluding_cid);
    assert!(!colluding_audit.reason.is_empty(), "Audit decision must have a reason");
    // Whether this specific CID is audited or not is deterministic but unpredictable;
    // the important thing is the mechanism works and ~3% are caught statistically.
}
