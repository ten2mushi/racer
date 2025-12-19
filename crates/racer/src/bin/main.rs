//! RACER CLI binary entry point.
//!
//! This binary requires the `cli` feature to be enabled.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "racer", version, about = "RACER consensus node")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run(racer::cli::run::Args),
    Config(racer::cli::config::Args),
    Keygen(racer::cli::keygen::Args),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run(args) => racer::cli::run::execute(args).await,
        Commands::Config(args) => racer::cli::config::execute(args),
        Commands::Keygen(args) => racer::cli::keygen::execute(args),
    }
}
