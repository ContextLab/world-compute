//! ProposalBoard — in-memory store for proposals and voting per US6.

use crate::error::{ErrorCode, WcError, WcResult};
use crate::governance::proposal::{GovernanceProposal, ProposalState, ProposalType};
use crate::governance::vote::{validate_vote, Vote, VoteChoice};
use crate::governance::voting::QuadraticVoteBudget;
use crate::types::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lightweight summary for listing proposals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalSummary {
    pub proposal_id: String,
    pub title: String,
    pub proposal_type: ProposalType,
    pub state: ProposalState,
    pub submitter_id: String,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub abstain_votes: u64,
}

/// In-memory board of governance proposals.
#[derive(Debug, Default)]
pub struct ProposalBoard {
    proposals: HashMap<String, GovernanceProposal>,
    next_id: u64,
}

impl ProposalBoard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Submit a new proposal. Returns the new proposal_id.
    pub fn submit_proposal(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        proposal_type: ProposalType,
        submitter_id: impl Into<String>,
    ) -> WcResult<String> {
        self.next_id += 1;
        let proposal_id = format!("prop-{:06}", self.next_id);
        let now = Timestamp::now();
        // Default open window: 7 days in microseconds
        let closes_at = Timestamp(now.0 + 7 * 24 * 3600 * 1_000_000);
        let proposal = GovernanceProposal {
            proposal_id: proposal_id.clone(),
            title: title.into(),
            body: body.into(),
            proposal_type,
            state: ProposalState::Draft,
            submitter_id: submitter_id.into(),
            created_at: now,
            closes_at,
            yes_votes: 0,
            no_votes: 0,
            abstain_votes: 0,
        };
        self.proposals.insert(proposal_id.clone(), proposal);
        Ok(proposal_id)
    }

    /// List proposals, optionally filtered by state.
    pub fn list_proposals(&self, filter_state: Option<ProposalState>) -> Vec<ProposalSummary> {
        let mut summaries: Vec<ProposalSummary> = self
            .proposals
            .values()
            .filter(|p| filter_state.is_none_or(|s| p.state == s))
            .map(|p| ProposalSummary {
                proposal_id: p.proposal_id.clone(),
                title: p.title.clone(),
                proposal_type: p.proposal_type,
                state: p.state,
                submitter_id: p.submitter_id.clone(),
                yes_votes: p.yes_votes,
                no_votes: p.no_votes,
                abstain_votes: p.abstain_votes,
            })
            .collect();
        summaries.sort_by(|a, b| a.proposal_id.cmp(&b.proposal_id));
        summaries
    }

    /// Cast a vote on an open proposal.
    ///
    /// `hp_score` is the voter's computed Humanity Points, used to determine
    /// quadratic cost from a per-session budget check (budget enforcement is
    /// the caller's responsibility; this method only applies tally).
    pub fn cast_vote(
        &mut self,
        proposal_id: &str,
        voter_id: impl Into<String>,
        choice: VoteChoice,
        hp_score: u32,
    ) -> WcResult<()> {
        let voter_id = voter_id.into();
        let proposal = self.proposals.get(proposal_id).ok_or_else(|| {
            WcError::new(ErrorCode::NotFound, format!("proposal {proposal_id} not found"))
        })?;

        let weight = QuadraticVoteBudget::cast_cost(hp_score).max(1);
        let vote = Vote {
            vote_id: format!("{proposal_id}:{voter_id}"),
            proposal_id: proposal_id.into(),
            voter_id: voter_id.clone(),
            choice,
            weight,
            signature: vec![],
            cast_at: Timestamp::now(),
        };

        validate_vote(&vote, proposal)?;

        let proposal = self.proposals.get_mut(proposal_id).unwrap();
        match choice {
            VoteChoice::Yes => proposal.yes_votes += 1,
            VoteChoice::No => proposal.no_votes += 1,
            VoteChoice::Abstain => proposal.abstain_votes += 1,
        }
        Ok(())
    }

    /// Get a proposal by ID.
    pub fn get_proposal(&self, proposal_id: &str) -> Option<&GovernanceProposal> {
        self.proposals.get(proposal_id)
    }

    /// Get a mutable proposal by ID.
    pub fn get_proposal_mut(&mut self, proposal_id: &str) -> Option<&mut GovernanceProposal> {
        self.proposals.get_mut(proposal_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_proposal(board: &mut ProposalBoard, proposal_id: &str) {
        board.get_proposal_mut(proposal_id).unwrap().transition(ProposalState::Open).unwrap();
    }

    #[test]
    fn submit_creates_proposal() {
        let mut board = ProposalBoard::new();
        let id =
            board.submit_proposal("Title", "Body", ProposalType::PolicyChange, "alice").unwrap();
        assert!(!id.is_empty());
        let p = board.get_proposal(&id).unwrap();
        assert_eq!(p.title, "Title");
        assert_eq!(p.state, ProposalState::Draft);
        assert_eq!(p.submitter_id, "alice");
    }

    #[test]
    fn list_all_proposals() {
        let mut board = ProposalBoard::new();
        board.submit_proposal("A", "Body", ProposalType::Compute, "alice").unwrap();
        board.submit_proposal("B", "Body", ProposalType::EmergencyHalt, "bob").unwrap();
        let all = board.list_proposals(None);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn list_filtered_by_state() {
        let mut board = ProposalBoard::new();
        let id = board.submit_proposal("A", "Body", ProposalType::Compute, "alice").unwrap();
        board.submit_proposal("B", "Body", ProposalType::Compute, "bob").unwrap();
        open_proposal(&mut board, &id);

        let open = board.list_proposals(Some(ProposalState::Open));
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].proposal_id, id);

        let draft = board.list_proposals(Some(ProposalState::Draft));
        assert_eq!(draft.len(), 1);
    }

    #[test]
    fn vote_yes_tallied() {
        let mut board = ProposalBoard::new();
        let id = board.submit_proposal("A", "Body", ProposalType::PolicyChange, "alice").unwrap();
        open_proposal(&mut board, &id);

        board.cast_vote(&id, "bob", VoteChoice::Yes, 1).unwrap();
        let p = board.get_proposal(&id).unwrap();
        assert_eq!(p.yes_votes, 1);
    }

    #[test]
    fn self_vote_rejected() {
        let mut board = ProposalBoard::new();
        let id = board.submit_proposal("A", "Body", ProposalType::PolicyChange, "alice").unwrap();
        open_proposal(&mut board, &id);

        let err = board.cast_vote(&id, "alice", VoteChoice::Yes, 1).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::PermissionDenied));
    }

    #[test]
    fn vote_on_draft_rejected() {
        let mut board = ProposalBoard::new();
        let id = board.submit_proposal("A", "Body", ProposalType::PolicyChange, "alice").unwrap();
        // Do NOT open the proposal
        let err = board.cast_vote(&id, "bob", VoteChoice::Yes, 1).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::InvalidManifest));
    }

    #[test]
    fn vote_on_missing_proposal_rejected() {
        let mut board = ProposalBoard::new();
        let err = board.cast_vote("nonexistent", "bob", VoteChoice::Yes, 1).unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::NotFound));
    }
}
