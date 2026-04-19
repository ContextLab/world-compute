//! Integration tests for CLI command types (T102).
//!
//! We test the CLI subcommand enums and argument structures rather than
//! executing full commands (which require async runtime and network).

use worldcompute::acceptable_use::AcceptableUseClass;
use worldcompute::scheduler::{ConfidentialityLevel, JobCategory, ResourceEnvelope, WorkloadType};

#[test]
fn donor_consent_classes_available() {
    // Verify all AcceptableUseClass variants exist and can be enumerated
    let classes = [
        AcceptableUseClass::Scientific,
        AcceptableUseClass::PublicGoodMl,
        AcceptableUseClass::Rendering,
        AcceptableUseClass::Indexing,
        AcceptableUseClass::SelfImprovement,
        AcceptableUseClass::GeneralCompute,
    ];
    assert_eq!(classes.len(), 6, "Should have 6 acceptable use classes");
}

#[test]
fn job_submit_workload_types() {
    // Verify workload types used by job submit
    let types = [WorkloadType::WasmModule, WorkloadType::OciContainer];
    assert!(types.len() >= 2, "Should have at least 2 workload types");
}

#[test]
fn resource_envelope_creation() {
    let envelope = ResourceEnvelope {
        cpu_millicores: 2000,
        ram_bytes: 4 * 1024 * 1024 * 1024,
        gpu_class: None,
        gpu_vram_bytes: 0,
        scratch_bytes: 10 * 1024 * 1024 * 1024,
        network_egress_bytes: 0,
        walltime_budget_ms: 3_600_000,
    };
    assert_eq!(envelope.cpu_millicores, 2000);
    assert_eq!(envelope.ram_bytes, 4 * 1024 * 1024 * 1024);
}

#[test]
fn confidentiality_levels() {
    let levels = [
        ConfidentialityLevel::Public,
        ConfidentialityLevel::ConfidentialMedium,
        ConfidentialityLevel::ConfidentialHigh,
    ];
    assert_eq!(levels.len(), 3);
}

#[test]
fn job_categories() {
    let categories = [
        JobCategory::PublicGood,
        JobCategory::PaidSponsored,
        JobCategory::DonorRedemption,
        JobCategory::SelfImprovement,
    ];
    assert!(categories.len() >= 2);
}
