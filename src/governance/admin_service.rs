//! AdminService gRPC stub handler per US6, FR-S031.
//!
//! Per FR-S031: halt() MUST require cryptographic authentication of the
//! caller's designated OnCallResponder role.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::governance::board::ProposalBoard;
use crate::governance::proposal::ProposalState;
use crate::governance::roles::{GovernanceRole, RoleType};

/// Stub gRPC handler for AdminService RPCs.
pub struct AdminServiceHandler {
    pub board: ProposalBoard,
    pub halted: bool,
}

impl AdminServiceHandler {
    pub fn new() -> Self {
        Self { board: ProposalBoard::new(), halted: false }
    }

    /// Halt RPC — sets cluster halt flag.
    ///
    /// Per FR-S031: requires the caller to have an active OnCallResponder role.
    /// Rejects unauthorized callers with PermissionDenied.
    pub fn halt(
        &mut self,
        reason: impl Into<String>,
        caller_peer_id: &str,
        caller_roles: &[GovernanceRole],
    ) -> WcResult<()> {
        // Verify caller has OnCallResponder role
        let has_responder_role = caller_roles.iter().any(|r| {
            r.peer_id == caller_peer_id && r.role == RoleType::OnCallResponder && r.is_active()
        });

        if !has_responder_role {
            return Err(WcError::new(
                ErrorCode::PermissionDenied,
                format!(
                    "halt() requires active OnCallResponder role — caller '{caller_peer_id}' is not authorized"
                ),
            ));
        }

        let reason_str = reason.into();
        tracing::warn!(
            caller = caller_peer_id,
            reason = %reason_str,
            "EMERGENCY HALT activated"
        );
        self.halted = true;
        Ok(())
    }

    /// Resume RPC — clears cluster halt flag.
    ///
    /// Also requires OnCallResponder role.
    pub fn resume(
        &mut self,
        caller_peer_id: &str,
        caller_roles: &[GovernanceRole],
    ) -> WcResult<()> {
        let has_responder_role = caller_roles.iter().any(|r| {
            r.peer_id == caller_peer_id && r.role == RoleType::OnCallResponder && r.is_active()
        });

        if !has_responder_role {
            return Err(WcError::new(
                ErrorCode::PermissionDenied,
                format!(
                    "resume() requires active OnCallResponder role — caller '{caller_peer_id}' is not authorized"
                ),
            ));
        }

        self.halted = false;
        Ok(())
    }

    /// Ban RPC stub — placeholder; real impl would update trust registry.
    pub fn ban(
        &mut self,
        _subject_id: impl Into<String>,
        _reason: impl Into<String>,
    ) -> WcResult<()> {
        Ok(())
    }

    /// Audit RPC stub — returns proposal state for the given ID.
    pub fn audit_proposal(&self, proposal_id: &str) -> Option<ProposalState> {
        self.board.get_proposal(proposal_id).map(|p| p.state)
    }
}

impl Default for AdminServiceHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::roles::GovernanceRole;

    fn make_responder_role(peer_id: &str) -> GovernanceRole {
        GovernanceRole::new(
            "role-test".into(),
            peer_id.into(),
            RoleType::OnCallResponder,
            "admin".into(),
        )
    }

    #[test]
    fn authorized_halt_succeeds() {
        let mut handler = AdminServiceHandler::new();
        let roles = vec![make_responder_role("peer-oncall")];
        assert!(handler.halt("test emergency", "peer-oncall", &roles).is_ok());
        assert!(handler.halted);
    }

    #[test]
    fn unauthorized_halt_rejected() {
        let mut handler = AdminServiceHandler::new();
        let roles = vec![make_responder_role("peer-oncall")];
        // Different peer trying to halt
        let err = handler.halt("test emergency", "peer-random", &roles).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
        assert!(!handler.halted);
    }

    #[test]
    fn halt_with_no_roles_rejected() {
        let mut handler = AdminServiceHandler::new();
        let roles: Vec<GovernanceRole> = vec![];
        let err = handler.halt("test", "peer-1", &roles).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    }

    #[test]
    fn authorized_resume_succeeds() {
        let mut handler = AdminServiceHandler::new();
        let roles = vec![make_responder_role("peer-oncall")];
        handler.halt("emergency", "peer-oncall", &roles).unwrap();
        assert!(handler.halted);
        handler.resume("peer-oncall", &roles).unwrap();
        assert!(!handler.halted);
    }

    #[test]
    fn unauthorized_resume_rejected() {
        let mut handler = AdminServiceHandler::new();
        let roles = vec![make_responder_role("peer-oncall")];
        handler.halt("emergency", "peer-oncall", &roles).unwrap();
        let err = handler.resume("peer-random", &roles).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
        assert!(handler.halted); // still halted
    }
}
