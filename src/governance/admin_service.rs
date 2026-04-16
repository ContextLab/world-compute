//! AdminService gRPC stub handler per US6.

use crate::error::WcResult;
use crate::governance::board::ProposalBoard;
use crate::governance::proposal::ProposalState;

/// Stub gRPC handler for AdminService RPCs.
pub struct AdminServiceHandler {
    pub board: ProposalBoard,
    pub halted: bool,
}

impl AdminServiceHandler {
    pub fn new() -> Self {
        Self { board: ProposalBoard::new(), halted: false }
    }

    /// Halt RPC stub — sets cluster halt flag.
    pub fn halt(&mut self, _reason: impl Into<String>) -> WcResult<()> {
        self.halted = true;
        Ok(())
    }

    /// Resume RPC stub — clears cluster halt flag.
    pub fn resume(&mut self) -> WcResult<()> {
        self.halted = false;
        Ok(())
    }

    /// Ban RPC stub — placeholder; real impl would update trust registry.
    pub fn ban(
        &mut self,
        _subject_id: impl Into<String>,
        _reason: impl Into<String>,
    ) -> WcResult<()> {
        Ok(())
    }

    /// Audit RPC stub — returns proposal state for the given ID.
    pub fn audit_proposal(&self, proposal_id: &str) -> Option<ProposalState> {
        self.board.get_proposal(proposal_id).map(|p| p.state)
    }
}

impl Default for AdminServiceHandler {
    fn default() -> Self {
        Self::new()
    }
}
