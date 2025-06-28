// Integration test for llm-bucket
// This test sets up a Config with a public git source, runs download::run, and asserts output dir populated.

use llm_bucket::config::{Config, GitSource, SourceAction};
use std::fs;
use std::path::Path;

#[test]
fn test_download_populates_directory() {
    // Always use the same output dir for the test
    let output_path = Path::new("test_output");

    // Construct a Config with one Git source (with Option<String> reference field)
    let config = Config {
        output_dir: output_path.into(),
        sources: vec![SourceAction::Git(GitSource {
            repo_url: "https://github.com/kasbuunk/llm-bucket".into(),
            reference: Some("main".into()), // branch, tag, or commit
                                            // Add other GitSource fields here if they exist
        })],
    };

    // This call should fail to compile until the download module & function exist
    let result = llm_bucket::download::run(&config);
    assert!(result.is_ok(), "download::run() should succeed");

    // Check that the output directory exists & isn't empty
    assert!(
        output_path.exists() && output_path.is_dir(),
        "Output directory should exist"
    );

    let entries = fs::read_dir(output_path).unwrap();
    let has_entries = entries.take(1).count() > 0;
    assert!(
        has_entries,
        "Output directory should contain content after download"
    );
}
