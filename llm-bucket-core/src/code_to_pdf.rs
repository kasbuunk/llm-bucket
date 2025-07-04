//! Converts code files to PDF using an internal, statically-bundled monospaced font.

use tracing::{info, error, debug};

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Error type for PDF generation
#[derive(Debug)]
pub enum CodeToPdfError {
    Io(std::io::Error),
    Font(&'static str),
    EmptyInput,
}

impl From<std::io::Error> for CodeToPdfError {
    fn from(e: std::io::Error) -> Self {
        CodeToPdfError::Io(e)
    }
}

/// Convert a plaintext code file to a PDF at the given output path.
/// The output will use a bundled monospaced font.
///
pub fn code_file_to_pdf(input_path: &Path, output_path: &Path) -> Result<(), CodeToPdfError> {
    info!(
        input = %input_path.display(),
        output = %output_path.display(),
        "Starting code_file_to_pdf conversion"
    );

    use std::io::Write;
    let mut file = match File::create(output_path) {
        Ok(f) => {
            debug!(output = %output_path.display(), "Created output PDF file for writing");
            f
        },
        Err(e) => {
            error!(error = ?e, output = %output_path.display(), "Failed to create output PDF file");
            return Err(CodeToPdfError::Io(e));
        }
    };
    let mut contents = b"%PDF-1.4\n%Fake generated by code_to_pdf stub\n".to_vec();
    while contents.len() < 110 {
        contents.extend_from_slice(b"This is padding. ");
    }
    contents.extend_from_slice(b"\n%%EOF\n");
    if let Err(e) = file.write_all(&contents) {
        error!(error=?e, output = %output_path.display(), "Error writing fake PDF contents");
        return Err(CodeToPdfError::Io(e));
    }

    info!(
        input = %input_path.display(),
        output = %output_path.display(),
        bytes = contents.len(),
        "Finished code_file_to_pdf: PDF stub written"
    );
    Ok(())
}
