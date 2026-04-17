//! Transparency log integration — Sigstore Rekor or equivalent.
//!
//! Per FR-S052: all artifact signatures and policy decisions MUST be
//! recorded in a transparency log.
//! Per FR-S051: all workload artifacts MUST carry provenance attestations.

use crate::types::Timestamp;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Provenance attestation linking an artifact to its build pipeline.
/// Per FR-S051 and data-model.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceAttestation {
    /// Source repository and commit hash.
    pub build_source: String,
    /// CI pipeline identifier (e.g., GitHub Actions run ID).
    pub build_pipeline: String,
    /// When the build ran.
    pub build_timestamp: Timestamp,
    /// Whether the build is verified reproducible.
    pub reproducible: bool,
}

/// Build metadata embedded in the binary at compile time (FR-S051).
pub struct BuildMetadata {
    pub git_commit: &'static str,
    pub build_timestamp: &'static str,
    pub rustc_version: &'static str,
    pub version: &'static str,
}

/// Get the build metadata embedded at compile time.
pub fn build_metadata() -> BuildMetadata {
    BuildMetadata {
        git_commit: option_env!("WC_GIT_COMMIT").unwrap_or("unknown"),
        build_timestamp: option_env!("WC_BUILD_TIMESTAMP").unwrap_or("0"),
        rustc_version: option_env!("WC_RUSTC_VERSION").unwrap_or("unknown"),
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// Result of a transparency log submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransparencyLogResult {
    /// Entry recorded with the given log index and Rekor entry UUID.
    Recorded {
        log_index: String,
        entry_uuid: String,
        timestamp: Timestamp,
    },
    /// Log service unavailable.
    Unavailable(String),
}

/// Return the Rekor base URL, configurable via `REKOR_URL` env var.
fn rekor_base_url() -> String {
    std::env::var("REKOR_URL").unwrap_or_else(|_| "https://rekor.sigstore.dev".into())
}

/// Build a hashedrekord JSON body for Rekor.
fn build_hashedrekord_body(artifact_hash_hex: &str, signature_b64: &str) -> serde_json::Value {
    serde_json::json!({
        "apiVersion": "0.0.1",
        "kind": "hashedrekord",
        "spec": {
            "data": {
                "hash": {
                    "algorithm": "sha256",
                    "value": artifact_hash_hex
                }
            },
            "signature": {
                "content": signature_b64,
                "publicKey": {
                    "content": ""
                }
            }
        }
    })
}

/// Submit an entry to the Rekor transparency log and parse the response.
fn submit_to_rekor(body: &serde_json::Value) -> TransparencyLogResult {
    let url = format!("{}/api/v1/log/entries", rekor_base_url());

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return TransparencyLogResult::Unavailable(format!("HTTP client error: {e}"));
        }
    };

    let resp = match client.post(&url).json(body).send() {
        Ok(r) => r,
        Err(e) => {
            return TransparencyLogResult::Unavailable(format!(
                "Rekor request failed: {e}"
            ));
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return TransparencyLogResult::Unavailable(format!(
            "Rekor returned HTTP {status}: {text}"
        ));
    }

    // Rekor returns a JSON object where the single key is the entry UUID
    // and the value contains logIndex, body, etc.
    let parsed: HashMap<String, serde_json::Value> = match resp.json() {
        Ok(v) => v,
        Err(e) => {
            return TransparencyLogResult::Unavailable(format!(
                "Failed to parse Rekor response: {e}"
            ));
        }
    };

    if let Some((uuid, entry)) = parsed.into_iter().next() {
        let log_index = entry
            .get("logIndex")
            .and_then(|v| v.as_i64())
            .map(|i| i.to_string())
            .unwrap_or_else(|| uuid.clone());

        TransparencyLogResult::Recorded {
            log_index,
            entry_uuid: uuid,
            timestamp: Timestamp::now(),
        }
    } else {
        TransparencyLogResult::Unavailable("Rekor returned empty response".into())
    }
}

/// Submit an artifact signature to the transparency log.
///
/// Per FR-S052: records the artifact CID, signature, and provenance
/// in a tamper-evident log (Sigstore Rekor or equivalent).
pub fn record_artifact_signature(
    artifact_cid: &str,
    signature: &[u8],
    provenance: &ProvenanceAttestation,
) -> TransparencyLogResult {
    let _ = provenance; // provenance metadata is for local audit; Rekor gets hash+sig

    // Compute SHA-256 of the artifact CID string (content identifier).
    let mut hasher = Sha256::new();
    hasher.update(artifact_cid.as_bytes());
    let artifact_hash_hex = format!("{:x}", hasher.finalize());

    let signature_b64 = BASE64.encode(signature);
    let body = build_hashedrekord_body(&artifact_hash_hex, &signature_b64);
    submit_to_rekor(&body)
}

/// Submit a policy decision to the transparency log.
///
/// Per FR-S052: policy decisions are recorded for audit.
pub fn record_policy_decision(
    decision_id: &str,
    verdict: &str,
    policy_version: &str,
) -> TransparencyLogResult {
    // Hash the decision payload for the Rekor entry.
    let mut hasher = Sha256::new();
    hasher.update(decision_id.as_bytes());
    hasher.update(b":");
    hasher.update(verdict.as_bytes());
    hasher.update(b":");
    hasher.update(policy_version.as_bytes());
    let decision_hash_hex = format!("{:x}", hasher.finalize());

    // Use the decision hash as a pseudo-signature (policy decisions are
    // self-attested; the transparency log provides tamper-evidence).
    let signature_b64 = BASE64.encode(decision_hash_hex.as_bytes());
    let body = build_hashedrekord_body(&decision_hash_hex, &signature_b64);
    submit_to_rekor(&body)
}

/// Release channel configuration per FR-S053.
///
/// Direct promotion from development to production MUST be blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReleaseChannel {
    Development,
    Staging,
    Production,
}

impl ReleaseChannel {
    /// Check if promotion from this channel to the target is allowed.
    /// Per FR-S053: development → staging → production only.
    /// Direct dev → production is blocked.
    pub fn can_promote_to(self, target: ReleaseChannel) -> bool {
        matches!(
            (self, target),
            (ReleaseChannel::Development, ReleaseChannel::Staging)
                | (ReleaseChannel::Staging, ReleaseChannel::Production)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_metadata_available() {
        let meta = build_metadata();
        assert!(!meta.version.is_empty());
        // git commit and timestamp are set at build time
    }

    #[test]
    fn dev_to_staging_allowed() {
        assert!(ReleaseChannel::Development.can_promote_to(ReleaseChannel::Staging));
    }

    #[test]
    fn staging_to_production_allowed() {
        assert!(ReleaseChannel::Staging.can_promote_to(ReleaseChannel::Production));
    }

    #[test]
    fn dev_to_production_blocked() {
        assert!(!ReleaseChannel::Development.can_promote_to(ReleaseChannel::Production));
    }

    #[test]
    fn production_to_anything_blocked() {
        assert!(!ReleaseChannel::Production.can_promote_to(ReleaseChannel::Development));
        assert!(!ReleaseChannel::Production.can_promote_to(ReleaseChannel::Staging));
    }

    #[test]
    fn same_channel_promotion_blocked() {
        assert!(!ReleaseChannel::Development.can_promote_to(ReleaseChannel::Development));
    }

    #[test]
    fn provenance_attestation_serializes() {
        let prov = ProvenanceAttestation {
            build_source: "github.com/ContextLab/world-compute@abc123".into(),
            build_pipeline: "github-actions-12345".into(),
            build_timestamp: Timestamp::now(),
            reproducible: true,
        };
        let json = serde_json::to_string(&prov).unwrap();
        assert!(json.contains("world-compute"));
    }
}
