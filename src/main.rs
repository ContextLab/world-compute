use clap::{Parser, Subcommand};

mod cli_dispatch;

/// Build-mode tag shown in `--version` output (spec 005 T011).
///
/// Production builds compile-time-enforce non-zero pinned fingerprints
/// (features.rs); dev builds permit the zero-pin bypass for testing. Operators
/// must see which mode their binary is in without having to inspect `Cargo.toml`.
#[cfg(feature = "production")]
const VERSION_WITH_MODE: &str = concat!(env!("CARGO_PKG_VERSION"), " (production)");

#[cfg(not(feature = "production"))]
const VERSION_WITH_MODE: &str = concat!(env!("CARGO_PKG_VERSION"), " (dev)");

#[derive(Parser)]
#[command(name = "worldcompute")]
#[command(about = "World Compute — a decentralized, volunteer-built compute public good")]
#[command(version = VERSION_WITH_MODE)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Donor operations: join, status, pause, resume, leave, credits
    Donor(worldcompute::cli::donor::DonorCli),
    /// Job operations: submit, status, results, cancel, list
    Job(worldcompute::cli::submitter::JobCli),
    /// Cluster operations: status, peers, ledger-head
    Cluster(cli_dispatch::ClusterCli),
    /// Governance operations: propose, list, vote, report
    Governance(worldcompute::cli::governance::GovernanceCli),
    /// Admin operations: halt, resume, ban, audit (requires admin cert)
    Admin(worldcompute::cli::admin::AdminCli),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Check if this is a daemon command (donor join --daemon)
    if let Commands::Donor(ref donor_cli) = cli.command {
        if let worldcompute::cli::donor::DonorCommand::Join { daemon: true, .. } = donor_cli.command
        {
            // Daemon mode — run the persistent P2P node (blocks until Ctrl+C)
            worldcompute::cli::donor::execute_daemon(&donor_cli.command)
                .await
                .map_err(|e| anyhow::anyhow!("Daemon error: {e}"))?;
            return Ok(());
        }
    }

    // Check if this is a remote job submission (job submit --executor <addr>)
    if let Commands::Job(ref job_cli) = cli.command {
        if let worldcompute::cli::submitter::JobCommand::Submit { executor: Some(_), .. } =
            &job_cli.command
        {
            worldcompute::cli::submitter::execute_remote_submit(&job_cli.command)
                .await
                .map_err(|e| anyhow::anyhow!("Remote dispatch error: {e}"))?;
            return Ok(());
        }
    }

    // Non-daemon commands — execute and print output
    let output = match cli.command {
        Commands::Donor(donor_cli) => worldcompute::cli::donor::execute(&donor_cli.command),
        Commands::Job(job_cli) => worldcompute::cli::submitter::execute(&job_cli.command),
        Commands::Cluster(cluster_cli) => cli_dispatch::execute_cluster(&cluster_cli.command),
        Commands::Governance(gov_cli) => worldcompute::cli::governance::execute(&gov_cli.command),
        Commands::Admin(admin_cli) => worldcompute::cli::admin::execute(&admin_cli.command),
    };

    println!("{output}");
    Ok(())
}
