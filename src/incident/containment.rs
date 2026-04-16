//! Containment action execution — implements the incident response primitives.
//!
//! Per contracts/incident.md: authorized responders (OnCallResponder role)
//! can trigger containment actions. Each action produces an IncidentRecord.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::incident::audit::IncidentRecord;
use crate::incident::ContainmentAction;
use crate::types::Timestamp;

/// Execute a containment action, returning an audit record.
///
/// Caller must verify OnCallResponder role before calling this function.
pub fn execute_containment(
    action: ContainmentAction,
    target: &str,
    actor_peer_id: &str,
    actor_role: &str,
    justification: &str,
    incident_id: &str,
) -> WcResult<IncidentRecord> {
    // Verify caller has appropriate role
    if actor_role != "OnCallResponder" {
        return Err(WcError::new(
            ErrorCode::PermissionDenied,
            format!("Containment actions require OnCallResponder role, got '{actor_role}'"),
        ));
    }

    let record_id = format!("ir-{}-{}", Timestamp::now().0, &target[..8.min(target.len())]);

    let record = IncidentRecord::new(
        record_id,
        incident_id.to_string(),
        action,
        target.to_string(),
        actor_peer_id.to_string(),
        actor_role.to_string(),
        justification.to_string(),
    );

    // TODO(Phase 7 T076-T080): Implement actual containment effects:
    // - FreezeHost: remove from scheduler's active pool
    // - QuarantineWorkloadClass: add to quarantine set checked by policy engine
    // - BlockSubmitter: add to ban list checked by policy engine
    // - RevokeArtifact: remove from approved artifact registry
    // - DrainHostPool: checkpoint + migrate running jobs

    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_role_rejected() {
        let result = execute_containment(
            ContainmentAction::FreezeHost,
            "host-123",
            "peer-abc",
            "RegularUser",
            "suspicious activity",
            "incident-001",
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    }

    #[test]
    fn authorized_action_succeeds() {
        let record = execute_containment(
            ContainmentAction::FreezeHost,
            "host-123",
            "peer-abc",
            "OnCallResponder",
            "suspicious activity",
            "incident-001",
        )
        .unwrap();
        assert_eq!(record.action_type, ContainmentAction::FreezeHost);
        assert!(record.reversible);
        assert_eq!(record.target, "host-123");
    }

    #[test]
    fn revoke_artifact_not_reversible() {
        let record = execute_containment(
            ContainmentAction::RevokeArtifact,
            "bafybeig...",
            "peer-abc",
            "OnCallResponder",
            "compromised artifact",
            "incident-002",
        )
        .unwrap();
        assert!(!record.reversible);
    }
}
