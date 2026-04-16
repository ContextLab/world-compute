//! GovernanceRole — separation-of-duties role assignments.
//!
//! Per FR-S032: no single identity may hold both WorkloadApprover AND
//! ArtifactSigner, or ArtifactSigner AND PolicyDeployer simultaneously.
//! Per data-model.md: roles have a default expiration of 90 days (renewable).

use crate::error::{ErrorCode, WcError, WcResult};
use crate::types::{PeerIdStr, Timestamp};
use serde::{Deserialize, Serialize};

/// Default role expiration in microseconds (90 days).
const DEFAULT_EXPIRATION_US: u64 = 90 * 24 * 3600 * 1_000_000;

/// Governance role types for separation of duties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleType {
    WorkloadApprover,
    ArtifactSigner,
    PolicyDeployer,
    OnCallResponder,
    GovernanceVoter,
}

/// A separation-of-duties role assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceRole {
    pub assignment_id: String,
    pub peer_id: PeerIdStr,
    pub role: RoleType,
    pub granted_by: PeerIdStr,
    pub granted_at: Timestamp,
    /// Defaults to 90 days from grant if not specified.
    pub expires_at: Timestamp,
    pub revoked: bool,
}

impl GovernanceRole {
    /// Create a new role assignment with default 90-day expiration.
    pub fn new(
        assignment_id: String,
        peer_id: PeerIdStr,
        role: RoleType,
        granted_by: PeerIdStr,
    ) -> Self {
        let now = Timestamp::now();
        Self {
            assignment_id,
            peer_id,
            role,
            granted_by,
            granted_at: now,
            expires_at: Timestamp(now.0 + DEFAULT_EXPIRATION_US),
            revoked: false,
        }
    }

    /// Check if this role assignment is currently active.
    pub fn is_active(&self) -> bool {
        !self.revoked && Timestamp::now().0 < self.expires_at.0
    }
}

/// Prohibited role combinations per FR-S032.
const PROHIBITED_PAIRS: &[(RoleType, RoleType)] = &[
    (RoleType::WorkloadApprover, RoleType::ArtifactSigner),
    (RoleType::ArtifactSigner, RoleType::PolicyDeployer),
];

/// Check if granting a new role to a peer would violate separation of duties.
pub fn check_separation_of_duties(
    peer_id: &str,
    new_role: RoleType,
    existing_roles: &[GovernanceRole],
) -> WcResult<()> {
    let active_roles: Vec<RoleType> = existing_roles
        .iter()
        .filter(|r| r.peer_id == peer_id && r.is_active())
        .map(|r| r.role)
        .collect();

    for (role_a, role_b) in PROHIBITED_PAIRS {
        let has_a = active_roles.contains(role_a) || new_role == *role_a;
        let has_b = active_roles.contains(role_b) || new_role == *role_b;
        let existing_has_a = active_roles.contains(role_a);
        let existing_has_b = active_roles.contains(role_b);

        // Violation if the new role combined with existing roles forms a prohibited pair
        if (new_role == *role_a && existing_has_b) || (new_role == *role_b && existing_has_a) {
            return Err(WcError::new(
                ErrorCode::PermissionDenied,
                format!(
                    "Separation of duties violation: peer {peer_id} cannot hold both {role_a:?} and {role_b:?}"
                ),
            ));
        }
        // Also check if both are in existing (shouldn't happen but guard)
        if has_a && has_b && existing_has_a && existing_has_b {
            return Err(WcError::new(
                ErrorCode::PermissionDenied,
                format!(
                    "Separation of duties violation: peer {peer_id} already holds both {role_a:?} and {role_b:?}"
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_role(peer_id: &str, role: RoleType) -> GovernanceRole {
        GovernanceRole::new(format!("test-{:?}", role), peer_id.into(), role, "admin".into())
    }

    #[test]
    fn approver_plus_signer_rejected() {
        let existing = vec![make_role("peer-1", RoleType::WorkloadApprover)];
        let result = check_separation_of_duties("peer-1", RoleType::ArtifactSigner, &existing);
        assert!(result.is_err());
    }

    #[test]
    fn signer_plus_deployer_rejected() {
        let existing = vec![make_role("peer-1", RoleType::ArtifactSigner)];
        let result = check_separation_of_duties("peer-1", RoleType::PolicyDeployer, &existing);
        assert!(result.is_err());
    }

    #[test]
    fn approver_plus_deployer_allowed() {
        let existing = vec![make_role("peer-1", RoleType::WorkloadApprover)];
        let result = check_separation_of_duties("peer-1", RoleType::PolicyDeployer, &existing);
        assert!(result.is_ok());
    }

    #[test]
    fn different_peers_no_conflict() {
        let existing = vec![make_role("peer-1", RoleType::WorkloadApprover)];
        let result = check_separation_of_duties("peer-2", RoleType::ArtifactSigner, &existing);
        assert!(result.is_ok());
    }

    #[test]
    fn responder_role_no_conflicts() {
        let existing = vec![
            make_role("peer-1", RoleType::WorkloadApprover),
            make_role("peer-1", RoleType::PolicyDeployer),
        ];
        let result = check_separation_of_duties("peer-1", RoleType::OnCallResponder, &existing);
        assert!(result.is_ok());
    }

    #[test]
    fn role_has_default_expiration() {
        let role = make_role("peer-1", RoleType::OnCallResponder);
        assert!(role.is_active());
        assert!(role.expires_at.0 > role.granted_at.0);
        // Should expire approximately 90 days from now
        let diff_days = (role.expires_at.0 - role.granted_at.0) / (24 * 3600 * 1_000_000);
        assert_eq!(diff_days, 90);
    }
}
