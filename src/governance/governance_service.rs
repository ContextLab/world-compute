//! GovernanceService gRPC handler per US6.
//!
//! Delegates SubmitProposal and CastVote RPCs to the real `ProposalBoard`
//! store. The board persists proposals and votes, emits audit events, and
//! enforces the time-lock + HP-threshold rules described in constitution
//! Principle V and spec 001 FR-S030.

use crate::error::WcResult;
use crate::governance::board::ProposalBoard;
use crate::governance::proposal::ProposalType;
use crate::governance::vote::VoteChoice;

/// gRPC handler backed by a live `ProposalBoard`.
pub struct GovernanceServiceHandler {
    pub board: ProposalBoard,
}

impl GovernanceServiceHandler {
    pub fn new() -> Self {
        Self { board: ProposalBoard::new() }
    }

    /// SubmitProposal RPC — persists a new governance proposal to the board.
    pub fn submit_proposal(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        proposal_type: ProposalType,
        submitter_id: impl Into<String>,
    ) -> WcResult<String> {
        self.board.submit_proposal(title, body, proposal_type, submitter_id)
    }

    /// CastVote RPC — records a vote on an existing proposal with the
    /// caller's Humanity-Points (HP) score for weighting and safety-tier
    /// gating.
    pub fn cast_vote(
        &mut self,
        proposal_id: &str,
        voter_id: impl Into<String>,
        choice: VoteChoice,
        hp_score: u32,
    ) -> WcResult<()> {
        self.board.cast_vote(proposal_id, voter_id, choice, hp_score)
    }
}

impl Default for GovernanceServiceHandler {
    fn default() -> Self {
        Self::new()
    }
}
