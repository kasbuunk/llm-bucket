#![doc = "llm-bucket-core: core logic library for llm-bucket."]

//! This crate contains all open-source logic, data models and pipelines for llm-bucket.
//! Proprietary upload or integration logic is not included here.
//! Begin new modules as submodules below.
//!
//! # Usage
//! Add this as a dependency for all shared pipeline, processing, config, and sync code.

pub mod code_to_pdf;
pub mod config;
pub mod download;
pub mod preprocess;
pub mod synchronise;
pub mod uploader;
