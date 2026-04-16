//! GovernanceProposal types and state machine per US6.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::types::Timestamp;
use serde::{Deserialize, Serialize};

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
}
