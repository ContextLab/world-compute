//! Individual policy rules for the deterministic evaluation pipeline.
//!
//! Each rule produces a `PolicyCheck` result. Rules are pure functions
//! operating on manifest data and submission context.

use crate::policy::decision::PolicyCheck;
use crate::policy::engine::SubmissionContext;
use crate::scheduler::manifest::JobManifest;

/// Step 2: Verify submitter identity is registered and meets HP threshold.
pub fn check_submitter_identity(ctx: &SubmissionContext) -> PolicyCheck {
    if ctx.submitter_peer_id.is_empty() {
        return PolicyCheck {
            check_name: "submitter_identity".into(),
            passed: false,
            detail: "Submitter peer ID is empty".into(),
        };
    }
    // Minimum HP score of 1 required for any submission
    if ctx.submitter_hp_score < 1 {
        return PolicyCheck {
            check_name: "submitter_identity".into(),
            passed: false,
            detail: format!(
                "Submitter HP score {} below minimum threshold 1",
                ctx.submitter_hp_score
            ),
        };
    }
    PolicyCheck {
        check_name: "submitter_identity".into(),
        passed: true,
        detail: format!(
            "Submitter {} verified with HP score {}",
            &ctx.submitter_peer_id, ctx.submitter_hp_score
        ),
    }
}

/// Step 3: Verify submitter signature is non-trivial.
///
/// Full cryptographic verification (Ed25519 against registered public key)
/// is implemented in Phase 2 (T018). This check rejects all-zero and empty
/// signatures as a structural gate per FR-S012.
pub fn check_signature(manifest: &JobManifest, _ctx: &SubmissionContext) -> PolicyCheck {
    if manifest.submitter_signature.is_empty() {
        return PolicyCheck {
            check_name: "signature_verification".into(),
            passed: false,
            detail: "Submitter signature is empty".into(),
        };
    }
    if manifest.submitter_signature.iter().all(|&b| b == 0) {
        return PolicyCheck {
            check_name: "signature_verification".into(),
            passed: false,
            detail: "Submitter signature is all zeros — rejected per FR-S012".into(),
        };
    }
    // TODO(Phase 2 T018): Full Ed25519 cryptographic verification against
    // ctx.submitter_public_key. For now, non-trivial signatures pass.
    PolicyCheck {
        check_name: "signature_verification".into(),
        passed: true,
        detail: "Signature is non-trivial (full crypto verification pending T018)".into(),
    }
}

/// Step 4: Check workload artifact CID against approved registry.
///
/// Full registry lookup is implemented in Phase 2 (T019). This check
/// verifies the CID is non-empty as a structural gate per FR-S013.
pub fn check_artifact_registry(manifest: &JobManifest) -> PolicyCheck {
    if manifest.workload_cid.to_string().is_empty() {
        return PolicyCheck {
            check_name: "artifact_registry".into(),
            passed: false,
            detail: "Workload CID is empty".into(),
        };
    }
    // TODO(Phase 2 T019): Lookup CID in ApprovedArtifact registry.
    // For now, any non-empty CID passes.
    PolicyCheck {
        check_name: "artifact_registry".into(),
        passed: true,
        detail: "Workload CID present (full registry lookup pending T019)".into(),
    }
}

/// Step 5: Check workload class is approved and not quarantined.
pub fn check_workload_class(manifest: &JobManifest) -> PolicyCheck {
    // Quarantine status will be wired in Phase 7 (T078).
    // For now, all non-empty acceptable_use_classes pass.
    if manifest.acceptable_use_classes.is_empty() {
        return PolicyCheck {
            check_name: "workload_class".into(),
            passed: false,
            detail: "No acceptable use classes declared".into(),
        };
    }
    PolicyCheck {
        check_name: "workload_class".into(),
        passed: true,
        detail: format!(
            "Workload class {:?} approved (quarantine check pending T078)",
            manifest.acceptable_use_classes
        ),
    }
}

/// Step 6: Check submitter quota.
pub fn check_quota(ctx: &SubmissionContext) -> PolicyCheck {
    if ctx.epoch_submission_count >= ctx.epoch_submission_quota {
        return PolicyCheck {
            check_name: "quota_enforcement".into(),
            passed: false,
            detail: format!(
                "Submitter has {} submissions this epoch, quota is {}",
                ctx.epoch_submission_count, ctx.epoch_submission_quota
            ),
        };
    }
    PolicyCheck {
        check_name: "quota_enforcement".into(),
        passed: true,
        detail: format!(
            "Quota OK: {}/{} submissions this epoch",
            ctx.epoch_submission_count, ctx.epoch_submission_quota
        ),
    }
}

/// Step 7: Check egress allowlist if network access requested.
///
/// Per FR-S021: jobs requesting `network_egress_bytes > 0` must declare
/// specific endpoint allowlists validated against an approved list.
pub fn check_egress_allowlist(manifest: &JobManifest) -> PolicyCheck {
    if manifest.resources.network_egress_bytes == 0 {
        return PolicyCheck {
            check_name: "egress_allowlist".into(),
            passed: true,
            detail: "No network egress requested — default-deny applies".into(),
        };
    }
    // Jobs requesting egress must have an approved allowlist.
    // TODO: Add endpoint allowlist field to JobManifest and validate here.
    // For now, any non-zero egress is rejected until allowlist is implemented.
    PolicyCheck {
        check_name: "egress_allowlist".into(),
        passed: false,
        detail: format!(
            "Network egress of {} bytes requested but endpoint allowlist not yet implemented",
            manifest.resources.network_egress_bytes
        ),
    }
}

/// Step 8: Check ban status.
pub fn check_ban_status(ctx: &SubmissionContext) -> PolicyCheck {
    if ctx.submitter_banned {
        return PolicyCheck {
            check_name: "ban_status".into(),
            passed: false,
            detail: "Submitter is currently banned".into(),
        };
    }
    PolicyCheck {
        check_name: "ban_status".into(),
        passed: true,
        detail: "Submitter is not banned".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::cid_store::compute_cid;
    use crate::policy::engine::SubmissionContext;
    use crate::scheduler::{
        ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
    };

    fn test_manifest() -> JobManifest {
        let cid = compute_cid(b"test workload").unwrap();
        JobManifest {
            manifest_cid: None,
            name: "test".into(),
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
            submitter_signature: vec![1u8; 64],
        }
    }

    fn test_ctx() -> SubmissionContext {
        SubmissionContext {
            submitter_peer_id: "12D3KooWTest".into(),
            submitter_public_key: vec![0u8; 32],
            submitter_hp_score: 10,
            submitter_banned: false,
            epoch_submission_count: 0,
            epoch_submission_quota: 100,
        }
    }

    #[test]
    fn all_zero_signature_fails() {
        let mut m = test_manifest();
        m.submitter_signature = vec![0u8; 64];
        let check = check_signature(&m, &test_ctx());
        assert!(!check.passed);
        assert!(check.detail.contains("all zeros"));
    }

    #[test]
    fn empty_signature_fails() {
        let mut m = test_manifest();
        m.submitter_signature = Vec::new();
        let check = check_signature(&m, &test_ctx());
        assert!(!check.passed);
    }

    #[test]
    fn valid_signature_passes() {
        let m = test_manifest();
        let check = check_signature(&m, &test_ctx());
        assert!(check.passed);
    }

    #[test]
    fn egress_without_allowlist_rejected() {
        let mut m = test_manifest();
        m.resources.network_egress_bytes = 1024;
        let check = check_egress_allowlist(&m);
        assert!(!check.passed);
    }

    #[test]
    fn no_egress_passes() {
        let m = test_manifest();
        let check = check_egress_allowlist(&m);
        assert!(check.passed);
    }

    #[test]
    fn banned_submitter_fails() {
        let mut ctx = test_ctx();
        ctx.submitter_banned = true;
        let check = check_ban_status(&ctx);
        assert!(!check.passed);
    }

    #[test]
    fn low_hp_fails() {
        let mut ctx = test_ctx();
        ctx.submitter_hp_score = 0;
        let check = check_submitter_identity(&ctx);
        assert!(!check.passed);
    }
}
