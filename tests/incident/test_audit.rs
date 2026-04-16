//! T074 [US5]: Containment action produces complete IncidentRecord.

use worldcompute::incident::containment::execute_containment;
use worldcompute::incident::ContainmentAction;

#[test]
fn containment_produces_complete_record() {
    let record = execute_containment(
        ContainmentAction::FreezeHost, "host-123", "peer-oncall",
        "OnCallResponder", "anomaly detected", "incident-001",
    ).unwrap();

    assert_eq!(record.action_type, ContainmentAction::FreezeHost);
    assert_eq!(record.target, "host-123");
    assert_eq!(record.actor_peer_id, "peer-oncall");
    assert_eq!(record.actor_role, "OnCallResponder");
    assert_eq!(record.justification, "anomaly detected");
    assert!(record.reversible);
    assert!(record.reversed_by.is_none());
    assert!(!record.record_id.is_empty());
    assert!(!record.incident_id.is_empty());
}

#[test]
fn revoke_artifact_not_reversible() {
    let record = execute_containment(
        ContainmentAction::RevokeArtifact, "cid-abc", "peer-oncall",
        "OnCallResponder", "compromised", "incident-002",
    ).unwrap();
    assert!(!record.reversible);
}
