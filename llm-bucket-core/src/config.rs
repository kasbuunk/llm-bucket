use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub output_dir: PathBuf,
    pub sources: Vec<crate::download::SourceAction>,
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
