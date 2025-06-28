// Integration test for llm-bucket
// This test sets up a Config with a public git source, runs download::run, and asserts output dir populated.

use llm_bucket::config::{Config, GitSource, SourceAction};
use std::fs;
use std::path::Path;

struct TestCase {
    name: &'static str,
    config: Config,
    expected_subdir: String,
}

#[test]
fn test_download_populates_directory_table_driven() {
    // Test values for 'llm-bucket'
    let repo_url = "https://github.com/kasbuunk/llm-bucket";
    let reference = "main";
    let output_dir = "test_output";
    let expected_subdir_llm = format!(
        "git_{}_{}",
        repo_url.trim_start_matches("https://").trim_start_matches("http://"),
        reference
    ).replace('/', "_");

    // Test values for 'ai'
    let ai_repo_url = "https://github.com/kasbuunk/ai";
    let ai_reference = "main";
    let expected_subdir_ai = format!(
        "git_{}_{}",
        ai_repo_url.trim_start_matches("https://").trim_start_matches("http://"),
        ai_reference
    ).replace('/', "_");

    let test_cases = vec![
        TestCase {
            name: "single public git repo: llm-bucket",
            config: Config {
                output_dir: output_dir.into(),
                sources: vec![SourceAction::Git(GitSource {
                    repo_url: repo_url.into(),
                    reference: Some(reference.into()),
                })],
            },
            expected_subdir: expected_subdir_llm.clone(),
        },
        TestCase {
            name: "single public git repo: ai",
            config: Config {
                output_dir: output_dir.into(),
                sources: vec![SourceAction::Git(GitSource {
                    repo_url: ai_repo_url.into(),
                    reference: Some(ai_reference.into()),
                })],
            },
            expected_subdir: expected_subdir_ai.clone(),
        }
    ];

    for tc in test_cases {
        // Always clean output dir before running each case for isolation
        let _ = std::fs::remove_dir_all(output_dir);

        let result = llm_bucket::download::run(&tc.config);
        assert!(result.is_ok(), "{}: download::run() should succeed", tc.name);

        let full_source_path = Path::new(output_dir).join(&tc.expected_subdir);
        assert!(
            full_source_path.exists() && full_source_path.is_dir(),
            "{}: Source subdirectory ('{}') should exist and be a directory",
            tc.name,
            full_source_path.display()
        );
        let entries = fs::read_dir(&full_source_path).unwrap();
        let has_entries = entries.take(1).count() > 0;
        assert!(
            has_entries,
            "{}: Source subdirectory ('{}') should contain content after download",
            tc.name,
            full_source_path.display()
        );
    }
}

#[test]
fn test_download_empty_sources_no_error() {
    let output_dir = "test_output_empty_sources";
    let _ = std::fs::remove_dir_all(output_dir);
    let config = Config {
        output_dir: output_dir.into(),
        sources: vec![],
    };

    let result = llm_bucket::download::run(&config);
    assert!(result.is_ok(), "download::run() should succeed with empty sources");
}
