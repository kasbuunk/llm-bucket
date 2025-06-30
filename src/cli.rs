use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;
use crate::synchronise::synchronise;
use crate::load_config::load_config;

/// CLI for llm-bucket: aggregate and publish knowledge snapshots.
#[derive(Parser)]
#[clap(
    name = "llm-bucket",
    version,
    about = "Aggregate and publish git/Confluence/Slack content snapshots for LLM ingestion"
)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Synchronize all sources to the target bucket using the given config file
    Sync {
        /// Path to the YAML config file
        #[clap(long)]
        config: PathBuf,
    }
}

/// Extracted async CLI logic entrypoint for integration tests and main()
pub async fn run(cli: Cli) -> Result<()> {
    // Emit a top-level 'trace_initialised' event at the very start
    tracing::info!("trace_initialised");

    let result = match cli.command {
        Commands::Sync { config } => {
            let config = load_config(config)?;
            println!("Synchronise starting...");
            match synchronise(&config).await {
                Ok(report) => {
                    println!("Synchronise complete.\nReport:");
                    println!("{:#?}", report);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("[ERROR] Synchronisation failed: {}", e);
                    Err(anyhow::Error::msg(e))
                }
            }
        }
    };

    // For CLI/test parity: Explicit process exit only in main(), not in run()
    result
}
