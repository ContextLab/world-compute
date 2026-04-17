//! CLI dispatch for cluster subcommands.
//! Cluster CLI did not have a pre-existing struct, so it's defined here.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Cluster operations — status, peers, ledger-head")]
pub struct ClusterCli {
    #[command(subcommand)]
    pub command: ClusterCommand,
}

#[derive(Subcommand)]
pub enum ClusterCommand {
    /// Show cluster health, node count, and coordinator status
    Status,
    /// List connected peers with trust scores
    Peers,
    /// Show current ledger head hash and height
    LedgerHead,
}

/// Execute a cluster CLI command. Returns a human-readable status string.
pub fn execute_cluster(cmd: &ClusterCommand) -> String {
    match cmd {
        ClusterCommand::Status => {
            "Cluster status: no active coordinator connection. Start a coordinator with `worldcompute donor join` first.".into()
        }
        ClusterCommand::Peers => {
            "Connected peers: none (not connected to cluster)".into()
        }
        ClusterCommand::LedgerHead => {
            "Ledger head: not available (not connected to cluster)".into()
        }
    }
}
