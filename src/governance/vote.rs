//! Vote types and validation per US6 / FR-059.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::governance::proposal::{GovernanceProposal, ProposalState, ProposalType};
use crate::types::Timestamp;
use serde::{Deserialize, Serialize};

/// Minimum Humanity Points score required to vote on safety-critical proposals.
/// Standard proposals require HP >= 1 (checked by policy engine).
/// Safety-critical proposals (EmergencyHalt, ConstitutionAmendment) require
/// this elevated threshold per FR-S030.
pub const SAFETY_CRITICAL_MIN_HP: u32 = 5;

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

/// Check if a proposal type is safety-critical per FR-S030.
pub fn is_safety_critical(proposal_type: ProposalType) -> bool {
    matches!(proposal_type, ProposalType::EmergencyHalt | ProposalType::ConstitutionAmendment)
}

/// Validate a vote against a proposal.
///
/// Checks:
/// - proposal is in `Open` state
/// - voter is not the proposal submitter (FR-059)
/// - for safety-critical proposals: voter HP meets elevated threshold (FR-S030)
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

/// Validate a vote with HP check for safety-critical proposals (FR-S030).
///
/// Extends `validate_vote` with:
/// - safety-critical proposals require voter HP >= SAFETY_CRITICAL_MIN_HP
pub fn validate_vote_with_hp(
    vote: &Vote,
    proposal: &GovernanceProposal,
    voter_hp: u32,
) -> WcResult<()> {
    // Run standard checks first
    validate_vote(vote, proposal)?;

    // Safety-critical proposals require elevated HP
    if is_safety_critical(proposal.proposal_type) && voter_hp < SAFETY_CRITICAL_MIN_HP {
        return Err(WcError::new(
            ErrorCode::PermissionDenied,
            format!(
                "Safety-critical proposals require HP >= {SAFETY_CRITICAL_MIN_HP}, voter has {voter_hp}"
            ),
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

    // ─── FR-S030: Safety-critical quorum tests ─────────────────────────

    fn make_safety_proposal(ptype: ProposalType) -> GovernanceProposal {
        GovernanceProposal {
            proposal_id: "p-safety".into(),
            title: "Safety".into(),
            body: "Body".into(),
            proposal_type: ptype,
            state: ProposalState::Open,
            submitter_id: "alice".into(),
            created_at: Timestamp::now(),
            closes_at: Timestamp::now(),
            yes_votes: 0,
            no_votes: 0,
            abstain_votes: 0,
        }
    }

    #[test]
    fn emergency_halt_requires_elevated_hp() {
        let proposal = make_safety_proposal(ProposalType::EmergencyHalt);
        let vote = make_vote("bob", VoteChoice::Yes);
        // HP = 3, below threshold of 5
        let err = validate_vote_with_hp(&vote, &proposal, 3).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    }

    #[test]
    fn emergency_halt_accepts_high_hp() {
        let proposal = make_safety_proposal(ProposalType::EmergencyHalt);
        let vote = make_vote("bob", VoteChoice::Yes);
        assert!(validate_vote_with_hp(&vote, &proposal, 10).is_ok());
    }

    #[test]
    fn constitution_amendment_requires_elevated_hp() {
        let proposal = make_safety_proposal(ProposalType::ConstitutionAmendment);
        let vote = make_vote("bob", VoteChoice::No);
        let err = validate_vote_with_hp(&vote, &proposal, 4).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    }

    #[test]
    fn standard_proposal_accepts_low_hp() {
        let proposal = make_proposal(ProposalState::Open);
        let vote = make_vote("bob", VoteChoice::Yes);
        // HP = 1, which is fine for non-safety proposals
        assert!(validate_vote_with_hp(&vote, &proposal, 1).is_ok());
    }

    #[test]
    fn is_safety_critical_classification() {
        assert!(is_safety_critical(ProposalType::EmergencyHalt));
        assert!(is_safety_critical(ProposalType::ConstitutionAmendment));
        assert!(!is_safety_critical(ProposalType::Compute));
        assert!(!is_safety_critical(ProposalType::PolicyChange));
    }
}
