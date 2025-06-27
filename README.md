# bucket-sync

A command-line utility for aggregating knowledge snapshots—including git repositories, Confluence spaces, Slack channels, and more—and publishing them into a flat storage bucket. Designed primarily to optimize content for downstream LLM (Large Language Model) ingestion, `bucket-sync` enables organizations to regularly export and harmonize knowledge from diverse sources into a unified, machine-ingestible format.

## Primary Use Cases

- Regularly export documentation, codebases, or chat logs for LLM training or retrieval-augmented generation.
- Automate consolidation of different knowledge silos into a single bucket.
- Schedule routine syncs of internal knowledge sources for analytics and search.

---

## Features

### Currently Supported

- **Export Git Branches:** Extract the contents of a specified git branch and publish to a local directory.

### Roadmap (Upcoming Features)

- **Confluence Space Export:** Sync Confluence pages/spaces to the bucket.
- **Slack Channel Export:** Extract message histories from Slack channels.
- **Multiple Source Integration:** Sync from multiple heterogeneous sources in a single run.
- **Scheduling:** Support for cron/scheduled automated runs.
- **Service Accounts:** Isolate credentials and authorization per source.
- **Enhanced Error Handling:** Rich reporting and export of sync results.

---

## Installation

### Prerequisites

- **Language:** [Replace with actual language, e.g., Python >=3.10, Go >=1.18, Node.js >=18, etc.]
- **Tools:** git (for source snapshot), storage bucket CLI/SDK (if publishing to cloud, future)
- **Authentication:** Access tokens/credentials for each source (see Configuration).

### Local Install

```shell
git clone https://github.com/kasbuunk/bucket-sync.git
cd bucket-sync
# For Python example:
pip install .
# For Node.js example:
npm install
# For Go example:
go build -o publish-cli ./cmd/publish-cli
```

---

## Configuration

### Credential Management

- **Environment Variables:** Use standard ENV vars for per-source secrets (e.g., `GIT_TOKEN`, `CONFLUENCE_API_KEY`).
- **Config File:** Optionally supply a YAML/JSON config file to store source definitions and credentials.
- **CLI Flags:** Override config values at runtime via flags (see Usage).

#### Example Config (`bucket-sync.yaml`)

```yaml
sources:
  - type: git
    repo_url: https://github.com/org/repo.git
    branch: main
    auth_token: <GIT_TOKEN>
    out_dir: ./exported/git-repo-1
  # - type: confluence
  #   space_key: <SPACE_KEY>
  #   base_url: https://company.atlassian.net/wiki
  #   auth_token: <CONFLUENCE_API_KEY>
  #   out_dir: ./exported/confluence-space-1
```

**Note:** Never commit credentials to version control! Instead, use placeholders that reference environment variables.

---

## Usage

### CLI Reference

```shell
publish-cli export git --repo-url https://github.com/org/repo.git --branch main --out-dir ./exported/repo-main
```

#### Flags

- `--repo-url` (**required**): URL of the git repository.
- `--branch` (**required**): Branch to export (e.g., `main`).
- `--out-dir` (**required**): Local output directory for the exported contents.
- `--auth-token` (optional): Git authentication token (can also use `GIT_TOKEN` env).

---

## Examples

### Minimal Example

Export a git branch to a local directory:

```shell
publish-cli export git --repo-url https://github.com/org/repo.git --branch main --out-dir ./dump/main
```

### Multi-Source Example (future)

Prepare a config with several sources:

```yaml
sources:
  - type: git
    repo_url: https://github.com/org/infra.git
    branch: develop
    auth_token: ${GIT_TOKEN}
    out_dir: ./exports/infra
  - type: confluence
    space_key: HR
    base_url: https://company.atlassian.net/wiki
    auth_token: ${CONFLUENCE_API_KEY}
    out_dir: ./exports/HR-wiki
  - type: slack
    channel: C0123456
    auth_token: ${SLACK_BOT_TOKEN}
    out_dir: ./exports/slack-hr
```
Invoke:
```shell
publish-cli export --config bucket-sync.yaml
```

---

## Logging & Reporting

- **Default:** Progress, errors, and summary reports output to stdout.
- **Verbose/JSON:** For automation, append `--log-format json` for machine-readable logs.
- **Log Files:** (Planned) Optionally write logs and reports to file for audit or debugging.
- **Exit Codes:** Non-zero on failure. Summaries report number of sources succeeded/failed.

---

## Extensibility Guide

### Adding New Source Handlers

- Implement new source modules under `src/sources/` (or equivalent).
- Each source handler should expose `export()` with common signature: `(config, credentials, out_dir)`

### Registering New Authentication Modules

- Add new auth strategies under `src/auth/`. Handlers should resolve credentials via config, env, or secret manager.
- Register the handler in the authentication registry or factory.

---

## Contribution & Roadmap

### Contributing

- Open issues/feature requests via [GitHub Issues](https://github.com/kasbuunk/bucket-sync/issues).
- Fork, branch, and submit PRs following conventional commit message standards.
- All contributions should include brief documentation/tests for new features.

### Roadmap and Epics

- Multi-source sync engine
- Support for new source types (Confluence, Slack, Google Drive, etc.)
- Config-driven scheduling (manual + cron/integrations)
- Pluggable authentication and error-handling modules
- Export format adapters for LLM vendor ingestion

---

**Questions/Feedback?** Open a GitHub issue or join the project discussion board.