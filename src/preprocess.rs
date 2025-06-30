#![allow(dead_code)]
//! Module for processing repo sources into uploadable items, e.g. converting README.md to PDF.

use std::path::{Path, PathBuf};
use crate::code_to_pdf::{code_file_to_pdf, CodeToPdfError};
use tempfile;

/// Processor configuration - describes how the sources are processed into uploadable items.
pub struct ProcessConfig {
    pub kind: ProcessorKind,
}

#[derive(Debug, Clone)]
pub enum ProcessorKind {
    /// For each source, outputs a single PDF (README.md converted)
    ReadmeToPDF,
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
pub fn process(config: &ProcessConfig, input: ProcessInput) -> Result<ExternalSourceInput, ProcessError> {
    match config.kind {
        ProcessorKind::ReadmeToPDF => process_readme_to_pdf(input),
        // Add more processor kinds as needed
    }
}

fn process_readme_to_pdf(input: ProcessInput) -> Result<ExternalSourceInput, ProcessError> {
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
