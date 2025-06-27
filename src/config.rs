// bucket-sync/src/config.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub output_dir: PathBuf,
    pub sources: Vec<SourceAction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SourceAction {
    Git(GitSource),
    // Future variants for other source types can be added here
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitSource {
    pub repo_url: String,
    pub branch: String,
    // Authentication options can be added later
    // e.g., token, SSH key, etc.
}
