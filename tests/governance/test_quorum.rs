//! T047 [US3]: EmergencyHalt requires elevated quorum threshold.

use worldcompute::error::ErrorCode;
use worldcompute::governance::proposal::{GovernanceProposal, ProposalState, ProposalType};
use worldcompute::governance::vote::{
    validate_vote_with_hp, Vote, VoteChoice, SAFETY_CRITICAL_MIN_HP,
};
use worldcompute::types::Timestamp;

fn make_emergency_proposal() -> GovernanceProposal {
    GovernanceProposal {
        proposal_id: "p-halt".into(),
        title: "Emergency Halt".into(),
        body: "Critical issue".into(),
        proposal_type: ProposalType::EmergencyHalt,
        state: ProposalState::Open,
        submitter_id: "alice".into(),
        created_at: Timestamp::now(),
        closes_at: Timestamp::now(),
        yes_votes: 0,
        no_votes: 0,
        abstain_votes: 0,
    }
}

fn make_vote(voter: &str) -> Vote {
    Vote {
        vote_id: "v1".into(),
        proposal_id: "p-halt".into(),
        voter_id: voter.into(),
        choice: VoteChoice::Yes,
        weight: 1,
        signature: vec![],
        cast_at: Timestamp::now(),
    }
}

#[test]
fn low_hp_voter_rejected_for_emergency_halt() {
    let proposal = make_emergency_proposal();
    let vote = make_vote("bob");
    let err = validate_vote_with_hp(&vote, &proposal, SAFETY_CRITICAL_MIN_HP - 1).unwrap_err();
    assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
}

#[test]
fn high_hp_voter_accepted_for_emergency_halt() {
    let proposal = make_emergency_proposal();
    let vote = make_vote("bob");
    assert!(validate_vote_with_hp(&vote, &proposal, SAFETY_CRITICAL_MIN_HP).is_ok());
}

#[test]
fn safety_critical_threshold_is_5() {
    assert_eq!(SAFETY_CRITICAL_MIN_HP, 5);
}
