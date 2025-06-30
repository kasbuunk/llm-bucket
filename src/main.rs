use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;
use llm_bucket::synchronise::synchronise;
use llm_bucket::load_config::load_config;

/// CLI for llm-bucket: aggregate and publish knowledge snapshots.
#[derive(Parser)]
#[clap(
    name = "llm-bucket",
    version,
    about = "Aggregate and publish git/Confluence/Slack content snapshots for LLM ingestion"
)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Synchronize all sources to the target bucket using the given config file
    Sync {
        /// Path to the YAML config file
        #[clap(long)]
        config: PathBuf,
    }
}



#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Sync { config } => {
            let config = load_config(config)?;
            println!("Synchronise starting...");
            match synchronise(&config).await {
                Ok(report) => {
                    println!("Synchronise complete.\nReport:");
                    println!("{:#?}", report);
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("[ERROR] Synchronisation failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
