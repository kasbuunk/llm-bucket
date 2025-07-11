use llm_bucket_core::contract::{ProcessConfig, ProcessInput, ProcessorKind};
use llm_bucket_core::preprocess::process;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_process_readmetopdf_single_source_to_pdf_item() {
    // Setup: Create fake repo dir with a README.md
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().to_path_buf();
    let readme_path = repo_path.join("README.md");
    {
        let mut readme = File::create(&readme_path).unwrap();
        writeln!(readme, "# Test\nHello world!").unwrap();
    }

    let process_input = ProcessInput {
        name: "test_repo".to_string(),
        repo_path: repo_path.clone(),
    };
    let process_config = ProcessConfig {
        kind: ProcessorKind::ReadmeToPDF,
    };

    let out_source = process(&process_config, process_input).expect("Should succeed");

    assert_eq!(out_source.external_items.len(), 1, "One item: README.pdf");
    let item = &out_source.external_items[0];
    assert_eq!(item.filename, "README.pdf");
    assert!(
        item.content.len() > 100,
        "Content should be a non-empty PDF"
    );
}
