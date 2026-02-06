use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub embedding_url: String,
    pub embedding_dimensions: usize,
    pub data_dir: String,
    pub port: String,
    #[serde(default)]
    pub insecure_skip_verify: bool,
    #[serde(default)]
    pub ca_cert_path: String,
    pub features: Features,

    // CORS configuration
    #[serde(default)]
    pub cors: CorsConfig,

    // Vector index configuration
    #[serde(default)]
    pub vector_index: VectorIndexConfig,

    // New config for parallel workers
    #[serde(default = "default_num_workers")]
    pub num_embedding_workers: usize,
    #[serde(default = "default_batch_size")]
    pub embedding_batch_size: usize,

    // Chunking configuration
    #[serde(default)]
    pub chunking: ChunkingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CorsConfig {
    #[serde(default = "default_cors_enabled")]
    pub enabled: bool,
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: Vec<String>,
}

fn default_cors_enabled() -> bool {
    true
}

fn default_allowed_origins() -> Vec<String> {
    vec!["http://localhost:5173".to_string(), "http://localhost:5174".to_string(), "http://localhost:5175".to_string()]
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: default_cors_enabled(),
            allowed_origins: default_allowed_origins(),
        }
    }
}

fn default_num_workers() -> usize {
    10 // Based on Phase 0 benchmarks: 10 workers × 20 docs/sec = 200 docs/sec
}

fn default_batch_size() -> usize {
    1 // Single-doc requests are most reliable based on benchmarks
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Features {
    #[serde(default)]
    pub embedding: bool,
    #[serde(default)]
    pub embedding_job: bool,
    #[serde(default)]
    pub embedding_cache: bool,
    #[serde(default)]
    pub vector_index: bool,
    #[serde(default)]
    pub hybrid_search: bool,
    #[serde(default)]
    pub chunking: bool,
    #[serde(default)]
    pub document_relations: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VectorIndexConfig {
    /// HNSW index mode: "auto" (enable at threshold), "always", "never"
    #[serde(default = "default_index_mode")]
    pub mode: String,

    /// Auto-enable HNSW when doc count exceeds this threshold
    #[serde(default = "default_index_threshold")]
    pub threshold: usize,

    /// HNSW M parameter (connections per layer, 4-64, default 16)
    #[serde(default = "default_hnsw_m")]
    pub hnsw_m: usize,

    /// HNSW ef_construction (build quality, 100-500, default 200)
    #[serde(default = "default_hnsw_ef_construction")]
    pub hnsw_ef_construction: usize,

    /// HNSW ef_search (search quality, 50-500, default 100)
    #[serde(default = "default_hnsw_ef_search")]
    pub hnsw_ef_search: usize,
}

fn default_index_mode() -> String {
    "auto".to_string()
}

fn default_index_threshold() -> usize {
    1000 // Auto-enable HNSW at 1K docs
}

fn default_hnsw_m() -> usize {
    16
}

fn default_hnsw_ef_construction() -> usize {
    200
}

fn default_hnsw_ef_search() -> usize {
    100
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChunkingConfig {
    #[serde(default)]
    pub enabled: bool,

    /// Auto-chunk documents exceeding this token count
    #[serde(default = "default_token_threshold")]
    pub token_threshold: usize,

    /// Target chunk size in tokens
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,

    /// Overlap between chunks in tokens
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,

    /// Chunking strategy: "fixed_tokens" or "custom"
    #[serde(default = "default_chunk_strategy")]
    pub strategy: String,
}

fn default_token_threshold() -> usize {
    512
}

fn default_chunk_size() -> usize {
    512
}

fn default_chunk_overlap() -> usize {
    50
}

fn default_chunk_strategy() -> String {
    "fixed_tokens".to_string()
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_threshold: default_token_threshold(),
            chunk_size: default_chunk_size(),
            chunk_overlap: default_chunk_overlap(),
            strategy: default_chunk_strategy(),
        }
    }
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            mode: default_index_mode(),
            threshold: default_index_threshold(),
            hnsw_m: default_hnsw_m(),
            hnsw_ef_construction: default_hnsw_ef_construction(),
            hnsw_ef_search: default_hnsw_ef_search(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    pub fn database_path(&self, db_name: &str) -> String {
        format!("{}/{}.db", self.data_dir, db_name)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            embedding_url: "http://localhost:1234".to_string(),
            embedding_dimensions: 2560,
            data_dir: "./data".to_string(),
            port: "8080".to_string(),
            insecure_skip_verify: false,
            ca_cert_path: String::new(),
            features: Features {
                embedding: false,
                embedding_job: false,
                embedding_cache: false,
                vector_index: false,
                hybrid_search: false,
                chunking: false,
                document_relations: false,
            },
            cors: CorsConfig::default(),
            vector_index: VectorIndexConfig::default(),
            num_embedding_workers: default_num_workers(),
            embedding_batch_size: default_batch_size(),
            chunking: ChunkingConfig::default(),
        }
    }
}
