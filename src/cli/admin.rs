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
///
/// Note: Admin operations require OnCallResponder role per FR-S031.
/// Without a running daemon and authenticated session, these commands
/// validate the request structure but cannot execute against the cluster.
pub fn execute(cmd: &AdminCommand) -> String {
    match cmd {
        AdminCommand::Halt { reason } => {
            format!(
                "Emergency halt requested.\n  Reason: {reason}\n  Status: requires OnCallResponder role and active admin service connection."
            )
        }
        AdminCommand::Resume => {
            "Resume requested. Requires OnCallResponder role and active admin service connection.".into()
        }
        AdminCommand::Ban { subject_id, reason } => {
            format!(
                "Ban requested.\n  Subject: {subject_id}\n  Reason: {reason}\n  Status: requires active admin service connection."
            )
        }
        AdminCommand::Audit { id } => {
            format!("Audit requested for {id}. Requires active admin service connection.")
        }
    }
}
