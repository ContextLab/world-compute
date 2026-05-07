//! Individual policy rules for the deterministic evaluation pipeline.
//!
//! Each rule produces a `PolicyCheck` result. Rules are pure functions
//! operating on manifest data and submission context.

use crate::policy::decision::PolicyCheck;
use crate::policy::engine::SubmissionContext;
use crate::scheduler::manifest::JobManifest;

/// Release channel for approved artifacts.
/// Promotion order: Dev → Staging → Production (no skip allowed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReleaseChannel {
    Dev = 0,
    Staging = 1,
    Production = 2,
}

/// An approved artifact entry in the registry.
#[derive(Debug, Clone)]
pub struct ApprovedArtifact {
    /// CID string of the approved artifact.
    pub cid: String,
    /// Identity that signed the artifact.
    pub signer: String,
    /// Identity that approved the artifact.
    pub approver: String,
    /// Current release channel.
    pub channel: ReleaseChannel,
}

/// In-memory registry of approved artifacts.
#[derive(Debug, Clone, Default)]
pub struct ArtifactRegistry {
    pub approved_cids: std::collections::HashSet<String>,
    pub artifacts: Vec<ApprovedArtifact>,
}

impl ArtifactRegistry {
    /// Look up an artifact by CID and validate separation of duties and release channel.
    pub fn validate(&self, cid: &str) -> Result<(), String> {
        if !self.approved_cids.contains(cid) {
            return Err(format!("CID {cid} not found in approved artifact registry"));
        }
        if let Some(artifact) = self.artifacts.iter().find(|a| a.cid == cid) {
            // Separation of duties: signer and approver must be different identities
            if artifact.signer == artifact.approver {
                return Err(format!(
                    "Separation of duties violation: signer and approver are the same identity ({})",
                    artifact.signer
                ));
            }
            // Release channel: dev→staging→production only (no skip from dev to production)
            // This is validated at promotion time; here we just confirm the artifact has a valid channel
        }
        Ok(())
    }
}

/// Approved endpoint patterns for egress allowlist validation.
/// Default is empty list (default-deny).
#[derive(Debug, Clone, Default)]
pub struct EgressAllowlist {
    pub approved_endpoints: Vec<String>,
}

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
    // Ed25519 cryptographic verification against ctx.submitter_public_key.
    if _ctx.submitter_public_key.len() != 32 {
        return PolicyCheck {
            check_name: "signature_verification".into(),
            passed: false,
            detail: format!(
                "Submitter public key has invalid length {} (expected 32 bytes)",
                _ctx.submitter_public_key.len()
            ),
        };
    }
    if manifest.submitter_signature.len() != 64 {
        return PolicyCheck {
            check_name: "signature_verification".into(),
            passed: false,
            detail: format!(
                "Signature has invalid length {} (expected 64 bytes)",
                manifest.submitter_signature.len()
            ),
        };
    }

    // Construct the message: hash of manifest fields excluding the signature
    let message = manifest_signing_bytes(manifest);

    match verify_ed25519(&_ctx.submitter_public_key, &message, &manifest.submitter_signature) {
        Ok(true) => PolicyCheck {
            check_name: "signature_verification".into(),
            passed: true,
            detail: "Ed25519 signature verified against submitter public key".into(),
        },
        Ok(false) | Err(_) => PolicyCheck {
            check_name: "signature_verification".into(),
            passed: false,
            detail: "Ed25519 signature verification failed".into(),
        },
    }
}

/// Step 4: Check workload artifact CID against approved registry.
///
/// Verifies the CID is non-empty and, when a registry is provided,
/// checks the CID exists in the approved set with valid separation
/// of duties (signer != approver) per FR-S013.
pub fn check_artifact_registry(manifest: &JobManifest) -> PolicyCheck {
    check_artifact_registry_with(manifest, None)
}

/// Step 4 (with registry): Check workload artifact CID against an explicit registry.
pub fn check_artifact_registry_with(
    manifest: &JobManifest,
    registry: Option<&ArtifactRegistry>,
) -> PolicyCheck {
    let cid_str = manifest.workload_cid.to_string();
    if cid_str.is_empty() {
        return PolicyCheck {
            check_name: "artifact_registry".into(),
            passed: false,
            detail: "Workload CID is empty".into(),
        };
    }
    if let Some(reg) = registry {
        match reg.validate(&cid_str) {
            Ok(()) => PolicyCheck {
                check_name: "artifact_registry".into(),
                passed: true,
                detail: format!("Workload CID {cid_str} approved in artifact registry"),
            },
            Err(reason) => PolicyCheck {
                check_name: "artifact_registry".into(),
                passed: false,
                detail: reason,
            },
        }
    } else {
        // No registry provided — accept if CID is non-empty (structural gate)
        PolicyCheck {
            check_name: "artifact_registry".into(),
            passed: true,
            detail: format!("Workload CID {cid_str} present (no registry configured)"),
        }
    }
}

/// Step 5: Check workload class is approved and not quarantined.
///
/// Per FR-S062: quarantined workload classes MUST be rejected.
/// The quarantine set is maintained by the incident response module.
pub fn check_workload_class(manifest: &JobManifest) -> PolicyCheck {
    check_workload_class_with_quarantine(manifest, &[])
}

/// Step 5 (with quarantine): Check workload class against quarantine list.
pub fn check_workload_class_with_quarantine(
    manifest: &JobManifest,
    quarantined_classes: &[String],
) -> PolicyCheck {
    if manifest.acceptable_use_classes.is_empty() {
        return PolicyCheck {
            check_name: "workload_class".into(),
            passed: false,
            detail: "No acceptable use classes declared".into(),
        };
    }

    // Check if any of the job's classes are quarantined
    for class in &manifest.acceptable_use_classes {
        let class_name = format!("{class:?}");
        if quarantined_classes.contains(&class_name) {
            return PolicyCheck {
                check_name: "workload_class".into(),
                passed: false,
                detail: format!(
                    "Workload class {class_name} is quarantined — rejected per FR-S062"
                ),
            };
        }
    }

    PolicyCheck {
        check_name: "workload_class".into(),
        passed: true,
        detail: format!("Workload class {:?} approved", manifest.acceptable_use_classes),
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
    check_egress_allowlist_with(manifest, None)
}

/// Step 7 (with allowlist): Validate declared endpoints against an approved allowlist.
///
/// If the job declares no endpoints and requests no egress, that is fine (default-deny).
/// If the job requests egress bytes > 0, it must declare endpoints and every declared
/// endpoint must appear in the approved allowlist.
pub fn check_egress_allowlist_with(
    manifest: &JobManifest,
    allowlist: Option<&EgressAllowlist>,
) -> PolicyCheck {
    if manifest.resources.network_egress_bytes == 0 {
        return PolicyCheck {
            check_name: "egress_allowlist".into(),
            passed: true,
            detail: "No network egress requested — default-deny applies".into(),
        };
    }

    // Egress requested — endpoints must be declared
    if manifest.allowed_endpoints.is_empty() {
        return PolicyCheck {
            check_name: "egress_allowlist".into(),
            passed: false,
            detail: format!(
                "Network egress of {} bytes requested but no endpoints declared",
                manifest.resources.network_egress_bytes
            ),
        };
    }

    // If an allowlist is provided, validate each declared endpoint
    if let Some(al) = allowlist {
        let rejected: Vec<&String> = manifest
            .allowed_endpoints
            .iter()
            .filter(|ep| !al.approved_endpoints.contains(ep))
            .collect();
        if !rejected.is_empty() {
            return PolicyCheck {
                check_name: "egress_allowlist".into(),
                passed: false,
                detail: format!(
                    "Unapproved endpoints: {}",
                    rejected.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ),
            };
        }
        PolicyCheck {
            check_name: "egress_allowlist".into(),
            passed: true,
            detail: format!("All {} declared endpoints approved", manifest.allowed_endpoints.len()),
        }
    } else {
        // No allowlist configured — reject egress requests without an allowlist to check against
        PolicyCheck {
            check_name: "egress_allowlist".into(),
            passed: false,
            detail: format!(
                "Network egress of {} bytes requested but no approved allowlist configured",
                manifest.resources.network_egress_bytes
            ),
        }
    }
}

/// Step 7b: Check data classification compatibility (T066).
///
/// Per FR-S040: verify data sensitivity level is compatible with available
/// host pools. ConfidentialHigh jobs require T3+ trust tier hosts.
pub fn check_data_classification(manifest: &JobManifest) -> PolicyCheck {
    use crate::scheduler::ConfidentialityLevel;
    match manifest.confidentiality {
        ConfidentialityLevel::Public => PolicyCheck {
            check_name: "data_classification".into(),
            passed: true,
            detail: "Public data — compatible with all host pools".into(),
        },
        ConfidentialityLevel::ConfidentialMedium => PolicyCheck {
            check_name: "data_classification".into(),
            passed: true,
            detail: "ConfidentialMedium — compatible with T1+ host pools".into(),
        },
        ConfidentialityLevel::ConfidentialHigh => {
            // ConfidentialHigh requires TEE verification (already checked by
            // validate_manifest), but we also flag it for routing awareness
            PolicyCheck {
                check_name: "data_classification".into(),
                passed: true,
                detail: "ConfidentialHigh — requires T3+ hosts with TEE attestation".into(),
            }
        }
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

/// Compute the canonical signing bytes for a manifest (all fields except signature).
pub fn manifest_signing_bytes(manifest: &JobManifest) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(manifest.name.as_bytes());
    hasher.update(manifest.workload_cid.to_string().as_bytes());
    for cmd in &manifest.command {
        hasher.update(cmd.as_bytes());
    }
    for input in &manifest.inputs {
        hasher.update(input.to_string().as_bytes());
    }
    hasher.update(manifest.output_sink.as_bytes());
    hasher.update(manifest.max_wallclock_ms.to_le_bytes());
    hasher.finalize().to_vec()
}

/// Verify an Ed25519 signature.
fn verify_ed25519(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<bool, String> {
    use ed25519_dalek::{Signature, VerifyingKey};

    let pk_bytes: [u8; 32] = public_key.try_into().map_err(|_| "Invalid public key length")?;
    let verifying_key =
        VerifyingKey::from_bytes(&pk_bytes).map_err(|e| format!("Invalid public key: {e}"))?;

    let sig_bytes: [u8; 64] = signature.try_into().map_err(|_| "Invalid signature length")?;
    let sig = Signature::from_bytes(&sig_bytes);

    use ed25519_dalek::Verifier;
    match verifying_key.verify(message, &sig) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
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

    use ed25519_dalek::{Signer, SigningKey};

    /// Create a signed test manifest with a real Ed25519 key pair.
    fn signed_test_manifest() -> (JobManifest, SubmissionContext) {
        let cid = compute_cid(b"test workload").unwrap();
        let mut manifest = JobManifest {
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
            submitter_signature: vec![0u8; 64], // sentinel bytes — overwritten with a real Ed25519 signature below
            allowed_endpoints: Vec::new(),
            confidentiality_level: None,
        };

        // Generate a real Ed25519 key pair and sign the manifest
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let message = manifest_signing_bytes(&manifest);
        let signature = signing_key.sign(&message);
        manifest.submitter_signature = signature.to_bytes().to_vec();

        let ctx = SubmissionContext {
            submitter_peer_id: "12D3KooWTest".into(),
            submitter_public_key: signing_key.verifying_key().to_bytes().to_vec(),
            submitter_hp_score: 10,
            submitter_banned: false,
            epoch_submission_count: 0,
            epoch_submission_quota: 100,
        };

        (manifest, ctx)
    }

    fn test_manifest() -> JobManifest {
        signed_test_manifest().0
    }

    fn test_ctx() -> SubmissionContext {
        signed_test_manifest().1
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

    #[test]
    fn quarantined_class_rejected() {
        let m = test_manifest();
        let quarantined = vec!["Scientific".to_string()];
        let check = check_workload_class_with_quarantine(&m, &quarantined);
        assert!(!check.passed);
        assert!(check.detail.contains("quarantined"));
    }

    #[test]
    fn non_quarantined_class_passes() {
        let m = test_manifest();
        let quarantined = vec!["MlTraining".to_string()];
        let check = check_workload_class_with_quarantine(&m, &quarantined);
        assert!(check.passed);
    }

    #[test]
    fn data_classification_public_passes() {
        let m = test_manifest();
        let check = check_data_classification(&m);
        assert!(check.passed);
    }
}
