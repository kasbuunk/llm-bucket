use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::{Deserialize};
use std::fs;
use llm_bucket::synchronise::{
    SynchroniseConfig, DownloadConfig, UploadConfig, SourceAction, GitSource, synchronise,
};
use llm_bucket::preprocess::{ProcessConfig, ProcessorKind};

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

// Structures to deserialize CLI config YAML file (matches DownloadConfig & ProcessConfig)
#[derive(Deserialize)]
struct CliConfig {
    download: DownloadSection,
    process: ProcessSection,
    upload: UploadSection,
}

#[derive(Deserialize)]
struct DownloadSection {
    output_dir: PathBuf,
    #[serde(default)]
    sources: Vec<SourceActionYaml>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum SourceActionYaml {
    #[serde(rename = "git")]
    Git {
        repo_url: String,
        #[serde(default)]
        reference: Option<String>,
    },
    // Extend for confluence/slack/etc later
}

#[derive(Deserialize)]
struct ProcessSection {
    kind: String,
}

#[derive(Deserialize)]
struct UploadSection {
    #[allow(dead_code)]
    bucket_id: Option<i64>, // not used, always loaded from env
    #[allow(dead_code)]
    api_key: Option<String>, // not used, always loaded from env
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Sync { config } => {
            let config_content = fs::read_to_string(&config)
                .with_context(|| format!("Failed to read config file {:?}", config))?;
            let cli_conf: CliConfig = serde_yaml::from_str(&config_content)
                .context("Failed to parse config YAML")?;

            // Read required env vars
            let bucket_id = std::env::var("BUCKET_ID")
                .context("BUCKET_ID environment variable not set")?
                .parse::<i64>()
                .context("BUCKET_ID must be a valid integer")?;

            let api_key = std::env::var("OCP_APIM_SUBSCRIPTION_KEY")
                .context("OCP_APIM_SUBSCRIPTION_KEY environment variable not set")?;

            let download_config = DownloadConfig {
                output_dir: cli_conf.download.output_dir,
                sources: cli_conf.download.sources.into_iter().map(|s| match s {
                    SourceActionYaml::Git { repo_url, reference } =>
                        SourceAction::Git(GitSource { repo_url, reference }),
                }).collect(),
            };
            let process_kind = match cli_conf.process.kind.as_str() {
                "FlattenFiles" => ProcessorKind::FlattenFiles,
                "ReadmeToPDF" => ProcessorKind::ReadmeToPDF,
                other => {
                    eprintln!("Unsupported process.kind: {}", other);
                    std::process::exit(2);
                }
            };
            let process_config = ProcessConfig { kind: process_kind };

            let upload_config = UploadConfig {
                bucket_id,
                api_key: Some(api_key),
            };

            let config = SynchroniseConfig {
                download: download_config,
                process: process_config,
                upload: upload_config,
            };

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
