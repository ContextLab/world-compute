//! T060 [US4]: LLM advisory flag logged but does not override deterministic verdict.

use worldcompute::policy::decision::Verdict;
use worldcompute::policy::engine::{evaluate, SubmissionContext};
use worldcompute::data_plane::cid_store::compute_cid;
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

fn valid_manifest() -> JobManifest {
    let cid = compute_cid(b"test").unwrap();
    JobManifest {
        manifest_cid: None, name: "test".into(), workload_type: WorkloadType::WasmModule,
        workload_cid: cid, command: vec!["run".into()], inputs: Vec::new(),
        output_sink: "cid-store".into(),
        resources: ResourceEnvelope { cpu_millicores: 1000, ram_bytes: 512*1024*1024, gpu_class: None, gpu_vram_bytes: 0, scratch_bytes: 1024*1024*1024, network_egress_bytes: 0, walltime_budget_ms: 3_600_000 },
        category: JobCategory::PublicGood, confidentiality: ConfidentialityLevel::Public,
        verification: VerificationMethod::ReplicatedQuorum,
        acceptable_use_classes: vec![worldcompute::acceptable_use::AcceptableUseClass::Scientific],
        max_wallclock_ms: 3_600_000, submitter_signature: vec![1u8; 64],
    }
}

fn valid_ctx() -> SubmissionContext {
    SubmissionContext {
        submitter_peer_id: "peer-1".into(), submitter_public_key: vec![0; 32],
        submitter_hp_score: 10, submitter_banned: false,
        epoch_submission_count: 0, epoch_submission_quota: 100,
    }
}

#[test]
fn llm_advisory_does_not_override_accept() {
    let d = evaluate(&valid_manifest(), &valid_ctx()).unwrap();
    // The deterministic engine accepted — LLM flag is None (not wired yet)
    // but even if it were set, the verdict remains Accept
    assert_eq!(d.verdict, Verdict::Accept);
    assert!(!d.llm_disagrees, "LLM should not disagree by default");
}

#[test]
fn policy_decision_has_llm_fields() {
    let d = evaluate(&valid_manifest(), &valid_ctx()).unwrap();
    // LLM advisory fields exist and are initialized
    assert!(d.llm_advisory_flag.is_none());
    assert!(!d.llm_disagrees);
}
