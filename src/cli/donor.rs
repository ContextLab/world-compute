//! CLI `worldcompute donor` subcommand per FR-002, FR-054, FR-090 (T049).

use clap::{Parser, Subcommand};

use crate::acceptable_use::AcceptableUseClass;
use crate::agent::config::AgentConfig;
use crate::agent::lifecycle::AgentInstance;

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
            let classes: Vec<AcceptableUseClass> =
                consent.split(',').filter_map(|s| parse_use_class(s.trim())).collect();

            if classes.is_empty() {
                return "Error: no valid consent classes provided. Valid classes: scientific, public-good-ml, rendering, indexing, self-improvement, general".into();
            }

            let config = AgentConfig::default();
            let mut agent = AgentInstance::new(config);
            match agent.enroll(classes.clone()) {
                Ok(result) => {
                    format!(
                        "Enrolled as donor.\n  Peer ID: {}\n  Trust tier: {:?}\n  Caliber: {:?}\n  Sandbox: {:?}\n  Consent: {:?}",
                        result.peer_id, result.trust_tier, result.caliber_class,
                        result.sandbox_capability, classes
                    )
                }
                Err(e) => format!("Error enrolling: {e}"),
            }
        }
        DonorCommand::Status => {
            "Donor status: agent daemon not running. Start with `worldcompute donor join`.".into()
        }
        DonorCommand::Pause => "Pause: agent daemon not running. Nothing to pause.".into(),
        DonorCommand::Resume => "Resume: agent daemon not running. Nothing to resume.".into(),
        DonorCommand::Leave => {
            "Leave: agent daemon not running. No cluster state to clean up.".into()
        }
        DonorCommand::Credits { verify } => {
            if *verify {
                "Credits: no ledger connection available for verification.".into()
            } else {
                "Credits: agent daemon not running. No credit history available.".into()
            }
        }
        DonorCommand::Logs { lines } => {
            format!("Logs: no agent log file found. Requested last {lines} lines.")
        }
    }
}

fn parse_use_class(s: &str) -> Option<AcceptableUseClass> {
    match s {
        "scientific" => Some(AcceptableUseClass::Scientific),
        "public-good-ml" => Some(AcceptableUseClass::PublicGoodMl),
        "rendering" => Some(AcceptableUseClass::Rendering),
        "indexing" => Some(AcceptableUseClass::Indexing),
        "self-improvement" => Some(AcceptableUseClass::SelfImprovement),
        "general" => Some(AcceptableUseClass::GeneralCompute),
        _ => None,
    }
}
