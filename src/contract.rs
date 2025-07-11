#![allow(unused)]

//! # uploader: Universal interface for external source/item upload
//!
//! This module defines a single trait (`Uploader`) and concrete supporting types
//! for uploading external sources (e.g. repositories, document spaces)
//! and their items (files, documents) into a project knowledge bucket via
//! an external API, local system, or a mock/test implementation.
//!
//! ## Interface & Extensibility
//! - Implement the [`Uploader`] trait to create new upload clients (e.g. API, file-based).
//! - All methods are async, returning results and using boxed error types.
//! - Error handling is uniform: all API/caller errors return boxed trait objects.
//! - Meant for both production code and robust mocking in tests.
//!
//! ## Mocking & Testing
//! - The trait is annotated for `mockall` so consumers can generate deterministic mocks for unit/integration tests.
//!
//! ## Type Sources
//! - Request and response types (e.g., `NewExternalSource`, `ExternalSource`, `NewExternalItem`, `ExternalItem`) are plain data; see docs for field descriptions.
//!
//! ## Example Usage
//! - See the core binary crate or test suite for concrete implementors—API client, test-mock, etc.
//!
//! ## Adding New Upload Destinations
//! - Implement the trait for your destination.
//! - Ensure methods are infallible in their contract: convert all meaningful upstream errors to a boxed error.
//! - Return concrete, understandable error variants on user/config/connection issues.

use async_trait::async_trait;

use mockall::{automock, predicate::*};

/// Represents the bare minimum data needed to create an external source.
pub struct NewExternalSource<'a> {
    /// Human-readable name for the external source (e.g., the repository name).
    pub name: &'a str,
    /// The bucket this source belongs to.
    pub bucket_id: i32,
}

/// Represents the returned external source after creation.
#[derive(Clone)]
pub struct ExternalSource {
    pub bucket_id: i32,
    pub external_source_id: i32,
    pub external_source_name: String,
    pub updated_by: i32,
    pub updated_datetime: Option<String>,
}

/// Represents the minimal data needed to upload a new item (file/document) to a source.
pub struct NewExternalItem<'a> {
    /// The raw file contents, typically UTF-8 text.
    pub content: &'a str,
    /// URL that must identify the item uniquely (can be a VCS or filesystem URL).
    pub url: &'a str,
    /// The parent bucket id.
    pub bucket_id: i64,
    /// The id of the external source to which this item belongs.
    pub external_source_id: i64,
    /// Optional state for processing. (Leave unpopulated to use default.)
    pub processing_state: Option<&'a str>,
}

/// Represents the created/returned item.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExternalItem {
    pub content_hash: String,
    pub external_item_id: i64,
    pub external_source_id: i64,
    pub processing_state: String,
    pub state: String,
    pub updated_datetime: Option<String>,
    pub url: String,
}

/// Error type for Downloader trait (simple boxed error for now)
pub type DownloadError = Box<dyn std::error::Error + Send + Sync>;

/// Manifest returned from a download operation, describing exactly what was downloaded and where.
#[derive(Debug, Clone)]
pub struct DownloadedManifest {
    pub sources: Vec<DownloadedSource>,
}

/// Describes a successfully downloaded source in the manifest.
#[derive(Debug, Clone)]
pub struct DownloadedSource {
    /// Human-readable logical name (e.g., repo URL or space name)
    pub logical_name: String,
    /// Filesystem path to the downloaded/extracted source directory
    pub local_path: std::path::PathBuf,
    /// Original declared source action (for audit)
    pub original_source: crate::download::SourceAction,
}

/// Trait for downloading all sources as specified in configuration.
/// Allows plugging in real, test, or mockable downloaders (like with Uploader).
#[cfg_attr(any(test, feature = "test-export-mocks"), automock)]
#[async_trait]
pub trait Downloader: Send + Sync {
    /// Download all sources from the downloader's config into the configured output directory,
    /// returning a manifest of what was downloaded and where.
    async fn download_all(&self) -> Result<DownloadedManifest, DownloadError>;
}

/// Trait for uploading and managing external sources/items in a bucket.
/// The implementor is responsible for connecting to a backing service or storage API.
///
/// *NOTE:* This file acts as the *interface* only. Types referenced here
/// (e.g. NewExternalSource, ExternalSource, etc.) must be imported by
/// dependents from their public sources.
/// The trait is implemented by real clients and by test mocks.
///
/// The trait is `Send` + `Sync` + `'static` and intended for async/await usage.
#[cfg_attr(any(test, feature = "test-export-mocks"), automock)]
#[async_trait]
pub trait Uploader: Send + Sync {
    /// Create a new external source (such as a repository or a folder).
    async fn create_source<'a>(
        &self,
        req: NewExternalSource<'a>,
    ) -> Result<ExternalSource, Box<dyn std::error::Error + Send + Sync>>;

    /// Create a new item (such as a file) in an external source.
    ///
    /// Implementor is responsible for content handling and required API fields.
    async fn create_item<'a>(
        &self,
        req: NewExternalItem<'a>,
    ) -> Result<ExternalItem, Box<dyn std::error::Error + Send + Sync>>;

    /// Fetch a single external source by its ID.
    async fn get_source_by_id(
        &self,
        external_source_id: i32,
    ) -> Result<ExternalSource, Box<dyn std::error::Error + Send + Sync>>;

    /// Delete an external source by ID.
    async fn delete_source_by_id(
        &self,
        external_source_id: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Delete an external item by both external source and item ID.
    async fn delete_item_by_id(
        &self,
        external_source_id: i64,
        external_item_id: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// List all external sources for the bucket.
    async fn list_sources(
        &self,
    ) -> Result<Vec<ExternalSource>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Processor configuration - describes how the sources are processed into uploadable items.
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub kind: ProcessorKind,
}

/// Types/kinds of processing strategy.
#[derive(Debug, Clone)]
pub enum ProcessorKind {
    /// For each source, outputs a single PDF (README.md converted)
    ReadmeToPDF,
    /// Flattens all files in the repo, uploading them with directory encoded in name
    FlattenFiles,
    // Future: CodeToPDF, DirectoryToPDF, etc
}

impl From<&str> for ProcessorKind {
    fn from(s: &str) -> Self {
        match s {
            "ReadmeToPDF" | "readme_to_pdf" | "readme2pdf" => ProcessorKind::ReadmeToPDF,
            "FlattenFiles" | "flattenfiles" | "flatten_files" => ProcessorKind::FlattenFiles,
            other => {
                tracing::warn!(
                    kind = other,
                    "Unknown processor kind, defaulting to FlattenFiles"
                );
                ProcessorKind::FlattenFiles
            }
        }
    }
}

/// Input for processing step: a single source location (name, local path, etc)
#[derive(Debug, Clone)]
pub struct ProcessInput {
    pub name: String,
    pub repo_path: std::path::PathBuf,
    // Extend as needed
}

/// Output for processing: A source with items to be uploaded
#[derive(Debug, Clone)]
pub struct ExternalSourceInput {
    pub name: String,
    pub external_items: Vec<ExternalItemInput>,
}

/// An item for upload: filename and content (e.g. PDF data)
#[derive(Debug, Clone)]
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

/// Trait for preprocessing (used in synchronise orchestration).
/// Implemented by concrete processors and by mocks in testing.
#[automock]
#[async_trait]
pub trait Preprocessor: Send + Sync {
    /// Process an input source and return a processed external source with items, or error.
    async fn process(
        &self,
        config: &ProcessConfig,
        input: ProcessInput,
    ) -> Result<ExternalSourceInput, ProcessError>;
}
