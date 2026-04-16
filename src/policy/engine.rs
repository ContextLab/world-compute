//! Core policy engine — orchestrates the evaluation pipeline.
//!
//! Per FR-S040: wraps `validate_manifest()` as one step in a larger pipeline.
//! Per contracts/policy-engine.md: 10-step sequential pipeline, short-circuits
//! on first rejection.

use crate::error::WcResult;
use crate::policy::decision::{PolicyCheck, PolicyDecision};
use crate::policy::rules;
use crate::scheduler::manifest::{self, JobManifest};
use crate::types::Timestamp;

/// Context provided alongside a manifest for policy evaluation.
#[derive(Debug, Clone)]
pub struct SubmissionContext {
    /// Submitter's peer ID string.
    pub submitter_peer_id: String,
    /// Submitter's public key bytes for signature verification.
    pub submitter_public_key: Vec<u8>,
    /// Current Humanity Points score for the submitter.
    pub submitter_hp_score: u32,
    /// Whether the submitter is currently banned.
    pub submitter_banned: bool,
    /// Submissions this epoch by this submitter.
    pub epoch_submission_count: u32,
    /// Maximum submissions per epoch for this submitter.
    pub epoch_submission_quota: u32,
}

/// Current policy version. Incremented on any policy rule change.
pub const POLICY_VERSION: &str = "002-safety-hardening-v1";

/// Evaluate a job submission through the deterministic policy pipeline.
///
/// Returns an auditable `PolicyDecision` with full reasoning.
/// The pipeline short-circuits on the first rejection.
pub fn evaluate(manifest: &JobManifest, ctx: &SubmissionContext) -> WcResult<PolicyDecision> {
    let decision_id = format!(
        "pd-{}-{}",
        Timestamp::now().0,
        &ctx.submitter_peer_id[..8.min(ctx.submitter_peer_id.len())]
    );
    let manifest_cid = manifest.workload_cid;
    let mut checks = Vec::new();

    // Step 1: Manifest structural validation (delegates to existing validate_manifest)
    let structural_check = match manifest::validate_manifest(manifest) {
        Ok(()) => PolicyCheck {
            check_name: "manifest_structural".into(),
            passed: true,
            detail: "Manifest passes structural validation".into(),
        },
        Err(e) => {
            let check = PolicyCheck {
                check_name: "manifest_structural".into(),
                passed: false,
                detail: format!("Structural validation failed: {e}"),
            };
            checks.push(check);
            return Ok(PolicyDecision::reject(
                decision_id,
                manifest_cid,
                ctx.submitter_peer_id.clone(),
                POLICY_VERSION.into(),
                checks,
                format!("Structural validation failed: {e}"),
            ));
        }
    };
    checks.push(structural_check);

    // Step 2: Submitter identity check
    let identity_check = rules::check_submitter_identity(ctx);
    let passed = identity_check.passed;
    checks.push(identity_check);
    if !passed {
        return Ok(PolicyDecision::reject(
            decision_id,
            manifest_cid,
            ctx.submitter_peer_id.clone(),
            POLICY_VERSION.into(),
            checks,
            "Submitter identity check failed".into(),
        ));
    }

    // Step 3: Signature verification
    let sig_check = rules::check_signature(manifest, ctx);
    let passed = sig_check.passed;
    checks.push(sig_check);
    if !passed {
        return Ok(PolicyDecision::reject(
            decision_id,
            manifest_cid,
            ctx.submitter_peer_id.clone(),
            POLICY_VERSION.into(),
            checks,
            "Signature verification failed".into(),
        ));
    }

    // Step 4: Artifact registry lookup
    let artifact_check = rules::check_artifact_registry(manifest);
    let passed = artifact_check.passed;
    checks.push(artifact_check);
    if !passed {
        return Ok(PolicyDecision::reject(
            decision_id,
            manifest_cid,
            ctx.submitter_peer_id.clone(),
            POLICY_VERSION.into(),
            checks,
            "Workload artifact not in approved registry".into(),
        ));
    }

    // Step 5: Workload class approval (including quarantine check)
    let class_check = rules::check_workload_class(manifest);
    let passed = class_check.passed;
    checks.push(class_check);
    if !passed {
        return Ok(PolicyDecision::reject(
            decision_id,
            manifest_cid,
            ctx.submitter_peer_id.clone(),
            POLICY_VERSION.into(),
            checks,
            "Workload class not approved or quarantined".into(),
        ));
    }

    // Step 6: Resource limit / quota check
    let quota_check = rules::check_quota(ctx);
    let passed = quota_check.passed;
    checks.push(quota_check);
    if !passed {
        return Ok(PolicyDecision::reject(
            decision_id,
            manifest_cid,
            ctx.submitter_peer_id.clone(),
            POLICY_VERSION.into(),
            checks,
            "Submission quota exceeded".into(),
        ));
    }

    // Step 7: Endpoint allowlist (if egress requested)
    let egress_check = rules::check_egress_allowlist(manifest);
    let passed = egress_check.passed;
    checks.push(egress_check);
    if !passed {
        return Ok(PolicyDecision::reject(
            decision_id,
            manifest_cid,
            ctx.submitter_peer_id.clone(),
            POLICY_VERSION.into(),
            checks,
            "Network egress requested without approved endpoint allowlist".into(),
        ));
    }

    // Step 8: Ban status check
    let ban_check = rules::check_ban_status(ctx);
    let passed = ban_check.passed;
    checks.push(ban_check);
    if !passed {
        return Ok(PolicyDecision::reject(
            decision_id,
            manifest_cid,
            ctx.submitter_peer_id.clone(),
            POLICY_VERSION.into(),
            checks,
            "Submitter is banned".into(),
        ));
    }

    // All checks passed
    Ok(PolicyDecision::accept(
        decision_id,
        manifest_cid,
        ctx.submitter_peer_id.clone(),
        POLICY_VERSION.into(),
        checks,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::cid_store::compute_cid;
    use crate::policy::decision::Verdict;
    use crate::scheduler::{
        ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
    };

    fn test_manifest() -> JobManifest {
        let cid = compute_cid(b"test workload image").unwrap();
        JobManifest {
            manifest_cid: None,
            name: "test-job".into(),
            workload_type: WorkloadType::WasmModule,
            workload_cid: cid,
            command: vec!["run".into()],
            inputs: Vec::new(),
            output_sink: "cid-store".into(),
            resources: ResourceEnvelope {
                cpu_millicores: 1000,
                ram_bytes: 512 * 1024 * 1024,
                gpu_class: None,
                gpu_vram_bytes: 0,
                scratch_bytes: 1024 * 1024 * 1024,
                network_egress_bytes: 0,
                walltime_budget_ms: 3_600_000,
            },
            category: JobCategory::PublicGood,
            confidentiality: ConfidentialityLevel::Public,
            verification: VerificationMethod::ReplicatedQuorum,
            acceptable_use_classes: vec![crate::acceptable_use::AcceptableUseClass::Scientific],
            max_wallclock_ms: 3_600_000,
            submitter_signature: vec![1u8; 64], // non-zero
        }
    }

    fn test_context() -> SubmissionContext {
        SubmissionContext {
            submitter_peer_id: "12D3KooWTestPeerId".into(),
            submitter_public_key: vec![0u8; 32],
            submitter_hp_score: 10,
            submitter_banned: false,
            epoch_submission_count: 0,
            epoch_submission_quota: 100,
        }
    }

    #[test]
    fn valid_submission_accepted() {
        let manifest = test_manifest();
        let ctx = test_context();
        let decision = evaluate(&manifest, &ctx).unwrap();
        assert_eq!(decision.verdict, Verdict::Accept);
        assert!(decision.reject_reason.is_none());
    }

    #[test]
    fn banned_submitter_rejected() {
        let manifest = test_manifest();
        let mut ctx = test_context();
        ctx.submitter_banned = true;
        let decision = evaluate(&manifest, &ctx).unwrap();
        assert_eq!(decision.verdict, Verdict::Reject);
        assert!(decision.reject_reason.unwrap().contains("banned"));
    }

    #[test]
    fn quota_exceeded_rejected() {
        let manifest = test_manifest();
        let mut ctx = test_context();
        ctx.epoch_submission_count = 101;
        ctx.epoch_submission_quota = 100;
        let decision = evaluate(&manifest, &ctx).unwrap();
        assert_eq!(decision.verdict, Verdict::Reject);
        assert!(decision.reject_reason.unwrap().contains("quota"));
    }

    #[test]
    fn zero_signature_rejected() {
        let mut manifest = test_manifest();
        manifest.submitter_signature = vec![0u8; 64];
        let ctx = test_context();
        let decision = evaluate(&manifest, &ctx).unwrap();
        assert_eq!(decision.verdict, Verdict::Reject);
        let reason = decision.reject_reason.unwrap();
        assert!(
            reason.contains("signature") || reason.contains("Signature"),
            "Expected signature-related rejection, got: {reason}"
        );
    }
}
