use dotenv;
use llm_bucket::upload::{LLMClient, NewExternalSource, Uploader};
use serial_test::serial;
use std::path::PathBuf;
use tempfile::tempdir;
use uuid::Uuid;

// Import the synchronise reporting structs directly
use llm_bucket::synchronise::{ExternalItemReport, ExternalSourceReport, SynchroniseReport};

// These are the *intended* configuration roots for the new synchronise pipeline:
use llm_bucket::preprocess::{ProcessConfig, ProcessorKind};
use llm_bucket::synchronise::{
    synchronise, // pipeline entrypoint
    DownloadConfig,
    GitSource,
    SourceAction,
    SynchroniseConfig,
    UploadConfig,
};

#[tokio::test]
#[serial]
async fn test_synchronise_readme_to_pdf_upload() {
    let temp_out = tempdir().unwrap();
    let output_dir = temp_out.path().to_path_buf();

    // Download config is for handling sources (git, etc)
    let download = DownloadConfig {
        output_dir: output_dir.clone(),
        sources: vec![
            // Minimal real public repo with a README.md expected
            SourceAction::Git(GitSource {
                repo_url: "git@github.com:kasbuunk/llm-bucket.git".to_string(),
                reference: None,
            }),
        ],
    };

    // Process config: specifies to only convert the README.md to PDF, for each source
    let process = ProcessConfig {
        kind: ProcessorKind::ReadmeToPDF,
    };

    // Load .env config before reading env vars
    dotenv::dotenv().ok();

    // Explicitly load credentials, panic (fail test) if missing
    let api_key = std::env::var("OCP_APIM_SUBSCRIPTION_KEY")
        .expect("OCP_APIM_SUBSCRIPTION_KEY env var must be set for integration test");
    let bucket_id = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID env var must be set for integration test")
        .parse::<i64>()
        .expect("BUCKET_ID must be an integer");

    // Upload config (explicit, never loads from env in synchronise)
    let upload = UploadConfig {
        bucket_id,
        api_key: Some(api_key),
        // ...add other upload parameters as needed
    };

    let config = SynchroniseConfig {
        download,
        process,
        upload,
    };

    // Run the synchronisation, expecting for each git source:
    // - one external source, with one item (README.pdf)
    let res = synchronise(&config).await;
    assert!(
        res.is_ok(),
        "Synchronise should succeed in ReadmeToPDF mode"
    );
    let report = res.expect("Synchronise should succeed and return a report");

    // Check that at least one source and item are present, and that ids/names are non-empty
    assert!(
        !report.sources.is_empty(),
        "At least one source should be reported"
    );
    for src in &report.sources {
        assert!(
            !src.items.is_empty(),
            "Each source should have at least one item"
        );
        assert!(src.source_id > 0, "Source id should be positive");
        assert!(
            !src.source_name.is_empty(),
            "Source name should not be empty"
        );
        for item in &src.items {
            assert!(item.item_id > 0, "Item id should be positive");
            assert!(!item.item_name.is_empty(), "Item name should not be empty");
        }
    }
}

#[tokio::test]
#[serial]
async fn test_empty_bucket_removes_all_sources() {
    // Use real LLMClient implementation for the test:
    let client = LLMClient::new_from_env().expect("Failed to create client from .env settings");

    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");

    // Arrange: create two sources to ensure there are sources to remove
    let _ = client
        .create_source(NewExternalSource {
            name: &format!("Empty Test Source 1 {}", Uuid::new_v4()),
            bucket_id,
        })
        .await
        .expect("Create source 1 failed");

    let _ = client
        .create_source(NewExternalSource {
            name: &format!("Empty Test Source 2 {}", Uuid::new_v4()),
            bucket_id,
        })
        .await
        .expect("Create source 2 failed");

    // Act: empty the bucket using the synchronise module (to be implemented)
    llm_bucket::synchronise::empty_bucket(&client)
        .await
        .expect("empty_bucket should succeed");

    // Assert: bucket should be empty
    let sources = client
        .list_sources()
        .await
        .expect("list_sources should succeed");
    assert!(
        sources.is_empty(),
        "Bucket should have no sources after empty_bucket"
    );
}

// Types now imported from llm_bucket::synchronise

#[tokio::test]
#[serial]
async fn test_synchronise_confluence_to_pdf_upload() {
    use llm_bucket::synchronise::ConfluenceSource;

    let temp_out = tempdir().unwrap();
    let output_dir = temp_out.path().to_path_buf();

    // Ensure environment is loaded so required test vars are available
    dotenv::dotenv().ok();

    // Configure test confluence space and base URL from env (must be set for this test!)
    let base_url = std::env::var("CONFLUENCE_BASE_URL")
        .expect("CONFLUENCE_BASE_URL env var must be set for integration test");
    let space_key = std::env::var("CONFLUENCE_SPACE_KEY")
        .expect("CONFLUENCE_SPACE_KEY env var must be set for integration test");

    let download = DownloadConfig {
        output_dir: output_dir.clone(),
        sources: vec![
            SourceAction::Confluence(ConfluenceSource {
                base_url,
                space_key,
            }),
        ],
    };

    let process = ProcessConfig {
        kind: ProcessorKind::FlattenFiles,
    };

    dotenv::dotenv().ok();

    let api_key = std::env::var("OCP_APIM_SUBSCRIPTION_KEY")
        .expect("OCP_APIM_SUBSCRIPTION_KEY env var must be set for integration test");
    let bucket_id = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID env var must be set for integration test")
        .parse::<i64>()
        .expect("BUCKET_ID must be an integer");

    let upload = UploadConfig {
        bucket_id,
        api_key: Some(api_key),
    };

    let config = SynchroniseConfig {
        download,
        process,
        upload,
    };

    // Run the synchronisation pipeline.
    let res = synchronise(&config).await;
    assert!(
        res.is_ok(),
        "Synchronise should succeed for Confluence source in ReadmeToPDF mode"
    );
    let report = res.expect("Synchronise should return a report");

    // Check that at least one source and item are present, similar to Git test
    assert!(
        !report.sources.is_empty(),
        "At least one source should be reported for Confluence"
    );
    for src in &report.sources {
        assert!(
            !src.items.is_empty(),
            "Each source should have at least one item (Confluence)"
        );
        assert!(src.source_id > 0, "Source id should be positive (Confluence)");
        assert!(
            !src.source_name.is_empty(),
            "Source name should not be empty (Confluence)"
        );
        for item in &src.items {
            assert!(item.item_id > 0, "Item id should be positive (Confluence)");
            assert!(
                !item.item_name.is_empty(),
                "Item name should not be empty (Confluence)"
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_synchronise_removes_existing_sources_before_upload() {
    use llm_bucket::upload::{LLMClient, NewExternalSource};

    dotenv::dotenv().ok();
    let api_key = std::env::var("OCP_APIM_SUBSCRIPTION_KEY")
        .expect("OCP_APIM_SUBSCRIPTION_KEY env var must be set for integration test");
    let bucket_id = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID env var must be set for integration test")
        .parse::<i64>()
        .expect("BUCKET_ID must be an integer");

    // Arrange: insert a dummy source before synchronising
    let client = LLMClient::new_from_env().expect("Failed to create LLMClient from .env");
    let dummy_name = format!("DUMMY-SOURCE-{}", uuid::Uuid::new_v4());
    let dummy_source = client
        .create_source(NewExternalSource {
            name: &dummy_name,
            bucket_id: bucket_id as i32,
        })
        .await
        .expect("Failed to create dummy source");

    // Ensure dummy source was created
    let sources = client
        .list_sources()
        .await
        .expect("list_sources should succeed");
    assert!(
        sources
            .iter()
            .any(|src| src.external_source_id == dummy_source.external_source_id),
        "Dummy source should exist before sync"
    );

    // Setup for synchronisation, make sure it will upload one source
    let temp_out = tempfile::tempdir().unwrap();
    let output_dir = temp_out.path().to_path_buf();

    let download = DownloadConfig {
        output_dir: output_dir.clone(),
        sources: vec![SourceAction::Git(GitSource {
            repo_url: "git@github.com:kasbuunk/llm-bucket.git".to_string(),
            reference: None,
        })],
    };

    let process = ProcessConfig {
        kind: ProcessorKind::ReadmeToPDF,
    };

    let upload = UploadConfig {
        bucket_id,
        api_key: Some(api_key),
    };

    let config = SynchroniseConfig {
        download,
        process,
        upload,
    };

    // Act: run synchronise (should empty first, then upload new one)
    let res = synchronise(&config).await;
    assert!(res.is_ok(), "Synchronise should succeed");
    let report = res.expect("Synchronise should return a report");

    // Assert: dummy source is gone, only new ones remain
    let sources_after = client
        .list_sources()
        .await
        .expect("list_sources should succeed after");
    assert!(
        sources_after
            .iter()
            .all(|src| src.external_source_id != dummy_source.external_source_id),
        "Dummy source should be removed after synchronise"
    );
    // At least one source must exist, and none are the dummy
    assert!(
        !sources_after.is_empty(),
        "At least one source (new) should remain after"
    );
}

/// NEW TEST: End-to-end: two sources (git + confluence) both present in report with non-empty items.
/// This is red until synchronise supports multiple sources.
#[tokio::test]
#[serial]
async fn test_synchronise_multiple_sources_reports_each_uploaded() {
    // Load environment (Confluence and Git/Upload credentials expected)
    dotenv::dotenv().ok();

    // Prepare output dir
    let temp_out = tempfile::tempdir().unwrap();
    let output_dir = temp_out.path().to_path_buf();

    // Git source
    let git_source = SourceAction::Git(GitSource {
        repo_url: "git@github.com:kasbuunk/llm-bucket.git".to_string(),
        reference: None,
    });

    // Confluence source (env settings must exist)
    let base_url = std::env::var("CONFLUENCE_BASE_URL").expect("CONFLUENCE_BASE_URL must be set");
    let space_key = std::env::var("CONFLUENCE_SPACE_KEY").expect("CONFLUENCE_SPACE_KEY must be set");
    // Limit page count for speed
    std::env::set_var("CONFLUENCE_PAGE_LIMIT", "2");

    let confluence_source = SourceAction::Confluence(llm_bucket::synchronise::ConfluenceSource {
        base_url,
        space_key,
    });

    // Process config
    let process = ProcessConfig {
        kind: ProcessorKind::FlattenFiles,
    };

    // Upload config (credentials mandatory)
    let api_key = std::env::var("OCP_APIM_SUBSCRIPTION_KEY")
        .expect("OCP_APIM_SUBSCRIPTION_KEY must be set for integration test");
    let bucket_id = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID must be set for integration test")
        .parse::<i64>()
        .expect("BUCKET_ID must be an integer");

    let upload = UploadConfig {
        bucket_id,
        api_key: Some(api_key),
    };

    // Synchronise config with both sources
    let config = SynchroniseConfig {
        download: DownloadConfig {
            output_dir,
            sources: vec![git_source, confluence_source],
        },
        process,
        upload,
    };

    // Run pipeline
    let result = synchronise(&config).await;
    assert!(result.is_ok(), "Synchronise should succeed for mixed sources");
    let report = result.expect("Synchronise should return a report");

    // Main assertion: both sources present and each has at least one item
    assert_eq!(
        report.sources.len(),
        2,
        "Should report one result for each input source (git + confluence)"
    );
    for (idx, src) in report.sources.iter().enumerate() {
        assert!(
            !src.items.is_empty(),
            "Source at index {} ({}) should have at least one item",
            idx, src.source_name
        );
    }
}

#[tokio::test]
#[serial]
async fn test_synchronise_flattenfiles_uploads_codebase_files() {
    let temp_out = tempdir().unwrap();
    let output_dir = temp_out.path().to_path_buf();

    let download = DownloadConfig {
        output_dir: output_dir.clone(),
        sources: vec![SourceAction::Git(GitSource {
            repo_url: "git@github.com:kasbuunk/llm-bucket.git".to_string(),
            reference: None,
        })],
    };

    let process = ProcessConfig {
        kind: ProcessorKind::FlattenFiles,
    };

    dotenv::dotenv().ok();

    let api_key = std::env::var("OCP_APIM_SUBSCRIPTION_KEY")
        .expect("OCP_APIM_SUBSCRIPTION_KEY env var must be set for integration test");
    let bucket_id = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID env var must be set for integration test")
        .parse::<i64>()
        .expect("BUCKET_ID must be an integer");

    let upload = UploadConfig {
        bucket_id,
        api_key: Some(api_key),
    };

    let config = SynchroniseConfig {
        download,
        process,
        upload,
    };

    let res = synchronise(&config).await;
    assert!(
        res.is_ok(),
        "Synchronise with FlattenFiles should succeed in end-to-end integration"
    );
    let report = res.expect("Synchronise should succeed and return a report");

    assert!(
        !report.sources.is_empty(),
        "At least one source should be reported for FlattenFiles"
    );
    for src in &report.sources {
        assert!(
            !src.items.is_empty(),
            "Each source should have at least one item (FlattenFiles)"
        );
        assert!(
            src.source_id > 0,
            "Source id should be positive in FlattenFiles"
        );
        assert!(
            !src.source_name.is_empty(),
            "Source name should not be empty in FlattenFiles"
        );
        for item in &src.items {
            assert!(
                item.item_id > 0,
                "Item id should be positive in FlattenFiles"
            );
            assert!(
                !item.item_name.is_empty(),
                "Item name should not be empty in FlattenFiles"
            );
        }
    }
}
