use crate::load_config::load_config;
use crate::upload::LLMClient;
use anyhow::Result;
use clap::{Parser, Subcommand};
use llm_bucket_core::synchronise::synchronise;
use std::path::PathBuf;

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
    },
}

/// Extracted async CLI logic entrypoint for integration tests and main()
pub async fn run(cli: Cli) -> Result<()> {
    // Emit a top-level 'trace_initialised' event at the very start
    tracing::info!("trace_initialised");

    let result = match cli.command {
        Commands::Sync { config } => {
            let config = load_config(config)?;
            tracing::info!(command = "sync", "Starting synchronisation process");
            let uploader =
                LLMClient::new_from_env().expect("Failed to construct uploader from env");
            match synchronise(&config, &uploader).await {
                Ok(report) => {
                    tracing::info!(command = "sync", ?report, "Synchronisation complete");
                    Ok(())
                }
                Err(e) => {
                    tracing::error!(command = "sync", error = %e, "Synchronisation failed");
                    Err(anyhow::Error::msg(e))
                }
            }
        }
    };

    // For CLI/test parity: Explicit process exit only in main(), not in run()
    result
}
