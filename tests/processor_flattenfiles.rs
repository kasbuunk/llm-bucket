use std::fs::{File, create_dir_all};
use std::io::Write;
use tempfile::tempdir;
use llm_bucket::preprocess::{ProcessConfig, ProcessorKind, ProcessInput, process};

#[test]
fn test_process_flattenfiles_flattens_recursively_with_double_underscore_separator() {
    // Setup: Create nested directory structure
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path().to_path_buf();
    let subdir = repo_path.join("src/module");
    create_dir_all(&subdir).unwrap();

    let file1_path = repo_path.join("root.txt");
    let file2_path = subdir.join("nested.md");

    // Write files
    {
        let mut f1 = File::create(&file1_path).unwrap();
        writeln!(f1, "hello root").unwrap();
        let mut f2 = File::create(&file2_path).unwrap();
        writeln!(f2, "hello nested").unwrap();
    }

    let process_input = ProcessInput {
        name: "test_flatten".to_string(),
        repo_path: repo_path.clone(),
    };
    let process_config = ProcessConfig {
        kind: ProcessorKind::FlattenFiles, // <-- This variant must now exist!
    };

    let out_source = process(&process_config, process_input).expect("Should succeed");

    // Should contain both files, flattened!
    assert_eq!(out_source.external_items.len(), 2);
    let filenames: Vec<_> = out_source.external_items.iter().map(|i| i.filename.as_str()).collect();
    assert!(filenames.contains(&"root.txt"));
    assert!(filenames.contains(&"src__module__nested.md"));

    // Content matches
    for item in &out_source.external_items {
        if item.filename == "root.txt" {
            assert!(std::str::from_utf8(&item.content).unwrap().contains("hello root"));
        } else if item.filename == "src__module__nested.md" {
            assert!(std::str::from_utf8(&item.content).unwrap().contains("hello nested"));
        } else {
            panic!("Unexpected filename {}", item.filename);
        }
    }
}

#[test]
fn test_flattenfiles_skips_dotgit_and_target_dirs() {
    use std::fs::{File, create_dir_all};
    use std::io::Write;
    use tempfile::tempdir;
    use llm_bucket::preprocess::{ProcessConfig, ProcessorKind, ProcessInput, process};

    let tmp = tempdir().unwrap();
    let repo_path = tmp.path();

    // Good file
    let file1_path = repo_path.join("keepme.txt");
    {
        let mut f = File::create(&file1_path).unwrap();
        writeln!(f, "should be present").unwrap();
    }

    // Nested good file
    let nested_dir = repo_path.join("src");
    create_dir_all(&nested_dir).unwrap();
    let file2_path = nested_dir.join("ok.rs");
    {
        let mut f = File::create(&file2_path).unwrap();
        writeln!(f, "include this too").unwrap();
    }

    // .git and target files
    let dotgit_dir = repo_path.join(".git/info");
    let target_dir = repo_path.join("target/deep");
    create_dir_all(&dotgit_dir).unwrap();
    create_dir_all(&target_dir).unwrap();

    let file3_path = dotgit_dir.join("config");
    let file4_path = target_dir.join("temp.obj");
    {
        File::create(&file3_path).unwrap();
        File::create(&file4_path).unwrap();
    }

    let process_input = ProcessInput {
        name: "test_flatten_skip_dotgit_target".to_string(),
        repo_path: repo_path.to_path_buf(),
    };
    let process_config = ProcessConfig {
        kind: ProcessorKind::FlattenFiles,
    };

    let out_source = process(&process_config, process_input).expect("Should succeed");
    let filenames: Vec<_> = out_source.external_items.iter().map(|i| i.filename.as_str()).collect();

    // Should only include non-dotgit/non-target files
    assert!(filenames.contains(&"keepme.txt"));
    assert!(filenames.contains(&"src__ok.rs"));

    assert!(!filenames.iter().any(|n| n.contains(".git")));
    assert!(!filenames.iter().any(|n| n.contains("target")));
}
