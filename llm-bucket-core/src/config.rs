// bucket-sync/src/config.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, debug, error};

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
