//! Vote types and validation per US6 / FR-059.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::governance::proposal::{GovernanceProposal, ProposalState};
use crate::types::Timestamp;
use serde::{Deserialize, Serialize};

/// A voter's choice on a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

/// A recorded vote on a governance proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub vote_id: String,
    pub proposal_id: String,
    pub voter_id: String,
    pub choice: VoteChoice,
    /// Quadratic vote weight (HP-scaled).
    pub weight: u32,
    /// Ed25519 signature over (proposal_id || voter_id || choice).
    pub signature: Vec<u8>,
    pub cast_at: Timestamp,
}

/// Validate a vote against a proposal.
///
/// Checks:
/// - proposal is in `Open` state
/// - voter is not the proposal submitter (FR-059)
pub fn validate_vote(vote: &Vote, proposal: &GovernanceProposal) -> WcResult<()> {
    if proposal.state != ProposalState::Open {
        return Err(WcError::new(
            ErrorCode::InvalidManifest,
            format!("cannot vote on proposal in state {:?}; must be Open", proposal.state),
        ));
    }
    if vote.voter_id == proposal.submitter_id {
        return Err(WcError::new(
            ErrorCode::PermissionDenied,
            "submitter may not vote on their own proposal (FR-059)",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::proposal::{GovernanceProposal, ProposalState, ProposalType};

    fn make_proposal(state: ProposalState) -> GovernanceProposal {
        GovernanceProposal {
            proposal_id: "p1".into(),
            title: "Test".into(),
            body: "Body".into(),
            proposal_type: ProposalType::PolicyChange,
            state,
            submitter_id: "alice".into(),
            created_at: Timestamp::now(),
            closes_at: Timestamp::now(),
            yes_votes: 0,
            no_votes: 0,
            abstain_votes: 0,
        }
    }

    fn make_vote(voter_id: &str, choice: VoteChoice) -> Vote {
        Vote {
            vote_id: "v1".into(),
            proposal_id: "p1".into(),
            voter_id: voter_id.into(),
            choice,
            weight: 1,
            signature: vec![],
            cast_at: Timestamp::now(),
        }
    }

    #[test]
    fn valid_vote_accepted() {
        let proposal = make_proposal(ProposalState::Open);
        let vote = make_vote("bob", VoteChoice::Yes);
        assert!(validate_vote(&vote, &proposal).is_ok());
    }

    #[test]
    fn self_voting_rejected() {
        let proposal = make_proposal(ProposalState::Open);
        let vote = make_vote("alice", VoteChoice::Yes); // alice is submitter
        let err = validate_vote(&vote, &proposal).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    }

    #[test]
    fn voting_on_closed_proposal_rejected() {
        let proposal = make_proposal(ProposalState::Passed);
        let vote = make_vote("bob", VoteChoice::No);
        let err = validate_vote(&vote, &proposal).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::InvalidManifest));
    }

    #[test]
    fn voting_on_draft_proposal_rejected() {
        let proposal = make_proposal(ProposalState::Draft);
        let vote = make_vote("bob", VoteChoice::Abstain);
        assert!(validate_vote(&vote, &proposal).is_err());
    }
}
