///
/// This module implements the full CLI interface for llm-bucket—handling command parsing,
/// argument validation, main entrypoints, and user-visible invocations.
///
/// All core business logic (data models, pipelines, and processing) lives in the [`llm-bucket-core`] crate.
/// This module is strictly for CLI glue, ergonomic argument exposure, and orchestration.
///
/// ## Features
/// - Entry struct [`Cli`] defines all user-facing options and subcommands (see below).
/// - Subcommand routing (e.g., `sync`) and argument validation.
/// - Async entrypoint (`run`) for programmatic invocation and integration testing.
/// - Logging, tracing, and structured error output at CLI level.
///
/// ## How To Use
/// - For command-line users: use the installed `llm-bucket` binary with `--help`.
/// - For programmatic/integration use: call [`run`] with a constructed [`Cli`].
///
/// ## Extending
/// When adding features or subcommands, update [`Commands`] below
/// and keep all non-trivial business logic inside `llm-bucket-core`.
///
/// ---
///
/// See crate root docs and [`llm-bucket-core`] for overall architecture.
///
/// ---
///
/// [`llm-bucket-core`]: ../../llm-bucket-core/
/// [`Cli`]: struct.Cli.html
/// [`run`]: fn.run.html
/// [`Commands`]: enum.Commands.html
use crate::load_config::load_config;
use crate::upload::LLMClient;
use anyhow::Result;
use clap::{Parser, Subcommand};
/// # llm-bucket CLI Interface (Module)
use llm_bucket_core::contract::Downloader;
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
            // Here, config is the loaded YAML Config, which has output_dir and sources fields.
            // Map process section to ProcessConfig before passing.
            let process_config = llm_bucket_core::preprocess::ProcessConfig {
                kind: llm_bucket_core::preprocess::ProcessorKind::from(
                    config.process.kind.as_str(),
                ),
            };
            let output_dir = config.download.output_dir.clone();
            let sources = config.download.sources.clone();
            let download_config = llm_bucket_core::download::DownloadConfig {
                output_dir,
                sources,
            };
            let downloader = llm_bucket_core::download::DefaultDownloader::new(download_config);
            let manifest = Downloader::download_all(&downloader)
                .await
                .map_err(|e| anyhow::Error::msg(format!("Download failed: {e:?}")))?;
            match synchronise(&process_config, &uploader, &manifest.sources).await {
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

    result
}
