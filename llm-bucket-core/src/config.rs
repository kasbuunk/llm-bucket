//! Config module: loaded YAML config, source definitions, and tracing utilities
//!
//! This module defines the strongly-typed config contract and associated models for llm-bucket.
//! It maps YAML schemas (as shown in the README) to safe Rust types, guarded by serde validation,
//! and provides tracing helpers for structured diagnostics and auditing.
//!
//! # Main Components
//! - [`Config`]: The top-level struct for YAML parsing, representing the full config file.
//! - [`SourceAction`], [`GitSource`], [`ConfluenceSource`]: Per-source type blocks used in download/pipeline.
//! - Tracing: All structs have `trace_loaded()` methods to log structured config details at load-time.
//!
//! # Extension Points
//! - To add a new source, extend the `SourceAction` enum and update the deserialization contract
//!   (see comments—ensuring backwards/forwards compatibility).
//! - To validate or preprocess config on load, use `trace_loaded()` and insert further checks as needed.
//!
//! # Field Extension & Schema Safety
//! - It is safe to extend structs with new optional fields—be mindful of serde rename/tag rules for enums.
//! - Required changes to the YAML schema should be reflected in both this module and the example configs/documentation.
//!
//! # Example
//!
//! ```yaml
//! download:
//!   output_dir: ./output
//!   sources:
//!     - type: git
//!       repo_url: "https://github.com/example/repo.git"
//!       reference: main
//! process:
//!   kind: FlattenFiles
//! ```
//!
//! ```rust
//! // Rust usage
//! use llm_bucket_core::config::Config;
//! let config: Config = serde_yaml::from_str(&yaml_str)?;
//! config.trace_loaded();
//! ```
//!
//! # Q&A
//!
//! **Q:** How do I add a field to `Config` or `GitSource`?
//! **A:** Add the field to the struct with a sensible default if not required. Document in the top doc above.
//!
//! **Q:** How do I validate new fields on load?
//! **A:** Add extra checks/logs to `trace_loaded()` for the affected struct.
//!
//! **Q:** Will serde ignore unrecognized YAML from future config versions?
//! **A:** Yes, by default. Only known fields are loaded, extra YAML keys are ignored (if not set to `deny_unknown_fields`).
//!
//! For any config-format changes, always update this module and the documentation accordingly.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub output_dir: PathBuf,
    pub sources: Vec<SourceAction>,
}

/// Describes a Confluence download source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfluenceSource {
    pub base_url: String,
    pub space_key: String,
    // Add more fields as needed, e.g. parent_page, filters, etc.
}

impl Config {
    pub fn trace_loaded(&self) {
        info!(
            output_dir = %self.output_dir.display(),
            sources_count = self.sources.len(),
            "Loaded Config"
        );
        debug!(?self, "Config loaded (full debug)");
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SourceAction {
    Git(GitSource),
    Confluence(ConfluenceSource),
    // Future variants for other source types can be added here
}

impl SourceAction {
    pub fn trace_loaded(&self) {
        match self {
            SourceAction::Git(g) => {
                info!(
                    repo_url = %g.repo_url,
                    reference = g.reference.as_deref().unwrap_or("main"),
                    "Loaded Git SourceAction"
                );
            }
            SourceAction::Confluence(c) => {
                info!(
                    base_url = %c.base_url,
                    space_key = %c.space_key,
                    "Loaded Confluence SourceAction"
                );
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitSource {
    pub repo_url: String,
    pub reference: Option<String>, // branch, tag, or commit
                                   // Authentication options can be added later
                                   // e.g., token, SSH key, etc.
}

impl GitSource {
    pub fn trace_loaded(&self) {
        info!(
            repo_url = %self.repo_url,
            reference = self.reference.as_deref().unwrap_or("main"),
            "Loaded Git source"
        );
    }
}
