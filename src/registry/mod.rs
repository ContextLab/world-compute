//! Approved artifact registry — CID-based lookup for signed workload artifacts.
//!
//! Per FR-S013: workload artifact CIDs MUST be checked against this registry
//! before dispatch. Unsigned or unregistered artifacts are rejected.

pub mod transparency;

use crate::types::{Cid, PeerIdStr, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A workload artifact that has passed review and is registered for dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedArtifact {
    /// Content-addressed identifier (primary key).
    pub artifact_cid: Cid,
    /// Category of workload.
    pub workload_class: String,
    /// Identity of the artifact signer.
    pub signer_peer_id: PeerIdStr,
    /// Identity of the approver (must differ from signer per FR-S032).
    pub approved_by: PeerIdStr,
    /// When the artifact was approved.
    pub approved_at: Timestamp,
    /// Whether the artifact has been revoked.
    pub revoked: bool,
    /// When revoked, if applicable.
    pub revoked_at: Option<Timestamp>,
    /// Sigstore/Rekor log index, if available.
    pub transparency_log_entry: Option<String>,
}

/// Thread-safe in-memory artifact registry.
#[derive(Debug, Clone)]
pub struct ArtifactRegistry {
    artifacts: Arc<RwLock<HashMap<String, ApprovedArtifact>>>,
}

impl ArtifactRegistry {
    pub fn new() -> Self {
        Self {
            artifacts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new approved artifact.
    pub fn register(&self, artifact: ApprovedArtifact) -> Result<(), String> {
        if artifact.signer_peer_id == artifact.approved_by {
            return Err("Signer and approver must be different identities (FR-S032)".into());
        }
        let key = artifact.artifact_cid.to_string();
        let mut map = self.artifacts.write().map_err(|e| e.to_string())?;
        map.insert(key, artifact);
        Ok(())
    }

    /// Look up an artifact by CID. Returns None if not found or revoked.
    pub fn lookup(&self, cid: &Cid) -> Option<ApprovedArtifact> {
        let map = self.artifacts.read().ok()?;
        let artifact = map.get(&cid.to_string())?;
        if artifact.revoked {
            None
        } else {
            Some(artifact.clone())
        }
    }

    /// Revoke an artifact by CID.
    pub fn revoke(&self, cid: &Cid) -> Result<(), String> {
        let mut map = self.artifacts.write().map_err(|e| e.to_string())?;
        let key = cid.to_string();
        if let Some(artifact) = map.get_mut(&key) {
            artifact.revoked = true;
            artifact.revoked_at = Some(Timestamp::now());
            Ok(())
        } else {
            Err("Artifact not found in registry".into())
        }
    }
}

impl Default for ArtifactRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::cid_store::compute_cid;

    fn test_artifact() -> ApprovedArtifact {
        let cid = compute_cid(b"test workload artifact").unwrap();
        ApprovedArtifact {
            artifact_cid: cid,
            workload_class: "scientific-batch".into(),
            signer_peer_id: "signer-peer-id".into(),
            approved_by: "approver-peer-id".into(),
            approved_at: Timestamp::now(),
            revoked: false,
            revoked_at: None,
            transparency_log_entry: None,
        }
    }

    #[test]
    fn register_and_lookup() {
        let registry = ArtifactRegistry::new();
        let artifact = test_artifact();
        let cid = artifact.artifact_cid;
        registry.register(artifact).unwrap();
        assert!(registry.lookup(&cid).is_some());
    }

    #[test]
    fn revoked_artifact_not_found() {
        let registry = ArtifactRegistry::new();
        let artifact = test_artifact();
        let cid = artifact.artifact_cid;
        registry.register(artifact).unwrap();
        registry.revoke(&cid).unwrap();
        assert!(registry.lookup(&cid).is_none());
    }

    #[test]
    fn same_signer_and_approver_rejected() {
        let mut artifact = test_artifact();
        artifact.approved_by = artifact.signer_peer_id.clone();
        let registry = ArtifactRegistry::new();
        assert!(registry.register(artifact).is_err());
    }
}
