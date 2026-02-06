use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::embedder::Embedder;
use crate::store::DocumentStore;

/// Type alias for search result tuples
type SearchResultTuple = (
    String,                             // id
    String,                             // content
    HashMap<String, serde_json::Value>, // metadata
    f64,                                // score
    bool,                               // is_chunk
    Option<String>,                     // parent_id
    Option<i32>,                        // chunk_index
);

/// Type alias for RRF score accumulator
type RrfScoreMap = HashMap<
    String, // document id
    (
        f64,                                // combined score
        Option<f64>,                        // fts_rank
        Option<f64>,                        // vector_similarity
        String,                             // content
        HashMap<String, serde_json::Value>, // metadata
        bool,                               // is_chunk
        Option<String>,                     // parent_id
        Option<i32>,                        // chunk_index
    ),
>;

/// Hybrid search combining FTS5 and vector similarity
pub struct HybridSearcher {
    k: usize, // RRF parameter (typically 60)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub score: f64,
    pub fts_rank: Option<f64>,
    pub vector_similarity: Option<f64>,
    // Chunking fields
    pub is_chunk: bool,
    pub parent_id: Option<String>,
    pub chunk_index: Option<i32>,
}

impl HybridSearcher {
    pub fn new() -> Self {
        Self { k: 60 }
    }

    /// Perform hybrid search combining FTS5 and vector similarity
    pub async fn search(
        &self,
        store: &mut DocumentStore,
        embedder: Option<&dyn Embedder>,
        db_id: &str,
        table_name: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // Get FTS5 results
        let fts_results = store
            .search_fts(db_id, table_name, query, limit * 2)
            .await?;

        // Get vector results if embedder available
        let vector_results = if let Some(emb) = embedder {
            let query_vector = emb.embed(query).await?;
            store
                .search_vector(db_id, table_name, &query_vector, limit * 2)
                .await?
        } else {
            Vec::new()
        };

        // Merge with RRF
        let merged = self.reciprocal_rank_fusion(&fts_results, &vector_results);

        // Return top results
        Ok(merged.into_iter().take(limit).collect())
    }

    /// Reciprocal Rank Fusion algorithm
    /// RRF score = sum(1 / (k + rank))
    fn reciprocal_rank_fusion(
        &self,
        fts_results: &[SearchResultTuple],
        vector_results: &[SearchResultTuple],
    ) -> Vec<SearchResult> {
        let mut scores: RrfScoreMap = HashMap::new();

        // Add FTS ranks
        for (rank, (id, content, metadata, fts_score, is_chunk, parent_id, chunk_index)) in
            fts_results.iter().enumerate()
        {
            let rrf_score = 1.0 / (self.k as f64 + rank as f64 + 1.0);
            scores.insert(
                id.clone(),
                (
                    rrf_score,
                    Some(*fts_score),
                    None,
                    content.clone(),
                    metadata.clone(),
                    *is_chunk,
                    parent_id.clone(),
                    *chunk_index,
                ),
            );
        }

        // Add vector ranks
        for (rank, (id, content, metadata, vec_score, is_chunk, parent_id, chunk_index)) in
            vector_results.iter().enumerate()
        {
            let rrf_score = 1.0 / (self.k as f64 + rank as f64 + 1.0);

            scores
                .entry(id.clone())
                .and_modify(|(score, _fts, vec, _, _, _, _, _)| {
                    *score += rrf_score;
                    *vec = Some(*vec_score);
                })
                .or_insert((
                    rrf_score,
                    None,
                    Some(*vec_score),
                    content.clone(),
                    metadata.clone(),
                    *is_chunk,
                    parent_id.clone(),
                    *chunk_index,
                ));
        }

        // Convert to sorted results
        let mut results: Vec<SearchResult> = scores
            .into_iter()
            .map(
                |(
                    id,
                    (
                        score,
                        fts_rank,
                        vector_similarity,
                        content,
                        metadata,
                        is_chunk,
                        parent_id,
                        chunk_index,
                    ),
                )| {
                    SearchResult {
                        id,
                        content,
                        metadata,
                        score,
                        fts_rank,
                        vector_similarity,
                        is_chunk,
                        parent_id,
                        chunk_index,
                    }
                },
            )
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        results
    }
}

impl Default for HybridSearcher {
    fn default() -> Self {
        Self::new()
    }
}
