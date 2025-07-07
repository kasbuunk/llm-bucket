#![doc = "Uploader integration for CLI and core: bridges trait abstraction to actual API client, facilitating upload of sources/items to external endpoint."]
//
//! # Uploader Integration (CLI <-> Core)
//!
//! This module provides the bridge between the CLI workflow and the upload API abstraction in
//! [`llm-bucket-core::uploader`]. It wires up the `Uploader` trait for real use against a remote
//! API (the ChatNS backend), and provides the `LLMClient` used by the CLI for networked uploads.
//!
//! - Use this module to implement or invoke uploading logic in the main CLI binary.
//! - The [`Uploader`] trait is designed for async and testable usage; see core docs for API contract.
//! - See also: [llm-bucket-core::uploader] for trait, types, and mock/test behavior.
//!
//! ## Client Usage
//!
//! - Construct [`LLMClient`] using environment variables (`BUCKET_ID`, `OCP_APIM_SUBSCRIPTION_KEY`).
//! - Use trait methods for end-to-end upload (create_source, create_item, list_sources, etc.)
//! - All transport, serialization, and error handling are encapsulated in the client implementation.
//!
//! For full trait documentation and item/source contract, see core's [`uploader`] module.

use async_trait::async_trait;

/// Abstraction for uploading sources (repositories) and items (files) to the ChatNS API backend.
/// Designed for asynchronous usage and easy mocking.
///
/// The implementation will handle the server URL and Ocp-Apim-Subscription-Key.
/// The trait itself is agnostic of authentication and transport details.
pub use llm_bucket_core::uploader::{
    ExternalItem, ExternalSource, NewExternalItem, NewExternalSource,
};

/// Trait for uploading and managing sources and items asynchronously.
use llm_bucket_core::uploader::Uploader;

use std::env;

// Use generated openapi-client crate
use openapi::apis::configuration::{ApiKey, Configuration};
use openapi::apis::external_api::{
    create_external_source_v1_buckets_bucket_id_external_sources_post,
    delete_external_source_v1_buckets_bucket_id_external_sources_external_sou,
    get_external_source_by_id_v1_buckets_bucket_id_external_sources_external,
    get_external_sources_for_bucket_v1_buckets_bucket_id_external_sources_get,
    CreateExternalSourceV1BucketsBucketIdExternalSourcesPostError,
};
use openapi::models::CreateExternalSource;

pub struct LLMClient {
    conf: Configuration,
    bucket_id: i64,
}

impl LLMClient {
    pub fn new_from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        dotenvy::dotenv().ok(); // loads environment variables from .env if present
        match (env::var("OCP_APIM_SUBSCRIPTION_KEY"), env::var("BUCKET_ID")) {
            (Ok(api_key), Ok(bucket_id_raw)) => {
                let bucket_id = bucket_id_raw.parse::<i64>().map_err(|e| {
                    tracing::error!(error = ?e, raw = %bucket_id_raw, "Failed to parse BUCKET_ID from env");
                    e
                })?;
                let mut conf = Configuration::default();
                conf.api_key = Some(ApiKey {
                    prefix: None,
                    key: api_key.clone(),
                });
                tracing::info!(
                    api_key_set = api_key.len() > 0,
                    bucket_id,
                    "Initialized LLMClient from environment"
                );
                Ok(LLMClient { conf, bucket_id })
            }
            (Err(e), _) => {
                tracing::error!(error = ?e, "OCP_APIM_SUBSCRIPTION_KEY missing in environment");
                Err(Box::new(e))
            }
            (_, Err(e)) => {
                tracing::error!(error = ?e, "BUCKET_ID missing in environment");
                Err(Box::new(e))
            }
        }
    }
}

#[async_trait]
impl Uploader for LLMClient {
    async fn create_source<'a>(
        &self,
        req: NewExternalSource<'a>,
    ) -> Result<ExternalSource, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            bucket_id = req.bucket_id,
            source_name = req.name,
            "Uploading new external source"
        );
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
            Ok(api_src) => {
                tracing::info!(
                    source_id = api_src.external_source_id,
                    "Successfully created external source"
                );
                Ok(ExternalSource {
                    bucket_id: api_src.bucket_id,
                    external_source_id: api_src.external_source_id,
                    external_source_name: api_src.external_source_name,
                    updated_by: api_src.updated_by,
                    updated_datetime: api_src.updated_datetime,
                })
            }
            Err(e) => {
                tracing::error!(error = ?e, "API error creating external source");
                Err(format!("API error: {e:?}").into())
            }
        }
    }

    async fn create_item<'a>(
        &self,
        req: NewExternalItem<'a>,
    ) -> Result<ExternalItem, Box<dyn std::error::Error + Send + Sync>> {
        use openapi::models::{CreateExternalItem as ApiNewItem, ProcessingState};
        use sha2::{Digest, Sha256};

        tracing::info!(
            file_url = req.url,
            external_source_id = req.external_source_id,
            bucket_id = req.bucket_id,
            "Uploading new external item"
        );

        // Compute a SHA256 content hash
        let content_hash = {
            let mut hasher = Sha256::new();
            hasher.update(req.content.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        // Parse processing_state (accepts Option<str> or just None)
        let api_processing_state = req
            .processing_state
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
        ).await;

        match api_result {
            Ok(api_item) => {
                tracing::info!(
                    external_item_id = api_item.external_item_id,
                    url = api_item.url,
                    state = ?api_item.state,
                    "Successfully uploaded external item"
                );
                Ok(ExternalItem {
                    content_hash: api_item.content_hash,
                    external_item_id: api_item.external_item_id as i64,
                    external_source_id: api_item.external_source_id as i64,
                    processing_state: format!("{:?}", api_item.processing_state),
                    state: format!("{:?}", api_item.state),
                    updated_datetime: api_item.updated_datetime,
                    url: api_item.url,
                })
            }
            Err(e) => {
                tracing::error!(error = ?e, url = req.url, "API error uploading external item");
                Err(Box::new(e))
            }
        }
    }

    async fn get_source_by_id(
        &self,
        external_source_id: i32,
    ) -> Result<ExternalSource, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(external_source_id, "Fetching external source by ID");
        let api_result = get_external_source_by_id_v1_buckets_bucket_id_external_sources_external(
            &self.conf,
            self.bucket_id as i32,
            external_source_id,
            None,
        )
        .await;

        match api_result {
            Ok(api_src) => {
                tracing::info!(
                    external_source_id = api_src.external_source_id,
                    "Fetched external source"
                );
                Ok(ExternalSource {
                    bucket_id: api_src.bucket_id,
                    external_source_id: api_src.external_source_id,
                    external_source_name: api_src.external_source_name,
                    updated_by: api_src.updated_by,
                    updated_datetime: api_src.updated_datetime,
                })
            }
            Err(e) => {
                tracing::error!(error = ?e, external_source_id, "Failed to fetch external source by ID");
                Err(Box::new(e))
            }
        }
    }

    async fn list_sources(
        &self,
    ) -> Result<Vec<ExternalSource>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            bucket_id = self.bucket_id,
            "Listing all external sources for bucket"
        );
        let api_results =
            get_external_sources_for_bucket_v1_buckets_bucket_id_external_sources_get(
                &self.conf,
                self.bucket_id as i32,
                None,
            )
            .await;

        match api_results {
            Ok(sources) => {
                tracing::info!(count = sources.len(), "Fetched all sources in bucket");
                Ok(sources
                    .into_iter()
                    .map(|api_src| ExternalSource {
                        bucket_id: api_src.bucket_id,
                        external_source_id: api_src.external_source_id,
                        external_source_name: api_src.external_source_name,
                        updated_by: api_src.updated_by,
                        updated_datetime: api_src.updated_datetime,
                    })
                    .collect())
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to list sources for bucket");
                Err(Box::new(e))
            }
        }
    }

    async fn delete_source_by_id(
        &self,
        external_source_id: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            bucket_id = self.bucket_id,
            external_source_id,
            "Deleting external source"
        );
        let res = delete_external_source_v1_buckets_bucket_id_external_sources_external_sou(
            &self.conf,
            self.bucket_id as i32,
            external_source_id,
            None,
        )
        .await;
        match res {
            Ok(_) => {
                tracing::info!(external_source_id, "Successfully deleted external source");
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = ?e, external_source_id, "Failed to delete external source");
                Err(format!("API error deleting external source: {e:?}").into())
            }
        }
    }

    async fn delete_item_by_id(
        &self,
        external_source_id: i64,
        external_item_id: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            bucket_id = self.bucket_id,
            external_source_id,
            external_item_id,
            "Deleting external item"
        );
        let res = openapi::apis::external_api::delete_external_item_v1_buckets_bucket_id_external_sources_external_sourc(
            &self.conf,
            self.bucket_id as i32,
            external_source_id as i32,
            external_item_id as i32,
            None,
        )
        .await;
        match res {
            Ok(_) => {
                tracing::info!(
                    external_item_id,
                    external_source_id,
                    "Successfully deleted external item"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = ?e, external_item_id, external_source_id, "Failed to delete external item");
                Err(format!("API error deleting external item: {e:?}").into())
            }
        }
    }
}
