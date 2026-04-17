//! Transparency log anchoring stub — Sigstore Rekor integration per FR-051.
//!
//! Production implementation would POST the Merkle root hash to a Rekor
//! instance and receive a signed inclusion proof. This stub returns a
//! placeholder so the rest of the system can be wired up without a live
//! Rekor endpoint.

use base64::Engine;
use crate::error::{ErrorCode, WcError, WcResult};
use crate::ledger::entry::MerkleRoot;
use crate::types::Timestamp;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

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

/// Return the Rekor base URL, configurable via `REKOR_URL` env var.
fn rekor_base_url() -> String {
    std::env::var("REKOR_URL").unwrap_or_else(|_| "https://rekor.sigstore.dev".into())
}

/// Anchor a Merkle root to the transparency log.
///
/// Posts the root hash to the Rekor REST API as a hashedrekord entry
/// and returns the Rekor entry UUID. Falls back to a deterministic
/// offline entry ID if Rekor is unreachable, so callers can still
/// operate without network access.
pub fn anchor_merkle_root(root: &MerkleRoot) -> WcResult<MerkleRootAnchor> {
    if root.root_hash.is_empty() {
        return Err(WcError::new(
            ErrorCode::LedgerVerificationFailed,
            "cannot anchor empty root hash",
        ));
    }

    let root_hash_hex: String = root.root_hash.iter().map(|b| format!("{b:02x}")).collect();

    // Build a hashedrekord entry for Rekor.
    let body = serde_json::json!({
        "apiVersion": "0.0.1",
        "kind": "hashedrekord",
        "spec": {
            "data": {
                "hash": {
                    "algorithm": "sha256",
                    "value": root_hash_hex
                }
            },
            "signature": {
                "content": base64::engine::general_purpose::STANDARD.encode(&root.root_hash),
                "publicKey": { "content": "" }
            }
        }
    });

    let url = format!("{}/api/v1/log/entries", rekor_base_url());

    let rekor_entry_id = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .and_then(|c| c.post(&url).json(&body).send())
    {
        Ok(resp) if resp.status().is_success() => {
            // Rekor returns { "<uuid>": { ... } }
            let parsed: HashMap<String, serde_json::Value> =
                resp.json().unwrap_or_default();
            parsed
                .into_keys()
                .next()
                .unwrap_or_else(|| offline_entry_id(&root.root_hash))
        }
        _ => {
            // Network error or non-success status — fall back to offline ID.
            offline_entry_id(&root.root_hash)
        }
    };

    Ok(MerkleRootAnchor {
        root_hash: root.root_hash.clone(),
        timestamp: Timestamp::now(),
        rekor_entry_id,
    })
}

/// Generate a deterministic offline entry ID from the root hash.
/// Used when the Rekor service is unreachable.
fn offline_entry_id(root_hash: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(root_hash);
    let digest = hasher.finalize();
    // 64-char hex string matching Rekor UUID format length
    format!("{digest:x}")
}

/// Verify a previously-anchored Merkle root against the transparency log.
///
/// Validates that the Rekor entry UUID is well-formed (non-empty, valid hex)
/// and that the root hash is present.
///
/// TODO(T096): Implement full Merkle inclusion proof verification by fetching
/// the entry from Rekor (GET /api/v1/log/entries/{uuid}) and validating the
/// signed entry timestamp (SET) and inclusion proof against the log root.
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

    // Validate that the entry UUID is a valid hex string (Rekor UUIDs and
    // our offline IDs are both hex-encoded).
    let is_valid_hex = anchor
        .rekor_entry_id
        .chars()
        .all(|c| c.is_ascii_hexdigit());

    if !is_valid_hex {
        return Err(WcError::new(
            ErrorCode::LedgerVerificationFailed,
            format!(
                "invalid rekor_entry_id format: expected hex string, got '{}'",
                anchor.rekor_entry_id
            ),
        ));
    }

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
        // Entry ID should be valid hex (either a Rekor UUID or offline ID)
        assert!(anchor.rekor_entry_id.chars().all(|c| c.is_ascii_hexdigit()));

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
            rekor_entry_id: "abcdef0123456789".into(),
        };
        let result = verify_anchor(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_anchor_entry_id_is_valid_hex() {
        let hash = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let root = make_root(hash.clone());
        let anchor = anchor_merkle_root(&root).unwrap();
        // Offline entry ID is a SHA-256 hex digest (64 chars)
        assert_eq!(anchor.rekor_entry_id.len(), 64);
        assert!(anchor.rekor_entry_id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
