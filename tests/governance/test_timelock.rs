//! T048 [US3]: ConstitutionAmendment enforces 7-day review period.

use worldcompute::error::ErrorCode;
use worldcompute::governance::proposal::{GovernanceProposal, ProposalState, ProposalType};
use worldcompute::types::Timestamp;

fn make_amendment() -> GovernanceProposal {
    GovernanceProposal {
        proposal_id: "p-amend".into(),
        title: "Amendment".into(),
        body: "Change principle".into(),
        proposal_type: ProposalType::ConstitutionAmendment,
        state: ProposalState::Draft,
        submitter_id: "alice".into(),
        created_at: Timestamp::now(),
        closes_at: Timestamp::now(),
        yes_votes: 10,
        no_votes: 0,
        abstain_votes: 0,
    }
}

#[test]
fn amendment_sets_7_day_review_on_open() {
    let mut p = make_amendment();
    p.open_for_voting().unwrap();
    let diff_days = (p.closes_at.0 - Timestamp::now().0) / (24 * 3600 * 1_000_000);
    assert!(diff_days >= 6, "Review period should be ~7 days, got {diff_days}");
}

#[test]
fn amendment_tally_blocked_during_review() {
    let mut p = make_amendment();
    p.open_for_voting().unwrap();
    let err = p.tally().unwrap_err();
    assert_eq!(err.code(), Some(ErrorCode::InvalidManifest));
}

#[test]
fn standard_proposal_tally_immediate() {
    let mut p = GovernanceProposal {
        proposal_id: "p-std".into(),
        title: "Standard".into(),
        body: "Body".into(),
        proposal_type: ProposalType::PolicyChange,
        state: ProposalState::Open,
        submitter_id: "alice".into(),
        created_at: Timestamp::now(),
        closes_at: Timestamp::now(),
        yes_votes: 5,
        no_votes: 2,
        abstain_votes: 0,
    };
    let state = p.tally().unwrap();
    assert_eq!(state, ProposalState::Passed);
}
