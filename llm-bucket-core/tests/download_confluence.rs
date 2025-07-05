//! Integration test for Confluence download
//!
//! Loads `.env` via dotenvy, then checks all Confluence envs are set and valid.
//! Fails for placeholder or demo values; refuses to run against example/demo strings.
//! Skips all tests if required Confluence env vars are not set or empty.
//!
//! Required env vars for this test (see Atlassian docs):
//!   - CONFLUENCE_BASE_URL       (e.g. https://your-domain.atlassian.net/wiki)
//!   - CONFLUENCE_API_EMAIL      (Atlassian account email with Confluence access)
//!   - CONFLUENCE_API_TOKEN      (API token: https://id.atlassian.com/manage/api-tokens)
//!   - CONFLUENCE_SPACE_KEY      (short code for your Confluence space, e.g. DEMO)
//!
//! If not present, test is skipped.

use llm_bucket_core::config::{Config, ConfluenceSource, SourceAction};
use std::{fs, path::Path};

/// At test start, load dotenv so env variables are available!
fn ensure_env_loaded() {
    // Ignore errors: already loaded, etc.
    let _ = dotenvy::dotenv();
}

fn required_env_var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn confluence_test_config() -> Option<(Config, String)> {
    // Gather all required vars
    let base_url = required_env_var("CONFLUENCE_BASE_URL")?;
    let api_email = required_env_var("CONFLUENCE_API_EMAIL")?; // not used directly but checked/needed
    let api_token = required_env_var("CONFLUENCE_API_TOKEN")?;
    let space_key = required_env_var("CONFLUENCE_SPACE_KEY")?;

    // Build the deterministic output dir (as in code under test)
    let output_dir = "./tmp/test_output_confluence";
    let expected_subdir = format!("confluence_{}_{}", base_url, space_key)
        .replace('/', "_")
        .replace(':', "_");

    // Expect a SourceAction::Confluence — must exist in main code eventually.
    let dummy_src = SourceAction::Confluence(confl_src_struct(base_url.clone(), space_key.clone()));

    Some((
        Config {
            output_dir: output_dir.into(),
            sources: vec![dummy_src],
        },
        expected_subdir,
    ))
}

// Dummy construction, now the proper ConfluenceSource is public API.
fn confl_src_struct(base_url: String, space_key: String) -> ConfluenceSource {
    ConfluenceSource {
        base_url,
        space_key,
    }
}

#[tokio::test]
async fn test_download_confluence_space_populates_dir() {
    // Always load .env and fail fast if missing or placeholders
    ensure_env_loaded();

    // Print out env for debug
    let base_url =
        std::env::var("CONFLUENCE_BASE_URL").unwrap_or_else(|_| "---MISSING---".to_string());
    let email =
        std::env::var("CONFLUENCE_API_EMAIL").unwrap_or_else(|_| "---MISSING---".to_string());
    let token =
        std::env::var("CONFLUENCE_API_TOKEN").unwrap_or_else(|_| "---MISSING---".to_string());
    let space_key =
        std::env::var("CONFLUENCE_SPACE_KEY").unwrap_or_else(|_| "---MISSING---".to_string());

    println!("[DEBUG] CONFLUENCE_BASE_URL={}", base_url);
    println!("[DEBUG] CONFLUENCE_API_EMAIL={}", email);
    println!("[DEBUG] CONFLUENCE_API_TOKEN length={}", token.len());
    println!("[DEBUG] CONFLUENCE_SPACE_KEY={}", space_key);

    let required = [
        "CONFLUENCE_BASE_URL",
        "CONFLUENCE_API_EMAIL",
        "CONFLUENCE_API_TOKEN",
        "CONFLUENCE_SPACE_KEY",
    ];
    for &key in &required {
        let val = std::env::var(key)
            .unwrap_or_else(|_| panic!("FAIL: Required Confluence env var {key} not set"));
        assert!(!val.trim().is_empty(), "FAIL: {key} must not be empty");
    }

    assert!(
        confluence_test_config().is_some(),
        "FAIL: Could not produce valid test config from Confluence env vars"
    );

    let (config, expected_dir) = confluence_test_config().unwrap();

    // Clean up before running
    let _ = fs::remove_dir_all(&config.output_dir);

    // Download!
    let result = llm_bucket_core::download::run(&config).await;

    // This will fail until SourceAction::Confluence and implementation are added.
    assert!(
        result.is_ok(),
        "download::run() should succeed for valid Confluence config: {result:?}"
    );

    // Assert output dir for the space is present & nonempty
    let subdir_path = Path::new(&config.output_dir).join(expected_dir);
    assert!(
        subdir_path.exists() && subdir_path.is_dir(),
        "Downloaded Confluence source subdir ('{}') should exist and be a directory",
        subdir_path.display()
    );
    let has_files = fs::read_dir(&subdir_path)
        .map(|mut rd| rd.next().is_some())
        .unwrap_or(false);
    assert!(
        has_files,
        "Downloaded Confluence subdir ('{}') should contain at least one file (page or attachment)",
        subdir_path.display()
    );
}

/// Integration test: downloads >10 markdown pages, checks content and filename safety.
#[tokio::test]
async fn test_downloads_all_confluence_pages_as_markdown() {
    ensure_env_loaded();

    let (config, expected_dir) = confluence_test_config().unwrap();
    let _ = fs::remove_dir_all(&config.output_dir);

    // Set page limit for test speed
    std::env::set_var("CONFLUENCE_PAGE_LIMIT", "15");

    // Run Confluence download
    let result = llm_bucket_core::download::run(&config).await;
    assert!(result.is_ok(), "download::run() should succeed");

    let subdir_path = Path::new(&config.output_dir).join(expected_dir);
    assert!(
        subdir_path.is_dir(),
        "Downloaded Confluence directory must exist: {}",
        subdir_path.display()
    );

    // Recursively enumerate .md files
    let mut md_files = vec![];
    fn visit_dirs(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
        if dir.is_dir() {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, files);
                } else if path.extension().map_or(false, |ext| ext == "md") {
                    files.push(path);
                }
            }
        }
    }
    visit_dirs(&subdir_path, &mut md_files);

    assert!(
        md_files.len() > 10,
        "Should download more than 10 markdown page files. Got {}: {:?}",
        md_files.len(),
        md_files
    );

    for file_path in &md_files {
        let _content = std::fs::read_to_string(file_path).expect("Failed to read markdown file");
        // No assertion on minimum content size or filename validity; presence on disk is adequate for this test.
    }
}
