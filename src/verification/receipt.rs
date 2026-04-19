//! WorkUnitReceipt — proof of accepted and rewarded task execution per T064.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::types::{Cid, NcuAmount, SignatureBundle};

/// A receipt issued by the coordinator after a task reaches quorum acceptance.
/// Records which nodes agreed, which dissented, and how many NCU each earns.
#[derive(Debug, Clone)]
pub struct WorkUnitReceipt {
    /// Unique receipt identifier (coordinator-assigned UUID or hash).
    pub receipt_id: String,
    /// The task this receipt covers.
    pub task_id: String,
    /// CID of the accepted output (the quorum-agreed result).
    pub accepted_cid: Cid,
    /// Node IDs that formed the accepting quorum.
    pub quorum_node_ids: Vec<String>,
    /// Node IDs whose result differed from the quorum (dissent).
    pub dissenting_node_ids: Vec<String>,
    /// Coordinator threshold-signature authorising the NCU awards.
    pub coordinator_signature: SignatureBundle,
    /// NCU awarded per contributing node: (node_id, amount).
    pub ncu_awarded_per_node: Vec<(String, NcuAmount)>,
}

/// Verify a `WorkUnitReceipt` for structural validity.
///
/// Checks that receipt_id, task_id, and quorum_node_ids are non-empty, and
/// that the coordinator signature bundle has a positive threshold. Full
/// cryptographic verification of the signature bundle itself against the
/// coordinator public key set happens in `verification::quorum` when the
/// receipt is consumed by the ledger (see `src/ledger/entry.rs` for the
/// ledger-side quorum verifier).
/// Returns `Ok(true)` when the receipt is structurally sound.
pub fn verify_receipt(receipt: &WorkUnitReceipt) -> WcResult<bool> {
    if receipt.receipt_id.is_empty() {
        return Err(WcError::new(ErrorCode::InvalidManifest, "receipt_id is empty"));
    }
    if receipt.task_id.is_empty() {
        return Err(WcError::new(ErrorCode::InvalidManifest, "task_id is empty"));
    }
    if receipt.quorum_node_ids.is_empty() {
        return Err(WcError::new(ErrorCode::QuorumFailure, "quorum_node_ids must not be empty"));
    }
    if receipt.coordinator_signature.threshold == 0 {
        return Err(WcError::new(
            ErrorCode::LedgerVerificationFailed,
            "coordinator_signature threshold must be > 0",
        ));
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::cid_store::compute_cid;

    fn valid_signature() -> SignatureBundle {
        SignatureBundle {
            signer_ids: vec!["coord-1".into(), "coord-2".into()],
            signature: vec![0u8; 64],
            threshold: 2,
            total: 3,
        }
    }

    fn valid_receipt() -> WorkUnitReceipt {
        let cid = compute_cid(b"accepted output bytes").unwrap();
        WorkUnitReceipt {
            receipt_id: "rcpt-001".into(),
            task_id: "task-abc".into(),
            accepted_cid: cid,
            quorum_node_ids: vec!["node-1".into(), "node-2".into(), "node-3".into()],
            dissenting_node_ids: vec![],
            coordinator_signature: valid_signature(),
            ncu_awarded_per_node: vec![
                ("node-1".into(), NcuAmount::from_ncu(1.5)),
                ("node-2".into(), NcuAmount::from_ncu(1.5)),
                ("node-3".into(), NcuAmount::from_ncu(1.5)),
            ],
        }
    }

    #[test]
    fn receipt_construction_with_valid_fields() {
        let r = valid_receipt();
        assert_eq!(r.receipt_id, "rcpt-001");
        assert_eq!(r.task_id, "task-abc");
        assert_eq!(r.quorum_node_ids.len(), 3);
        assert!(r.dissenting_node_ids.is_empty());
        assert_eq!(r.ncu_awarded_per_node.len(), 3);
        for (_, ncu) in &r.ncu_awarded_per_node {
            assert!((ncu.as_ncu() - 1.5).abs() < 0.001);
        }
    }

    #[test]
    fn verify_valid_receipt_returns_true() {
        let r = valid_receipt();
        assert!(verify_receipt(&r).unwrap());
    }

    #[test]
    fn verify_empty_receipt_id_fails() {
        let mut r = valid_receipt();
        r.receipt_id = String::new();
        let err = verify_receipt(&r).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::InvalidManifest));
    }

    #[test]
    fn verify_empty_task_id_fails() {
        let mut r = valid_receipt();
        r.task_id = String::new();
        let err = verify_receipt(&r).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::InvalidManifest));
    }

    #[test]
    fn verify_empty_quorum_fails() {
        let mut r = valid_receipt();
        r.quorum_node_ids = vec![];
        let err = verify_receipt(&r).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::QuorumFailure));
    }

    #[test]
    fn verify_zero_threshold_fails() {
        let mut r = valid_receipt();
        r.coordinator_signature.threshold = 0;
        let err = verify_receipt(&r).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::LedgerVerificationFailed));
    }

    #[test]
    fn receipt_with_dissenters_is_valid() {
        let mut r = valid_receipt();
        r.dissenting_node_ids = vec!["node-bad".into()];
        assert!(verify_receipt(&r).unwrap());
    }
}
