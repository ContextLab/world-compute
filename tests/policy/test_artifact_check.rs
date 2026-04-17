//! T039 [US2]: Unsigned workload artifact rejected at admission.

use worldcompute::policy::decision::Verdict;
use worldcompute::policy::engine::{evaluate, SubmissionContext};
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
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"test artifact").unwrap();
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
