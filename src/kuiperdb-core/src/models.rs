use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Document represents a stored document with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub db: String,
    pub table: String,
    pub content: String, // Markdown text
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_embedded: bool,

    // Chunking fields
    #[serde(default = "default_true")]
    pub vectorize: bool, // Per-document embedding toggle
    #[serde(default)]
    pub is_chunk: bool, // True if this is a chunk
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>, // If chunk, points to parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<i32>, // Position in parent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count: Option<i32>, // Cached token count
    #[serde(default)]
    pub is_vectorized: bool, // Whether document has embeddings
}

fn default_true() -> bool {
    true
}

/// StoreDocumentRequest represents the request to store a document
#[derive(Debug, Clone, Deserialize)]
pub struct StoreDocumentRequest {
    #[serde(default)]
    pub id: Option<String>, // Optional, generated if not provided
    pub content: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_true")]
    pub vectorize: bool, // Per-document embedding toggle
}

/// SearchRequest represents a search query
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(rename = "type", default)]
    pub search_type: SearchType,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub filters: HashMap<String, serde_json::Value>,
    #[serde(default = "default_true")]
    pub include_chunks: bool, // Include chunks in results
    #[serde(default)]
    pub group_by_parent: bool, // Group chunks under parent
}

/// SearchType defines the type of search to perform
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    Vector,
    #[serde(rename = "fulltext")]
    FullText,
    #[default]
    Hybrid,
}

/// SearchResponse represents the search results
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<crate::search::SearchResult>,
    pub query: String,
    #[serde(rename = "type")]
    pub search_type: SearchType,
    pub db: String,
    pub total: usize,
}

/// DBInfo represents information about a database
#[derive(Debug, Serialize)]
pub struct DBInfo {
    pub name: String,
    pub document_count: i64,
    pub embedded_count: i64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub size_bytes: i64,
}

/// ErrorResponse represents an API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// DocumentRelation represents a relationship between two documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRelation {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// CreateRelationRequest represents a request to create a document relationship
#[derive(Debug, Deserialize)]
pub struct CreateRelationRequest {
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// GraphTraversalRequest represents a request to traverse the document graph
#[derive(Debug, Deserialize)]
pub struct GraphTraversalRequest {
    pub start_id: String,
    #[serde(default = "default_depth")]
    pub depth: usize,
    #[serde(default)]
    pub relation_types: Vec<String>, // Filter by relation types
}

fn default_depth() -> usize {
    3
}
