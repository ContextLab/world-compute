//! GovernanceProposal types and state machine per US6.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::types::Timestamp;
use serde::{Deserialize, Serialize};

/// Mandatory review period for ConstitutionAmendment proposals (7 days in microseconds).
/// Per FR-S030: ConstitutionAmendment votes cannot be tallied until this period elapses.
pub const CONSTITUTION_REVIEW_PERIOD_US: u64 = 7 * 24 * 3600 * 1_000_000;

/// Categories of governance proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProposalType {
    Compute,
    PolicyChange,
    AcceptableUseRule,
    PriorityRebalance,
    EmergencyHalt,
    ConstitutionAmendment,
}

/// Lifecycle state of a governance proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProposalState {
    Draft,
    Open,
    Passed,
    Rejected,
    Withdrawn,
    Enacted,
}

/// A governance proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceProposal {
    pub proposal_id: String,
    pub title: String,
    pub body: String,
    pub proposal_type: ProposalType,
    pub state: ProposalState,
    pub submitter_id: String,
    pub created_at: Timestamp,
    pub closes_at: Timestamp,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub abstain_votes: u64,
}

impl GovernanceProposal {
    /// Open the proposal for voting. For ConstitutionAmendment proposals,
    /// sets closes_at to enforce the mandatory 7-day review period (FR-S030).
    pub fn open_for_voting(&mut self) -> WcResult<ProposalState> {
        if self.state != ProposalState::Draft {
            return Err(WcError::new(
                ErrorCode::InvalidManifest,
                format!("cannot open proposal in state {:?}; must be Draft", self.state),
            ));
        }
        let now = Timestamp::now();
        if self.proposal_type == ProposalType::ConstitutionAmendment {
            // Enforce mandatory 7-day review period
            self.closes_at = Timestamp(now.0 + CONSTITUTION_REVIEW_PERIOD_US);
        }
        self.state = ProposalState::Open;
        Ok(self.state)
    }

    /// Check if the review period has elapsed (for time-locked proposals).
    /// Returns true if voting can be tallied, false if still in review.
    pub fn review_period_elapsed(&self) -> bool {
        if self.proposal_type == ProposalType::ConstitutionAmendment {
            Timestamp::now().0 >= self.closes_at.0
        } else {
            true // non-amendment proposals have no time-lock
        }
    }

    /// Tally votes and transition to Passed or Rejected.
    /// For ConstitutionAmendment proposals, enforces the review period.
    pub fn tally(&mut self) -> WcResult<ProposalState> {
        if self.state != ProposalState::Open {
            return Err(WcError::new(
                ErrorCode::InvalidManifest,
                format!("cannot tally proposal in state {:?}", self.state),
            ));
        }
        if !self.review_period_elapsed() {
            return Err(WcError::new(
                ErrorCode::InvalidManifest,
                "ConstitutionAmendment proposals require a 7-day review period before tallying",
            ));
        }
        if self.yes_votes > self.no_votes {
            self.state = ProposalState::Passed;
        } else {
            self.state = ProposalState::Rejected;
        }
        Ok(self.state)
    }

    /// Attempt a state transition. Returns the new state on success.
    pub fn transition(&mut self, new_state: ProposalState) -> WcResult<ProposalState> {
        let valid = matches!(
            (self.state, new_state),
            (ProposalState::Draft, ProposalState::Open)
                | (ProposalState::Draft, ProposalState::Withdrawn)
                | (ProposalState::Open, ProposalState::Passed)
                | (ProposalState::Open, ProposalState::Rejected)
                | (ProposalState::Open, ProposalState::Withdrawn)
                | (ProposalState::Passed, ProposalState::Enacted)
        );
        if valid {
            self.state = new_state;
            Ok(self.state)
        } else {
            Err(WcError::new(
                ErrorCode::InvalidManifest,
                format!("invalid transition {:?} -> {:?}", self.state, new_state),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proposal(state: ProposalState) -> GovernanceProposal {
        GovernanceProposal {
            proposal_id: "p1".into(),
            title: "Test".into(),
            body: "Body".into(),
            proposal_type: ProposalType::PolicyChange,
            state,
            submitter_id: "user1".into(),
            created_at: Timestamp::now(),
            closes_at: Timestamp::now(),
            yes_votes: 0,
            no_votes: 0,
            abstain_votes: 0,
        }
    }

    #[test]
    fn draft_to_open_valid() {
        let mut p = make_proposal(ProposalState::Draft);
        assert!(p.transition(ProposalState::Open).is_ok());
        assert_eq!(p.state, ProposalState::Open);
    }

    #[test]
    fn draft_to_withdrawn_valid() {
        let mut p = make_proposal(ProposalState::Draft);
        assert!(p.transition(ProposalState::Withdrawn).is_ok());
    }

    #[test]
    fn open_to_passed_valid() {
        let mut p = make_proposal(ProposalState::Open);
        assert!(p.transition(ProposalState::Passed).is_ok());
    }

    #[test]
    fn open_to_rejected_valid() {
        let mut p = make_proposal(ProposalState::Open);
        assert!(p.transition(ProposalState::Rejected).is_ok());
    }

    #[test]
    fn passed_to_enacted_valid() {
        let mut p = make_proposal(ProposalState::Passed);
        assert!(p.transition(ProposalState::Enacted).is_ok());
    }

    #[test]
    fn draft_to_enacted_invalid() {
        let mut p = make_proposal(ProposalState::Draft);
        let r = p.transition(ProposalState::Enacted);
        assert!(r.is_err());
        // State must be unchanged on error
        assert_eq!(p.state, ProposalState::Draft);
    }

    #[test]
    fn enacted_to_open_invalid() {
        let mut p = make_proposal(ProposalState::Enacted);
        assert!(p.transition(ProposalState::Open).is_err());
    }

    #[test]
    fn rejected_to_passed_invalid() {
        let mut p = make_proposal(ProposalState::Rejected);
        assert!(p.transition(ProposalState::Passed).is_err());
    }

    // ─── FR-S030: Time-lock tests ─────��────────────────────────────────

    #[test]
    fn constitution_amendment_gets_7_day_review_period() {
        let mut p = GovernanceProposal {
            proposal_id: "p-amend".into(),
            title: "Amend".into(),
            body: "Body".into(),
            proposal_type: ProposalType::ConstitutionAmendment,
            state: ProposalState::Draft,
            submitter_id: "alice".into(),
            created_at: Timestamp::now(),
            closes_at: Timestamp::now(),
            yes_votes: 10,
            no_votes: 0,
            abstain_votes: 0,
        };
        p.open_for_voting().unwrap();
        assert_eq!(p.state, ProposalState::Open);
        // closes_at should be ~7 days from now
        let diff_days = (p.closes_at.0 - Timestamp::now().0) / (24 * 3600 * 1_000_000);
        assert!(diff_days >= 6, "Review period should be ~7 days, got {diff_days}");
    }

    #[test]
    fn constitution_amendment_tally_blocked_during_review() {
        let mut p = GovernanceProposal {
            proposal_id: "p-amend2".into(),
            title: "Amend".into(),
            body: "Body".into(),
            proposal_type: ProposalType::ConstitutionAmendment,
            state: ProposalState::Draft,
            submitter_id: "alice".into(),
            created_at: Timestamp::now(),
            closes_at: Timestamp::now(),
            yes_votes: 10,
            no_votes: 0,
            abstain_votes: 0,
        };
        p.open_for_voting().unwrap();
        // Try to tally immediately — should fail (review period not elapsed)
        let err = p.tally().unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::InvalidManifest));
    }

    #[test]
    fn standard_proposal_tally_works_immediately() {
        let mut p = make_proposal(ProposalState::Draft);
        p.transition(ProposalState::Open).unwrap();
        p.yes_votes = 5;
        p.no_votes = 2;
        let state = p.tally().unwrap();
        assert_eq!(state, ProposalState::Passed);
    }

    #[test]
    fn tally_rejects_when_no_majority() {
        let mut p = make_proposal(ProposalState::Draft);
        p.transition(ProposalState::Open).unwrap();
        p.yes_votes = 2;
        p.no_votes = 5;
        let state = p.tally().unwrap();
        assert_eq!(state, ProposalState::Rejected);
    }
}
