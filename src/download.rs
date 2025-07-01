use std::fs;
use std::path::Path;
use std::process::Command;
use std::fs::File;
use std::io::Write;

pub async fn run(config: &crate::config::Config) -> Result<(), ()> {
    // Now supports Git and Confluence sources
    for source in &config.sources {
        match source {
            crate::config::SourceAction::Git(git_source) => {
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
            }
            crate::config::SourceAction::Confluence(confluence_source) => {
                use reqwest::Client;
                use std::fs;
                use tracing::{info, error};

                let base_url = confluence_source.base_url.trim_end_matches('/'); // avoid "//"
                let space_key = &confluence_source.space_key;
                let email =
                    std::env::var("CONFLUENCE_API_EMAIL").expect("CONFLUENCE_API_EMAIL missing");
                let token =
                    std::env::var("CONFLUENCE_API_TOKEN").expect("CONFLUENCE_API_TOKEN missing");

                let out_dir = &config.output_dir;
                let source_dir_name = format!(
                    "confluence_{}_{}",
                    base_url, space_key
                ).replace('/', "_").replace(':', "_");
                let full_source_path = Path::new(&out_dir).join(&source_dir_name);

                // Clean existing if present
                if full_source_path.exists() {
                    if let Err(e) = fs::remove_dir_all(&full_source_path) {
                        error!(error = ?e, path = %full_source_path.display(), "Failed to remove existing Confluence source subdir");
                        return Err(());
                    }
                }
                // Ensure output dir exists
                if !Path::new(out_dir).exists() {
                    if let Err(e) = fs::create_dir_all(out_dir) {
                        error!(error = ?e, path = %Path::new(out_dir).display(), "Failed to create output directory");
                        return Err(());
                    }
                }
                // Create source subdir
                if let Err(e) = fs::create_dir_all(&full_source_path) {
                    error!(error = ?e, path = %full_source_path.display(), "Failed to create Confluence source directory");
                    return Err(());
                }

                // Download the space (minimum: /rest/api/space/{spaceKey})
                let client = Client::new();

                let url = format!("{}/rest/api/space/{}", base_url, space_key);
                info!(url = %url, "Fetching Confluence space API");

                let response = client
                    .get(&url)
                    .basic_auth(email.clone(), Some(token.clone()))
                    .send()
                    .await;

                match response {
                    Ok(resp) => {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_else(|_| String::from("<Failed to decode response body>"));
                        if !status.is_success() {
                            error!(
                                status = %status,
                                url = %url,
                                email = %email,
                                "Confluence API returned error. Response body: {text}"
                            );
                            eprintln!(
                                "Confluence API error:\n  url: {url}\n  status: {status}\n  response_body: {text}"
                            );
                            return Err(());
                        }
                        let space_json_path = full_source_path.join("space.json");
                        let mut f = File::create(&space_json_path)
                            .map_err(|e| {
                                error!(error=?e, file=?space_json_path, "Failed to create space.json");
                                ()
                            })?;
                        f.write_all(text.as_bytes()).map_err(|e| {
                            error!(error=?e, file=?space_json_path, "Failed to write space.json");
                            ()
                        })?;
                        info!(path = %space_json_path.display(), "Downloaded Confluence space.json");
                        continue;
                    }
                    Err(e) => {
                        error!(error = ?e, url = %url, email = %email, "Failed to fetch Confluence API");
                        eprintln!("Failed to fetch Confluence API: {e}\nurl: {url}\nuser: {email}");
                        return Err(());
                    }
                }
            }
        }
    }
    tracing::info!("All sources successfully downloaded, exiting download::run with Ok");
    Ok(())
}
