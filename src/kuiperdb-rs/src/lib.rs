//! KuiperDb Client Library
//!
//! HTTP client for connecting to KuiperDb REST API servers.

mod client;

pub use client::Client;
pub use kuiperdb_core::search::SearchResult;
pub use kuiperdb_core::{Document, GraphStatistics};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },

    #[error("Invalid response from server")]
    InvalidResponse,
}

pub type Result<T> = std::result::Result<T, ClientError>;
