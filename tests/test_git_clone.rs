// bucket-sync/tests/test_git_clone.rs

use std::path::PathBuf;
use tokio::runtime::Runtime;

use bucket_sync::config::{Config, GitSource, SourceAction};

// Placeholder for the clone/update function
async fn clone_or_update_git_repo(source: &GitSource, output_dir: &PathBuf) -> Result<(), String> {
    // Here, an actual clone or fetch implementation would go.
    // For now, just pretend it's successful.
    Ok(())
}

#[test]
fn test_clone_or_update_git_repo() {
    // Initialize async runtime
    let rt = Runtime::new().unwrap();

    // Configure the source with your public GitHub repo SSH URL
    let git_source = GitSource {
        repo_url: "git@github.com:kasbuunk/llm-bucket.git".to_string(),
        branch: "main".to_string(),
    };

    // Set the output directory (e.g., target directory within tests)
    let output_dir = PathBuf::from("target/test_clone_output");
    // Clean up if exists
    let _ = std::fs::remove_dir_all(&output_dir);

    // Prepare the config
    let config = Config {
        output_dir: output_dir.clone(),
        sources: vec![SourceAction::Git(git_source.clone())],
    };

    // Call the clone/update function with the first source
    let result = rt.block_on(async {
        match &config.sources[0] {
            SourceAction::Git(source) => clone_or_update_git_repo(source, &config.output_dir).await,
            // For other variants, handle accordingly
        }
    });

    assert!(
        result.is_ok(),
        "Clone or update the repo failed: {:?}",
        result
    );

    // Verify that the output directory exists and has content
    assert!(output_dir.exists(), "Output directory does not exist");
    let entries: Vec<_> = std::fs::read_dir(&output_dir).unwrap().collect();
    assert!(
        !entries.is_empty(),
        "Output directory is empty, clone may have failed"
    );
}
