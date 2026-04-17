//! PolicyDecision — auditable record of a deterministic policy evaluation.
//!
//! Per FR-S041 and data-model.md: every evaluation produces an immutable
//! record with full reasoning.

use crate::types::{Cid, PeerIdStr, Timestamp};
use serde::{Deserialize, Serialize};

/// Verdict of a policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Verdict {
    Accept,
    Reject,
}

/// Result of a single policy check within the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCheck {
    /// Name of the check (e.g., "submitter_identity", "workload_class").
    pub check_name: String,
    /// Whether this check passed.
    pub passed: bool,
    /// Human-readable explanation of the result.
    pub detail: String,
}

/// An auditable record of a deterministic policy engine evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Unique identifier for this evaluation.
    pub decision_id: String,
    /// CID of the evaluated job manifest.
    pub manifest_cid: Cid,
    /// Identity of the submitter.
    pub submitter_peer_id: PeerIdStr,
    /// Version of the policy ruleset applied.
    pub policy_version: String,
    /// Individual check results.
    pub checks: Vec<PolicyCheck>,
    /// Final verdict.
    pub verdict: Verdict,
    /// Human-readable reason if rejected.
    pub reject_reason: Option<String>,
    /// LLM advisory opinion if provided.
    pub llm_advisory_flag: Option<String>,
    /// True if LLM flagged but policy approved (or vice versa).
    pub llm_disagrees: bool,
    /// Result of artifact CID lookup against ApprovedArtifact registry
    pub artifact_registry_result: Option<String>,
    /// Result of egress endpoint validation
    pub egress_validation_result: Option<String>,
    /// When the evaluation occurred.
    pub timestamp: Timestamp,
}

impl PolicyDecision {
    /// Create a new accepted decision.
    pub fn accept(
        decision_id: String,
        manifest_cid: Cid,
        submitter_peer_id: PeerIdStr,
        policy_version: String,
        checks: Vec<PolicyCheck>,
    ) -> Self {
        Self {
            decision_id,
            manifest_cid,
            submitter_peer_id,
            policy_version,
            checks,
            verdict: Verdict::Accept,
            reject_reason: None,
            llm_advisory_flag: None,
            llm_disagrees: false,
            artifact_registry_result: None,
            egress_validation_result: None,
            timestamp: Timestamp::now(),
        }
    }

    /// Create a new rejected decision.
    pub fn reject(
        decision_id: String,
        manifest_cid: Cid,
        submitter_peer_id: PeerIdStr,
        policy_version: String,
        checks: Vec<PolicyCheck>,
        reason: String,
    ) -> Self {
        Self {
            decision_id,
            manifest_cid,
            submitter_peer_id,
            policy_version,
            checks,
            verdict: Verdict::Reject,
            reject_reason: Some(reason),
            llm_advisory_flag: None,
            llm_disagrees: false,
            artifact_registry_result: None,
            egress_validation_result: None,
            timestamp: Timestamp::now(),
        }
    }
}
