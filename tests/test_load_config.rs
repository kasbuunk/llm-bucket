use std::fs::write;
use std::env;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use serial_test::serial;

/// This test ensures that a static config plus required env vars produces a valid SynchroniseConfig.
#[tokio::test]
#[serial]
async fn test_load_config_success_injects_env_into_upload() {
    // Write a static config file with NO sensitive fields
    let config_yaml = r#"
download:
  output_dir: ./tmp/exports
  sources:
    - type: git
      repo_url: "https://github.com/example/repo.git"
      reference: main
process:
  kind: FlattenFiles
"#;
    let config_file = NamedTempFile::new().expect("temp file");
    write(config_file.path(), config_yaml).unwrap();

    // Set env vars as would be required for full config
    env::set_var("BUCKET_ID", "1234");
    env::set_var("OCP_APIM_SUBSCRIPTION_KEY", "top-secret-test-key");

    // Import the new config loader (to be implemented)
    let config = llm_bucket::load_config::load_config(config_file.path()).expect("Config should load");

    // Spot-check the merged (dynamic+static) result
    assert_eq!(config.download.output_dir, PathBuf::from("./tmp/exports"));
    assert_eq!(config.download.sources.len(), 1);
    let repo = match &config.download.sources[0] {
        llm_bucket::synchronise::SourceAction::Git(g) => g,
        llm_bucket::synchronise::SourceAction::Confluence(_) => panic!("Unexpected Confluence source in this test"),
    };
    assert_eq!(repo.repo_url, "https://github.com/example/repo.git");
    assert_eq!(repo.reference.as_deref(), Some("main"));

    // Upload config must come directly from environment
    assert_eq!(config.upload.bucket_id, 1234);
    assert_eq!(config.upload.api_key.as_deref(), Some("top-secret-test-key"));
}

/// This test ensures both git and confluence sources can be loaded when supported.
#[tokio::test]
#[serial]
async fn test_load_config_with_confluence_source() {
   let config_yaml = r#"
download:
 output_dir: ./tmp/exports
 sources:
   - type: git
     repo_url: "https://github.com/example/repo.git"
     reference: main
   - type: confluence
     base_url: "https://yourcompany.atlassian.net/wiki"
     space_key: "DOCS"
process:
 kind: FlattenFiles
"#;
   let config_file = NamedTempFile::new().expect("temp file");
   write(config_file.path(), config_yaml).unwrap();

   // Provide required env vars so we don't fail early
   env::set_var("BUCKET_ID", "5678");
   env::set_var("OCP_APIM_SUBSCRIPTION_KEY", "test-key-2");

   let config = llm_bucket::load_config::load_config(config_file.path())
       .expect("Config should load with Git and Confluence sources");

   assert_eq!(config.download.output_dir, PathBuf::from("./tmp/exports"));
   assert_eq!(config.download.sources.len(), 2);

   let mut found_git = false;
   let mut found_confluence = false;
   for src in &config.download.sources {
       match src {
           llm_bucket::synchronise::SourceAction::Git(g) => {
               found_git = true;
               assert_eq!(g.repo_url, "https://github.com/example/repo.git");
               assert_eq!(g.reference.as_deref(), Some("main"));
           }
           llm_bucket::synchronise::SourceAction::Confluence(c) => {
               found_confluence = true;
               assert_eq!(c.base_url, "https://yourcompany.atlassian.net/wiki");
               assert_eq!(c.space_key, "DOCS");
           }
       }
   }
   assert!(found_git, "Did not find expected Git source");
   assert!(found_confluence, "Did not find expected Confluence source");
}

/// This test ensures that missing required env vars makes the loader fail.
#[tokio::test]
#[serial]
async fn test_load_config_errors_on_missing_env() {
    let config_yaml = r#"
download:
  output_dir: ./tmp/exports
  sources:
    - type: git
      repo_url: "https://github.com/example/repo.git"
      reference: main
process:
  kind: ReadmeToPDF
"#;
    let config_file = NamedTempFile::new().expect("temp file");
    write(config_file.path(), config_yaml).unwrap();

    // Remove env vars to simulate missing secret scenario
    env::remove_var("BUCKET_ID");
    env::remove_var("OCP_APIM_SUBSCRIPTION_KEY");

    let err = llm_bucket::load_config::load_config(config_file.path()).unwrap_err();
    let msg = err.to_string();

    assert!(
        msg.contains("BUCKET_ID") || msg.contains("OCP_APIM_SUBSCRIPTION_KEY"),
        "Must error for missing env var, got: {msg}"
    );
}

/// This test ensures that if the config file is not valid YAML, load_config errors and reports as such.
#[tokio::test]
#[serial]
async fn test_load_config_errors_for_invalid_file() {
    let config_file = NamedTempFile::new().expect("temp file");
    write(config_file.path(), b"not-yaml: [:::").unwrap();

    // Provide env so we don't fail early
    env::set_var("BUCKET_ID", "111");
    env::set_var("OCP_APIM_SUBSCRIPTION_KEY", "invalid-but-present");

    let err = llm_bucket::load_config::load_config(config_file.path()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("parse") || msg.contains("YAML"),
        "Parse error expected, got: {msg}"
    );
}
