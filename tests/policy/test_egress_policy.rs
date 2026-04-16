//! T058 [US4]: Egress request without approved allowlist rejected.

use worldcompute::data_plane::cid_store::compute_cid;
use worldcompute::policy::rules::check_egress_allowlist;
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

fn manifest_with_egress(egress_bytes: u64) -> JobManifest {
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
            network_egress_bytes: egress_bytes,
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
fn egress_request_without_allowlist_rejected() {
    let m = manifest_with_egress(1024);
    let check = check_egress_allowlist(&m);
    assert!(!check.passed);
}

#[test]
fn zero_egress_passes() {
    let m = manifest_with_egress(0);
    let check = check_egress_allowlist(&m);
    assert!(check.passed);
}
