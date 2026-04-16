//! T046 [US3]: Single actor cannot hold WorkloadApprover + ArtifactSigner.

use worldcompute::governance::roles::{check_separation_of_duties, GovernanceRole, RoleType};

fn make_role(peer_id: &str, role: RoleType) -> GovernanceRole {
    GovernanceRole::new(format!("test-{role:?}"), peer_id.into(), role, "admin".into())
}

#[test]
fn approver_plus_signer_same_peer_rejected() {
    let existing = vec![make_role("peer-1", RoleType::WorkloadApprover)];
    assert!(check_separation_of_duties("peer-1", RoleType::ArtifactSigner, &existing).is_err());
}

#[test]
fn signer_plus_deployer_same_peer_rejected() {
    let existing = vec![make_role("peer-1", RoleType::ArtifactSigner)];
    assert!(check_separation_of_duties("peer-1", RoleType::PolicyDeployer, &existing).is_err());
}

#[test]
fn approver_plus_deployer_allowed() {
    let existing = vec![make_role("peer-1", RoleType::WorkloadApprover)];
    assert!(check_separation_of_duties("peer-1", RoleType::PolicyDeployer, &existing).is_ok());
}

#[test]
fn different_peers_no_conflict() {
    let existing = vec![make_role("peer-1", RoleType::WorkloadApprover)];
    assert!(check_separation_of_duties("peer-2", RoleType::ArtifactSigner, &existing).is_ok());
}

#[test]
fn oncall_responder_has_no_conflicts() {
    let existing = vec![
        make_role("peer-1", RoleType::WorkloadApprover),
        make_role("peer-1", RoleType::PolicyDeployer),
    ];
    assert!(check_separation_of_duties("peer-1", RoleType::OnCallResponder, &existing).is_ok());
}
