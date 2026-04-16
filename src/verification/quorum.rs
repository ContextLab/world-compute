//! R=3 canonical-hash quorum verification per FR-024 (T061).
//!
//! Default verification: R=3 replicas execute on disjoint nodes.
//! A canonical-hash quorum (majority agreement on output hash) decides
//! the accepted result. Disagreeing replicas are flagged for audit.

use crate::error::{ErrorCode, WcError};
use crate::types::Cid;
use std::collections::HashMap;

/// A single replica's result.
#[derive(Debug, Clone)]
pub struct ReplicaResult {
    /// Node that produced this result.
    pub node_id: String,
    /// CID (hash) of the result data.
    pub result_cid: Cid,
    /// Execution duration in milliseconds.
    pub execution_ms: u64,
}

/// Outcome of a quorum vote.
#[derive(Debug, Clone)]
pub struct QuorumOutcome {
    /// The accepted result CID (majority vote).
    pub accepted_cid: Cid,
    /// Nodes that agreed with the majority.
    pub agreeing_nodes: Vec<String>,
    /// Nodes that disagreed (flagged for audit / trust score penalty).
    pub dissenting_nodes: Vec<String>,
    /// Whether a strict majority was reached.
    pub quorum_reached: bool,
}

/// Evaluate a set of replica results and determine the quorum outcome.
/// Requires at least `min_replicas` results; majority wins.
pub fn evaluate_quorum(
    results: &[ReplicaResult],
    min_replicas: u32,
) -> Result<QuorumOutcome, WcError> {
    if results.len() < min_replicas as usize {
        return Err(WcError::new(
            ErrorCode::QuorumFailure,
            format!("Only {} replicas reported, need at least {}", results.len(), min_replicas),
        ));
    }

    // Count votes by result CID
    let mut vote_counts: HashMap<Cid, Vec<String>> = HashMap::new();
    for r in results {
        vote_counts.entry(r.result_cid).or_default().push(r.node_id.clone());
    }

    // Find the CID with the most votes
    let (winning_cid, winning_nodes) = vote_counts
        .iter()
        .max_by_key(|(_, nodes)| nodes.len())
        .map(|(cid, nodes)| (*cid, nodes.clone()))
        .ok_or_else(|| WcError::new(ErrorCode::QuorumFailure, "No results to evaluate"))?;

    let majority_threshold = results.len() / 2 + 1;
    let quorum_reached = winning_nodes.len() >= majority_threshold;

    // All nodes not in the winning set are dissenters
    let dissenting_nodes: Vec<String> =
        results.iter().filter(|r| r.result_cid != winning_cid).map(|r| r.node_id.clone()).collect();

    if !quorum_reached {
        return Err(WcError::new(
            ErrorCode::QuorumFailure,
            format!(
                "No majority: best result has {}/{} votes (need {})",
                winning_nodes.len(),
                results.len(),
                majority_threshold
            ),
        ));
    }

    Ok(QuorumOutcome {
        accepted_cid: winning_cid,
        agreeing_nodes: winning_nodes,
        dissenting_nodes,
        quorum_reached,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::cid_store::compute_cid;

    #[test]
    fn unanimous_quorum_passes() {
        let cid = compute_cid(b"correct result").unwrap();
        let results = vec![
            ReplicaResult { node_id: "A".into(), result_cid: cid, execution_ms: 100 },
            ReplicaResult { node_id: "B".into(), result_cid: cid, execution_ms: 110 },
            ReplicaResult { node_id: "C".into(), result_cid: cid, execution_ms: 105 },
        ];
        let outcome = evaluate_quorum(&results, 3).unwrap();
        assert!(outcome.quorum_reached);
        assert_eq!(outcome.agreeing_nodes.len(), 3);
        assert!(outcome.dissenting_nodes.is_empty());
    }

    #[test]
    fn two_of_three_quorum_passes_with_dissenter() {
        let good = compute_cid(b"correct result").unwrap();
        let bad = compute_cid(b"wrong result").unwrap();
        let results = vec![
            ReplicaResult { node_id: "A".into(), result_cid: good, execution_ms: 100 },
            ReplicaResult { node_id: "B".into(), result_cid: good, execution_ms: 110 },
            ReplicaResult { node_id: "C".into(), result_cid: bad, execution_ms: 105 },
        ];
        let outcome = evaluate_quorum(&results, 3).unwrap();
        assert!(outcome.quorum_reached);
        assert_eq!(outcome.accepted_cid, good);
        assert_eq!(outcome.dissenting_nodes, vec!["C"]);
    }

    #[test]
    fn no_majority_fails() {
        let a = compute_cid(b"result A").unwrap();
        let b = compute_cid(b"result B").unwrap();
        let c = compute_cid(b"result C").unwrap();
        let results = vec![
            ReplicaResult { node_id: "A".into(), result_cid: a, execution_ms: 100 },
            ReplicaResult { node_id: "B".into(), result_cid: b, execution_ms: 110 },
            ReplicaResult { node_id: "C".into(), result_cid: c, execution_ms: 105 },
        ];
        let outcome = evaluate_quorum(&results, 3);
        assert!(outcome.is_err());
    }

    #[test]
    fn insufficient_replicas_fails() {
        let cid = compute_cid(b"result").unwrap();
        let results =
            vec![ReplicaResult { node_id: "A".into(), result_cid: cid, execution_ms: 100 }];
        let outcome = evaluate_quorum(&results, 3);
        assert!(outcome.is_err());
    }

    #[test]
    fn five_of_five_unanimous() {
        let cid = compute_cid(b"result").unwrap();
        let results: Vec<_> = (0..5)
            .map(|i| ReplicaResult {
                node_id: format!("node-{i}"),
                result_cid: cid,
                execution_ms: 100 + i * 10,
            })
            .collect();
        let outcome = evaluate_quorum(&results, 5).unwrap();
        assert!(outcome.quorum_reached);
        assert_eq!(outcome.agreeing_nodes.len(), 5);
    }
}
