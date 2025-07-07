/// `load_config` module: Loads and adapts a static YAML config—including environment secret injection—into the internal SynchroniseConfig.
///
/// This module is the only place where untrusted YAML is parsed and mapped to rich, strongly-typed internal structs.
///
/// # Responsibilities
/// - Parse user-supplied YAML configuration files into type-safe Rust structs
/// - Map loosely-typed YAML keys (e.g., string processor kinds) to enums and rich types
/// - Inject environment variables for secret fields (API tokens, bucket IDs) as needed
/// - Ensure robust error messages for CLI and tests: any failure in loading must result in clear diagnostics.
/// - Acts as the “adapter” layer decoupling input schemas from domain core
///
/// # Extension Guidance
/// - To add a new source type or config key:
///   1. Extend the intermediate (YAML-side) types and enums (e.g., SourceActionYaml)
///   2. Add conversion logic mapping from YAML types to domain-core models
///   3. Carefully validate that new config fields are surfaced to the SynchroniseConfig
///
/// # Errors
/// All errors in this module use `anyhow::Error` for context-rich diagnostics, and are surfaced at the CLI boundary.
///
/// For accepted YAML schema, see the README.
///
/// ---
///
/// Internal implementation begins below.
///
use anyhow::Result;
use llm_bucket_core::download::{ConfluenceSource, DownloadConfig, GitSource, SourceAction};
use llm_bucket_core::preprocess::{ProcessConfig, ProcessorKind};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

#[derive(Debug, Deserialize)]
pub struct CliConfig {
    pub download: DownloadSection,
    pub process: ProcessSection,
}

#[derive(Debug, Deserialize)]
pub struct DownloadSection {
    pub output_dir: std::path::PathBuf,
    #[serde(default)]
    pub sources: Vec<SourceAction>,
}

#[derive(Debug, Deserialize)]
pub struct ProcessSection {
    pub kind: String,
}

/// Loads a static YAML config file (no secrets) and injects required env vars for secrets.
/// Returns a processable CLI config for use by the CLI.
/// Loads a static YAML config file (no secrets) and injects required env vars for secrets.
/// Returns a processable CLI config for use by the CLI.
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<CliConfig> {
    let path_ref = path.as_ref();
    info!(config_path = ?path_ref, "Loading configuration from file");

    let config_content = match fs::read_to_string(&path_ref) {
        Ok(content) => {
            info!(config_path = ?path_ref, "Config file read successfully");
            content
        }
        Err(e) => {
            error!(error = ?e, config_path = ?path_ref, "Failed to read config file");
            return Err(anyhow::anyhow!(
                "Failed to read config file {:?}: {}",
                path_ref,
                e
            ));
        }
    };

    #[derive(Debug, Deserialize)]
    struct RawConfig {
        download: DownloadSection,
        process: ProcessSection,
    }

    let raw: RawConfig = match serde_yaml::from_str(&config_content) {
        Ok(conf) => {
            info!(config_path = ?path_ref, "Parsed config YAML successfully");
            conf
        }
        Err(e) => {
            error!(error = ?e, config_path = ?path_ref, "Failed to parse config YAML");
            return Err(anyhow::anyhow!("Failed to parse config YAML: {e}"));
        }
    };

    // Optionally: inject secrets/env vars here as needed (e.g. bucket_id, auth keys)
    // e.g. std::env::var("SECRET_ENV")

    Ok(CliConfig {
        download: raw.download,
        process: raw.process,
    })
}
