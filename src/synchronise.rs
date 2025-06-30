//! Coordinating module for download-process-upload pipeline.

use futures::future::try_join_all;
use tracing::{info, error, debug};

use crate::preprocess;
pub use preprocess::{
    ProcessConfig, ProcessorKind, ProcessInput, ExternalSourceInput, ExternalItemInput, ProcessError, process
};

use std::path::PathBuf;
extern crate tokio; // Use extern crate for runtime context
use crate::upload::Uploader; // trait for async upload calls

/// The top-level synchronise configuration.
#[derive(Debug)]
pub struct SynchroniseConfig {
    pub download: DownloadConfig,
    pub process: ProcessConfig,
    pub upload: UploadConfig,
}

/// Download configuration - what sources to fetch and where.
#[derive(Debug)]
pub struct DownloadConfig {
    pub output_dir: PathBuf,
    pub sources: Vec<SourceAction>,
}

#[derive(Debug, Clone)]
pub enum SourceAction {
    Git(GitSource),
    // Extendable for other source types
}

#[derive(Debug, Clone)]
pub struct GitSource {
    pub repo_url: String,
    pub reference: Option<String>,
    // Extendable (token, ssh, etc)
}

/// Upload configuration - where/how to upload the processed result.
#[derive(Debug)]
pub struct UploadConfig {
    pub bucket_id: i64,
    pub api_key: Option<String>,
    // Extendable: API endpoint, user, etc
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

pub async fn synchronise(config: &SynchroniseConfig) -> Result<SynchroniseReport, String> {
    // Step 0: Empty the bucket before doing anything else.
    let uploader = match &config.upload.api_key {
        Some(api_key) => {
            info!(api_key_len = api_key.len(), "[SYNC] API key for upload provided");
            crate::upload::LLMClient::new_from_env()
        }
        None => {
            info!("[SYNC] No explicit API key for upload, using env/default.");
            crate::upload::LLMClient::new_from_env()
        },
    }
    .map_err(|e| {
        error!(error = ?e, "[SYNC][ERROR] Failed to construct uploader");
        format!("Failed to construct uploader: {e:?}")
    })?;

    info!("[SYNC] Starting full synchronisation pipeline");

    if let Err(e) = empty_bucket(&uploader).await {
        error!(error = ?e, "[SYNC][ERROR] Failed to empty bucket before sync");
        return Err(format!("Failed to empty bucket before sync: {e:?}"));
    }
    info!("[SYNC] Emptied bucket before sync");

    // Step 1: Download
    for source in &config.download.sources {
        info!(source = ?source, "[SYNC] Starting download for source");
        let dl_config = crate::config::Config {
            output_dir: config.download.output_dir.clone(),
            sources: vec![match source {
                SourceAction::Git(g) => crate::config::SourceAction::Git(crate::config::GitSource {
                    repo_url: g.repo_url.clone(),
                    reference: g.reference.clone(),
                }),
            }],
        };
        match crate::download::run(&dl_config) {
            Ok(_) => {
                info!(source = ?source, "[SYNC] Download succeeded");
            },
            Err(e) => {
                error!(source = ?source, error = ?e, "[SYNC][ERROR] Download failed");
                return Err(format!("Download failed for {:?}: {:?}", source, e));
            }
        }
    }

    // For now, only handle singleton list of one repo/source
    let source = config.download.sources.get(0).ok_or("No sources specified")?;
    let (name, local_path) = match source {
        SourceAction::Git(git) => {
            // Reconstruct actual cloned directory name logic from download/run
            let reference = git.reference.as_deref().unwrap_or("main");
            let dir_name = format!("git_{}_{}", git.repo_url, reference)
                .replace('/', "_")
                .replace(':', "_");
            let full_path = config.download.output_dir.join(dir_name);
            (git.repo_url.clone(), full_path)
        }
    };

    // Step 2: Process (README to PDF)
    let process_input = ProcessInput {
        name: name.clone(),
        repo_path: local_path,
    };
    info!(repo_name = %name, "[SYNC] Invoking processing step (process README to PDF)");
    let source_for_upload = match preprocess::process(&config.process, process_input) {
        Ok(src) => {
            info!(items = src.external_items.len(), "[SYNC] Processing succeeded");
            src
        },
        Err(e) => {
            error!(error = ?e, "[SYNC][ERROR] Process step failed");
            return Err(format!("Process step failed: {:?}", e));
        }
    };

    // Step 3: Upload
    // Uploader already constructed above

    // Create new external source
    let new_source = crate::upload::NewExternalSource {
        name: &source_for_upload.name,
        bucket_id: config.upload.bucket_id as i32,
    };

    info!(source_name = %source_for_upload.name, "[SYNC][UPLOAD] Creating new external source");
    let ext_source =
        match uploader.create_source(new_source).await {
            Ok(src) => {
                info!(external_source_id = src.external_source_id, "[SYNC][UPLOAD] create_source succeeded");
                src
            }
            Err(e) => {
                error!(error = ?e, "[SYNC][ERROR][UPLOAD] create_source (external source) failed");
                return Err(format!("[UPLOAD fail @ create_source]: {e:?}"));
            }
        };

    // For accumulating upload responses
    let mut uploaded_items_report: Vec<ExternalItemReport> = Vec::new();

    // Upload all items, and record their IDs/names from upload responses
    for ext_item in &source_for_upload.external_items {
        info!(filename = %ext_item.filename, "[SYNC][UPLOAD] Preparing upload for file");
        let content = String::from_utf8_lossy(&ext_item.content);
        let item_req = crate::upload::NewExternalItem {
            content: &content, // Re-upload as UTF-8 text (for test, but might need to change to raw binary for real PDF upload)
            url: &ext_item.filename, // Use the file name as URL for now
            bucket_id: config.upload.bucket_id as i64,
            external_source_id: ext_source.external_source_id as i64,
            processing_state: None,
        };
        let uploaded = match uploader.create_item(item_req).await {
            Ok(resp) => {
                info!(file = %ext_item.filename, state = %resp.processing_state, "[SYNC][UPLOAD] create_item succeeded");
                // Print full struct as JSON for debug
                match serde_json::to_string_pretty(&resp) {
                    Ok(json) => debug!(json = %json, file = %ext_item.filename, "[SYNC][UPLOAD][DEBUG] Uploaded ExternalItem as JSON"),
                    Err(e) => error!(file = %ext_item.filename, error = ?e, "[SYNC][UPLOAD][DEBUG] Failed to serialize ExternalItem as JSON"),
                }
                // Add the ID and name to the report immediately
                uploaded_items_report.push(ExternalItemReport {
                    item_id: resp.external_item_id as i64,
                    item_name: ext_item.filename.clone(),
                });
                resp
            }
            Err(e) => {
                error!(file = %ext_item.filename, error = ?e, "[SYNC][ERROR][UPLOAD] create_item (external item) failed");
                return Err(format!("[UPLOAD fail @ create_item for file={}]: {e:?}", ext_item.filename));
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

    // Report includes actual uploaded IDs and names
    let sources_report = vec![
        ExternalSourceReport {
            source_id: ext_source.external_source_id as i64,
            source_name: ext_source.external_source_name.clone(),
            items: uploaded_items_report,
        }
    ];

    Ok(SynchroniseReport { sources: sources_report })
}

/// Removes all sources in the bucket using the given client. Public async API.
pub async fn empty_bucket<C>(client: &C) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    C: crate::upload::Uploader,
{
    let sources = client.list_sources().await?;
    let deletions = sources
        .into_iter()
        .map(|src| client.delete_source_by_id(src.external_source_id));
    // Try to delete all sources (fail fast)
    try_join_all(deletions).await?;
    Ok(())
}
