//! Proof-of-personhood verification via BrightID.
//!
//! Per FR-S070: connects the `proof_of_personhood: bool` field in
//! HumanityPoints to a real verification provider.
//!
//! Decision (T086): BrightID chosen as primary provider because:
//! - Decentralized (no single authority controls verification)
//! - Free (no per-verification cost)
//! - No biometric collection (aligned with volunteer privacy ethos)
//! - REST API for verification checks
//!
//! See GitHub issue for exploring additional providers.

use serde::{Deserialize, Serialize};

/// BrightID verification context for World Compute.
const BRIGHTID_CONTEXT: &str = "WorldCompute";

/// BrightID node URL for verification queries.
const BRIGHTID_NODE_URL: &str = "https://app.brightid.org/node/v6";

/// Result of a proof-of-personhood verification attempt.
#[derive(Debug, Clone)]
pub enum PersonhoodResult {
    /// Verification succeeded — user is verified unique human.
    Verified,
    /// User is registered but not yet verified (needs more connections).
    Pending { connections_needed: u32 },
    /// Verification failed with reason.
    Failed(String),
    /// Provider is unavailable.
    ProviderUnavailable(String),
}

/// BrightID verification response (subset of API response).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrightIdVerification {
    /// Whether the user is verified in this context.
    pub verified: bool,
    /// Unique human indicator.
    #[serde(default)]
    pub unique: bool,
    /// Context ID used for verification.
    #[serde(default)]
    pub context_id: String,
    /// Error message if verification failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// Generate a BrightID deep link for user verification.
///
/// The user opens this link in their BrightID app to link their
/// World Compute identity to their BrightID social graph.
pub fn brightid_link_url(context_id: &str) -> String {
    format!(
        "https://app.brightid.org/link-verification/http:%2f%2fnode.brightid.org/{BRIGHTID_CONTEXT}/{context_id}"
    )
}

/// Verify proof-of-personhood for a user via BrightID.
///
/// Checks the BrightID node API to see if the given context_id
/// (derived from the user's PeerId) is verified as a unique human.
///
/// This function makes an HTTP request to the BrightID node.
/// In production, it should be called at enrollment time and
/// re-verified at trust score recalculation intervals.
pub fn verify_personhood(context_id: &str) -> PersonhoodResult {
    let url = format!(
        "{BRIGHTID_NODE_URL}/verifications/{BRIGHTID_CONTEXT}/{context_id}"
    );

    // Use a blocking HTTP client for simplicity.
    // In production, this should be async via reqwest or hyper.
    // For now, we attempt the request and handle failures gracefully.
    match ureq_get_brightid(&url) {
        Ok(verification) => {
            if verification.unique {
                PersonhoodResult::Verified
            } else if verification.verified {
                // Verified in context but not marked unique
                PersonhoodResult::Verified
            } else {
                PersonhoodResult::Pending {
                    connections_needed: 3, // BrightID typically requires ~3 connections
                }
            }
        }
        Err(e) => {
            // Distinguish between network errors and verification failures
            if e.contains("404") || e.contains("Not Found") {
                PersonhoodResult::Pending { connections_needed: 3 }
            } else {
                PersonhoodResult::ProviderUnavailable(format!("BrightID check failed: {e}"))
            }
        }
    }
}

/// Make a GET request to BrightID verification endpoint.
///
/// Returns the parsed verification response or an error string.
/// This is a synchronous HTTP call; production should use async.
fn ureq_get_brightid(_url: &str) -> Result<BrightIdVerification, String> {
    // TODO: Replace with real HTTP client (reqwest or ureq).
    // The BrightID API endpoint is:
    // GET /node/v6/verifications/{context}/{contextId}
    //
    // Response: { "data": { "unique": true, "contextIds": [...] } }
    //
    // For now, return an error indicating the HTTP client is not wired.
    // This allows the code to compile and tests to verify the flow
    // without adding an HTTP dependency yet.
    Err("HTTP client not yet integrated — add ureq or reqwest dependency to Cargo.toml".into())
}

/// Derive a BrightID context ID from a World Compute PeerId.
///
/// The context ID is a deterministic, hex-encoded hash of the PeerId
/// to avoid exposing the raw PeerId to BrightID.
pub fn peer_id_to_context_id(peer_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(format!("wc-brightid-{peer_id}").as_bytes());
    hex::encode(&hash[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_id_is_deterministic() {
        let id1 = peer_id_to_context_id("12D3KooWTest");
        let id2 = peer_id_to_context_id("12D3KooWTest");
        assert_eq!(id1, id2);
    }

    #[test]
    fn different_peers_different_context_ids() {
        let id1 = peer_id_to_context_id("12D3KooWA");
        let id2 = peer_id_to_context_id("12D3KooWB");
        assert_ne!(id1, id2);
    }

    #[test]
    fn context_id_is_hex_encoded() {
        let id = peer_id_to_context_id("test-peer");
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(id.len(), 32); // 16 bytes = 32 hex chars
    }

    #[test]
    fn brightid_link_contains_context() {
        let link = brightid_link_url("abc123");
        assert!(link.contains("WorldCompute"));
        assert!(link.contains("abc123"));
    }

    #[test]
    fn verify_returns_unavailable_without_http_client() {
        match verify_personhood("test-context") {
            PersonhoodResult::ProviderUnavailable(msg) => {
                assert!(msg.contains("HTTP client"));
            }
            other => panic!("Expected ProviderUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn brightid_verification_deserializes() {
        let json = r#"{"verified": true, "unique": true, "context_id": "abc"}"#;
        let v: BrightIdVerification = serde_json::from_str(json).unwrap();
        assert!(v.verified);
        assert!(v.unique);
    }
}
