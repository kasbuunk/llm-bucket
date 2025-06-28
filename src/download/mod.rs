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
                    eprintln!("Error removing source subdir: {e}");
                    return Err(());
                }
            } else {
                // Ensure output dir exists for placing subdirectories
                if !Path::new(out_dir).exists() {
                    if let Err(e) = fs::create_dir_all(out_dir) {
                        eprintln!("Error creating output dir: {e}");
                        return Err(());
                    }
                }
            }

            // `git clone <repo_url> <full_source_path>`
            let status = Command::new("git")
                .arg("clone")
                .arg(repo_url)
                .arg(&full_source_path)
                .status();

            if let Ok(s) = status {
                if !s.success() {
                    eprintln!("git exited with code {}", s);
                    return Err(());
                }
            } else if let Err(e) = status {
                eprintln!("Failed to launch git: {e}");
                return Err(());
            }

            // After cloning, checkout the correct reference (branch, tag, or commit SHA)
            let checkout_status = Command::new("git")
                .arg("-C")
                .arg(&full_source_path)
                .arg("checkout")
                .arg(reference)
                .status();

            match checkout_status {
                Ok(s) if s.success() => continue,
                Ok(s) => {
                    eprintln!("git checkout exited with code {}", s);
                    return Err(());
                }
                Err(e) => {
                    eprintln!("Failed to launch git checkout: {e}");
                    return Err(());
                }
            }
        } else {
            eprintln!("Source type not supported yet by download::run");
            return Err(());
        }
    }
    Ok(())
}
