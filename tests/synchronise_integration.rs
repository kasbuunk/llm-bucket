use std::path::PathBuf;
use tempfile::tempdir;
use dotenv;

// These are the *intended* configuration roots for the new synchronise pipeline:
use llm_bucket::synchronise::{
    SynchroniseConfig, DownloadConfig, UploadConfig, SourceAction, GitSource,
    synchronise, // pipeline entrypoint
};
use llm_bucket::preprocess::{ProcessConfig, ProcessorKind};

#[test]
fn test_synchronise_readme_to_pdf_upload() {
    let temp_out = tempdir().unwrap();
    let output_dir = temp_out.path().to_path_buf();

    // Download config is for handling sources (git, etc)
    let download = DownloadConfig {
        output_dir: output_dir.clone(),
        sources: vec![
            // Minimal real public repo with a README.md expected
            SourceAction::Git(
                GitSource {
                    repo_url: "git@github.com:kasbuunk/llm-bucket.git".to_string(),
                    reference: None,
                }
            ),
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
    let res = synchronise(&config);
    assert!(res.is_ok(), "Synchronise should succeed in ReadmeToPDF mode");

    // Optionally: check the number of uploaded external sources/items, or PDF presence on disk
    // (Requires real credentials and bucket, or a mocked uploader)
}

#[test]
fn test_synchronise_flattenfiles_uploads_codebase_files() {
    let temp_out = tempdir().unwrap();
    let output_dir = temp_out.path().to_path_buf();

    let download = DownloadConfig {
        output_dir: output_dir.clone(),
        sources: vec![
            SourceAction::Git(
                GitSource {
                    repo_url: "git@github.com:kasbuunk/llm-bucket.git".to_string(),
                    reference: None,
                }
            ),
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

    let res = synchronise(&config);
    assert!(res.is_ok(), "Synchronise with FlattenFiles should succeed in end-to-end integration");
    // Optionally: Future improvement—check output item count, or that select filenames are present.
}
