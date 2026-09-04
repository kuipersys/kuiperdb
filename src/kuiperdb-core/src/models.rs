use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Metadata = HashMap<String, serde_json::Value>;

/// An application-defined object with a stable identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: Metadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for an atomic record/vector upsert. If `id` is absent a UUID is assigned.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordInput {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: Metadata,
    #[serde(default)]
    pub vectors: Vec<NamedVector>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedVector {
    pub space: String,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

impl DistanceMetric {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Cosine => "cosine",
            Self::Euclidean => "euclidean",
            Self::DotProduct => "dot_product",
        }
    }

    pub(crate) fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "cosine" => Ok(Self::Cosine),
            "euclidean" => Ok(Self::Euclidean),
            "dot_product" => Ok(Self::DotProduct),
            _ => anyhow::bail!("unknown distance metric: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Normalization {
    #[default]
    None,
    Unit,
}

impl Normalization {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Unit => "unit",
        }
    }

    pub(crate) fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "none" => Ok(Self::None),
            "unit" => Ok(Self::Unit),
            _ => anyhow::bail!("unknown normalization expectation: {value}"),
        }
    }
}

/// Mathematical and indexing contract shared by all vectors in a named space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorSpace {
    pub name: String,
    pub dimensions: usize,
    pub distance_metric: DistanceMetric,
    #[serde(default)]
    pub normalization: Normalization,
    #[serde(default)]
    pub index: crate::index::IndexConfig,
}

/// Equality constraints combined with logical AND. Nested JSON values are allowed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MetadataFilter {
    #[serde(default, flatten)]
    pub equals: Metadata,
}

impl MetadataFilter {
    pub fn matches(&self, metadata: &Metadata) -> bool {
        self.equals
            .iter()
            .all(|(key, expected)| metadata.get(key) == Some(expected))
    }

    pub fn is_empty(&self) -> bool {
        self.equals.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorQuery {
    pub space: String,
    pub vector: Vec<f32>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub filter: MetadataFilter,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub record: Record,
    /// Metric distance; lower values are always nearer.
    pub distance: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
    #[serde(default)]
    pub metadata: Metadata,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationInput {
    #[serde(default)]
    pub id: Option<String>,
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexStatus {
    pub space: String,
    pub database_generation: i64,
    pub loaded_generation: Option<i64>,
    pub indexed_vectors: usize,
    pub stale: bool,
}

#[derive(Debug, Clone)]
pub struct OpenOptions {
    pub busy_timeout: std::time::Duration,
    pub max_connections: u32,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            busy_timeout: std::time::Duration::from_secs(5),
            max_connections: 8,
        }
    }
}
