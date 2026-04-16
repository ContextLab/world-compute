//! GovernanceService gRPC stub handler per US6.

use crate::error::WcResult;
use crate::governance::board::ProposalBoard;
use crate::governance::proposal::ProposalType;
use crate::governance::vote::VoteChoice;

/// Stub gRPC handler for GovernanceService RPCs.
pub struct GovernanceServiceHandler {
    pub board: ProposalBoard,
}

impl GovernanceServiceHandler {
    pub fn new() -> Self {
        Self { board: ProposalBoard::new() }
    }

    /// SubmitProposal RPC stub.
    pub fn submit_proposal(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        proposal_type: ProposalType,
        submitter_id: impl Into<String>,
    ) -> WcResult<String> {
        self.board.submit_proposal(title, body, proposal_type, submitter_id)
    }

    /// CastVote RPC stub.
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
