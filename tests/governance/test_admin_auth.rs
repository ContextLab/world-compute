//! T049 [US3]: Unauthorized halt() call is rejected.

use worldcompute::error::ErrorCode;
use worldcompute::governance::admin_service::AdminServiceHandler;
use worldcompute::governance::roles::{GovernanceRole, RoleType};

fn responder_role(peer_id: &str) -> GovernanceRole {
    GovernanceRole::new(
        "role-resp".into(),
        peer_id.into(),
        RoleType::OnCallResponder,
        "admin".into(),
    )
}

#[test]
fn authorized_halt_succeeds() {
    let mut handler = AdminServiceHandler::new();
    let roles = vec![responder_role("peer-oncall")];
    assert!(handler.halt("emergency", "peer-oncall", &roles).is_ok());
    assert!(handler.halted);
}

#[test]
fn unauthorized_halt_rejected() {
    let mut handler = AdminServiceHandler::new();
    let roles = vec![responder_role("peer-oncall")];
    let err = handler.halt("emergency", "peer-random", &roles).unwrap_err();
    assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    assert!(!handler.halted);
}

#[test]
fn halt_with_no_roles_rejected() {
    let mut handler = AdminServiceHandler::new();
    let err = handler.halt("emergency", "peer-1", &[]).unwrap_err();
    assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
}

#[test]
fn resume_requires_auth_too() {
    let mut handler = AdminServiceHandler::new();
    let roles = vec![responder_role("peer-oncall")];
    handler.halt("emergency", "peer-oncall", &roles).unwrap();
    let err = handler.resume("peer-random", &roles).unwrap_err();
    assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    assert!(handler.halted);
}
