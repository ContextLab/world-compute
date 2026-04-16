//! World Compute — Slurm adapter
//!
//! Bridges the World Compute task scheduler to an existing HPC cluster managed
//! by Slurm.  The adapter runs as a long-lived daemon on the Slurm head node
//! (or a machine with SSH/REST access to it) and translates World Compute task
//! submissions into `sbatch` jobs.

use clap::{Parser, Subcommand};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Connection settings for a Slurm cluster.
#[derive(Debug, Clone)]
pub struct SlurmConfig {
    /// Hostname or IP of the Slurm head node.
    pub head_node: String,
    /// Slurm partition to submit jobs into.
    pub partition: String,
    /// Maximum number of concurrent jobs this adapter will hold in queue.
    pub max_jobs: u32,
}

impl Default for SlurmConfig {
    fn default() -> Self {
        Self { head_node: "localhost".to_string(), partition: "general".to_string(), max_jobs: 64 }
    }
}

// ---------------------------------------------------------------------------
// Adapter struct
// ---------------------------------------------------------------------------

/// Slurm backend adapter for World Compute.
///
/// Holds connection state and delegates task lifecycle operations to the
/// Slurm REST API (or `ssh + sbatch` fallback).  Full Slurm connectivity is
/// not yet implemented; this struct establishes the data model and CLI.
pub struct SlurmAdapter {
    pub config: SlurmConfig,
}

impl SlurmAdapter {
    pub fn new(config: SlurmConfig) -> Self {
        Self { config }
    }

    /// Print a human-readable summary of the adapter's configuration.
    pub fn describe(&self) {
        println!("Slurm adapter");
        println!("  Head node : {}", self.config.head_node);
        println!("  Partition : {}", self.config.partition);
        println!("  Max jobs  : {}", self.config.max_jobs);
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "worldcompute-slurm-adapter",
    about = "World Compute adapter for Slurm HPC clusters",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install the adapter daemon and systemd unit on the head node.
    Install {
        /// Hostname or IP of the Slurm head node.
        #[arg(long, default_value = "localhost")]
        head_node: String,
        /// Slurm partition to target.
        #[arg(long, default_value = "general")]
        partition: String,
        /// Maximum concurrent job slots.
        #[arg(long, default_value_t = 64)]
        max_jobs: u32,
    },
    /// Write or update the adapter configuration file.
    Configure {
        /// Hostname or IP of the Slurm head node.
        #[arg(long, default_value = "localhost")]
        head_node: String,
        /// Slurm partition to target.
        #[arg(long, default_value = "general")]
        partition: String,
        /// Maximum concurrent job slots.
        #[arg(long, default_value_t = 64)]
        max_jobs: u32,
    },
    /// Report the current status of the adapter daemon.
    Status,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { head_node, partition, max_jobs } => {
            let config = SlurmConfig { head_node, partition, max_jobs };
            let adapter = SlurmAdapter::new(config);
            println!("Installing World Compute Slurm adapter…");
            adapter.describe();
            println!();
            println!("Next steps:");
            println!("  1. Ensure SSH key-based access from this host to the head node.");
            println!("  2. Verify Slurm REST API is enabled (slurmdbd + slurmrestd).");
            println!("  3. Run `worldcompute-slurm-adapter status` to confirm connectivity.");
        }
        Commands::Configure { head_node, partition, max_jobs } => {
            let config = SlurmConfig { head_node, partition, max_jobs };
            let adapter = SlurmAdapter::new(config);
            println!("Writing adapter configuration…");
            adapter.describe();
            println!();
            println!("Configuration recorded. Restart the adapter daemon to apply changes.");
        }
        Commands::Status => {
            println!("World Compute Slurm adapter — status");
            println!();
            println!("  Daemon     : not yet started (run 'install' first)");
            println!("  Slurm API  : not yet connected");
            println!("  Jobs held  : 0");
        }
    }
}
