#![allow(dead_code)]
//! Module for processing repo sources into uploadable items, e.g. converting README.md to PDF.

use llm_bucket_core::code_to_pdf::{code_file_to_pdf, CodeToPdfError};
use std::path::{Path, PathBuf};
use tempfile;
use tracing::{debug, error, info};

/// Processor configuration - describes how the sources are processed into uploadable items.
#[derive(Debug)]
pub struct ProcessConfig {
    pub kind: ProcessorKind,
}

#[derive(Debug, Clone)]
pub enum ProcessorKind {
    /// For each source, outputs a single PDF (README.md converted)
    ReadmeToPDF,
    /// Flattens all files in the repo, uploading them with directory encoded in name
    FlattenFiles,
    // Future: CodeToPDF, DirectoryToPDF, etc
}

/// Input for processing step: a single source location (name, local path, etc)
pub struct ProcessInput {
    pub name: String,
    pub repo_path: PathBuf,
    // Extend as needed
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
pub fn process(
    config: &ProcessConfig,
    input: ProcessInput,
) -> Result<ExternalSourceInput, ProcessError> {
    info!(processor = ?config.kind, name = input.name, "Starting processing for source");
    let result = match config.kind {
        ProcessorKind::ReadmeToPDF => process_readme_to_pdf(input),
        ProcessorKind::FlattenFiles => process_flatten_files(input),
        // Add more processor kinds as needed
    };
    match &result {
        Ok(ext) => info!(
            items = ext.external_items.len(),
            "Processing completed successfully"
        ),
        Err(e) => error!(error = ?e, "Processing failed"),
    };
    result
}

///
fn process_readme_to_pdf(input: ProcessInput) -> Result<ExternalSourceInput, ProcessError> {
    let readme_path = input.repo_path.join("README.md");
    debug!(repo_path = %input.repo_path.display(), "Looking for README.md in repo path");

    if !readme_path.exists() {
        error!(path = %readme_path.display(), "No README.md found in repository");
        return Err(ProcessError::NoReadme);
    }

    // Prepare a temp output file path for pdf generation
    let tmp_pdf = tempfile::NamedTempFile::new().map_err(|e| {
        error!(error = ?e, "Failed to create temp file for PDF output");
        ProcessError::Io(e)
    })?;
    let tmp_pdf_path = tmp_pdf.path();

    // Call the code_to_pdf module (on-disk)
    code_file_to_pdf(&readme_path, tmp_pdf_path)
        .map_err(|e| {
            match &e {
                CodeToPdfError::Io(err) => error!(path = %readme_path.display(), error = ?err, "IO error during PDF generation"),
                CodeToPdfError::EmptyInput => error!("Attempted PDF generation with empty input"),
                CodeToPdfError::Font(desc) => error!(desc = *desc, "Font error during PDF generation"),
            }
            match e {
                CodeToPdfError::Io(e) => ProcessError::Io(e),
                CodeToPdfError::EmptyInput => ProcessError::Other("PDF: Empty input".into()),
                CodeToPdfError::Font(_) => ProcessError::Other("PDF: font error".into()),
            }
        })?;

    // Read PDF as bytes
    let content = std::fs::read(tmp_pdf_path).map_err(|e| {
        error!(error = ?e, path = %tmp_pdf_path.display(), "Failed to read generated PDF from disk");
        ProcessError::Io(e)
    })?;

    // Prepare the result structures
    let ext_item = ExternalItemInput {
        filename: "README.pdf".to_string(),
        content,
    };

    info!(
        filename = "README.pdf",
        size = ext_item.content.len(),
        "Generated README.pdf from README.md"
    );
    Ok(ExternalSourceInput {
        name: input.name,
        external_items: vec![ext_item],
    })
}

/// Recursively flatten all files and output as items with "__" as directory separator.
fn process_flatten_files(input: ProcessInput) -> Result<ExternalSourceInput, ProcessError> {
    info!(path = %input.repo_path.display(), "Flattening files in repository");
    let mut external_items = Vec::new();
    let repo_path = &input.repo_path;
    let base_len = repo_path.components().count();

    fn visit_dir(
        dir: &std::path::Path,
        repo_path: &std::path::Path,
        results: &mut Vec<ExternalItemInput>,
    ) -> Result<(), ProcessError> {
        for entry_res in std::fs::read_dir(dir)? {
            let entry = entry_res?;
            let path = entry.path();
            if path.is_dir() {
                // Skip .git and target directories
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == ".git" || file_name == "target" {
                    debug!(path = %path.display(), "Skipping directory");
                    continue;
                }
                visit_dir(&path, repo_path, results)?;
            } else if path.is_file() {
                // compute a flat filename with "__" as a separator, with truncation logic
                let rel_path = path.strip_prefix(repo_path).unwrap();
                let mut segments: Vec<String> = Vec::new();
                for comp in rel_path.components() {
                    segments.push(comp.as_os_str().to_string_lossy().into_owned());
                }
                if segments.is_empty() {
                    continue;
                }
                let basename = segments.pop().unwrap();
                let mut joined: String;
                let max_len = 180;
                // Try to include as many trailing segments as possible
                let mut from = 0;
                loop {
                    joined = if segments.len() > from {
                        segments[from..].join("__") + "__" + &basename
                    } else {
                        basename.clone()
                    };
                    if joined.len() <= max_len || from >= segments.len() {
                        break;
                    }
                    from += 1;
                }
                let flat_name = joined;
                match std::fs::read(&path) {
                    Ok(content) => {
                        debug!(filename = %flat_name, size = content.len(), "Flattened file");
                        results.push(ExternalItemInput {
                            filename: flat_name,
                            content,
                        });
                    }
                    Err(e) => {
                        error!(error = ?e, path = %path.display(), "Failed to read file while flattening");
                        return Err(ProcessError::Io(e));
                    }
                }
            }
        }
        Ok(())
    }
    if let Err(e) = visit_dir(repo_path, repo_path, &mut external_items) {
        error!(error = ?e, "Error occurred during directory flattening");
        return Err(e);
    }

    info!(
        count = external_items.len(),
        "Completed flattening files in repository"
    );
    Ok(ExternalSourceInput {
        name: input.name,
        external_items,
    })
}
