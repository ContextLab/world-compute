//! Transparency log anchoring stub — Sigstore Rekor integration per FR-051.
//!
//! Production implementation would POST the Merkle root hash to a Rekor
//! instance and receive a signed inclusion proof. This stub returns a
//! placeholder so the rest of the system can be wired up without a live
//! Rekor endpoint.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::ledger::entry::MerkleRoot;
use crate::types::Timestamp;

/// An anchored Merkle root record, as returned by Sigstore Rekor.
#[derive(Debug, Clone)]
pub struct MerkleRootAnchor {
    /// The raw root hash that was anchored.
    pub root_hash: Vec<u8>,
    /// Timestamp at which the anchor was recorded.
    pub timestamp: Timestamp,
    /// Rekor entry UUID (or placeholder in stub mode).
    pub rekor_entry_id: String,
}

/// Anchor a Merkle root to the transparency log.
///
/// In production this would call the Rekor REST API. This stub returns a
/// deterministic placeholder derived from the root hash so callers can
/// exercise the full code path in tests.
pub fn anchor_merkle_root(root: &MerkleRoot) -> WcResult<MerkleRootAnchor> {
    if root.root_hash.is_empty() {
        return Err(WcError::new(
            ErrorCode::LedgerVerificationFailed,
            "cannot anchor empty root hash",
        ));
    }

    // Stub: build a fake Rekor entry ID from the first 8 bytes of the hash.
    let hex_prefix: String = root.root_hash.iter().take(8).map(|b| format!("{b:02x}")).collect();
    let rekor_entry_id = format!("stub-rekor-{hex_prefix}");

    Ok(MerkleRootAnchor {
        root_hash: root.root_hash.clone(),
        timestamp: Timestamp::now(),
        rekor_entry_id,
    })
}

/// Verify a previously-anchored Merkle root against the transparency log.
///
/// In production this would fetch the Rekor entry by ID and check the
/// inclusion proof. This stub accepts any non-empty anchor as valid.
pub fn verify_anchor(anchor: &MerkleRootAnchor) -> WcResult<bool> {
    if anchor.rekor_entry_id.is_empty() {
        return Err(WcError::new(
            ErrorCode::LedgerVerificationFailed,
            "anchor has empty rekor_entry_id",
        ));
    }
    if anchor.root_hash.is_empty() {
        return Err(WcError::new(
            ErrorCode::LedgerVerificationFailed,
            "anchor has empty root_hash",
        ));
    }
    // Stub: always valid if fields are populated.
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::entry::MerkleRoot;
    use crate::types::{SignatureBundle, Timestamp};

    fn dummy_sig() -> SignatureBundle {
        SignatureBundle {
            signer_ids: vec!["coord-1".into()],
            signature: vec![0u8; 64],
            threshold: 1,
            total: 1,
        }
    }

    fn make_root(root_hash: Vec<u8>) -> MerkleRoot {
        MerkleRoot {
            root_hash,
            height: 1,
            timestamp: Timestamp::now(),
            shard_heads: vec![],
            coordinator_signature: dummy_sig(),
            rekor_entry_id: None,
        }
    }

    #[test]
    fn test_anchor_round_trip() {
        let root = make_root(vec![0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe]);
        let anchor = anchor_merkle_root(&root).expect("anchor should succeed");

        assert_eq!(anchor.root_hash, root.root_hash);
        assert!(!anchor.rekor_entry_id.is_empty());
        assert!(anchor.rekor_entry_id.starts_with("stub-rekor-"));

        let valid = verify_anchor(&anchor).expect("verify should succeed");
        assert!(valid);
    }

    #[test]
    fn test_anchor_empty_hash_fails() {
        let root = make_root(vec![]);
        let result = anchor_merkle_root(&root);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::LedgerVerificationFailed));
    }

    #[test]
    fn test_verify_empty_entry_id_fails() {
        let anchor = MerkleRootAnchor {
            root_hash: vec![1, 2, 3],
            timestamp: Timestamp::now(),
            rekor_entry_id: String::new(),
        };
        let result = verify_anchor(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_empty_root_hash_fails() {
        let anchor = MerkleRootAnchor {
            root_hash: vec![],
            timestamp: Timestamp::now(),
            rekor_entry_id: "stub-rekor-abc".into(),
        };
        let result = verify_anchor(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_anchor_entry_id_encodes_hash() {
        let hash = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let root = make_root(hash.clone());
        let anchor = anchor_merkle_root(&root).unwrap();
        assert!(anchor.rekor_entry_id.contains("0102030405060708"));
    }
}
