use clap::{Parser, Subcommand};

mod cli_dispatch;

#[derive(Parser)]
#[command(name = "worldcompute")]
#[command(about = "World Compute — a decentralized, volunteer-built compute public good")]
#[command(version)]
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
