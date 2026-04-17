//! Integration tests for acceptable_use filter (T100).

use worldcompute::acceptable_use::filter::{
    check_acceptable_use, check_acceptable_use_with_policy, RejectedCategory,
};
use worldcompute::acceptable_use::AcceptableUseClass;
use worldcompute::data_plane::cid_store::compute_cid;
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

fn base_manifest(classes: Vec<AcceptableUseClass>) -> JobManifest {
    let cid = compute_cid(b"test workload").unwrap();
    JobManifest {
        manifest_cid: None,
        name: "filter-integration-test".into(),
        workload_type: WorkloadType::WasmModule,
        workload_cid: cid,
        command: vec!["run".into()],
        inputs: Vec::new(),
        output_sink: "cid-store".into(),
        resources: ResourceEnvelope {
            cpu_millicores: 500,
            ram_bytes: 256 * 1024 * 1024,
            gpu_class: None,
            gpu_vram_bytes: 0,
            scratch_bytes: 512 * 1024 * 1024,
            network_egress_bytes: 0,
            walltime_budget_ms: 3_600_000,
        },
        category: JobCategory::PublicGood,
        confidentiality: ConfidentialityLevel::Public,
        verification: VerificationMethod::ReplicatedQuorum,
        acceptable_use_classes: classes,
        max_wallclock_ms: 3_600_000,
        submitter_signature: vec![0u8; 64],
        allowed_endpoints: Vec::new(),
        confidentiality_level: None,
    }
}

#[test]
fn general_compute_is_accepted() {
    let manifest = base_manifest(vec![AcceptableUseClass::GeneralCompute]);
    assert!(check_acceptable_use(&manifest).is_ok());
}

#[test]
fn malware_class_rejected_via_policy() {
    let manifest = base_manifest(vec![AcceptableUseClass::GeneralCompute]);
    let policy = vec![(AcceptableUseClass::GeneralCompute, RejectedCategory::MalwareDistribution)];
    let result = check_acceptable_use_with_policy(&manifest, &policy);
    assert!(result.is_err());
}

#[test]
fn empty_classes_handled() {
    let manifest = base_manifest(vec![]);
    assert!(check_acceptable_use(&manifest).is_ok());
}

#[test]
fn multiple_classes_all_checked() {
    let manifest = base_manifest(vec![
        AcceptableUseClass::Scientific,
        AcceptableUseClass::Rendering,
        AcceptableUseClass::Indexing,
    ]);
    assert!(check_acceptable_use(&manifest).is_ok());
}
