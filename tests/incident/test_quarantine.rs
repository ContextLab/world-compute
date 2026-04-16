//! T073 [US5]: QuarantineWorkloadClass causes policy engine rejection.

use worldcompute::data_plane::cid_store::compute_cid;
use worldcompute::policy::rules::check_workload_class_with_quarantine;
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

fn test_manifest() -> JobManifest {
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
fn quarantined_class_causes_rejection() {
    let m = test_manifest();
    let quarantined = vec!["Scientific".to_string()];
    let check = check_workload_class_with_quarantine(&m, &quarantined);
    assert!(!check.passed, "Quarantined class must be rejected by policy engine");
}
