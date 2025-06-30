use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run(config: &crate::config::Config) -> Result<(), ()> {
    // Only supports Git sources for now.
    for source in &config.sources {
        if let crate::config::SourceAction::Git(git_source) = source {
            let repo_url = &git_source.repo_url;
            let reference = git_source.reference.as_deref().unwrap_or("main");
            let out_dir = &config.output_dir;

            // Build deterministic subdirectory for this git source:
            // use the full repo_url (including https:// or git@), replace / and : with _
            let source_dir_name = format!(
                "git_{}_{}",
                repo_url,
                reference
            )
            .replace('/', "_")
            .replace(':', "_");
            let full_source_path = Path::new(&out_dir).join(&source_dir_name);

            // If full_source_path exists, remove it for a clean clone
            if full_source_path.exists() {
                if let Err(e) = fs::remove_dir_all(&full_source_path) {
                    tracing::error!(
                        error = ?e,
                        path = %full_source_path.display(),
                        "Failed to remove existing source subdir"
                    );
                    return Err(());
                } else {
                    tracing::debug!(
                        path = %full_source_path.display(),
                        "Removed existing source subdir"
                    );
                }
            } else {
                // Ensure output dir exists for placing subdirectories
                if !Path::new(out_dir).exists() {
                    if let Err(e) = fs::create_dir_all(out_dir) {
                        tracing::error!(
                            error = ?e,
                            path = %Path::new(out_dir).display(),
                            "Failed to create output directory"
                        );
                        return Err(());
                    } else {
                        tracing::debug!(
                            path = %Path::new(out_dir).display(),
                            "Created output directory"
                        );
                    }
                }
            }

            // `git clone <repo_url> <full_source_path>`
            let status = Command::new("git")
                .arg("clone")
                .arg(repo_url)
                .arg(&full_source_path)
                .status();

            match status {
                Ok(s) if s.success() => {
                    tracing::info!(
                        repo_url = repo_url,
                        reference = reference,
                        path = %full_source_path.display(),
                        status = ?s,
                        "Successfully cloned git repository"
                    );
                },
                Ok(s) => {
                    tracing::error!(
                        repo_url = repo_url,
                        reference = reference,
                        path = %full_source_path.display(),
                        "Git exited with non-zero code: {}", s
                    );
                    return Err(());
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        repo_url = repo_url,
                        reference = reference,
                        path = %full_source_path.display(),
                        "Failed to launch git process"
                    );
                    return Err(());
                }
            }

            // After cloning, checkout the correct reference (branch, tag, or commit SHA)
            let checkout_status = Command::new("git")
                .arg("-C")
                .arg(&full_source_path)
                .arg("checkout")
                .arg(reference)
                .status();

            match checkout_status {
                Ok(s) if s.success() => {
                    tracing::info!(
                        repo_url = repo_url,
                        reference = reference,
                        path = %full_source_path.display(),
                        status = ?s,
                        "Checked out git reference"
                    );
                    continue
                }
                Ok(s) => {
                    tracing::error!(
                        repo_url = repo_url,
                        reference = reference,
                        path = %full_source_path.display(),
                        "Git checkout exited with non-zero code: {}", s
                    );
                    return Err(());
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        repo_url = repo_url,
                        reference = reference,
                        path = %full_source_path.display(),
                        "Failed to launch git checkout"
                    );
                    return Err(());
                }
            }
        } else {
            tracing::error!("Source type not supported yet by download::run");
            return Err(());
        }
    }
    tracing::info!("All sources successfully downloaded, exiting download::run with Ok");
    Ok(())
}
