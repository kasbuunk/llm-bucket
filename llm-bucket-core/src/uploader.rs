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
