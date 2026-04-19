//! T056/T058: Egress allowlist policy checks.

use worldcompute::data_plane::cid_store::compute_cid;
use worldcompute::policy::rules::{
    check_egress_allowlist, check_egress_allowlist_with, EgressAllowlist,
};
use worldcompute::scheduler::manifest::JobManifest;
use worldcompute::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};

fn manifest_with_egress(egress_bytes: u64, endpoints: Vec<String>) -> JobManifest {
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
        allowed_endpoints: endpoints,
        confidentiality_level: None,
    }
}

#[test]
fn egress_request_without_allowlist_rejected() {
    let m = manifest_with_egress(1024, Vec::new());
    let check = check_egress_allowlist(&m);
    assert!(!check.passed);
}

#[test]
fn zero_egress_passes() {
    let m = manifest_with_egress(0, Vec::new());
    let check = check_egress_allowlist(&m);
    assert!(check.passed);
}

// T056: Approved endpoints → accepted
#[test]
fn egress_approved_endpoints_accepted() {
    let allowlist = EgressAllowlist {
        approved_endpoints: vec![
            "https://api.example.com".into(),
            "https://data.example.org".into(),
        ],
    };
    let m = manifest_with_egress(1024, vec!["https://api.example.com".into()]);
    let check = check_egress_allowlist_with(&m, Some(&allowlist));
    assert!(check.passed, "Expected pass, got: {}", check.detail);
}

// T056: Unapproved endpoint → rejected
#[test]
fn egress_unapproved_endpoint_rejected() {
    let allowlist = EgressAllowlist { approved_endpoints: vec!["https://api.example.com".into()] };
    let m = manifest_with_egress(1024, vec!["https://evil.example.net".into()]);
    let check = check_egress_allowlist_with(&m, Some(&allowlist));
    assert!(!check.passed, "Expected rejection for unapproved endpoint");
    assert!(
        check.detail.contains("Unapproved"),
        "Expected 'Unapproved' in detail, got: {}",
        check.detail
    );
}

// Mixed: one approved, one not → rejected
#[test]
fn egress_mixed_endpoints_rejected() {
    let allowlist = EgressAllowlist { approved_endpoints: vec!["https://api.example.com".into()] };
    let m = manifest_with_egress(
        1024,
        vec!["https://api.example.com".into(), "https://evil.example.net".into()],
    );
    let check = check_egress_allowlist_with(&m, Some(&allowlist));
    assert!(!check.passed);
}

// Zero egress with endpoints declared still passes (no egress needed)
#[test]
fn zero_egress_with_endpoints_passes() {
    let m = manifest_with_egress(0, vec!["https://api.example.com".into()]);
    let check = check_egress_allowlist(&m);
    assert!(check.passed);
}

// Egress requested, endpoints declared, but no allowlist configured → rejected
#[test]
fn egress_no_allowlist_configured_rejected() {
    let m = manifest_with_egress(1024, vec!["https://api.example.com".into()]);
    let check = check_egress_allowlist_with(&m, None);
    assert!(!check.passed);
}
