//! T040 [US2]: Valid attestation + valid signature = job admitted.

use ed25519_dalek::{Signer, SigningKey};
use worldcompute::policy::decision::Verdict;
use worldcompute::policy::engine::{evaluate, SubmissionContext};
use worldcompute::policy::rules::manifest_signing_bytes;
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

fn test_ctx() -> SubmissionContext {
    SubmissionContext {
        submitter_peer_id: "12D3KooWTest".into(),
        submitter_public_key: SigningKey::from_bytes(&[42u8; 32])
            .verifying_key()
            .to_bytes()
            .to_vec(),
        submitter_hp_score: 10,
        submitter_banned: false,
        epoch_submission_count: 0,
        epoch_submission_quota: 100,
    }
}

fn valid_manifest() -> JobManifest {
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"valid artifact").unwrap();
    let mut manifest = JobManifest {
        manifest_cid: None,
        name: "valid-job".into(),
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
        submitter_signature: vec![0u8; 64],
    };
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let message = manifest_signing_bytes(&manifest);
    let signature = signing_key.sign(&message);
    manifest.submitter_signature = signature.to_bytes().to_vec();
    manifest
}

#[test]
fn valid_submission_accepted() {
    let manifest = valid_manifest();
    let ctx = test_ctx();
    let decision = evaluate(&manifest, &ctx).unwrap();
    assert_eq!(decision.verdict, Verdict::Accept);
    assert!(decision.reject_reason.is_none());
    assert!(!decision.checks.is_empty());
}
