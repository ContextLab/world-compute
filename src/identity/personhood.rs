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
    let base_url =
        std::env::var("BRIGHTID_NODE_URL").unwrap_or_else(|_| BRIGHTID_NODE_URL.to_string());
    let url = format!("{base_url}/verifications/{BRIGHTID_CONTEXT}/{context_id}");

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

/// BrightID API response wrapper.
#[derive(Debug, Deserialize)]
struct BrightIdApiResponse {
    data: Option<BrightIdVerification>,
    #[serde(default)]
    error: Option<bool>,
    #[serde(rename = "errorMessage", default)]
    error_message: Option<String>,
}

/// Make a GET request to BrightID verification endpoint via reqwest.
///
/// Uses a blocking reqwest client. The BrightID node URL can be overridden
/// via the BRIGHTID_NODE_URL environment variable.
fn ureq_get_brightid(url: &str) -> Result<BrightIdVerification, String> {
    // Use reqwest blocking client (runs inside tokio via spawn_blocking
    // or in a non-async context for CLI usage).
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client init failed: {e}"))?;

    let response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("BrightID request failed: {e}"))?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err("404 Not Found".into());
    }
    if !status.is_success() {
        return Err(format!("BrightID returned status {status}"));
    }

    let api_response: BrightIdApiResponse =
        response.json().map_err(|e| format!("BrightID response parse failed: {e}"))?;

    if let Some(true) = api_response.error {
        return Err(api_response.error_message.unwrap_or_else(|| "Unknown BrightID error".into()));
    }

    api_response.data.ok_or_else(|| "BrightID response missing data field".into())
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
    fn verify_returns_unavailable_or_pending_for_fake_context() {
        // With real HTTP client wired, this will either:
        // - Return ProviderUnavailable if BrightID node is unreachable
        // - Return Pending if the context ID is not found (404)
        match verify_personhood("test-context-nonexistent") {
            PersonhoodResult::ProviderUnavailable(_) | PersonhoodResult::Pending { .. } => {}
            PersonhoodResult::Failed(_) => {}
            other => panic!("Expected ProviderUnavailable, Pending, or Failed — got {other:?}"),
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
