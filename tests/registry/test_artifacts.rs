//! Integration tests for artifact registry (T108).

use worldcompute::data_plane::cid_store::compute_cid;
use worldcompute::registry::{ApprovedArtifact, ArtifactRegistry};
use worldcompute::types::Timestamp;

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
fn approved_cid_accepted() {
    let registry = ArtifactRegistry::new();
    let artifact = test_artifact();
    let cid = artifact.artifact_cid;
    registry.register(artifact).unwrap();
    assert!(registry.lookup(&cid).is_some(), "Approved CID should be found");
}

#[test]
fn unknown_cid_rejected() {
    let registry = ArtifactRegistry::new();
    let unknown_cid = compute_cid(b"unknown artifact").unwrap();
    assert!(registry.lookup(&unknown_cid).is_none(), "Unknown CID should not be found");
}

#[test]
fn separation_of_duties_enforced() {
    let mut artifact = test_artifact();
    artifact.approved_by = artifact.signer_peer_id.clone(); // same identity
    let registry = ArtifactRegistry::new();
    let result = registry.register(artifact);
    assert!(result.is_err(), "Same signer and approver should be rejected");
}

#[test]
fn revoked_artifact_not_found() {
    let registry = ArtifactRegistry::new();
    let artifact = test_artifact();
    let cid = artifact.artifact_cid;
    registry.register(artifact).unwrap();
    registry.revoke(&cid).unwrap();
    assert!(registry.lookup(&cid).is_none(), "Revoked artifact should not be found");
}
