use anyhow::Result;
use hnsw_rs::prelude::*;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// Vector index using HNSW for fast approximate nearest neighbor search
pub struct VectorIndex {
    /// HNSW index (thread-safe)
    hnsw: Arc<RwLock<Option<Hnsw<'static, f32, DistCosine>>>>,

    /// Mapping from HNSW index -> document ID
    id_map: Arc<RwLock<Vec<String>>>,

    /// Reverse mapping from doc ID -> HNSW index
    reverse_map: Arc<RwLock<std::collections::HashMap<String, usize>>>,

    /// Vector dimensions
    dimensions: usize,

    /// HNSW configuration
    config: IndexConfig,
}

#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub hnsw_m: usize,               // Max connections per layer (default: 16)
    pub hnsw_ef_construction: usize, // Build quality (default: 200)
    pub hnsw_ef_search: usize,       // Search quality (default: 100)
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            hnsw_m: 16,
            hnsw_ef_construction: 200,
            hnsw_ef_search: 100,
        }
    }
}

impl VectorIndex {
    /// Create a new empty vector index
    pub fn new(dimensions: usize, config: IndexConfig) -> Self {
        Self {
            hnsw: Arc::new(RwLock::new(None)),
            id_map: Arc::new(RwLock::new(Vec::new())),
            reverse_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            dimensions,
            config,
        }
    }

    /// Build index from vectors
    pub fn build(&self, documents: Vec<(String, Vec<f32>)>) -> Result<()> {
        if documents.is_empty() {
            info!("No documents to index");
            return Ok(());
        }

        info!(
            "Building HNSW index for {} documents (dims={})",
            documents.len(),
            self.dimensions
        );

        // Create HNSW index with proper parameters
        let hnsw: Hnsw<'static, f32, DistCosine> = Hnsw::new(
            self.config.hnsw_m,
            documents.len(),
            self.dimensions,
            self.config.hnsw_ef_construction,
            DistCosine,
        );

        // Insert all vectors
        let mut id_map = Vec::new();
        let mut reverse_map = std::collections::HashMap::new();

        for (idx, (doc_id, vector)) in documents.into_iter().enumerate() {
            if vector.len() != self.dimensions {
                warn!(
                    "Skipping document {} with wrong dimensions: {} (expected {})",
                    doc_id,
                    vector.len(),
                    self.dimensions
                );
                continue;
            }

            // Insert as (data, id) tuple
            hnsw.insert((&vector, idx));
            id_map.push(doc_id.clone());
            reverse_map.insert(doc_id, idx);
        }

        // Store index
        *self.hnsw.write().unwrap() = Some(hnsw);
        *self.id_map.write().unwrap() = id_map;
        *self.reverse_map.write().unwrap() = reverse_map;

        info!(
            "HNSW index built successfully with {} vectors",
            self.id_map.read().unwrap().len()
        );

        Ok(())
    }

    /// Add a single document to the index
    pub fn add(&self, doc_id: String, vector: Vec<f32>) -> Result<()> {
        if vector.len() != self.dimensions {
            anyhow::bail!(
                "Vector dimension mismatch: {} (expected {})",
                vector.len(),
                self.dimensions
            );
        }

        let mut hnsw_lock = self.hnsw.write().unwrap();
        let mut id_map = self.id_map.write().unwrap();
        let mut reverse_map = self.reverse_map.write().unwrap();

        // Check if document already exists
        if reverse_map.contains_key(&doc_id) {
            debug!("Document {} already in index, skipping", doc_id);
            return Ok(());
        }

        // Get or create HNSW index
        if hnsw_lock.is_none() {
            // Create new index
            let hnsw: Hnsw<'static, f32, DistCosine> = Hnsw::new(
                self.config.hnsw_m,
                10000, // Initial capacity
                self.dimensions,
                self.config.hnsw_ef_construction,
                DistCosine,
            );
            *hnsw_lock = Some(hnsw);
            info!("Created new HNSW index");
        }

        let idx = id_map.len();
        hnsw_lock.as_ref().unwrap().insert((&vector, idx));
        id_map.push(doc_id.clone());
        reverse_map.insert(doc_id, idx);

        debug!("Added document to index (total: {})", id_map.len());

        Ok(())
    }

    /// Search for k nearest neighbors
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        if query.len() != self.dimensions {
            anyhow::bail!(
                "Query dimension mismatch: {} (expected {})",
                query.len(),
                self.dimensions
            );
        }

        let hnsw_lock = self.hnsw.read().unwrap();
        let id_map = self.id_map.read().unwrap();

        if hnsw_lock.is_none() {
            return Ok(Vec::new());
        }

        let hnsw = hnsw_lock.as_ref().unwrap();

        // Search HNSW (returns Vec<Neighbour>)
        let neighbors = hnsw.search(query, k, self.config.hnsw_ef_search);

        // Map indices to document IDs with similarity scores
        let results: Vec<(String, f32)> = neighbors
            .into_iter()
            .filter_map(|neighbor| {
                let idx = neighbor.d_id;
                if idx < id_map.len() {
                    let doc_id = id_map[idx].clone();
                    // Convert distance to similarity (cosine distance -> similarity)
                    let similarity = 1.0 - neighbor.distance;
                    Some((doc_id, similarity))
                } else {
                    warn!("Invalid index in HNSW: {}", idx);
                    None
                }
            })
            .collect();

        debug!("HNSW search returned {} results", results.len());

        Ok(results)
    }

    /// Get number of indexed documents
    pub fn len(&self) -> usize {
        self.id_map.read().unwrap().len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if index is built
    pub fn is_built(&self) -> bool {
        self.hnsw.read().unwrap().is_some()
    }

    /// Clear the index
    pub fn clear(&self) {
        *self.hnsw.write().unwrap() = None;
        self.id_map.write().unwrap().clear();
        self.reverse_map.write().unwrap().clear();
        info!("Vector index cleared");
    }
}
