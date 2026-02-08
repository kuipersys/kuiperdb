use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

// Serialize DateTime<Utc> as Unix timestamp in milliseconds
fn serialize_datetime_as_millis<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(dt.timestamp_millis())
}

// Deserialize Unix timestamp in milliseconds to DateTime<Utc>
fn deserialize_datetime_from_millis<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let millis = i64::deserialize(deserializer)?;
    DateTime::from_timestamp_millis(millis)
        .ok_or_else(|| Error::custom(format!("invalid timestamp: {}", millis)))
}

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
    #[serde(skip)]
    pub vector: Option<Vec<f32>>,
    #[serde(
        serialize_with = "serialize_datetime_as_millis",
        deserialize_with = "deserialize_datetime_from_millis"
    )]
    pub created_at: DateTime<Utc>,
    #[serde(
        serialize_with = "serialize_datetime_as_millis",
        deserialize_with = "deserialize_datetime_from_millis"
    )]
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
    #[serde(serialize_with = "serialize_datetime_as_millis")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime_as_millis")]
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
    #[serde(
        serialize_with = "serialize_datetime_as_millis",
        deserialize_with = "deserialize_datetime_from_millis"
    )]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_serialization() {
        let doc = Document {
            id: "test-123".to_string(),
            db: "test_db".to_string(),
            table: "test_table".to_string(),
            content: "Test content".to_string(),
            metadata: HashMap::new(),
            tags: vec!["test".to_string()],
            vector: Some(vec![0.1, 0.2, 0.3]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_embedded: true,
            vectorize: true,
            is_chunk: false,
            parent_id: None,
            chunk_index: None,
            token_count: Some(10),
            is_vectorized: true,
        };

        let json = serde_json::to_string(&doc).expect("Failed to serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse");
        
        // Verify timestamps are numbers (milliseconds)
        assert!(value["created_at"].is_number(), "created_at should be a number");
        assert!(value["updated_at"].is_number(), "updated_at should be a number");
        
        // Verify vector is not serialized
        assert!(value.get("vector").is_none(), "vector should not be serialized");
    }

    #[test]
    fn test_relation_timestamp_serialization() {
        let relation = DocumentRelation {
            id: "rel-123".to_string(),
            source_id: "doc-1".to_string(),
            target_id: "doc-2".to_string(),
            relation_type: "related_to".to_string(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&relation).expect("Failed to serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse");
        
        // Verify timestamp is a number (milliseconds)
        assert!(value["created_at"].is_number(), "created_at should be a number");
    }
}
