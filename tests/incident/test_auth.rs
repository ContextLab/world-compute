//! T075 [US5]: Unauthorized containment action rejected.

use worldcompute::error::ErrorCode;
use worldcompute::incident::containment::execute_containment;
use worldcompute::incident::ContainmentAction;

#[test]
fn unauthorized_containment_rejected() {
    let result = execute_containment(
        ContainmentAction::FreezeHost,
        "host-123",
        "peer-random",
        "RegularUser",
        "suspicious",
        "incident-001",
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), Some(ErrorCode::PermissionDenied));
}

#[test]
fn authorized_containment_succeeds() {
    let result = execute_containment(
        ContainmentAction::QuarantineWorkloadClass,
        "MlTraining",
        "peer-oncall",
        "OnCallResponder",
        "vulnerability found",
        "incident-003",
    );
    assert!(result.is_ok());
}
