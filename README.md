# llm-bucket

A fast, structured CLI utility for aggregating knowledge snapshots from Git repositories (and soon other sources) into ready-to-ingest local outputs and/or uploading them to an API for knowledge base workflows, LLM training, and auditing. Designed for automation, repeatability, and clean output.

---

## Features

- **Sync multiple Git repositories** (public or private) in a single run via YAML config.
- **Processing options:** Flatten all files or (optionally) convert repository README.md to PDF.
- **Deterministic, isolated output folders** for easy downstream use.
- **Single-command CLI**—no interactive/manual steps required.
- **Uploads to API (if configured)**: integrates with external knowledge stores (requires environment config).
- **Extensible pipeline:** Designed for additional source types (Confluence, Slack, etc.) in future.

---

## Quick Start

### 1. Install Prerequisites

- [Rust](https://rustup.rs/) (stable, edition 2021)
- [Git](https://git-scm.com/) (must be on your PATH)

### 2. Build the CLI

```sh
git clone https://github.com/kasbuunk/llm-bucket.git
cd llm-bucket
cargo build --release
```

The executable will be at `./target/release/llm-bucket`.

---

## Configuration

All actions are configured in a [YAML](https://yaml.org/) file. No command-line flags for input sources.

### Example minimal config (`config.yaml`):

```yaml
download:
  output_dir: ./output
  sources:
    - type: git
      repo_url: "https://github.com/youruser/yourrepo.git"
      reference: main         # optional: branch/tag/commit

process:
  kind: FlattenFiles          # or ReadmeToPDF
```

- `output_dir`: Root directory for clones & processed data (recommended: gitignore this in production).
- `sources`: List of source blocks. Currently only `type: git` is supported.
    - `repo_url`: HTTPS or SSH URL for the git repo.
    - `reference`: Optional; branch/tag/commit (default: main).
- `process.kind`: Currently accepts:
    - `FlattenFiles`: Flatten all files for upload.
    - `ReadmeToPDF`: Convert repository README.md to PDF (if implemented for your repo).

---

## Usage

After configuring `config.yaml`, run:

```sh
./target/release/llm-bucket sync --config config.yaml
```

- The CLI clones each repo and processes it as specified.
- Output is placed under `output_dir` (subdirectories per repo, deterministic naming).

**Note:** Only subcommand available is `sync` (see below).

---

## Upload/API Integration

Uploading to a remote knowledge base/API requires these environment variables:

- `BUCKET_ID` — Integer bucket/project ID (provided by backend/admin)
- `OCP_APIM_SUBSCRIPTION_KEY` — API key/token for upload

You can use a `.env` file (auto-loaded by the CLI) or set variables in your environment:

```sh
export BUCKET_ID=1234
export OCP_APIM_SUBSCRIPTION_KEY=your-token-here
```

---

## Output & Structure

- All results are placed within the configured `output_dir`.
    - Each source is mapped to a uniquely named subfolder (sanitized from repo URL and branch).
    - Data format and structure matches your selected `process.kind`.
- Example: For a single repo, with FlattenFiles, output might look like:
    ```
    ./output/
      git_github_com_youruser_yourrepo_git_main/
        src/
        README.md
        ...
    ```

---

## Logging & Diagnostics

- All actions are logged via [tracing](https://docs.rs/tracing).
- Log summaries printed to stdout/stderr, including errors and high-level summaries.

---

## Development & Testing

Run all checks and tests:

```sh
cargo test
```

Tests cover:
- End-to-end sync with test repos
- CLI invocation with configs
- Processor output validation

---

## Contribution & Roadmap

- All contributions welcome via PR or issues.
- Please keep PRs small and focused.
- Future: support for Confluence, Slack, Google Drive, robust error reports, scheduling, more processors.

---

## FAQ

**Q: How do I add a new source type?**  
A: See the `src/` directory for modular structure; implement new `SourceAction` and expand the YAML loader, then add download, processing, and (optional) upload logic.

**Q: Is interactive usage supported?**  
A: No—llm-bucket is for declarative, repeatable workflows. Only config files.

**Q: Is output safe for public commit?**  
A: No. Output is meant for ingestion/upload, not VCS; it should be gitignored.

---

## License

MIT (see LICENSE).

---

**For design notes and future directions, see [`notes.md`](notes.md).**
