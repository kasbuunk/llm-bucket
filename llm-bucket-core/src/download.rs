use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Download configuration - what sources to fetch and where.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DownloadConfig {
    pub output_dir: PathBuf,
    pub sources: Vec<SourceAction>,
}

/// Selects the type of source for download (Git, Confluence, etc.)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SourceAction {
    Git(GitSource),
    Confluence(ConfluenceSource),
    // Extendable for other source types.
}

/// Describes a Confluence download source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfluenceSource {
    pub base_url: String,
    pub space_key: String,
    // Add more fields as needed, e.g. parent_page, filters, etc.
}

/// Describes a Git repository download source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitSource {
    pub repo_url: String,
    pub reference: Option<String>,
    // Extendable (token, ssh, etc)
}

// Export source types and config for use outside this module

use crate::contract::{DownloadError, DownloadedManifest, DownloadedSource, Downloader};

/// DefaultDownloader holds a DownloadConfig (sources and output_dir) and delegates to download::run.
/// After downloading, it produces a DownloadedManifest describing all downloaded sources and local paths.
pub struct DefaultDownloader {
    config: DownloadConfig,
}

impl DefaultDownloader {
    pub fn new(config: DownloadConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Downloader for DefaultDownloader {
    async fn download_all(&self) -> Result<DownloadedManifest, DownloadError> {
        // Compose legacy config and run download
        let legacy_config = crate::config::Config {
            output_dir: self.config.output_dir.clone(),
            sources: self.config.sources.clone(),
        };
        crate::download::run(&legacy_config).await.map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("download::run failed: {:?}", e).into()
            },
        )?;
        // Now, build the DownloadedManifest with deterministic paths matched to each source.
        let mut sources = Vec::new();
        for source in &self.config.sources {
            let (logical_name, local_path) = match source {
                SourceAction::Git(git) => {
                    let reference = git.reference.as_deref().unwrap_or("main");
                    let dir_name = format!("git_{}_{}", git.repo_url, reference)
                        .replace('/', "_")
                        .replace(':', "_");
                    let full_path = self.config.output_dir.join(dir_name);
                    (git.repo_url.clone(), full_path)
                }
                SourceAction::Confluence(confluence) => {
                    let dir_name = format!(
                        "confluence_{}_{}",
                        confluence.base_url, confluence.space_key
                    )
                    .replace('/', "_")
                    .replace(':', "_");
                    let full_path = self.config.output_dir.join(dir_name);
                    (
                        format!("{}:{}", confluence.base_url, confluence.space_key),
                        full_path,
                    )
                }
            };
            sources.push(DownloadedSource {
                logical_name,
                local_path,
                original_source: source.clone(),
            });
        }
        Ok(DownloadedManifest { sources })
    }
}

pub async fn run(config: &crate::config::Config) -> Result<(), ()> {
    // Now supports Git and Confluence sources
    for source in &config.sources {
        match source {
            SourceAction::Git(git_source) => {
                let repo_url = &git_source.repo_url;
                let reference = git_source.reference.as_deref().unwrap_or("main");
                let out_dir = &config.output_dir;

                // Build deterministic subdirectory for this git source:
                // use the full repo_url (including https:// or git@), replace / and : with _
                let source_dir_name = format!("git_{}_{}", repo_url, reference)
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
                    }
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
                        continue;
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
            SourceAction::Confluence(confluence_source) => {
                use reqwest::Client;
                use std::fs;
                use tracing::{error, info};

                let base_url = confluence_source.base_url.trim_end_matches('/'); // avoid "//"
                let space_key = &confluence_source.space_key;
                let email =
                    std::env::var("CONFLUENCE_API_EMAIL").expect("CONFLUENCE_API_EMAIL missing");
                let token =
                    std::env::var("CONFLUENCE_API_TOKEN").expect("CONFLUENCE_API_TOKEN missing");

                let out_dir = &config.output_dir;
                let source_dir_name = format!("confluence_{}_{}", base_url, space_key)
                    .replace('/', "_")
                    .replace(':', "_");
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
                        let text = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| String::from("<Failed to decode response body>"));
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
                        let mut f = File::create(&space_json_path).map_err(|e| {
                            error!(error=?e, file=?space_json_path, "Failed to create space.json");
                            ()
                        })?;
                        f.write_all(text.as_bytes()).map_err(|e| {
                            error!(error=?e, file=?space_json_path, "Failed to write space.json");
                            ()
                        })?;
                        info!(path = %space_json_path.display(), "Downloaded Confluence space.json");

                        // === Fetch all pages for the space as markdown ===

                        // Helper function to convert path components to sanitized double-underscore separated file path
                        fn sanitize_to_fs_safe(parts: &[&str]) -> String {
                            let mut name = parts
                                .iter()
                                .map(|s| {
                                    let s = s.replace(
                                        &['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..],
                                        "_",
                                    );
                                    let s = s.replace(std::path::MAIN_SEPARATOR, "_");
                                    let s = s.replace("__", "_");
                                    s
                                })
                                .collect::<Vec<_>>()
                                .join("__");
                            // Remove leading/trailing/empty segments
                            while name.starts_with('_') || name.starts_with('.') {
                                name = name[1..].to_string();
                            }
                            while name.ends_with('_') || name.ends_with('.') {
                                name.pop();
                            }
                            name
                        }

                        // Get all pages in the space using pagination
                        let mut start = 0;
                        // Use env var for CONFLUENCE_PAGE_LIMIT or default to 15
                        // Only use the limit for requests, not for total collection, unless env is set
                        let page_limit = std::env::var("CONFLUENCE_PAGE_LIMIT")
                            .ok()
                            .and_then(|v| v.parse::<usize>().ok());
                        let api_batch_limit = 100;
                        let mut pages = Vec::new();
                        'fetch_pages: loop {
                            let content_url = format!(
                                "{}/rest/api/content?spaceKey={}&limit={}&start={}&expand=title,body.storage,ancestors",
                                base_url, space_key, api_batch_limit, start
                            );
                            let resp = client
                                .get(&content_url)
                                .basic_auth(email.clone(), Some(token.clone()))
                                .send()
                                .await;

                            let resp = match resp {
                                Ok(r) => r,
                                Err(e) => {
                                    error!(error = ?e, url = %content_url, "Failed to fetch Confluence pages");
                                    break 'fetch_pages;
                                }
                            };
                            let status = resp.status();
                            let json_val = match resp.json::<serde_json::Value>().await {
                                Ok(val) => val,
                                Err(e) => {
                                    error!(error = ?e, url = %content_url, "Failed to parse Confluence pages JSON");
                                    break 'fetch_pages;
                                }
                            };
                            if !status.is_success() {
                                error!(status = %status, url = %content_url, "Confluence API returned error for pages");
                                break 'fetch_pages;
                            }

                            // Expect pages in "results" array and metadata in "_links" (possibly "next" link)
                            let results = json_val
                                .get("results")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let size = results.len();

                            // Push as many as needed (if there's a cap) or all
                            if let Some(limit) = page_limit {
                                let remaining = if pages.len() >= limit {
                                    0
                                } else {
                                    limit - pages.len()
                                };
                                if remaining > 0 {
                                    // Take up to remaining
                                    let to_add =
                                        results.into_iter().take(remaining).collect::<Vec<_>>();
                                    pages.extend(to_add);
                                }
                                if pages.len() >= limit {
                                    pages.truncate(limit);
                                    break 'fetch_pages;
                                }
                            } else {
                                pages.extend(results);
                            }

                            if size < api_batch_limit {
                                // last page
                                break 'fetch_pages;
                            }
                            start += api_batch_limit;
                        }

                        // Optionally limit total number of pages after all pages are collected, using CONFLUENCE_PAGE_LIMIT
                        let page_limit = std::env::var("CONFLUENCE_PAGE_LIMIT")
                            .ok()
                            .and_then(|v| v.parse::<usize>().ok());
                        if let Some(limit) = page_limit {
                            if pages.len() > limit {
                                pages.truncate(limit);
                            }
                        }

                        // Directory creation & writing markdown files
                        for page in pages {
                            let title = page
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or("untitled");
                            let body_md = page
                                .get("body")
                                .and_then(|b| b.get("storage"))
                                .and_then(|s| s.get("value"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            // Get the ancestor titles for hierarchy
                            let mut hierarchy: Vec<&str> = Vec::new();
                            if let Some(ancestors) =
                                page.get("ancestors").and_then(|v| v.as_array())
                            {
                                for anc in ancestors {
                                    if let Some(t) = anc.get("title").and_then(|t| t.as_str()) {
                                        hierarchy.push(t);
                                    }
                                }
                            }
                            hierarchy.push(title);
                            let file_stem = sanitize_to_fs_safe(&hierarchy);

                            // Final file path (no subdirectories, just double underscore separated)
                            let out_file_path = full_source_path.join(format!("{}.md", file_stem));

                            // Confluence storage format is HTML, convert minimally to markdown-like (strip tags naively)
                            fn html_to_markdown_minimal(html: &str) -> String {
                                // This is only minimal: replace headings, paragraphs, remove *most* html tags.
                                // For a proper solution, use a crate (html2md or ammonia, etc.), but here is quick & dirty:
                                let mut md = String::from(html);
                                for i in (1..=6).rev() {
                                    md = md.replace(
                                        &format!("<h{i}>"),
                                        &format!("\n{} ", "#".repeat(i)),
                                    );
                                    md = md.replace(&format!("</h{i}>"), "\n");
                                }
                                md = md.replace("<p>", "\n\n").replace("</p>", "\n");
                                md = md.replace("<br>", "\n").replace("<br/>", "\n");
                                md = md.replace("<ul>", "\n").replace("</ul>", "\n");
                                md = md.replace("<ol>", "\n").replace("</ol>", "\n");
                                md = md.replace("<li>", "- ").replace("</li>", "\n");
                                // Strip remaining tags (very naive, does not handle everything)
                                md = regex::Regex::new(r"<[^>]+>")
                                    .unwrap()
                                    .replace_all(&md, "")
                                    .to_string();
                                md
                            }
                            let markdown = html_to_markdown_minimal(body_md);

                            // Write the markdown file
                            if let Err(e) = std::fs::write(&out_file_path, markdown) {
                                error!(error = ?e, path = %out_file_path.display(), "Failed to write Confluence page markdown");
                            }
                        }

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
