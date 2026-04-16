//! CLI `worldcompute governance` subcommand per US6 / FR-090.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Governance operations — propose, list, vote, report")]
pub struct GovernanceCli {
    #[command(subcommand)]
    pub command: GovernanceCommand,
}

#[derive(Subcommand)]
pub enum GovernanceCommand {
    /// Submit a new governance proposal
    Propose {
        /// Proposal title
        #[arg(long)]
        title: String,
        /// Proposal body / description
        #[arg(long)]
        body: String,
        /// Proposal type (compute, policy-change, acceptable-use-rule, priority-rebalance, emergency-halt, constitution-amendment)
        #[arg(long, default_value = "policy-change")]
        proposal_type: String,
    },
    /// List governance proposals
    List {
        /// Filter by state (draft, open, passed, rejected, withdrawn, enacted)
        #[arg(long)]
        state: Option<String>,
    },
    /// Cast a vote on a proposal
    Vote {
        /// Proposal ID to vote on
        #[arg(long)]
        proposal_id: String,
        /// Vote choice (yes, no, abstain)
        #[arg(long)]
        choice: String,
    },
    /// Show a governance report for a proposal
    Report {
        /// Proposal ID
        #[arg(long)]
        proposal_id: String,
    },
}

/// Execute a governance CLI command. Returns a human-readable status string.
pub fn execute(cmd: &GovernanceCommand) -> String {
    match cmd {
        GovernanceCommand::Propose { title, body, proposal_type } => {
            format!(
                "Submitting proposal '{title}' (type: {proposal_type}): not yet connected to governance service\nBody: {body}"
            )
        }
        GovernanceCommand::List { state } => {
            if let Some(s) = state {
                format!("Listing proposals with state={s}: not yet implemented")
            } else {
                "Listing all proposals: not yet implemented".into()
            }
        }
        GovernanceCommand::Vote { proposal_id, choice } => {
            format!("Casting vote '{choice}' on proposal {proposal_id}: not yet implemented")
        }
        GovernanceCommand::Report { proposal_id } => {
            format!("Governance report for proposal {proposal_id}: not yet implemented")
        }
    }
}
