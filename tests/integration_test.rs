// Integration test for llm-bucket
// This test sets up a Config with a public git source, runs download::run, and asserts output dir populated.

use llm_bucket::config::{Config, GitSource, SourceAction};
use std::fs;
use std::path::Path;

#[test]
fn test_download_populates_directory_table_driven() {
    struct TestCase {
        name: &'static str,
        config: Config,
        expected_subdir: String,
    }

    let repo_url = "https://github.com/kasbuunk/llm-bucket";
    let reference = "main";
    let output_dir = "test_output";
    let expected_subdir = format!(
        "git_{}_{}",
        repo_url.trim_start_matches("https://").trim_start_matches("http://"),
        reference
    ).replace('/', "_");

    let test_cases = vec![
        TestCase {
            name: "single public git repo",
            config: Config {
                output_dir: output_dir.into(),
                sources: vec![SourceAction::Git(GitSource {
                    repo_url: repo_url.into(),
                    reference: Some(reference.into()),
                })],
            },
            expected_subdir: expected_subdir.clone(),
        }
    ];

    for tc in test_cases {
        // Run download
        let result = llm_bucket::download::run(&tc.config);
        assert!(result.is_ok(), "{}: download::run() should succeed", tc.name);

        // Check for deterministic subdir
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
