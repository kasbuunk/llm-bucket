/// llm-bucket: Top-level CLI entrypoint for source aggregation and upload.
///
/// This crate provides the main CLI executable, argument parsing, configuration loading, and upload
/// workflows for the llm-bucket system. All long-lived business logic and shared models live in
/// [`llm-bucket-core`]; this crate is a thin shell focused exclusively on CLI, config, and invocation.
///
/// # Usage
///
/// - Binary crate: provides the CLI executable (`llm-bucket`).
/// - Entry module: parses commands, loads YAML config, orchestrates download/process/upload (see `cli`).
/// - To use the code programmatically (e.g., for integration tests, call [`run`] directly or use the [`Cli`] types.
///
/// # CLI Features
/// - Aggregates and processes sources (git/Confluence…) as described in a YAML config.
/// - Supports extensible processors (flatten files, README→PDF, etc.)
/// - Handles uploading to remote knowledge stores using env-based authentication.
///
/// # Dependency Structure
/// - All actual business logic (synchronisation, config models, processing, uploader traits) live in [llm-bucket-core].
/// - This crate should only handle CLI argument parsing, one-time setup, tracing and orchestration.
/// - Consider extending core functionality in `llm-bucket-core` before expanding CLI code here.
///
/// # See Also
/// - [`llm-bucket-core`]: core logic, data models, and processing.
///
/// # Example
/// ```sh
/// llm-bucket sync --config config.yaml
/// ```
///
/// For configuration schema, see the README and `core` crate documentation.
pub mod cli;
pub mod load_config;
pub mod upload;
pub use cli::{run, Cli, Commands};
