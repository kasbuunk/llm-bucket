//! High-level pipeline: orchestrates download → process → upload for repo sources.
//!
//! This module provides the top-level orchestration logic for "synchronising" all configured sources
//! as described in the loaded config. It implements a coordinated pipeline that:
//!   - Downloads each declared source (currently Git or Confluence) into a deterministic folder
//!   - Processes downloads using a specified processor (e.g., flatten files, README→PDF, etc.)
//!   - Uploads processed sources/items to a remote API/knowledge base via [`uploader::Uploader`]
//!   - Aggregates and returns a report of what succeeded and failed.
//!
//! # Major Types
//! - [`SynchroniseConfig`]: Bundles download and process config for a "run"
//! - [`SynchroniseReport`]: Output report with all uploaded sources/items for downstream audit etc
//!
//! # Responsibilities
//! - Atomic, fail-fast orchestration for a given config (if one source step fails, stops there)
//! - Invokes logging throughout for traceability (see tracing spans/events)
//! - Does not persist or mutate config files: all inputs are in-memory
//!
//! # Callable From
//! - Used by both the CLI crate and integration tests
//! - Expects a concrete (async) [`Uploader`] implementation for uploads
//!
//! # Extension Points
//! - To add a new source type: extend `SourceAction` and ensure downstream `match` arms are updated
//! - To add new pipeline logic (e.g., pre/post hooks or deduplication), inject at the orchestration step
//!
//! # Error Handling
//! Each failed step (download, process, upload) returns immediately with a formatted error; callers should log and surface these to users/test logs
//!
//! # Navigation
//! - Main entrypoint: [`synchronise`]
//! - Supporting types: [`SynchroniseConfig`], [`SynchroniseReport`].
//!

use futures::future::try_join_all;
use tracing::{debug, error, info};

use crate::config;
use crate::download;

use crate::preprocess;
pub use preprocess::{
    process, ExternalItemInput, ExternalSourceInput, ProcessConfig, ProcessError, ProcessInput,
    ProcessorKind,
};

use std::path::PathBuf;
extern crate tokio; // Use extern crate for runtime context
use crate::contract::Uploader; // use trait from core crate
use crate::download::{ConfluenceSource, GitSource, SourceAction};

/// The top-level synchronise configuration.
#[derive(Debug)]
pub struct SynchroniseConfig {
    pub download: crate::download::DownloadConfig,
    pub process: ProcessConfig,
}

/// Entrypoint: synchronise the pipeline according to config.
#[derive(Debug)]
pub struct SynchroniseReport {
    pub sources: Vec<ExternalSourceReport>,
}

#[derive(Debug)]
pub struct ExternalSourceReport {
    pub source_id: i64,
    pub source_name: String,
    pub items: Vec<ExternalItemReport>,
}

#[derive(Debug)]
pub struct ExternalItemReport {
    pub item_id: i64,
    pub item_name: String,
}

pub async fn synchronise<U>(
    config: &SynchroniseConfig,
    uploader: &U,
) -> Result<SynchroniseReport, String>
where
    U: Uploader + Sync,
{
    info!("[SYNC] Starting full synchronisation pipeline");

    if let Err(e) = empty_bucket(uploader).await {
        error!(error = ?e, "[SYNC][ERROR] Failed to empty bucket before sync");
        return Err(format!("Failed to empty bucket before sync: {e:?}"));
    }
    info!("[SYNC] Emptied bucket before sync");

    // Step 1-3: For each source, download, process, and upload; accumulate reports.
    let mut sources_report: Vec<ExternalSourceReport> = Vec::new();

    for source in &config.download.sources {
        // --- Step 1: Download ---
        info!(source = ?source, "[SYNC] Starting download for source");
        let dl_config = config::Config {
            output_dir: config.download.output_dir.clone(),
            sources: vec![match source {
                SourceAction::Git(g) => SourceAction::Git(GitSource {
                    repo_url: g.repo_url.clone(),
                    reference: g.reference.clone(),
                }),
                SourceAction::Confluence(c) => SourceAction::Confluence(ConfluenceSource {
                    base_url: c.base_url.clone(),
                    space_key: c.space_key.clone(),
                }),
            }],
        };
        match download::run(&dl_config).await {
            Ok(_) => {
                info!(source = ?source, "[SYNC] Download succeeded");
            }
            Err(e) => {
                error!(source = ?source, error = ?e, "[SYNC][ERROR] Download failed");
                return Err(format!("Download failed for {:?}: {:?}", source, e));
            }
        }

        // --- Step 2: Construct the source-specific processing input ---
        let (name, local_path) = match source {
            SourceAction::Git(git) => {
                let reference = git.reference.as_deref().unwrap_or("main");
                let dir_name = format!("git_{}_{}", git.repo_url, reference)
                    .replace('/', "_")
                    .replace(':', "_");
                let full_path = config.download.output_dir.join(dir_name);
                (git.repo_url.clone(), full_path)
            }
            SourceAction::Confluence(confluence) => {
                let dir_name = format!(
                    "confluence_{}_{}",
                    confluence.base_url, confluence.space_key
                )
                .replace('/', "_")
                .replace(':', "_");
                let full_path = config.download.output_dir.join(dir_name);
                (
                    format!("{}:{}", confluence.base_url, confluence.space_key),
                    full_path,
                )
            }
        };

        let process_input = ProcessInput {
            name: name.clone(),
            repo_path: local_path,
        };
        info!(repo_name = %name, "[SYNC] Invoking processing step (process README to PDF)");
        let source_for_upload = match preprocess::process(&config.process, process_input) {
            Ok(src) => {
                info!(
                    items = src.external_items.len(),
                    "[SYNC] Processing succeeded"
                );
                src
            }
            Err(e) => {
                error!(error = ?e, "[SYNC][ERROR] Process step failed");
                return Err(format!("Process step failed: {:?}", e));
            }
        };

        // --- Step 3: Upload ---
        // The caller must now provide bucket_id as part of the process input or config, not via uploader trait.
        // For now, assume the upload destination bucket_id must be discovered from a field attached to each source, or a global known constant for all uploads.
        // Here, we try to get it from an environment variable for minimal disruption - in a true refactor it would be upstream.
        let bucket_id: i32 = std::env::var("BUCKET_ID")
            .expect("BUCKET_ID env var must be set for uploader")
            .parse()
            .expect("BUCKET_ID must be an integer");
        let new_source = crate::contract::NewExternalSource {
            name: &source_for_upload.name,
            bucket_id,
        };

        info!(source_name = %source_for_upload.name, "[SYNC][UPLOAD] Creating new external source");
        let ext_source = match uploader.create_source(new_source).await {
            Ok(src) => {
                info!(
                    external_source_id = src.external_source_id,
                    "[SYNC][UPLOAD] create_source succeeded"
                );
                src
            }
            Err(e) => {
                error!(error = ?e, "[SYNC][ERROR][UPLOAD] create_source (external source) failed");
                return Err(format!("[UPLOAD fail @ create_source]: {e:?}"));
            }
        };

        let mut uploaded_items_report: Vec<ExternalItemReport> = Vec::new();

        // Upload all items, and record their IDs/names from upload responses
        for ext_item in &source_for_upload.external_items {
            info!(filename = %ext_item.filename, "[SYNC][UPLOAD] Preparing upload for file");
            let content = String::from_utf8_lossy(&ext_item.content);
            let item_req = crate::contract::NewExternalItem {
                content: &content, // Re-upload as UTF-8 text (for test, but might need to change to raw binary for real PDF upload)
                url: &ext_item.filename, // Use the file name as URL for now
                bucket_id: bucket_id as i64,
                external_source_id: ext_source.external_source_id as i64,
                processing_state: None,
            };
            let uploaded = match uploader.create_item(item_req).await {
                Ok(resp) => {
                    info!(file = %ext_item.filename, state = %resp.processing_state, "[SYNC][UPLOAD] create_item succeeded");
                    match serde_json::to_string_pretty(&resp) {
                        Ok(json) => {
                            debug!(json = %json, file = %ext_item.filename, "[SYNC][UPLOAD][DEBUG] Uploaded ExternalItem as JSON")
                        }
                        Err(e) => {
                            error!(file = %ext_item.filename, error = ?e, "[SYNC][UPLOAD][DEBUG] Failed to serialize ExternalItem as JSON")
                        }
                    }
                    uploaded_items_report.push(ExternalItemReport {
                        item_id: resp.external_item_id as i64,
                        item_name: ext_item.filename.clone(),
                    });
                    resp
                }
                Err(e) => {
                    error!(file = %ext_item.filename, error = ?e, "[SYNC][ERROR][UPLOAD] create_item (external item) failed");
                    return Err(format!(
                        "[UPLOAD fail @ create_item for file={}]: {e:?}",
                        ext_item.filename
                    ));
                }
            };

            if uploaded.processing_state != "Submitted" {
                error!(file = %ext_item.filename, state = %uploaded.processing_state, "[SYNC][ERROR][UPLOAD] Uploaded item's processing_state was not 'Submitted'");
                return Err(format!(
                    "[UPLOAD fail @ create_item post-state: file={}] Uploaded item's processing_state was not 'Submitted': {:?}",
                    ext_item.filename, uploaded.processing_state
                ));
            }
        }

        sources_report.push(ExternalSourceReport {
            source_id: ext_source.external_source_id as i64,
            source_name: ext_source.external_source_name.clone(),
            items: uploaded_items_report,
        });
    }

    Ok(SynchroniseReport {
        sources: sources_report,
    })
}

/// Removes all sources in the bucket using the given client. Public async API.
pub async fn empty_bucket<C>(client: &C) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    C: Uploader,
{
    let sources = client.list_sources().await?;
    let deletions = sources
        .into_iter()
        .map(|src| client.delete_source_by_id(src.external_source_id));
    // Try to delete all sources (fail fast)
    try_join_all(deletions).await?;
    Ok(())
}
