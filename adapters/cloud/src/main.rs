//! World Compute — Cloud adapter
//!
//! Enables virtual machine instances on AWS, GCP, or Azure to join the
//! World Compute network as donor nodes.  The adapter runs inside the VM,
//! registers with a coordinator, and routes workload submissions to the
//! local container runtime.

use clap::{Parser, Subcommand};

// ---------------------------------------------------------------------------
// Cloud provider enum
// ---------------------------------------------------------------------------

/// Supported public cloud providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    /// Amazon Web Services (EC2).
    Aws,
    /// Google Cloud Platform (Compute Engine).
    Gcp,
    /// Microsoft Azure (Virtual Machines).
    Azure,
}

impl CloudProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aws => "AWS",
            Self::Gcp => "GCP",
            Self::Azure => "Azure",
        }
    }
}

impl std::fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for CloudProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "aws" => Ok(Self::Aws),
            "gcp" => Ok(Self::Gcp),
            "azure" => Ok(Self::Azure),
            other => Err(format!("unknown provider '{other}'; expected aws, gcp, or azure")),
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter struct
// ---------------------------------------------------------------------------

/// Cloud VM adapter for World Compute.
pub struct CloudAdapter {
    /// Cloud provider hosting this instance.
    pub provider: CloudProvider,
    /// Cloud-provider-assigned instance identifier.
    pub instance_id: String,
}

impl CloudAdapter {
    pub fn new(provider: CloudProvider, instance_id: impl Into<String>) -> Self {
        Self { provider, instance_id: instance_id.into() }
    }

    pub fn describe(&self) {
        println!("Cloud adapter");
        println!("  Provider    : {}", self.provider);
        println!("  Instance ID : {}", self.instance_id);
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "worldcompute-cloud-adapter",
    about = "World Compute adapter for cloud VM instances (AWS / GCP / Azure)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register this VM instance with the World Compute network.
    Join {
        /// Cloud provider: aws, gcp, or azure.
        #[arg(long)]
        provider: String,
        /// Cloud-provider instance identifier (e.g. i-0abc123def456789a).
        #[arg(long)]
        instance_id: String,
        /// World Compute coordinator gRPC endpoint.
        #[arg(long, default_value = "https://coordinator.worldcompute.io:443")]
        coordinator: String,
    },
    /// Show the current registration and health status of this instance.
    Status,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Join { provider, instance_id, coordinator } => {
            let provider_enum = match provider.parse::<CloudProvider>() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            };
            let adapter = CloudAdapter::new(provider_enum, &instance_id);
            println!("Joining World Compute network…");
            adapter.describe();
            println!("  Coordinator : {coordinator}");
            println!();
            println!("Next steps:");
            println!("  1. Ensure outbound gRPC (port 443) to the coordinator is allowed.");
            println!("  2. The adapter will fetch a join token from the coordinator.");
            println!("  3. Run `worldcompute-cloud-adapter status` to verify registration.");
        }
        Commands::Status => {
            println!("World Compute cloud adapter — status");
            println!();
            println!("  Registration : not yet joined (run 'join' first)");
            println!("  Coordinator  : not connected");
            println!("  Tasks held   : 0");
        }
    }
}
