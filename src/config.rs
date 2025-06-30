// bucket-sync/src/config.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, debug, error};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub output_dir: PathBuf,
    pub sources: Vec<SourceAction>,
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

#[derive(Debug, Serialize, Deserialize)]
pub enum SourceAction {
    Git(GitSource),
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
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
