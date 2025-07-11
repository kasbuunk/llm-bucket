#![doc = "llm-bucket: core logic and data models for the llm-bucket aggregator CLI."]

//! # llm-bucket
//!
//! This crate provides all primary domain logic, data models, and processing pipelines for use in `llm-bucket` workflows.
//!
//! - **Scope:** Only open-source pipeline, processing, config, and repository sync code is here.
//!   *Proprietary upload/integration logic is handled outside this crate.*
//!
//! ## Architecture & Usage
//!
//! - This library is intended for maximum code reuse across the CLI, tests, and (potentially) other tools.
//! - All fundamental types (e.g., configs, processor inputs/outputs) and clean-pipeline functions live here, with clear interfaces for extensibility.
//!
//! ## Major Modules
//!
//! - [`config`]: Typed config models loaded from YAML, including source definitions.
//! - [`download`]: Download logic for source repositories (e.g., Git, Confluence), creating disk snapshots.
//! - [`preprocess`]: Processing/conversion of downloaded repos to uploadable items (PDFs, file flattening, etc).
//! - [`synchronise`]: High-level pipeline for end-to-end sync (download-process-upload/report).
//! - [`contract`]: Interface trait for uploading sources/items (mockable for test).
//! - [`code_to_pdf`]: Minimal stub conversion of code/README to PDF files.
//!
//! ## Example
//! ```rust
//! use llm_bucket::{download, preprocess, synchronise};
//! // (Example code to show intended usage...)
//! ```
//!
//! ## Contribution and Extending
//! When adding a new source or pipeline, declare your data model and process logic in a new submodule, and extend root/aggregate orchestrators in this crate first, not in the CLI shell.
//!

pub mod code_to_pdf;
pub mod contract;
pub mod download;
pub mod preprocess;
pub mod synchronise;
