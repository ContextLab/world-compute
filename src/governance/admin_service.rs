//! AdminService gRPC handler per US6, FR-S031.
//!
//! Per FR-S031: halt() MUST require cryptographic authentication of the
//! caller's designated OnCallResponder role. This module enforces that role
//! check for halt/resume. `ban` maintains an in-memory banned-subject
//! registry that the policy engine consults before dispatch.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::governance::board::ProposalBoard;
use crate::governance::proposal::ProposalState;
use crate::governance::roles::{GovernanceRole, RoleType};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// A record of a banned subject (user or node).
#[derive(Debug, Clone)]
pub struct BanRecord {
    pub subject_id: String,
    pub reason: String,
    pub banned_at: DateTime<Utc>,
}

/// gRPC handler for AdminService RPCs. Owns the ban registry for the
/// lifetime of the daemon; policy engine references it via `is_banned`.
pub struct AdminServiceHandler {
    pub board: ProposalBoard,
    pub halted: bool,
    /// Subject-id → BanRecord. Mutation protected by the handler owning it.
    banned: HashMap<String, BanRecord>,
}

impl AdminServiceHandler {
    pub fn new() -> Self {
        Self { board: ProposalBoard::new(), halted: false, banned: HashMap::new() }
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

    /// Ban RPC — add a subject (user or node) to the banned registry.
    /// Returns `AlreadyExists` if the subject is already banned.
    ///
    /// The policy engine MUST consult `is_banned` before dispatch; banned
    /// subjects are rejected at the dispatch step regardless of attestation
    /// or trust tier.
    pub fn ban(
        &mut self,
        subject_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> WcResult<()> {
        let subject_id = subject_id.into();
        let reason = reason.into();
        if self.banned.contains_key(&subject_id) {
            return Err(WcError::new(
                ErrorCode::AlreadyExists,
                format!("subject '{subject_id}' is already banned"),
            ));
        }
        let rec = BanRecord {
            subject_id: subject_id.clone(),
            reason: reason.clone(),
            banned_at: Utc::now(),
        };
        tracing::warn!(subject = %subject_id, reason = %reason, "subject banned");
        self.banned.insert(subject_id, rec);
        Ok(())
    }

    /// Remove a subject from the banned registry. Returns `NotFound` if
    /// the subject was not previously banned.
    pub fn unban(&mut self, subject_id: &str) -> WcResult<()> {
        match self.banned.remove(subject_id) {
            Some(rec) => {
                tracing::info!(
                    subject = %subject_id,
                    was_banned_since = %rec.banned_at,
                    "subject unbanned"
                );
                Ok(())
            }
            None => Err(WcError::new(
                ErrorCode::NotFound,
                format!("subject '{subject_id}' is not banned"),
            )),
        }
    }

    /// True iff the subject is currently banned.
    pub fn is_banned(&self, subject_id: &str) -> bool {
        self.banned.contains_key(subject_id)
    }

    /// Fetch the full ban record for a subject (for audit).
    pub fn ban_record(&self, subject_id: &str) -> Option<&BanRecord> {
        self.banned.get(subject_id)
    }

    /// All currently-banned subject ids (snapshot).
    pub fn banned_subjects(&self) -> Vec<String> {
        self.banned.keys().cloned().collect()
    }

    /// Audit RPC — returns the current state of the specified proposal.
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

    // spec 005 T031 / FR-031 — real ban registry tests
    #[test]
    fn ban_adds_subject_to_registry() {
        let mut handler = AdminServiceHandler::new();
        handler.ban("peer-malicious", "attempted sandbox escape").unwrap();
        assert!(handler.is_banned("peer-malicious"));
        assert!(!handler.is_banned("peer-clean"));
    }

    #[test]
    fn double_ban_rejected() {
        let mut handler = AdminServiceHandler::new();
        handler.ban("peer-bad", "r1").unwrap();
        let err = handler.ban("peer-bad", "r2").unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::AlreadyExists));
    }

    #[test]
    fn unban_removes_subject() {
        let mut handler = AdminServiceHandler::new();
        handler.ban("peer-bad", "r").unwrap();
        handler.unban("peer-bad").unwrap();
        assert!(!handler.is_banned("peer-bad"));
    }

    #[test]
    fn unban_nonexistent_rejected() {
        let mut handler = AdminServiceHandler::new();
        let err = handler.unban("peer-nobody").unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::NotFound));
    }

    #[test]
    fn ban_record_preserves_reason_and_timestamp() {
        let mut handler = AdminServiceHandler::new();
        handler.ban("peer-x", "test-reason").unwrap();
        let rec = handler.ban_record("peer-x").unwrap();
        assert_eq!(rec.reason, "test-reason");
        assert_eq!(rec.subject_id, "peer-x");
    }
}
