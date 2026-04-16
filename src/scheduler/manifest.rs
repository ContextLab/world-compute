//! Job manifest parsing and validation per FR-020 (T052-T054).

use crate::acceptable_use::AcceptableUseClass;
use crate::error::{ErrorCode, WcError};
use crate::scheduler::{
    ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
};
use crate::types::Cid;
use serde::{Deserialize, Serialize};

/// A job manifest — the immutable, signed, declarative specification of work.
/// Per data-model §3.5.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobManifest {
    /// CID of this manifest (computed from its canonical serialization).
    pub manifest_cid: Option<Cid>,
    /// Human-readable name for the job.
    pub name: String,
    /// Workload artifact type.
    pub workload_type: WorkloadType,
    /// CID of the workload artifact (OCI image or WASM module).
    pub workload_cid: Cid,
    /// Command / entrypoint to run inside the workload.
    pub command: Vec<String>,
    /// Input data CIDs.
    pub inputs: Vec<Cid>,
    /// Output sink specification.
    pub output_sink: String,
    /// Resource requirements.
    pub resources: ResourceEnvelope,
    /// Job category (for accounting, not rigid scheduling).
    pub category: JobCategory,
    /// Confidentiality level.
    pub confidentiality: ConfidentialityLevel,
    /// Verification method.
    pub verification: VerificationMethod,
    /// Acceptable-use classes this job falls under.
    pub acceptable_use_classes: Vec<AcceptableUseClass>,
    /// Maximum wallclock time in milliseconds.
    pub max_wallclock_ms: u64,
    /// Submitter's signature over the canonical manifest bytes.
    pub submitter_signature: Vec<u8>,
}

/// Workflow template — a DAG of task templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub tasks: Vec<TaskTemplate>,
    /// Dependency edges: (from_index, to_index).
    pub edges: Vec<(usize, usize)>,
}

/// A single task template within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub name: String,
    pub workload_cid: Cid,
    pub command: Vec<String>,
    pub inputs: Vec<Cid>,
    pub resources: ResourceEnvelope,
}

/// Validate a job manifest. Returns Ok(()) or an error describing what's wrong.
pub fn validate_manifest(manifest: &JobManifest) -> Result<(), WcError> {
    // Check workload CID is non-empty
    if manifest.workload_cid.to_string().is_empty() {
        return Err(WcError::new(ErrorCode::InvalidManifest, "Workload CID is empty"));
    }

    // Check command is non-empty
    if manifest.command.is_empty() {
        return Err(WcError::new(ErrorCode::InvalidManifest, "Command is empty"));
    }

    // Check wallclock is reasonable (1s to 7 days)
    if manifest.max_wallclock_ms < 1_000 || manifest.max_wallclock_ms > 7 * 24 * 3600 * 1000 {
        return Err(WcError::new(
            ErrorCode::InvalidManifest,
            format!("Wallclock {} ms out of range (1s to 7 days)", manifest.max_wallclock_ms),
        ));
    }

    // Check submitter signature is present and non-trivial (FR-S012).
    // All-zero signatures are rejected. Full Ed25519 verification is done
    // by the policy engine; this is the structural gate.
    if manifest.submitter_signature.is_empty() {
        return Err(WcError::new(
            ErrorCode::InvalidManifest,
            "Submitter signature is empty",
        ));
    }
    if manifest.submitter_signature.iter().all(|&b| b == 0) {
        return Err(WcError::new(
            ErrorCode::InvalidManifest,
            "Submitter signature is all zeros — rejected per FR-S012",
        ));
    }

    // Check confidential jobs require appropriate verification
    if manifest.confidentiality == ConfidentialityLevel::ConfidentialHigh
        && !matches!(manifest.verification, VerificationMethod::TeeAttested)
    {
        return Err(WcError::new(
            ErrorCode::TrustTierMismatch,
            "ConfidentialHigh jobs must use TeeAttested verification",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::cid_store::compute_cid;

    fn test_manifest() -> JobManifest {
        let cid = compute_cid(b"test workload image").unwrap();
        JobManifest {
            manifest_cid: None,
            name: "test-job".into(),
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
            acceptable_use_classes: vec![AcceptableUseClass::Scientific],
            max_wallclock_ms: 3_600_000,
            submitter_signature: vec![1u8; 64],
        }
    }

    #[test]
    fn valid_manifest_passes() {
        assert!(validate_manifest(&test_manifest()).is_ok());
    }

    #[test]
    fn zero_signature_rejected() {
        let mut m = test_manifest();
        m.submitter_signature = vec![0u8; 64];
        assert!(validate_manifest(&m).is_err());
    }

    #[test]
    fn empty_signature_rejected() {
        let mut m = test_manifest();
        m.submitter_signature = Vec::new();
        assert!(validate_manifest(&m).is_err());
    }

    #[test]
    fn empty_command_rejected() {
        let mut m = test_manifest();
        m.command = Vec::new();
        assert!(validate_manifest(&m).is_err());
    }

    #[test]
    fn excessive_wallclock_rejected() {
        let mut m = test_manifest();
        m.max_wallclock_ms = 30 * 24 * 3600 * 1000; // 30 days
        assert!(validate_manifest(&m).is_err());
    }

    #[test]
    fn confidential_high_requires_tee() {
        let mut m = test_manifest();
        m.confidentiality = ConfidentialityLevel::ConfidentialHigh;
        m.verification = VerificationMethod::ReplicatedQuorum;
        assert!(validate_manifest(&m).is_err());

        m.verification = VerificationMethod::TeeAttested;
        assert!(validate_manifest(&m).is_ok());
    }
}
