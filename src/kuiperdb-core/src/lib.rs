//! KuiperDB's embedded, model-agnostic storage engine.
//!
//! The core stores records, metadata, relationships, and caller-supplied vectors.
//! It deliberately does not parse content, chunk documents, call models, cache
//! inference output, or expose a network protocol.

pub mod graph;
pub mod index;
pub mod models;
pub mod search;
pub mod store;

pub use graph::{Graph, GraphStatistics, ShortestPath, TraversalResult};
pub use index::IndexConfig;
pub use models::*;
pub use store::{Database, WriteBatch};
