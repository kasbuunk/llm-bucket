// Integration test for llm-bucket
// This test sets up a Config with a public git source, runs download::run, and asserts output dir populated.

use llm_bucket_core::config::Config;
use llm_bucket_core::download::{GitSource, SourceAction};
use std::fs;
use std::path::Path;

struct TestCase {
    name: &'static str,
    config: Config,
    expected_dirs: Vec<String>,
}

#[tokio::test]
async fn test_download_populates_directory_table_driven() {
    // Test values for 'llm-bucket'
    let repo_url = "https://github.com/kasbuunk/llm-bucket";
    let reference = "main";
    let output_dir = "./tmp/test_output";
    let expected_subdir_llm = format!("git_{}_{}", repo_url, reference)
        .replace('/', "_")
        .replace(':', "_");

    // Test values for 'ai'
    let ai_repo_url = "https://github.com/kasbuunk/ai";
    let ai_reference = "main";
    let expected_subdir_ai = format!("git_{}_{}", ai_repo_url, ai_reference)
        .replace('/', "_")
        .replace(':', "_");

    // Test values for private repo via SSH
    let private_ssh_url = "git@github.com:kasbuunk/private-repo-test.git";
    let private_reference = "main";
    // note: intentionally use same normalization logic for outputs as for https
    // this means the test expects directory based on path after host and branch
    let expected_subdir_private = format!("git_{}_{}", private_ssh_url, private_reference)
        .replace('/', "_")
        .replace(':', "_");

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
            expected_dirs: vec![expected_subdir_llm.clone()],
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
            expected_dirs: vec![expected_subdir_ai.clone()],
        },
        TestCase {
            name: "private git repo via SSH",
            config: Config {
                output_dir: output_dir.into(),
                sources: vec![SourceAction::Git(GitSource {
                    repo_url: private_ssh_url.into(),
                    reference: Some(private_reference.into()),
                })],
            },
            expected_dirs: vec![expected_subdir_private.clone()],
        },
        TestCase {
            name: "two sources: llm-bucket and ai",
            config: Config {
                output_dir: output_dir.into(),
                sources: vec![
                    SourceAction::Git(GitSource {
                        repo_url: repo_url.into(),
                        reference: Some(reference.into()),
                    }),
                    SourceAction::Git(GitSource {
                        repo_url: ai_repo_url.into(),
                        reference: Some(ai_reference.into()),
                    }),
                ],
            },
            expected_dirs: vec![expected_subdir_llm.clone(), expected_subdir_ai.clone()],
        },
        TestCase {
            name: "two refs in llm-bucket repo",
            config: Config {
                output_dir: output_dir.into(),
                sources: vec![
                    SourceAction::Git(GitSource {
                        repo_url: repo_url.into(),
                        reference: Some(reference.into()),
                    }),
                    SourceAction::Git(GitSource {
                        repo_url: repo_url.into(),
                        reference: Some("879e21e".into()),
                    }),
                ],
            },
            expected_dirs: vec![
                format!("git_{}_{}", repo_url, reference)
                    .replace('/', "_")
                    .replace(':', "_"),
                format!("git_{}_{}", repo_url, "879e21e")
                    .replace('/', "_")
                    .replace(':', "_"),
            ],
        },
    ];

    for tc in test_cases {
        // Always clean output dir before running each case for isolation
        let _ = std::fs::remove_dir_all(output_dir);

        let result = llm_bucket_core::download::run(&tc.config).await;
        assert!(
            result.is_ok(),
            "{}: download::run() should succeed",
            tc.name
        );

        // Assert all expected directories exist and are not empty
        for expected_dir in &tc.expected_dirs {
            let full_source_path = Path::new(output_dir).join(&expected_dir);
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
}

#[tokio::test]
async fn test_download_empty_sources_no_error() {
    let output_dir = "./tmp/test_output_empty_sources";
    let _ = std::fs::remove_dir_all(output_dir);
    let config = Config {
        output_dir: output_dir.into(),
        sources: vec![],
    };

    let result = llm_bucket_core::download::run(&config).await;
    assert!(
        result.is_ok(),
        "download::run() should succeed with empty sources"
    );
}
