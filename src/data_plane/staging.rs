//! Job input/output staging pipeline per T071.
//!
//! Resolves input CIDs from the store, captures output bytes back into the store.

use crate::data_plane::cid_store::{compute_cid, CidStore};
use crate::error::{ErrorCode, WcError, WcResult};
use crate::scheduler::manifest::JobManifest;
use crate::types::Cid;

/// A staged job: all input CIDs verified present in the store and ready to run.
#[derive(Debug, Clone)]
pub struct StagedJob {
    /// CID of the job manifest (if set on the manifest itself).
    pub manifest_cid: Option<Cid>,
    /// Input CIDs that have been verified present in the store.
    pub input_cids: Vec<Cid>,
    /// True when all inputs are resolved and the job can be dispatched.
    pub ready: bool,
}

/// Resolve all input CIDs from the manifest and verify they exist in the store.
/// Returns a `StagedJob` with `ready = true` when all inputs are present.
/// Returns `WcError` with `NotFound` if any input CID is missing.
pub fn stage_inputs(manifest: &JobManifest, store: &CidStore) -> WcResult<StagedJob> {
    let mut missing: Vec<String> = Vec::new();

    for cid in &manifest.inputs {
        if !store.has(cid) {
            missing.push(cid.to_string());
        }
    }

    if !missing.is_empty() {
        return Err(WcError::new(
            ErrorCode::NotFound,
            format!("Missing input CIDs: {}", missing.join(", ")),
        ));
    }

    Ok(StagedJob {
        manifest_cid: manifest.manifest_cid,
        input_cids: manifest.inputs.clone(),
        ready: true,
    })
}

/// Hash `data`, store it in `store`, and return the resulting CID.
pub fn capture_output(data: &[u8], store: &CidStore) -> WcResult<Cid> {
    let cid = store.put(data)?;
    Ok(cid)
}

/// Verify that an output CID stored by `capture_output` can be retrieved and
/// its content matches the original bytes (integrity check).
pub fn verify_output(cid: &Cid, expected: &[u8], store: &CidStore) -> WcResult<bool> {
    match store.get(cid) {
        None => Err(WcError::new(ErrorCode::NotFound, format!("Output CID {cid} not in store"))),
        Some(data) => {
            let expected_cid = compute_cid(expected)?;
            Ok(data == expected && cid == &expected_cid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acceptable_use::AcceptableUseClass;
    use crate::data_plane::cid_store::compute_cid;
    use crate::scheduler::manifest::JobManifest;
    use crate::scheduler::{
        ConfidentialityLevel, JobCategory, ResourceEnvelope, VerificationMethod, WorkloadType,
    };

    fn make_store_with_inputs(payloads: &[&[u8]]) -> (CidStore, Vec<Cid>) {
        let store = CidStore::new();
        let cids: Vec<Cid> = payloads.iter().map(|d| store.put(d).unwrap()).collect();
        (store, cids)
    }

    fn base_manifest(inputs: Vec<Cid>) -> JobManifest {
        let workload_cid = compute_cid(b"test workload").unwrap();
        JobManifest {
            manifest_cid: None,
            name: "staging-test".into(),
            workload_type: WorkloadType::WasmModule,
            workload_cid,
            command: vec!["run".into()],
            inputs,
            output_sink: "cid-store".into(),
            resources: ResourceEnvelope {
                cpu_millicores: 500,
                ram_bytes: 256 * 1024 * 1024,
                gpu_class: None,
                gpu_vram_bytes: 0,
                scratch_bytes: 512 * 1024 * 1024,
                network_egress_bytes: 0,
                walltime_budget_ms: 60_000,
            },
            category: JobCategory::PublicGood,
            confidentiality: ConfidentialityLevel::Public,
            verification: VerificationMethod::ReplicatedQuorum,
            acceptable_use_classes: vec![AcceptableUseClass::Scientific],
            max_wallclock_ms: 60_000,
            submitter_signature: vec![0u8; 64],
            allowed_endpoints: Vec::new(),
            confidentiality_level: None,
        }
    }

    #[test]
    fn stage_with_valid_cids_succeeds() {
        let (store, input_cids) = make_store_with_inputs(&[b"input-a", b"input-b"]);
        let manifest = base_manifest(input_cids.clone());
        let staged = stage_inputs(&manifest, &store).unwrap();
        assert!(staged.ready);
        assert_eq!(staged.input_cids, input_cids);
    }

    #[test]
    fn stage_with_no_inputs_succeeds() {
        let store = CidStore::new();
        let manifest = base_manifest(vec![]);
        let staged = stage_inputs(&manifest, &store).unwrap();
        assert!(staged.ready);
        assert!(staged.input_cids.is_empty());
    }

    #[test]
    fn stage_with_missing_cid_fails() {
        let store = CidStore::new();
        // CID that was never stored
        let phantom_cid = compute_cid(b"phantom data").unwrap();
        let manifest = base_manifest(vec![phantom_cid]);
        let err = stage_inputs(&manifest, &store).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::NotFound));
    }

    #[test]
    fn stage_with_partial_missing_fails() {
        let (store, mut input_cids) = make_store_with_inputs(&[b"real input"]);
        let phantom_cid = compute_cid(b"not in store").unwrap();
        input_cids.push(phantom_cid);
        let manifest = base_manifest(input_cids);
        let err = stage_inputs(&manifest, &store).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::NotFound));
    }

    #[test]
    fn capture_output_produces_verifiable_cid() {
        let store = CidStore::new();
        let output_data = b"hello output world";
        let cid = capture_output(output_data, &store).unwrap();
        // CID must be present in store
        assert!(store.has(&cid));
        // CID must match independently computed CID
        let expected_cid = compute_cid(output_data).unwrap();
        assert_eq!(cid, expected_cid);
        // verify_output must confirm integrity
        assert!(verify_output(&cid, output_data, &store).unwrap());
    }

    #[test]
    fn capture_output_same_data_same_cid() {
        let store = CidStore::new();
        let data = b"deterministic content";
        let cid1 = capture_output(data, &store).unwrap();
        let cid2 = capture_output(data, &store).unwrap();
        assert_eq!(cid1, cid2);
    }
}
