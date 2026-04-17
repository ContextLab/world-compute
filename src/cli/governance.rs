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
    use crate::governance::board::ProposalBoard;

    match cmd {
        GovernanceCommand::Propose { title, body, proposal_type } => {
            let mut board = ProposalBoard::new();
            let proposal_type_parsed = parse_proposal_type(proposal_type);
            match board.submit_proposal(
                title.clone(),
                body.clone(),
                proposal_type_parsed,
                "cli-user",
            ) {
                Ok(id) => format!("Proposal submitted.\n  ID: {id}\n  Title: {title}\n  Type: {proposal_type}"),
                Err(e) => format!("Error submitting proposal: {e}"),
            }
        }
        GovernanceCommand::List { state } => {
            let board = ProposalBoard::new();
            let filter = state.as_ref().and_then(|s| parse_proposal_state(s));
            let proposals = board.list_proposals(filter);
            if proposals.is_empty() {
                "No proposals found.".into()
            } else {
                let mut output = format!("Proposals ({}):\n", proposals.len());
                for p in &proposals {
                    output.push_str(&format!("  {} — {} [{:?}]\n", p.proposal_id, p.title, p.state));
                }
                output
            }
        }
        GovernanceCommand::Vote { proposal_id, choice } => {
            let vote_choice = match choice.as_str() {
                "yes" => "Yes",
                "no" => "No",
                "abstain" => "Abstain",
                _ => return format!("Error: invalid vote choice '{choice}'. Use: yes, no, abstain"),
            };
            format!("Vote '{vote_choice}' registered for proposal {proposal_id} (awaiting governance service connection).")
        }
        GovernanceCommand::Report { proposal_id } => {
            format!("Report for proposal {proposal_id}: no governance service connection.")
        }
    }
}

fn parse_proposal_type(s: &str) -> crate::governance::proposal::ProposalType {
    use crate::governance::proposal::ProposalType;
    match s {
        "compute" => ProposalType::Compute,
        "policy-change" => ProposalType::PolicyChange,
        "acceptable-use-rule" => ProposalType::AcceptableUseRule,
        "priority-rebalance" => ProposalType::PriorityRebalance,
        "emergency-halt" => ProposalType::EmergencyHalt,
        "constitution-amendment" => ProposalType::ConstitutionAmendment,
        _ => ProposalType::PolicyChange,
    }
}

fn parse_proposal_state(s: &str) -> Option<crate::governance::proposal::ProposalState> {
    use crate::governance::proposal::ProposalState;
    match s {
        "draft" => Some(ProposalState::Draft),
        "open" => Some(ProposalState::Open),
        "passed" => Some(ProposalState::Passed),
        "rejected" => Some(ProposalState::Rejected),
        "withdrawn" => Some(ProposalState::Withdrawn),
        "enacted" => Some(ProposalState::Enacted),
        _ => None,
    }
}
