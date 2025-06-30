//! Coordinating module for download-process-upload pipeline.

use crate::preprocess;
pub use preprocess::{
    ProcessConfig, ProcessorKind, ProcessInput, ExternalSourceInput, ExternalItemInput, ProcessError, process
};

use std::path::PathBuf;
extern crate tokio; // Use extern crate for runtime context
use crate::upload::Uploader; // trait for async upload calls

/// The top-level synchronise configuration.
pub struct SynchroniseConfig {
    pub download: DownloadConfig,
    pub process: ProcessConfig,
    pub upload: UploadConfig,
}

/// Download configuration - what sources to fetch and where.
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
pub struct UploadConfig {
    pub bucket_id: i64,
    pub api_key: Option<String>,
    // Extendable: API endpoint, user, etc
}

/// Entrypoint: synchronise the pipeline according to config.
pub fn synchronise(config: &SynchroniseConfig) -> Result<(), String> {
    // Step 1: Download
    for source in &config.download.sources {
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
                println!("[SYNC] Download succeeded for {:?}", source);
            },
            Err(e) => {
                eprintln!("[SYNC][ERROR] Download failed for {:?}: {:?}", source, e);
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
    let source_for_upload = match preprocess::process(&config.process, process_input) {
        Ok(src) => {
            println!("[SYNC] Processing succeeded: {} items", src.external_items.len());
            src
        },
        Err(e) => {
            eprintln!("[SYNC][ERROR] Process step failed: {:?}", e);
            return Err(format!("Process step failed: {:?}", e));
        }
    };

    // Step 3: Upload
    // Construct real uploader (uses env if api_key is None)
    let uploader = match &config.upload.api_key {
        Some(api_key) => {
            // Use custom config if desired (not implemented, stub uses env for now)
            println!("[SYNC] API key for upload provided ({} chars)", api_key.len());
            crate::upload::UploaderImpl::new_from_env()
        }
        None => {
            println!("[SYNC] No explicit API key for upload, using env/default.");
            crate::upload::UploaderImpl::new_from_env()
        },
    }
    .map_err(|e| {
        eprintln!("[SYNC][ERROR] Failed to construct uploader: {:?}", e);
        format!("Failed to construct uploader: {e:?}")
    })?;

    // Create new external source
    let new_source = crate::upload::NewExternalSource {
        name: &source_for_upload.name,
        bucket_id: config.upload.bucket_id as i32,
    };
    let rt = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("[SYNC][ERROR] Tokio runtime creation failed: {:?}", e);
            return Err(format!("Tokio RT error: {e:?}"));
        }
    };
    let ext_source =
        match rt.block_on(uploader.create_source(new_source)) {
            Ok(src) => {
                println!("[SYNC][UPLOAD] create_source succeeded: external_source_id={:?}", src.external_source_id);
                src
            }
            Err(e) => {
                eprintln!("[SYNC][ERROR][UPLOAD] create_source (external source) failed: {:?}", e);
                return Err(format!("[UPLOAD fail @ create_source]: {e:?}"));
            }
        };

    // Upload all items (should be only one for this processor)
    for ext_item in &source_for_upload.external_items {
        println!("[SYNC][UPLOAD] Preparing upload for file: {}", ext_item.filename);
        let content = String::from_utf8_lossy(&ext_item.content);
        let item_req = crate::upload::NewExternalItem {
            content: &content, // Re-upload as UTF-8 text (for test, but might need to change to raw binary for real PDF upload)
            url: &ext_item.filename, // Use the file name as URL for now
            bucket_id: config.upload.bucket_id as i64,
            external_source_id: ext_source.external_source_id as i64,
            processing_state: None,
        };
        let uploaded = match rt.block_on(uploader.create_item(item_req)) {
            Ok(resp) => {
                println!("[SYNC][UPLOAD] create_item succeeded: file={}, state={:?}", ext_item.filename, resp.processing_state);
                // Print full struct as JSON for debug
                match serde_json::to_string_pretty(&resp) {
                    Ok(json) => println!("[SYNC][UPLOAD][DEBUG] Uploaded ExternalItem as JSON:\n{}", json),
                    Err(e) => eprintln!("[SYNC][UPLOAD][DEBUG] Failed to serialize ExternalItem as JSON: {:?}", e),
                }
                resp
            }
            Err(e) => {
                eprintln!("[SYNC][ERROR][UPLOAD] create_item (external item) failed for file={}: {:?}", ext_item.filename, e);
                return Err(format!("[UPLOAD fail @ create_item for file={}]: {e:?}", ext_item.filename));
            }
        };
        // All upload steps completed for item: continue to next item (if any)

        if uploaded.processing_state != "Submitted" {
            eprintln!("[SYNC][ERROR][UPLOAD] Uploaded item's processing_state was not 'Submitted' for file={}: {:?}", ext_item.filename, uploaded.processing_state);
            return Err(format!(
                "[UPLOAD fail @ create_item post-state: file={}] Uploaded item's processing_state was not 'Submitted': {:?}",
                ext_item.filename, uploaded.processing_state
            ));
        }
    }
    // println!("[SYNC] Synchronise pipeline succeeded.");
    // Ok(())
    Ok(())
}
