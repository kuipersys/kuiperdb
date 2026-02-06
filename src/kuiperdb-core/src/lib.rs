//! KuiperDb Core Library
//!
//! This crate provides the core functionality for KuiperDb, including:
//! - Database storage layer
//! - Vector indexing with HNSW
//! - Graph operations
//! - Search functionality
//! - Embedding generation and chunking
//! - Caching layer

pub mod cache;
pub mod chunking;
pub mod config;
pub mod embedder;
pub mod graph;
pub mod index;
pub mod models;
pub mod search;
pub mod store;
pub mod worker;

// Re-export commonly used types
pub use cache::EmbeddingCache;
pub use config::Config;
pub use embedder::Embedder;
pub use graph::GraphStatistics;
pub use index::VectorIndex;
pub use models::*;
pub use search::{HybridSearcher, SearchResult};
pub use store::DocumentStore;
pub use worker::BackgroundWorker;
