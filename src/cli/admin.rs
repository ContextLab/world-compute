//! CLI `worldcompute admin` subcommand per US6 / FR-090.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Admin operations — halt, resume, ban, audit")]
pub struct AdminCli {
    #[command(subcommand)]
    pub command: AdminCommand,
}

#[derive(Subcommand)]
pub enum AdminCommand {
    /// Trigger an emergency halt of the cluster
    Halt {
        /// Reason for halt
        #[arg(long)]
        reason: String,
    },
    /// Resume cluster operations after a halt
    Resume,
    /// Ban a user or node from the cluster
    Ban {
        /// Subject ID (user or node) to ban
        #[arg(long)]
        subject_id: String,
        /// Reason for ban
        #[arg(long)]
        reason: String,
    },
    /// Audit a proposal or subject
    Audit {
        /// Proposal or subject ID to audit
        #[arg(long)]
        id: String,
    },
}

/// Execute an admin CLI command. Returns a human-readable status string.
pub fn execute(cmd: &AdminCommand) -> String {
    match cmd {
        AdminCommand::Halt { reason } => {
            format!("Halting cluster (reason: {reason}): not yet connected to admin service")
        }
        AdminCommand::Resume => "Resuming cluster: not yet implemented".into(),
        AdminCommand::Ban { subject_id, reason } => {
            format!("Banning {subject_id} (reason: {reason}): not yet implemented")
        }
        AdminCommand::Audit { id } => {
            format!("Auditing {id}: not yet implemented")
        }
    }
}
