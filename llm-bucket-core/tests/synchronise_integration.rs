use llm_bucket_core::contract::{
    ExternalItem, ExternalSource, MockUploader, NewExternalItem, NewExternalSource,
};
use serial_test::serial;
use std::path::Path;
use tempfile::tempdir;

use llm_bucket_core::contract::Downloader;
use llm_bucket_core::download::{ConfluenceSource, DownloadConfig, GitSource, SourceAction};
use llm_bucket_core::preprocess::{ProcessConfig, ProcessorKind};
use llm_bucket_core::synchronise::{empty_bucket, synchronise};

fn ensure_env_loaded_from_workspace() {
    // Loads .env from the workspace root regardless of cwd.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let env_path = Path::new(&manifest_dir).join("../.env");
        let _ = dotenvy::from_path(env_path);
    }
}

#[tokio::test]
#[serial]
async fn test_synchronise_readme_to_pdf_upload() {
    ensure_env_loaded_from_workspace();
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
        kind: ProcessorKind::ReadmeToPDF,
    };

    // MockUploader configuration
    let mut uploader = MockUploader::new();

    // Synchronisation clears all existing sources first:
    uploader.expect_list_sources().return_once(|| Ok(vec![]));

    // No source to delete, so .expect_delete_source_by_id() is not required for this test.

    uploader
        .expect_create_source()
        .returning(|req: NewExternalSource<'_>| {
            Ok(ExternalSource {
                bucket_id: req.bucket_id,
                external_source_id: 1,
                external_source_name: req.name.to_owned(),
                updated_by: 1,
                updated_datetime: None,
            })
        });

    uploader
        .expect_create_item()
        .returning(|req: NewExternalItem<'_>| {
            Ok(ExternalItem {
                content_hash: "hash123".to_string(),
                external_item_id: 101,
                external_source_id: req.external_source_id,
                processing_state: "Submitted".to_string(),
                state: "active".to_string(),
                updated_datetime: None,
                url: req.url.to_owned(),
            })
        });

    let downloader = llm_bucket_core::download::DefaultDownloader::new(download);
    let manifest = downloader
        .download_all()
        .await
        .expect("Download should succeed");
    let res = synchronise(&process, &uploader, &manifest.sources).await;
    assert!(
        res.is_ok(),
        "Synchronise should succeed in ReadmeToPDF mode"
    );
    let report = res.expect("Synchronise should succeed and return a report");

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
    ensure_env_loaded_from_workspace();
    // Setup mock with two sources
    let mut uploader = MockUploader::new();
    let dummy_sources = vec![
        ExternalSource {
            bucket_id: 1,
            external_source_id: 101,
            external_source_name: "Dummy1".into(),
            updated_by: 1,
            updated_datetime: None,
        },
        ExternalSource {
            bucket_id: 1,
            external_source_id: 102,
            external_source_name: "Dummy2".into(),
            updated_by: 2,
            updated_datetime: None,
        },
    ];

    uploader
        .expect_list_sources()
        .return_once(move || Ok(dummy_sources.clone()));

    let mut delete_calls = vec![];
    uploader
        .expect_delete_source_by_id()
        .times(2)
        .returning(move |id| {
            delete_calls.push(id);
            Ok(())
        });

    // On second call to list_sources (after deletion), should be empty
    uploader.expect_list_sources().return_once(|| Ok(vec![]));

    empty_bucket(&uploader)
        .await
        .expect("empty_bucket should succeed");

    // There is no runtime assert here (since the mock will enforce .times(2) etc),
    // but if you want to double-check, you could assert something about `delete_calls` if setup differently.
}

#[tokio::test]
#[serial]
async fn test_synchronise_confluence_to_pdf_upload() {
    // NOTE: This test now uses a mock uploader and does not check real Confluence network access.
    ensure_env_loaded_from_workspace();
    let temp_out = tempdir().unwrap();
    let output_dir = temp_out.path().to_path_buf();

    // Load real Confluence credentials from env
    let base_url =
        std::env::var("CONFLUENCE_BASE_URL").expect("Missing CONFLUENCE_BASE_URL in env");
    let space_key =
        std::env::var("CONFLUENCE_SPACE_KEY").expect("Missing CONFLUENCE_SPACE_KEY in env");

    let download = DownloadConfig {
        output_dir: output_dir.clone(),
        sources: vec![SourceAction::Confluence(ConfluenceSource {
            base_url,
            space_key,
        })],
    };

    let process = ProcessConfig {
        kind: ProcessorKind::FlattenFiles,
    };

    let mut uploader = MockUploader::new();

    uploader.expect_list_sources().return_once(|| Ok(vec![]));

    uploader.expect_create_source().returning(|req| {
        Ok(ExternalSource {
            bucket_id: req.bucket_id,
            external_source_id: 200,
            external_source_name: req.name.to_string(),
            updated_by: 1,
            updated_datetime: None,
        })
    });

    uploader.expect_create_item().returning(|req| {
        Ok(ExternalItem {
            content_hash: "hash-c".into(),
            external_item_id: 211,
            external_source_id: req.external_source_id,
            processing_state: "Submitted".to_string(),
            state: "active".to_string(),
            updated_datetime: None,
            url: req.url.into(),
        })
    });

    let downloader = llm_bucket_core::download::DefaultDownloader::new(download);
    let manifest = downloader
        .download_all()
        .await
        .expect("Download should succeed");
    let res = synchronise(&process, &uploader, &manifest.sources).await;
    assert!(
        res.is_ok(),
        "Synchronise should succeed for Confluence source in ReadmeToPDF mode"
    );
    let report = res.expect("Synchronise should return a report");

    assert!(
        !report.sources.is_empty(),
        "At least one source should be reported for Confluence"
    );
    for src in &report.sources {
        assert!(
            !src.items.is_empty(),
            "Each source should have at least one item (Confluence)"
        );
        assert!(
            src.source_id > 0,
            "Source id should be positive (Confluence)"
        );
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
    ensure_env_loaded_from_workspace();
    let dummy_source_id = 9001;
    let new_source_id = 9002;
    let mut uploader = MockUploader::new();

    // Initial dummy source exists
    uploader.expect_list_sources().return_once(move || {
        Ok(vec![ExternalSource {
            bucket_id: 1,
            external_source_id: dummy_source_id,
            external_source_name: "Dummy Source".into(),
            updated_by: 1,
            updated_datetime: None,
        }])
    });

    uploader
        .expect_delete_source_by_id()
        .withf(move |id| *id == dummy_source_id)
        .return_once(|_| Ok(()));

    uploader.expect_create_source().return_once(move |req| {
        Ok(ExternalSource {
            bucket_id: req.bucket_id,
            external_source_id: new_source_id,
            external_source_name: req.name.to_owned(),
            updated_by: 2,
            updated_datetime: None,
        })
    });

    uploader.expect_create_item().return_once(|req| {
        Ok(ExternalItem {
            content_hash: "hashnew".to_string(),
            external_item_id: 2020,
            external_source_id: req.external_source_id,
            processing_state: "Submitted".to_string(),
            state: "active".to_string(),
            updated_datetime: None,
            url: req.url.to_owned(),
        })
    });

    // Second list_sources to verify dummy is gone (returns new one)
    uploader.expect_list_sources().return_once(move || {
        Ok(vec![ExternalSource {
            bucket_id: 1,
            external_source_id: new_source_id,
            external_source_name: "Uploads".into(),
            updated_by: 2,
            updated_datetime: None,
        }])
    });

    // Prepare synchronisation config
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
        kind: ProcessorKind::ReadmeToPDF,
    };
    let downloader = llm_bucket_core::download::DefaultDownloader::new(download);
    let manifest = downloader
        .download_all()
        .await
        .expect("Download should succeed");
    let report = synchronise(&process, &uploader, &manifest.sources)
        .await
        .expect("Synchronise should succeed");

    // Dummy source should be gone, only new one present
    assert!(
        report
            .sources
            .iter()
            .all(|src| src.source_id != dummy_source_id as i64),
        "Dummy source should be removed after synchronise"
    );
    assert!(
        !report.sources.is_empty(),
        "At least one source (new) should remain after"
    );
}

#[tokio::test]
#[serial]
async fn test_synchronise_multiple_sources_reports_each_uploaded() {
    ensure_env_loaded_from_workspace();
    let temp_out = tempdir().unwrap();
    let _output_dir = temp_out.path().to_path_buf();

    let git_source = SourceAction::Git(GitSource {
        repo_url: "git@github.com:kasbuunk/llm-bucket.git".to_string(),
        reference: None,
    });

    // Load real Confluence credentials from env
    let base_url =
        std::env::var("CONFLUENCE_BASE_URL").expect("Missing CONFLUENCE_BASE_URL in env");
    let space_key =
        std::env::var("CONFLUENCE_SPACE_KEY").expect("Missing CONFLUENCE_SPACE_KEY in env");

    let confluence_source = SourceAction::Confluence(ConfluenceSource {
        base_url,
        space_key,
    });

    let process = ProcessConfig {
        kind: ProcessorKind::FlattenFiles,
    };

    let mut uploader = MockUploader::new();

    // Empties all sources at the start
    uploader.expect_list_sources().return_once(|| Ok(vec![]));

    // .expect_create_source() for both sources
    let mut create_source_count = 0;
    uploader
        .expect_create_source()
        .times(2)
        .returning(move |req| {
            create_source_count += 1;
            Ok(ExternalSource {
                bucket_id: req.bucket_id,
                external_source_id: 1000 + create_source_count,
                external_source_name: req.name.to_owned(),
                updated_by: 1,
                updated_datetime: None,
            })
        });

    // .expect_create_item() for both sources (allow at least up to 3 due to potential extra file from flattening)
    let mut item_count = 0;
    uploader.expect_create_item().returning(move |req| {
        item_count += 1;
        Ok(ExternalItem {
            content_hash: format!("hash-{}", item_count),
            external_item_id: 2000 + item_count,
            external_source_id: req.external_source_id,
            processing_state: "Submitted".into(),
            state: "active".into(),
            updated_datetime: None,
            url: req.url.to_owned(),
        })
    });

    let tmp_outdir2 = tempdir().unwrap();
    let download = llm_bucket_core::download::DownloadConfig {
        output_dir: tmp_outdir2.path().to_path_buf(),
        sources: vec![git_source, confluence_source],
    };
    let downloader = llm_bucket_core::download::DefaultDownloader::new(download);
    let manifest = downloader
        .download_all()
        .await
        .expect("Download should succeed");
    let result = synchronise(&process, &uploader, &manifest.sources).await;
    assert!(
        result.is_ok(),
        "Synchronise should succeed for mixed sources"
    );
    let report = result.expect("Synchronise should return a report");

    assert_eq!(
        report.sources.len(),
        2,
        "Should report one result for each input source (git + confluence)"
    );
    for (idx, src) in report.sources.iter().enumerate() {
        assert!(
            !src.items.is_empty(),
            "Source at index {} ({}) should have at least one item",
            idx,
            src.source_name
        );
    }
}

#[tokio::test]
#[serial]
async fn test_synchronise_flattenfiles_uploads_codebase_files() {
    ensure_env_loaded_from_workspace();
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

    let mut uploader = MockUploader::new();

    uploader.expect_list_sources().return_once(|| Ok(vec![]));
    uploader.expect_create_source().return_once(|req| {
        Ok(ExternalSource {
            bucket_id: req.bucket_id,
            external_source_id: 501,
            external_source_name: req.name.to_owned(),
            updated_by: 6,
            updated_datetime: None,
        })
    });
    let mut create_item_calls = 0;
    uploader.expect_create_item().returning(move |req| {
        create_item_calls += 1;
        Ok(ExternalItem {
            content_hash: format!("flatten-hash-{}", create_item_calls),
            external_item_id: 502 + create_item_calls,
            external_source_id: req.external_source_id,
            processing_state: "Submitted".into(),
            state: "active".into(),
            updated_datetime: None,
            url: req.url.into(),
        })
    });

    let downloader = llm_bucket_core::download::DefaultDownloader::new(download);
    let manifest = downloader
        .download_all()
        .await
        .expect("Download should succeed");
    let res = synchronise(&process, &uploader, &manifest.sources).await;
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
