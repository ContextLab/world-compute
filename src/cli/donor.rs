//! CLI `worldcompute donor` subcommand per FR-002, FR-054, FR-090 (T049).

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Donor operations — join, status, pause, resume, leave, credits")]
pub struct DonorCli {
    #[command(subcommand)]
    pub command: DonorCommand,
}

#[derive(Subcommand)]
pub enum DonorCommand {
    /// Enroll this machine as a World Compute donor
    Join {
        /// Workload classes to accept (comma-separated: scientific,public-good-ml,rendering,indexing,self-improvement,general)
        #[arg(long, default_value = "scientific,general")]
        consent: String,
    },
    /// Show current donor status, trust score, and caliber class
    Status,
    /// Pause the agent (checkpoint active work, stop advertising)
    Pause,
    /// Resume the agent after a pause
    Resume,
    /// Withdraw from the cluster (removes all host state)
    Leave,
    /// Show credit balance and history
    Credits {
        /// Cryptographically verify credits against the ledger
        #[arg(long)]
        verify: bool,
    },
    /// Show recent agent logs
    Logs {
        /// Number of recent log lines to show
        #[arg(long, default_value = "50")]
        lines: usize,
    },
}

/// Execute a donor CLI command. Returns a human-readable status string.
pub fn execute(cmd: &DonorCommand) -> String {
    match cmd {
        DonorCommand::Join { consent } => {
            format!("Enrolling as donor with consent classes: {consent}\n(Not yet connected to agent daemon)")
        }
        DonorCommand::Status => {
            "Donor status: not yet implemented (requires running agent daemon)".into()
        }
        DonorCommand::Pause => "Pausing agent: not yet implemented".into(),
        DonorCommand::Resume => "Resuming agent: not yet implemented".into(),
        DonorCommand::Leave => "Withdrawing from cluster: not yet implemented".into(),
        DonorCommand::Credits { verify } => {
            if *verify {
                "Credits (verified): not yet implemented".into()
            } else {
                "Credits: not yet implemented".into()
            }
        }
        DonorCommand::Logs { lines } => {
            format!("Showing last {lines} log lines: not yet implemented")
        }
    }
}
