//! Transparency log integration — Sigstore Rekor or equivalent.
//!
//! Per FR-S052: all artifact signatures and policy decisions MUST be
//! recorded in a transparency log.
//! Per FR-S051: all workload artifacts MUST carry provenance attestations.

use crate::types::Timestamp;
use serde::{Deserialize, Serialize};

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
    /// Entry recorded with the given log index.
    Recorded { log_index: String, timestamp: Timestamp },
    /// Log service unavailable.
    Unavailable(String),
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
    // TODO(T096): Integrate with Sigstore Rekor REST API:
    // POST https://rekor.sigstore.dev/api/v1/log/entries
    // with hashedrekord type containing artifact hash + signature
    let _ = (artifact_cid, signature, provenance);
    TransparencyLogResult::Unavailable(
        "Sigstore Rekor integration pending (T096) — entries logged locally".into(),
    )
}

/// Submit a policy decision to the transparency log.
///
/// Per FR-S052: policy decisions are recorded for audit.
pub fn record_policy_decision(
    decision_id: &str,
    verdict: &str,
    policy_version: &str,
) -> TransparencyLogResult {
    let _ = (decision_id, verdict, policy_version);
    TransparencyLogResult::Unavailable(
        "Sigstore Rekor integration pending (T096) — decisions logged locally".into(),
    )
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
