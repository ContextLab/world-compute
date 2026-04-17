//! Red Team Scenario 5: Supply-chain injection.
//!
//! Attack: Register a malicious artifact, bypass signer/approver separation,
//! promote directly from dev to production, inject forged provenance.

use worldcompute::registry::transparency::{build_metadata, ReleaseChannel};
use worldcompute::registry::{ApprovedArtifact, ArtifactRegistry};
use worldcompute::types::Timestamp;

#[test]
fn attack_5a_same_signer_and_approver_rejected() {
    let registry = ArtifactRegistry::new();
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"malicious artifact").unwrap();
    let artifact = ApprovedArtifact {
        artifact_cid: cid,
        workload_class: "scientific-batch".into(),
        signer_peer_id: "attacker".into(),
        approved_by: "attacker".into(), // same as signer — violation
        approved_at: Timestamp::now(),
        revoked: false,
        revoked_at: None,
        transparency_log_entry: None,
    };
    let result = registry.register(artifact);
    assert!(result.is_err(), "Same signer and approver must be rejected (FR-S032)");
}

#[test]
fn attack_5b_revoked_artifact_not_discoverable() {
    let registry = ArtifactRegistry::new();
    let cid = worldcompute::data_plane::cid_store::compute_cid(b"compromised artifact").unwrap();
    let artifact = ApprovedArtifact {
        artifact_cid: cid,
        workload_class: "scientific-batch".into(),
        signer_peer_id: "signer-a".into(),
        approved_by: "approver-b".into(),
        approved_at: Timestamp::now(),
        revoked: false,
        revoked_at: None,
        transparency_log_entry: None,
    };
    registry.register(artifact).unwrap();
    assert!(registry.lookup(&cid).is_some());

    // Revoke the compromised artifact
    registry.revoke(&cid).unwrap();
    assert!(registry.lookup(&cid).is_none(), "Revoked artifact must not be discoverable");
}

#[test]
fn attack_5c_dev_to_production_promotion_blocked() {
    assert!(
        !ReleaseChannel::Development.can_promote_to(ReleaseChannel::Production),
        "Direct dev→production promotion must be blocked (FR-S053)"
    );
}

#[test]
fn attack_5d_only_sequential_promotion_allowed() {
    assert!(ReleaseChannel::Development.can_promote_to(ReleaseChannel::Staging));
    assert!(ReleaseChannel::Staging.can_promote_to(ReleaseChannel::Production));
    assert!(!ReleaseChannel::Development.can_promote_to(ReleaseChannel::Production));
    assert!(!ReleaseChannel::Production.can_promote_to(ReleaseChannel::Development));
    assert!(!ReleaseChannel::Production.can_promote_to(ReleaseChannel::Staging));
}

#[test]
fn attack_5e_build_metadata_is_embedded() {
    let meta = build_metadata();
    assert!(!meta.version.is_empty(), "Build must embed version for provenance");
    // git commit and timestamp are set at build time
}
