use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::cache::EmbeddingCache;

/// Embedder trait for converting text to vectors
#[async_trait::async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}

/// OpenAI-compatible embedding client (works with llama.cpp /v1/embeddings)
/// With integrated two-tier caching
pub struct OpenAIEmbedder {
    client: Client,
    base_url: String,
    dimensions: usize,
    model: String,
    cache: Option<Arc<EmbeddingCache>>,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    input: serde_json::Value, // String or array of strings
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

impl OpenAIEmbedder {
    pub fn new(base_url: String, dimensions: usize, insecure_skip_verify: bool) -> Result<Self> {
        let client = if insecure_skip_verify {
            reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()?
        } else {
            reqwest::Client::new()
        };

        Ok(Self {
            client,
            base_url,
            dimensions,
            model: "default".to_string(),
            cache: None,
        })
    }

    /// Enable caching with specified cache instance
    pub fn with_cache(mut self, cache: Arc<EmbeddingCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Call GPU endpoint (bypassing cache)
    async fn embed_uncached(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            input: serde_json::Value::String(text.to_string()),
            model: self.model.clone(),
        };

        let response = self
            .client
            .post(format!("{}/v1/embeddings", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to call embedding service")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Embedding service returned status {}: {}", status, body);
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse embedding response")?;

        if embedding_response.data.is_empty() {
            anyhow::bail!("No embedding data in response");
        }

        let embedding = embedding_response.data[0].embedding.clone();

        if embedding.len() != self.dimensions {
            anyhow::bail!(
                "Expected embedding dimension {}, got {}",
                self.dimensions,
                embedding.len()
            );
        }

        Ok(embedding)
    }
}

#[async_trait::async_trait]
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first if enabled
        if let Some(cache) = &self.cache {
            if let Some(vector) = cache.get(text).await? {
                return Ok(vector);
            }
        }

        // Cache miss or no cache - call GPU
        let vector = self.embed_uncached(text).await?;

        // Store in cache if enabled
        if let Some(cache) = &self.cache {
            cache.put(text, vector.clone()).await?;
        }

        Ok(vector)
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // For batch, we'll process individually to leverage cache
        // GPU batching was unreliable in benchmarks
        let mut vectors = Vec::with_capacity(texts.len());

        for text in texts {
            let vector = self.embed(text).await?;
            vectors.push(vector);
        }

        Ok(vectors)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

/// Parallel embedding worker pool
/// Processes documents in parallel with configurable concurrency
pub struct ParallelEmbedder {
    embedder: Arc<dyn Embedder>,
    semaphore: Arc<Semaphore>,
    batch_size: usize,
}

impl ParallelEmbedder {
    pub fn new(embedder: Arc<dyn Embedder>, max_workers: usize, batch_size: usize) -> Self {
        Self {
            embedder,
            semaphore: Arc::new(Semaphore::new(max_workers)),
            batch_size,
        }
    }

    /// Embed texts in parallel using worker pool
    pub async fn embed_parallel(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        use futures::stream::{FuturesUnordered, StreamExt};

        let mut tasks = FuturesUnordered::new();

        // Process in batches to utilize any server-side batching
        for batch in texts.chunks(self.batch_size) {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let embedder = self.embedder.clone();
            // Clone the batch strings to avoid lifetime issues
            let batch_owned: Vec<String> = batch.to_vec();

            tasks.push(tokio::spawn(async move {
                let batch_refs: Vec<&str> = batch_owned.iter().map(|s| s.as_str()).collect();
                let result = embedder.embed_batch(&batch_refs).await;
                drop(permit); // Release worker slot
                result
            }));
        }

        // Collect all results
        let mut all_embeddings = Vec::new();
        while let Some(result) = tasks.next().await {
            let embeddings = result??;
            all_embeddings.extend(embeddings);
        }

        Ok(all_embeddings)
    }

    pub fn dimensions(&self) -> usize {
        self.embedder.dimensions()
    }
}
