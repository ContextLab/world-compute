//! T059 [US4]: Quota-exceeded submitter rejected.

use worldcompute::data_plane::cid_store::compute_cid;
use worldcompute::policy::decision::Verdict;
use worldcompute::policy::engine::{evaluate, SubmissionContext};
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

fn valid_manifest() -> JobManifest {
    let cid = compute_cid(b"test").unwrap();
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
        submitter_signature: vec![1u8; 64],
    }
}

#[test]
fn quota_exceeded_rejected() {
    let ctx = SubmissionContext {
        submitter_peer_id: "peer-1".into(),
        submitter_public_key: vec![0; 32],
        submitter_hp_score: 10,
        submitter_banned: false,
        epoch_submission_count: 101,
        epoch_submission_quota: 100,
    };
    let d = evaluate(&valid_manifest(), &ctx).unwrap();
    assert_eq!(d.verdict, Verdict::Reject);
    assert!(d.reject_reason.unwrap().contains("quota"));
}

#[test]
fn within_quota_accepted() {
    let ctx = SubmissionContext {
        submitter_peer_id: "peer-1".into(),
        submitter_public_key: vec![0; 32],
        submitter_hp_score: 10,
        submitter_banned: false,
        epoch_submission_count: 50,
        epoch_submission_quota: 100,
    };
    let d = evaluate(&valid_manifest(), &ctx).unwrap();
    assert_eq!(d.verdict, Verdict::Accept);
}
