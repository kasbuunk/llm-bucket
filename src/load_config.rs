use crate::synchronise::{SynchroniseConfig, DownloadConfig, UploadConfig, SourceAction, GitSource};
use crate::preprocess::{ProcessConfig, ProcessorKind};
use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use serde::Deserialize;
use tracing::{info, error};

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
    let path_ref = path.as_ref();
    info!(config_path = ?path_ref, "Loading configuration from file");

    let config_content = match fs::read_to_string(&path_ref) {
        Ok(content) => {
            info!(config_path = ?path_ref, "Config file read successfully");
            content
        },
        Err(e) => {
            error!(error = ?e, config_path = ?path_ref, "Failed to read config file");
            return Err(anyhow::anyhow!("Failed to read config file {:?}: {}", path_ref, e));
        }
    };

    let static_conf: StaticConfig = match serde_yaml::from_str(&config_content) {
        Ok(conf) => {
            info!(config_path = ?path_ref, "Parsed config YAML successfully");
            conf
        },
        Err(e) => {
            error!(error = ?e, config_path = ?path_ref, "Failed to parse config YAML");
            return Err(anyhow::anyhow!("Failed to parse config YAML: {e}"));
        }
    };

    let bucket_id = match std::env::var("BUCKET_ID") {
        Ok(var) => match var.parse::<i64>() {
            Ok(id) => id,
            Err(e) => {
                error!(error = ?e, var = ?var, "BUCKET_ID must be a valid integer");
                return Err(anyhow::anyhow!("BUCKET_ID must be a valid integer: {e}"));
            }
        },
        Err(e) => {
            error!(error = ?e, "BUCKET_ID environment variable not set");
            return Err(anyhow::anyhow!("BUCKET_ID environment variable not set: {e}"));
        }
    };

    let api_key = match std::env::var("OCP_APIM_SUBSCRIPTION_KEY") {
        Ok(key) => {
            info!("OCP_APIM_SUBSCRIPTION_KEY found in env");
            key
        },
        Err(e) => {
            error!(error = ?e, "OCP_APIM_SUBSCRIPTION_KEY environment variable not set");
            return Err(anyhow::anyhow!("OCP_APIM_SUBSCRIPTION_KEY environment variable not set: {e}"));
        }
    };

    let download_config = DownloadConfig {
        output_dir: static_conf.download.output_dir.clone(),
        sources: static_conf.download.sources.into_iter().map(|s| match s {
            SourceActionYaml::Git { repo_url, reference } => {
                info!(repo_url = %repo_url, "Parsed git source from config");
                SourceAction::Git(GitSource { repo_url, reference })
            }
        }).collect(),
    };

    let process_kind = match static_conf.process.kind.as_str() {
        "FlattenFiles" => ProcessorKind::FlattenFiles,
        "ReadmeToPDF" => ProcessorKind::ReadmeToPDF,
        other => {
            error!(kind = %other, "Unsupported process.kind in config");
            anyhow::bail!("Unsupported process.kind: {}", other);
        }
    };

    info!(?process_kind, "Selected process kind from config");

    let process_config = ProcessConfig { kind: process_kind };

    let upload_config = UploadConfig {
        bucket_id,
        api_key: Some(api_key),
    };

    info!(
        bucket_id = bucket_id,
        output_dir = %download_config.output_dir.display(),
        "Config loaded and merged successfully"
    );

    Ok(SynchroniseConfig {
        download: download_config,
        process: process_config,
        upload: upload_config,
    })
}
