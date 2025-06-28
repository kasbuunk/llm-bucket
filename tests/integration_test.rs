// Integration test for llm-bucket
// This test sets up a Config with a public git source, runs download::run, and asserts output dir populated.

use llm_bucket::config::{Config, GitSource, SourceAction};
use std::fs;
use std::path::Path;

#[test]
fn test_download_populates_directory() {
    // Always use the same output dir for the test
    let output_path = Path::new("test_output");

    // Test values for this source
    let repo_url = "https://github.com/kasbuunk/llm-bucket";
    let reference = "main";

    // Deterministic source subdir: git_github.com/kasbuunk/llm-bucket_main
    let source_dir_name = format!(
        "git_{}_{}",
        repo_url.trim_start_matches("https://").trim_start_matches("http://"),
        reference
    ).replace('/', "_");
    let full_source_path = output_path.join(&source_dir_name);

    // Construct a Config with one Git source (with Option<String> reference field)
    let config = Config {
        output_dir: output_path.into(),
        sources: vec![SourceAction::Git(GitSource {
            repo_url: repo_url.into(),
            reference: Some(reference.into()), // branch, tag, or commit
                                               // Add other GitSource fields here if they exist
        })],
    };

    // This call should fail to compile until the download module & function exist
    let result = llm_bucket::download::run(&config);
    assert!(result.is_ok(), "download::run() should succeed");

    // Check that the deterministic source directory exists within the output dir and isn't empty
    assert!(
        full_source_path.exists() && full_source_path.is_dir(),
        "Source subdirectory ('{}') should exist and be a directory",
        full_source_path.display()
    );

    let entries = fs::read_dir(&full_source_path).unwrap();
    let has_entries = entries.take(1).count() > 0;
    assert!(
        has_entries,
        "Source subdirectory ('{}') should contain content after download",
        full_source_path.display()
    );
}
