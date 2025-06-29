use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

// Assume your public API looks like this and handles font management internally.
use llm_bucket::code_to_pdf::code_file_to_pdf;

#[test]
fn test_code_file_to_pdf_creates_valid_pdf() {
    // Prepare temp directory for input and output
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("test_code.rs");
    let output_path = dir.path().join("test_code.pdf");

    // Write some demo Rust code to input file
    let mut input_file = File::create(&input_path).unwrap();
    writeln!(input_file, "fn main() {{ println!(\"hi world\"); }}").unwrap();

    // Call conversion function. Font is handled by the module - minimal interface!
    code_file_to_pdf(&input_path, &output_path)
        .expect("PDF conversion failed");

    // Assert output was created and is non-empty
    let metadata = fs::metadata(&output_path).unwrap();
    assert!(
        metadata.len() > 100,
        "Output PDF is too small and may not exist"
    );

    // Optionally: scan first bytes for PDF signature
    let pdf_bytes = fs::read(&output_path).unwrap();
    assert_eq!(
        &pdf_bytes[0..4],
        b"%PDF",
        "PDF file missing magic header"
    );
}
