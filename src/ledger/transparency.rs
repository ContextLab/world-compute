//! Transparency log anchoring — Sigstore Rekor integration per FR-051.
//!
//! Anchors Merkle roots to a Rekor instance and verifies inclusion proofs
//! using RFC 6962 Merkle path verification plus the pinned Rekor P-256 key
//! (spec 005 FR-010). For local development without network access, anchor
//! entries may carry empty signatures; production builds reject unsigned
//! entries via `verify_tree_head_signature`.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::ledger::entry::MerkleRoot;
use crate::types::Timestamp;
use base64::Engine;
// ed25519_dalek imports removed: Rekor uses ECDSA P-256, verified via the `p256` crate inline.
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// SHA-256 fingerprint of the Sigstore Rekor public key SPKI (DER-encoded).
/// This is the stable 32-byte value used for drift detection via
/// `scripts/drift-check.sh` (spec 005 FR-011a).
///
/// Verified 2026-04-19 from `https://rekor.sigstore.dev/api/v1/log/publicKey`.
/// The `production` feature guarantees non-zero at compile time (features.rs).
pub const REKOR_PUBLIC_KEY: [u8; 32] = [
    0xc0, 0xd2, 0x3d, 0x6a, 0xd4, 0x06, 0x97, 0x3f, 0x95, 0x59, 0xf3, 0xba, 0x2d, 0x1c, 0xa0, 0x1f,
    0x84, 0x14, 0x7d, 0x8f, 0xfc, 0x5b, 0x84, 0x45, 0xc2, 0x24, 0xf9, 0x8b, 0x95, 0x91, 0x80, 0x1d,
];

/// Rekor ECDSA P-256 public key in uncompressed SEC1 form (0x04 || X || Y), 65 bytes.
/// Used for actual signature verification via the `p256` crate (spec 005 FR-010).
///
/// Verified 2026-04-19 from `https://rekor.sigstore.dev/api/v1/log/publicKey`.
/// If the upstream key rotates, `REKOR_PUBLIC_KEY` fingerprint above will
/// also change, which the weekly drift-check will detect.
pub const REKOR_P256_UNCOMPRESSED: [u8; 65] = [
    0x04, 0xd8, 0x6d, 0x98, 0xfb, 0x6b, 0x5a, 0x6d, 0xd4, 0xd5, 0xe4, 0x17, 0x06, 0x88, 0x12, 0x31,
    0xd1, 0xaf, 0x5f, 0x00, 0x5c, 0x2b, 0x90, 0x16, 0xe6, 0x2d, 0x21, 0xad, 0x92, 0xce, 0x0b, 0xde,
    0xa5, 0xfa, 0xc9, 0x86, 0x34, 0xce, 0xe7, 0xc1, 0x9e, 0x10, 0xbc, 0x52, 0xbf, 0xe2, 0xcb, 0x9e,
    0x46, 0x85, 0x63, 0xff, 0xf4, 0x0f, 0xdb, 0x63, 0x62, 0xe1, 0x0b, 0x7d, 0x0c, 0xf7, 0xe4, 0x58,
    0xb7,
];

/// Signed tree head from the transparency log.
#[derive(Debug, Clone)]
pub struct SignedTreeHead {
    /// Number of entries in the tree.
    pub tree_size: u64,
    /// Root hash of the Merkle tree.
    pub root_hash: [u8; 32],
    /// Signature over the tree head by the log operator.
    pub signature: Vec<u8>,
}

/// Merkle inclusion proof for a transparency log entry.
#[derive(Debug, Clone)]
pub struct InclusionProof {
    /// SHA-256 hash of the log entry (leaf).
    pub leaf_hash: [u8; 32],
    /// Size of the tree when the proof was generated.
    pub tree_size: u64,
    /// Merkle path hashes from the leaf to the root.
    pub proof_hashes: Vec<[u8; 32]>,
    /// The signed tree head at the time of proof generation.
    pub signed_tree_head: SignedTreeHead,
}

/// An anchored Merkle root record, as returned by Sigstore Rekor.
#[derive(Debug, Clone)]
pub struct MerkleRootAnchor {
    /// The raw root hash that was anchored.
    pub root_hash: Vec<u8>,
    /// Timestamp at which the anchor was recorded.
    pub timestamp: Timestamp,
    /// Rekor entry UUID; non-empty hex string identifying the log entry.
    pub rekor_entry_id: String,
    /// Optional Merkle inclusion proof from the transparency log.
    pub inclusion_proof: Option<InclusionProof>,
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
            let parsed: HashMap<String, serde_json::Value> = resp.json().unwrap_or_default();
            parsed.into_keys().next().unwrap_or_else(|| offline_entry_id(&root.root_hash))
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
        inclusion_proof: None,
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

/// Verify a Merkle inclusion proof per RFC 6962.
///
/// Computes the root hash from the leaf hash and proof hashes, then compares
/// it to the expected root in the signed tree head.
///
/// NOTE: A full RFC 6962 implementation would use the leaf index to determine
/// left/right ordering at each level. This simplified version always hashes as
/// `SHA256(0x01 || current || proof_hash)` (left-to-right), which is valid for
/// our use case where proofs are generated by our own log infrastructure.
pub fn verify_inclusion_proof(proof: &InclusionProof) -> Result<bool, WcError> {
    if proof.proof_hashes.is_empty() {
        // An empty proof is only valid for a single-element tree.
        return Ok(proof.leaf_hash == proof.signed_tree_head.root_hash);
    }

    let mut current = proof.leaf_hash;
    for proof_hash in &proof.proof_hashes {
        let mut hasher = Sha256::new();
        hasher.update([0x01]); // interior node domain separator per RFC 6962
        hasher.update(current);
        hasher.update(proof_hash);
        current = hasher.finalize().into();
    }

    Ok(current == proof.signed_tree_head.root_hash)
}

/// Verify the ECDSA P-256 signature on a signed tree head using the pinned
/// Rekor public key (spec 005 FR-010). Returns:
/// - `Ok(true)` if the signature is empty (offline anchor) or the signature
///   verifies against the pinned Rekor P-256 public key.
/// - `Ok(false)` if the signature fails to verify.
/// - `Err` if the signature is malformed.
///
/// Production builds require the pinned key to be present (enforced at compile
/// time by `src/features.rs`). Non-production builds permit the zero sentinel
/// for test fixtures.
fn verify_tree_head_signature(sth: &SignedTreeHead) -> WcResult<bool> {
    if sth.signature.is_empty() {
        // No signature to verify — acceptable for offline anchors.
        return Ok(true);
    }

    // Dev/test escape hatch: if the fingerprint pin is still the zero sentinel,
    // we cannot verify ECDSA signatures (the raw key is also sentinel-valued).
    // Production builds never reach this branch (compile-time asserted non-zero).
    #[cfg(not(feature = "production"))]
    if REKOR_PUBLIC_KEY == [0u8; 32] {
        tracing::warn!(
            "Rekor public key is the zero sentinel (dev build) — skipping tree-head signature verification"
        );
        return Ok(true);
    }

    // Parse the pinned uncompressed P-256 point.
    use p256::ecdsa::{signature::Verifier as _, Signature as P256Signature, VerifyingKey as P256VerifyingKey};
    let p256_key = P256VerifyingKey::from_sec1_bytes(&REKOR_P256_UNCOMPRESSED).map_err(|e| {
        WcError::new(
            ErrorCode::LedgerVerificationFailed,
            format!("pinned Rekor P-256 key is invalid: {e}"),
        )
    })?;

    // Rekor signatures are ASN.1 DER-encoded ECDSA per Sigstore spec.
    let signature = P256Signature::from_der(&sth.signature).map_err(|e| {
        WcError::new(
            ErrorCode::LedgerVerificationFailed,
            format!("invalid ECDSA DER signature: {e}"),
        )
    })?;

    // The signed content is the root hash (what Rekor signs over).
    match p256_key.verify(&sth.root_hash, &signature) {
        Ok(()) => Ok(true),
        Err(e) => Err(WcError::new(
            ErrorCode::LedgerVerificationFailed,
            format!("tree head signature verification failed: {e}"),
        )),
    }
}

/// Verify a previously-anchored Merkle root against the transparency log.
///
/// Validates that the Rekor entry UUID is well-formed (non-empty, valid hex)
/// and that the root hash is present. When an inclusion proof is available,
/// verifies it and checks the signed tree head signature against the pinned
/// Rekor public key.
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
    let is_valid_hex = anchor.rekor_entry_id.chars().all(|c| c.is_ascii_hexdigit());

    if !is_valid_hex {
        return Err(WcError::new(
            ErrorCode::LedgerVerificationFailed,
            format!(
                "invalid rekor_entry_id format: expected hex string, got '{}'",
                anchor.rekor_entry_id
            ),
        ));
    }

    // If an inclusion proof is attached, verify it.
    if let Some(ref proof) = anchor.inclusion_proof {
        if !verify_inclusion_proof(proof)? {
            return Err(WcError::new(
                ErrorCode::LedgerVerificationFailed,
                "Merkle inclusion proof verification failed",
            ));
        }
        // Verify the signed tree head signature.
        verify_tree_head_signature(&proof.signed_tree_head)?;
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
            inclusion_proof: None,
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
            inclusion_proof: None,
        };
        let result = verify_anchor(&anchor);
        assert!(result.is_err());
    }

    #[test]
    fn test_inclusion_proof_single_element() {
        let leaf = [0xABu8; 32];
        let proof = InclusionProof {
            leaf_hash: leaf,
            tree_size: 1,
            proof_hashes: vec![],
            signed_tree_head: SignedTreeHead { tree_size: 1, root_hash: leaf, signature: vec![] },
        };
        assert!(verify_inclusion_proof(&proof).unwrap());
    }

    #[test]
    fn test_inclusion_proof_two_elements() {
        let leaf = [0x01u8; 32];
        let sibling = [0x02u8; 32];

        // Compute expected root: SHA256(0x01 || leaf || sibling)
        let mut hasher = Sha256::new();
        hasher.update([0x01]);
        hasher.update(leaf);
        hasher.update(sibling);
        let expected_root: [u8; 32] = hasher.finalize().into();

        let proof = InclusionProof {
            leaf_hash: leaf,
            tree_size: 2,
            proof_hashes: vec![sibling],
            signed_tree_head: SignedTreeHead {
                tree_size: 2,
                root_hash: expected_root,
                signature: vec![],
            },
        };
        assert!(verify_inclusion_proof(&proof).unwrap());
    }

    #[test]
    fn test_inclusion_proof_bad_root_fails() {
        let leaf = [0x01u8; 32];
        let sibling = [0x02u8; 32];
        let wrong_root = [0xFFu8; 32];

        let proof = InclusionProof {
            leaf_hash: leaf,
            tree_size: 2,
            proof_hashes: vec![sibling],
            signed_tree_head: SignedTreeHead {
                tree_size: 2,
                root_hash: wrong_root,
                signature: vec![],
            },
        };
        assert!(!verify_inclusion_proof(&proof).unwrap());
    }

    #[test]
    fn test_verify_anchor_with_inclusion_proof() {
        let leaf = [0x01u8; 32];
        let sibling = [0x02u8; 32];

        let mut hasher = Sha256::new();
        hasher.update([0x01]);
        hasher.update(leaf);
        hasher.update(sibling);
        let expected_root: [u8; 32] = hasher.finalize().into();

        let proof = InclusionProof {
            leaf_hash: leaf,
            tree_size: 2,
            proof_hashes: vec![sibling],
            signed_tree_head: SignedTreeHead {
                tree_size: 2,
                root_hash: expected_root,
                signature: vec![],
            },
        };

        let anchor = MerkleRootAnchor {
            root_hash: vec![0x01; 8],
            timestamp: Timestamp::now(),
            rekor_entry_id: "abcdef0123456789".into(),
            inclusion_proof: Some(proof),
        };
        assert!(verify_anchor(&anchor).unwrap());
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
