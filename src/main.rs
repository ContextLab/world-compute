use clap::{Parser, Subcommand};

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
    Donor,
    /// Job operations: submit, status, results, cancel, list
    Job,
    /// Cluster operations: status, peers, ledger-head
    Cluster,
    /// Governance operations: propose, list, vote, report
    Governance,
    /// Admin operations: halt, resume, ban, audit (requires admin cert)
    Admin,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Donor => {
            println!("worldcompute donor: not yet implemented");
        }
        Commands::Job => {
            println!("worldcompute job: not yet implemented");
        }
        Commands::Cluster => {
            println!("worldcompute cluster: not yet implemented");
        }
        Commands::Governance => {
            println!("worldcompute governance: not yet implemented");
        }
        Commands::Admin => {
            println!("worldcompute admin: not yet implemented");
        }
    }

    Ok(())
}
