//! T039/T053-T055: Artifact registry policy checks.

use worldcompute::data_plane::cid_store::compute_cid;
use worldcompute::policy::decision::Verdict;
use worldcompute::policy::engine::{evaluate, SubmissionContext};
use worldcompute::policy::rules::{
    check_artifact_registry, check_artifact_registry_with, ApprovedArtifact, ArtifactRegistry,
    ReleaseChannel,
};
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

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

fn test_manifest() -> JobManifest {
    let cid = compute_cid(b"test artifact").unwrap();
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
        acceptable_use_classes: vec![worldcompute::acceptable_use::AcceptableUseClass::Scientific],
        max_wallclock_ms: 3_600_000,
        submitter_signature: vec![0u8; 64], // all zeros — unsigned
        allowed_endpoints: Vec::new(),
        confidentiality_level: None,
    }
}

#[test]
fn unsigned_artifact_rejected() {
    let manifest = test_manifest(); // has all-zero signature
    let ctx = test_ctx();
    let decision = evaluate(&manifest, &ctx).unwrap();
    assert_eq!(decision.verdict, Verdict::Reject);
}

// T053: Valid CID in approved registry → accepted
#[test]
fn artifact_valid_cid_in_registry_accepted() {
    let manifest = test_manifest();
    let cid_str = manifest.workload_cid.to_string();
    let mut registry = ArtifactRegistry::default();
    registry.approved_cids.insert(cid_str.clone());
    registry.artifacts.push(ApprovedArtifact {
        cid: cid_str,
        signer: "alice".into(),
        approver: "bob".into(),
        channel: ReleaseChannel::Production,
    });
    let check = check_artifact_registry_with(&manifest, Some(&registry));
    assert!(check.passed, "Expected pass, got: {}", check.detail);
}

// T054: Unknown CID → rejected
#[test]
fn artifact_unknown_cid_rejected() {
    let manifest = test_manifest();
    let registry = ArtifactRegistry::default(); // empty registry
    let check = check_artifact_registry_with(&manifest, Some(&registry));
    assert!(!check.passed, "Expected rejection for unknown CID");
    assert!(
        check.detail.contains("not found"),
        "Expected 'not found' in detail, got: {}",
        check.detail
    );
}

// T055: Same signer and approver → separation of duties violation → rejected
#[test]
fn artifact_same_signer_approver_rejected() {
    let manifest = test_manifest();
    let cid_str = manifest.workload_cid.to_string();
    let mut registry = ArtifactRegistry::default();
    registry.approved_cids.insert(cid_str.clone());
    registry.artifacts.push(ApprovedArtifact {
        cid: cid_str,
        signer: "alice".into(),
        approver: "alice".into(), // same as signer — violation
        channel: ReleaseChannel::Production,
    });
    let check = check_artifact_registry_with(&manifest, Some(&registry));
    assert!(!check.passed, "Expected rejection for same signer/approver");
    assert!(
        check.detail.contains("Separation of duties"),
        "Expected separation of duties in detail, got: {}",
        check.detail
    );
}

// No registry → structural gate (non-empty CID passes)
#[test]
fn artifact_no_registry_passes_structural_gate() {
    let manifest = test_manifest();
    let check = check_artifact_registry(&manifest);
    assert!(check.passed);
}
