bucket-sync/src/upload/mod.rs
```
```rust
/// Abstraction for uploading sources (repositories) and items (files) to the ChatNS API backend.
/// Designed for asynchronous usage and easy mocking.
///
/// The implementation will handle the server URL and Ocp-Apim-Subscription-Key.
/// The trait itself is agnostic of authentication and transport details.
use async_trait::async_trait;

/// Represents the bare minimum data needed to create an external source.
pub struct NewExternalSource<'a> {
    /// Human-readable name for the external source (e.g., the repository name).
    pub name: &'a str,
    /// The bucket this source belongs to.
    pub bucket_id: i64,
}

/// Represents the returned external source after creation.
pub struct ExternalSource {
    pub bucket_id: i64,
    pub external_source_id: i64,
    pub external_source_name: String,
    pub updated_by: i64,
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
pub struct ExternalItem {
    pub content_hash: String,
    pub external_item_id: i64,
    pub external_source_id: i64,
    pub processing_state: String,
    pub state: String,
    pub updated_datetime: Option<String>,
    pub url: String,
}

/// Trait for uploading and managing sources and items asynchronously.
#[async_trait]
pub trait Uploader: Send + Sync {
    /// Create a new external source (repository, folder, etc).
    async fn create_source(
        &self,
        req: NewExternalSource<'_>,
    ) -> Result<ExternalSource, Box<dyn std::error::Error + Send + Sync>>;

    /// Create a new item (file) in an external source.
    ///
    /// Implementor is responsible for computing the content hash and filling all fields required by the API.
    async fn create_item(
        &self,
        req: NewExternalItem<'_>,
    ) -> Result<ExternalItem, Box<dyn std::error::Error + Send + Sync>>;
}
