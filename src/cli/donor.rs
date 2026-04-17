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
        /// Run as a persistent P2P daemon (listens for peers, publishes heartbeats)
        #[arg(long)]
        daemon: bool,
        /// TCP/QUIC listen port (default: 19999)
        #[arg(long, default_value = "19999")]
        port: u16,
        /// Bootstrap peer addresses to connect to (comma-separated multiaddrs)
        #[arg(long)]
        bootstrap: Option<String>,
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
/// For daemon mode, use `execute_async` instead.
pub fn execute(cmd: &DonorCommand) -> String {
    match cmd {
        DonorCommand::Join { consent, daemon, port, bootstrap: _ } => {
            let classes: Vec<AcceptableUseClass> =
                consent.split(',').filter_map(|s| parse_use_class(s.trim())).collect();

            if classes.is_empty() {
                return "Error: no valid consent classes provided. Valid classes: scientific, public-good-ml, rendering, indexing, self-improvement, general".into();
            }

            if *daemon {
                return format!(
                    "Daemon mode requested on port {port}. Use execute_async() for daemon startup."
                );
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

/// Execute a donor join command in daemon mode (async, blocks until shutdown).
pub async fn execute_daemon(cmd: &DonorCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        DonorCommand::Join { consent, daemon: _, port, bootstrap } => {
            let classes: Vec<AcceptableUseClass> =
                consent.split(',').filter_map(|s| parse_use_class(s.trim())).collect();

            if classes.is_empty() {
                return Err("No valid consent classes provided".into());
            }

            let config = AgentConfig::default();
            let mut agent = AgentInstance::new(config);
            let result = agent.enroll(classes.clone())?;

            println!(
                "Enrolled as donor.\n  Peer ID: {}\n  Trust tier: {:?}\n  Caliber: {:?}\n  Sandbox: {:?}\n  Consent: {:?}",
                result.peer_id, result.trust_tier, result.caliber_class,
                result.sandbox_capability, classes
            );

            // Parse bootstrap peers
            let bootstrap_peers: Vec<String> = bootstrap
                .as_deref()
                .map(|b| b.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            let daemon_config = crate::agent::daemon::DaemonConfig {
                tcp_port: *port,
                quic_port: *port,
                heartbeat_interval_secs: 30,
                bootstrap_peers,
            };

            // Start the persistent daemon — this blocks until Ctrl+C
            crate::agent::daemon::start_daemon(agent, daemon_config).await?;
            Ok(())
        }
        _ => Err("Only 'join' supports daemon mode".into()),
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
