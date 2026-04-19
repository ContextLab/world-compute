//! Acceptable-use filter per FR-080, FR-081.

use crate::acceptable_use::AcceptableUseClass;
use crate::error::{ErrorCode, WcError};
use crate::scheduler::manifest::JobManifest;

/// Categories of workloads that are rejected outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RejectedCategory {
    UnauthorizedScanning,
    MalwareDistribution,
    IllegalContent,
    TargetedSurveillance,
    CredentialCracking,
}

/// The blocklist: which `AcceptableUseClass` values map to rejected categories.
///
/// Currently no standard `AcceptableUseClass` maps to a rejected category —
/// these are workloads that would need to self-declare a prohibited class
/// (future extension) or that arrive via a known-bad class marker.
/// The function provides the policy enforcement point; as the enum grows,
/// new entries are added here without touching call sites.
fn blocked_classes() -> &'static [(AcceptableUseClass, RejectedCategory)] {
    // No current AcceptableUseClass values map to blocked categories.
    // Extension point: add entries as new classes are introduced.
    &[]
}

/// Check that a job manifest's acceptable-use classes are all permitted.
///
/// Returns `Ok(())` if the job is allowed, or an `AcceptableUseViolation`
/// error identifying the first blocked category found.
pub fn check_acceptable_use(manifest: &JobManifest) -> Result<(), WcError> {
    for class in &manifest.acceptable_use_classes {
        for (blocked_class, category) in blocked_classes() {
            if class == blocked_class {
                return Err(WcError::new(
                    ErrorCode::AcceptableUseViolation,
                    format!("Workload class is prohibited: {category:?}"),
                ));
            }
        }
    }
    Ok(())
}

/// Extended check that accepts an explicit list of rejected categories to
/// block. Useful for callers that perform dynamic policy lookups.
pub fn check_acceptable_use_with_policy(
    manifest: &JobManifest,
    blocked: &[(AcceptableUseClass, RejectedCategory)],
) -> Result<(), WcError> {
    for class in &manifest.acceptable_use_classes {
        for (blocked_class, category) in blocked {
            if class == blocked_class {
                return Err(WcError::new(
                    ErrorCode::AcceptableUseViolation,
                    format!("Workload class is prohibited: {category:?}"),
                ));
            }
        }
    }
    Ok(())
}

/// Banned keyword lists for workload classification.
const BANNED_KEYWORDS: &[(&str, RejectedCategory)] = &[
    ("port scan", RejectedCategory::UnauthorizedScanning),
    ("nmap", RejectedCategory::UnauthorizedScanning),
    ("vulnerability scan", RejectedCategory::UnauthorizedScanning),
    ("malware", RejectedCategory::MalwareDistribution),
    ("ransomware", RejectedCategory::MalwareDistribution),
    ("trojan", RejectedCategory::MalwareDistribution),
    ("exploit kit", RejectedCategory::MalwareDistribution),
    ("child exploitation", RejectedCategory::IllegalContent),
    ("csam", RejectedCategory::IllegalContent),
    ("surveillance", RejectedCategory::TargetedSurveillance),
    ("spyware", RejectedCategory::TargetedSurveillance),
    ("keylogger", RejectedCategory::TargetedSurveillance),
    ("credential stuffing", RejectedCategory::CredentialCracking),
    ("brute force password", RejectedCategory::CredentialCracking),
    ("password cracking", RejectedCategory::CredentialCracking),
];

/// Classify a workload description by scanning for prohibited keywords.
///
/// Returns `Ok(())` if the description is clean, or an error identifying
/// the rejected category if a banned keyword is found.
pub fn classify_workload(description: &str) -> Result<(), (RejectedCategory, String)> {
    let lower = description.to_ascii_lowercase();
    for (keyword, category) in BANNED_KEYWORDS {
        if lower.contains(keyword) {
            return Err((*category, format!("Prohibited keyword detected: '{keyword}'")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::cid_store::compute_cid;
    use crate::scheduler::{
        ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
    };

    fn base_manifest(classes: Vec<AcceptableUseClass>) -> JobManifest {
        let cid = compute_cid(b"test workload").unwrap();
        JobManifest {
            manifest_cid: None,
            name: "filter-test".into(),
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
    fn scientific_job_passes() {
        let manifest = base_manifest(vec![AcceptableUseClass::Scientific]);
        assert!(check_acceptable_use(&manifest).is_ok());
    }

    #[test]
    fn public_good_ml_passes() {
        let manifest = base_manifest(vec![AcceptableUseClass::PublicGoodMl]);
        assert!(check_acceptable_use(&manifest).is_ok());
    }

    #[test]
    fn surveillance_job_rejected_via_policy() {
        // Use the with_policy variant to simulate a future blocked class.
        // We map GeneralCompute → TargetedSurveillance as a stand-in for
        // a hypothetical "Surveillance" class not yet in the enum.
        let manifest = base_manifest(vec![AcceptableUseClass::GeneralCompute]);
        let policy =
            vec![(AcceptableUseClass::GeneralCompute, RejectedCategory::TargetedSurveillance)];
        let result = check_acceptable_use_with_policy(&manifest, &policy);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::AcceptableUseViolation));
    }

    #[test]
    fn malware_class_rejected_via_policy() {
        let manifest = base_manifest(vec![AcceptableUseClass::SelfImprovement]);
        let policy =
            vec![(AcceptableUseClass::SelfImprovement, RejectedCategory::MalwareDistribution)];
        let result = check_acceptable_use_with_policy(&manifest, &policy);
        assert!(result.is_err());
    }

    #[test]
    fn empty_classes_passes() {
        let manifest = base_manifest(vec![]);
        assert!(check_acceptable_use(&manifest).is_ok());
    }
}
