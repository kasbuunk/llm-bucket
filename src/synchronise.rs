//! Coordinating module for download-process-upload pipeline.

use std::path::PathBuf;
use crate::code_to_pdf::{code_file_to_pdf, CodeToPdfError};
use tempfile;
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

/// Processor configuration - describes how the sources are processed into uploadable items.
pub struct ProcessConfig {
    pub kind: ProcessorKind,
}

#[derive(Debug, Clone)]
pub enum ProcessorKind {
    /// For each git source, outputs an external source with a single PDF (README.md converted)
    ReadmeToPDF,
    // Future: CodeToPDF, DirectoryToPDF, etc
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
    let source_for_upload = match process(&config.process, process_input) {
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
                resp
            }
            Err(e) => {
                eprintln!("[SYNC][ERROR][UPLOAD] create_item (external item) failed for file={}: {:?}", ext_item.filename, e);
                return Err(format!("[UPLOAD fail @ create_item for file={}]: {e:?}", ext_item.filename));
            }
        };
        if uploaded.processing_state != "Submitted" {
            eprintln!("[SYNC][ERROR][UPLOAD] Uploaded item's processing_state was not 'Submitted' for file={}: {:?}", ext_item.filename, uploaded.processing_state);
            return Err(format!(
                "[UPLOAD fail @ create_item post-state: file={}] Uploaded item's processing_state was not 'Submitted': {:?}",
                ext_item.filename, uploaded.processing_state
            ));
        }
    }
    println!("[SYNC] Synchronise pipeline succeeded.");
    Ok(())
}

// ---- Begin processing types and API ----


/// Input for processing step: a single source location (name, local path, etc)
pub struct ProcessInput {
    pub name: String,
    pub repo_path: PathBuf,
    // Add fields as needed
}

/// Output for processing: A source with items to be uploaded
pub struct ExternalSourceInput {
    pub name: String,
    pub external_items: Vec<ExternalItemInput>,
}

/// An item for upload: filename and content (e.g. PDF data)
pub struct ExternalItemInput {
    pub filename: String,
    pub content: Vec<u8>,
}

#[derive(Debug)]
pub enum ProcessError {
    Io(std::io::Error),
    NoReadme,
    Other(String),
}

impl From<std::io::Error> for ProcessError {
    fn from(e: std::io::Error) -> Self {
        ProcessError::Io(e)
    }
}

/// Main processor function: for the kind specified in config, process this input and return a single source+items.
pub fn process(config: &ProcessConfig, input: ProcessInput) -> Result<ExternalSourceInput, ProcessError> {
    match config.kind {
        ProcessorKind::ReadmeToPDF => {
            let readme_path = input.repo_path.join("README.md");

            if !readme_path.exists() {
                return Err(ProcessError::NoReadme);
            }

            // Prepare a temp output file path for pdf generation
            let tmp_pdf = tempfile::NamedTempFile::new()
                .map_err(|e| ProcessError::Io(e))?;
            let tmp_pdf_path = tmp_pdf.path();

            // Call the code_to_pdf module (on-disk)
            code_file_to_pdf(&readme_path, tmp_pdf_path)
                .map_err(|e| match e {
                    CodeToPdfError::Io(e) => ProcessError::Io(e),
                    CodeToPdfError::EmptyInput => ProcessError::Other("PDF: Empty input".into()),
                    CodeToPdfError::Font(_) => ProcessError::Other("PDF: font error".into()),
                })?;

            // Read PDF as bytes
            let content = std::fs::read(tmp_pdf_path).map_err(ProcessError::Io)?;

            // Prepare the result structures
            let ext_item = ExternalItemInput {
                filename: "README.pdf".to_string(),
                content,
            };

            Ok(ExternalSourceInput {
                name: input.name,
                external_items: vec![ext_item],
            })
        }
        // Add more processor kinds as needed
    }
}
