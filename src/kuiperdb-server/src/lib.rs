//! kuiperdb - A vector database for LLM applications
//!
//! This crate provides both a library and binary for running kuiperdb.
//!
//! # Embedded Usage
//!
//! ```rust,no_run
//! use kuiperdb_core::*;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let store = store::DocumentStore::new("./data".to_string()).await?;
//!     // Use store directly...
//!     Ok(())
//! }
//! ```
//!
//! # Server Usage
//!
//! Run the binary to start the REST API server:
//! ```bash
//! kuiperdb
//! ```

pub use kuiperdb_core;

pub mod api;
