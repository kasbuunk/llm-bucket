use async_trait::async_trait;

/// Abstraction for uploading sources (repositories) and items (files) to the ChatNS API backend.
/// Designed for asynchronous usage and easy mocking.
///
/// The implementation will handle the server URL and Ocp-Apim-Subscription-Key.
/// The trait itself is agnostic of authentication and transport details.

/// Represents the bare minimum data needed to create an external source.
pub struct NewExternalSource<'a> {
    /// Human-readable name for the external source (e.g., the repository name).
    pub name: &'a str,
    /// The bucket this source belongs to.
    pub bucket_id: i32,
}

/// Represents the returned external source after creation.
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

use std::env;

// Use generated openapi-client crate
use openapi::apis::configuration::{ApiKey, Configuration};
use openapi::apis::external_api::{
    create_external_source_v1_buckets_bucket_id_external_sources_post,
    CreateExternalSourceV1BucketsBucketIdExternalSourcesPostError,
};
use openapi::models::CreateExternalSource;

pub struct UploaderImpl {
    conf: Configuration,
    bucket_id: i64,
}

impl UploaderImpl {
    pub fn new_from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        dotenvy::dotenv().ok(); // loads environment variables from .env if present
        let api_key = env::var("OCP_APIM_SUBSCRIPTION_KEY")?;
        let bucket_id = env::var("BUCKET_ID")?.parse::<i64>()?;
        let mut conf = Configuration::default();
        conf.api_key = Some(ApiKey {
            prefix: None,
            key: api_key,
        });
        Ok(UploaderImpl {
            conf,
            bucket_id,
        })
    }
}

#[async_trait]
impl Uploader for UploaderImpl {
    async fn create_source(
        &self,
        req: NewExternalSource<'_>,
    ) -> Result<ExternalSource, Box<dyn std::error::Error + Send + Sync>> {
        // Use the generated client and model type
        let body = CreateExternalSource {
            external_source_name: req.name.to_string(),
        };


        let result = create_external_source_v1_buckets_bucket_id_external_sources_post(
            &self.conf,
            req.bucket_id,
            None, // Let configuration supply API key, don't pass a redundant option
            Some(body),
        )
        .await;

        match result {
            Ok(api_src) => Ok(ExternalSource {
                bucket_id: api_src.bucket_id,
                external_source_id: api_src.external_source_id,
                external_source_name: api_src.external_source_name,
                updated_by: api_src.updated_by,
                updated_datetime: api_src.updated_datetime,
            }),
            Err(e) => Err(format!("API error: {e:?}").into()),
        }
    }

    async fn create_item(
        &self,
        req: NewExternalItem<'_>,
    ) -> Result<ExternalItem, Box<dyn std::error::Error + Send + Sync>> {
        use sha2::{Digest, Sha256};
        use openapi::models::{CreateExternalItem as ApiNewItem, ProcessingState};

        // Compute a SHA256 content hash
        let content_hash = {
            let mut hasher = Sha256::new();
            hasher.update(req.content.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        // Parse processing_state (accepts Option<str> or just None)
        let api_processing_state = req.processing_state
            .and_then(|s| serde_json::from_str::<ProcessingState>(&format!("\"{s}\"")).ok());

        // Build the API item payload
        let api_req = ApiNewItem {
            content: req.content.to_string(),
            content_hash: content_hash.clone(),
            url: req.url.to_string(),
            processing_state: api_processing_state,
        };

        let api_result = openapi::apis::external_api::create_external_item_v1_buckets_bucket_id_external_sources_external_sourc(
            &self.conf,
            req.bucket_id as i32,
            req.external_source_id as i32,
            None,
            Some(api_req),
        ).await?;

        Ok(ExternalItem {
            content_hash: api_result.content_hash,
            external_item_id: api_result.external_item_id as i64,
            external_source_id: api_result.external_source_id as i64,
            processing_state: format!("{:?}", api_result.processing_state),
            state: format!("{:?}", api_result.state),
            updated_datetime: api_result.updated_datetime,
            url: api_result.url,
        })
    }
}
