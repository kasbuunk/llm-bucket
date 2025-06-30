use crate::synchronise::{SynchroniseConfig, DownloadConfig, UploadConfig, SourceAction, GitSource};
use crate::preprocess::{ProcessConfig, ProcessorKind};
use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use serde::Deserialize;

#[derive(Deserialize)]
struct StaticConfig {
    download: DownloadSection,
    process: ProcessSection,
}

#[derive(Deserialize)]
struct DownloadSection {
    output_dir: std::path::PathBuf,
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
    // Future: confluence, slack, etc.
}

#[derive(Deserialize)]
struct ProcessSection {
    kind: String,
}

/// Loads a static YAML config file (no secrets) and injects required env vars for secrets.
/// Returns a fully merged SynchroniseConfig or an error.
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<SynchroniseConfig> {
    let config_content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file {:?}", path.as_ref()))?;

    let static_conf: StaticConfig = serde_yaml::from_str(&config_content)
        .context("Failed to parse config YAML")?;

    let bucket_id = std::env::var("BUCKET_ID")
        .context("BUCKET_ID environment variable not set")?
        .parse::<i64>()
        .context("BUCKET_ID must be a valid integer")?;

    let api_key = std::env::var("OCP_APIM_SUBSCRIPTION_KEY")
        .context("OCP_APIM_SUBSCRIPTION_KEY environment variable not set")?;

    let download_config = DownloadConfig {
        output_dir: static_conf.download.output_dir,
        sources: static_conf.download.sources.into_iter().map(|s| match s {
            SourceActionYaml::Git { repo_url, reference } =>
                SourceAction::Git(GitSource { repo_url, reference }),
        }).collect(),
    };

    let process_kind = match static_conf.process.kind.as_str() {
        "FlattenFiles" => ProcessorKind::FlattenFiles,
        "ReadmeToPDF" => ProcessorKind::ReadmeToPDF,
        other => {
            anyhow::bail!("Unsupported process.kind: {}", other);
        }
    };

    let process_config = ProcessConfig { kind: process_kind };

    let upload_config = UploadConfig {
        bucket_id,
        api_key: Some(api_key),
    };

    Ok(SynchroniseConfig {
        download: download_config,
        process: process_config,
        upload: upload_config,
    })
}
