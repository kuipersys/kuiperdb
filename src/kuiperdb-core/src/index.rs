use anyhow::Result;
use hnsw_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_m")]
    pub hnsw_m: usize,
    #[serde(default = "default_ef_construction")]
    pub hnsw_ef_construction: usize,
    #[serde(default = "default_ef_search")]
    pub hnsw_ef_search: usize,
}

const fn default_enabled() -> bool {
    true
}
const fn default_m() -> usize {
    16
}
const fn default_ef_construction() -> usize {
    200
}
const fn default_ef_search() -> usize {
    100
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hnsw_m: default_m(),
            hnsw_ef_construction: default_ef_construction(),
            hnsw_ef_search: default_ef_search(),
        }
    }
}

/// An in-memory ANN acceleration structure. Persisted vectors remain authoritative.
pub(crate) struct VectorIndex {
    hnsw: Hnsw<'static, f32, DistCosine>,
    ids: Vec<String>,
    generation: i64,
    ef_search: usize,
}

impl VectorIndex {
    pub(crate) fn build(
        dimensions: usize,
        config: &IndexConfig,
        generation: i64,
        vectors: Vec<(String, Vec<f32>)>,
    ) -> Result<Self> {
        if vectors.iter().any(|(_, vector)| vector.len() != dimensions) {
            anyhow::bail!("persisted vector has incompatible dimensions");
        }

        // hnsw_rs requires a non-zero capacity even for an empty logical index.
        let hnsw = Hnsw::new(
            config.hnsw_m,
            vectors.len().max(1),
            16,
            config.hnsw_ef_construction,
            DistCosine,
        );
        let mut ids = Vec::with_capacity(vectors.len());
        for (position, (record_id, vector)) in vectors.into_iter().enumerate() {
            hnsw.insert((&vector, position));
            ids.push(record_id);
        }
        Ok(Self {
            hnsw,
            ids,
            generation,
            ef_search: config.hnsw_ef_search,
        })
    }

    pub(crate) fn search(&self, query: &[f32], limit: usize) -> Vec<(String, f32)> {
        self.hnsw
            .search(query, limit, self.ef_search)
            .into_iter()
            .filter_map(|neighbor| {
                self.ids
                    .get(neighbor.d_id)
                    .cloned()
                    .map(|id| (id, neighbor.distance))
            })
            .collect()
    }

    pub(crate) fn generation(&self) -> i64 {
        self.generation
    }
    pub(crate) fn len(&self) -> usize {
        self.ids.len()
    }
}

pub(crate) type IndexMap = HashMap<String, VectorIndex>;
