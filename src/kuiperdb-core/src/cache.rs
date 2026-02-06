use anyhow::Result;
use chrono::{DateTime, Utc};
use lru::LruCache;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Compute SHA256 hash of content
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Embedding cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub hash: String,
    pub vector: Vec<f32>,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

/// Two-tier embedding cache: LRU memory + SQLite disk
pub struct EmbeddingCache {
    /// In-memory LRU cache (fast lookup)
    memory_cache: Arc<RwLock<LruCache<String, Vec<f32>>>>,
    /// SQLite connection pool for disk cache
    pool: SqlitePool,
    /// Model name for cache key
    model: String,
    /// Statistics
    hits: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub memory_hits: u64,
    pub disk_hits: u64,
    pub misses: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.memory_hits + self.disk_hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.memory_hits + self.disk_hits) as f64 / total as f64
        }
    }
}

impl EmbeddingCache {
    pub async fn new(pool: SqlitePool, model: String, memory_capacity: usize) -> Result<Self> {
        // Create cache table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS embedding_cache (
                content_hash TEXT PRIMARY KEY,
                vector BLOB NOT NULL,
                model TEXT NOT NULL,
                created_at DATETIME NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Create index on model for efficient lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_embedding_cache_model 
            ON embedding_cache(model, created_at DESC)
            "#,
        )
        .execute(&pool)
        .await?;

        let capacity = NonZeroUsize::new(memory_capacity).unwrap();

        Ok(Self {
            memory_cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            pool,
            model,
            hits: Arc::new(RwLock::new(CacheStats::default())),
        })
    }

    /// Get embedding from cache (memory → disk → None)
    pub async fn get(&self, content: &str) -> Result<Option<Vec<f32>>> {
        let hash = hash_content(content);

        // Check memory cache first
        {
            let mut cache = self.memory_cache.write().await;
            if let Some(vector) = cache.get(&hash) {
                self.hits.write().await.memory_hits += 1;
                tracing::debug!("Cache hit (memory): {}", &hash[..8]);
                return Ok(Some(vector.clone()));
            }
        }

        // Check disk cache
        let result: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT vector FROM embedding_cache WHERE content_hash = ? AND model = ?",
        )
        .bind(&hash)
        .bind(&self.model)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((vector_bytes,)) = result {
            let vector = deserialize_vector(&vector_bytes);

            // Populate memory cache
            self.memory_cache
                .write()
                .await
                .put(hash.clone(), vector.clone());

            self.hits.write().await.disk_hits += 1;
            tracing::debug!("Cache hit (disk): {}", &hash[..8]);
            return Ok(Some(vector));
        }

        // Cache miss
        self.hits.write().await.misses += 1;
        tracing::debug!("Cache miss: {}", &hash[..8]);
        Ok(None)
    }

    /// Store embedding in cache (both memory and disk)
    pub async fn put(&self, content: &str, vector: Vec<f32>) -> Result<()> {
        let hash = hash_content(content);
        let vector_bytes = serialize_vector(&vector);

        // Store in disk cache
        sqlx::query(
            r#"
            INSERT INTO embedding_cache (content_hash, vector, model, created_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(content_hash) DO UPDATE SET
                vector = excluded.vector,
                created_at = excluded.created_at
            "#,
        )
        .bind(&hash)
        .bind(&vector_bytes)
        .bind(&self.model)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        // Store in memory cache
        self.memory_cache.write().await.put(hash.clone(), vector);

        tracing::debug!("Cached embedding: {}", &hash[..8]);
        Ok(())
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        self.hits.read().await.clone()
    }

    /// Clear memory cache (keeps disk cache)
    pub async fn clear_memory(&self) {
        self.memory_cache.write().await.clear();
    }

    /// Evict old entries from disk cache (keep last N days)
    pub async fn evict_old(&self, days: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query("DELETE FROM embedding_cache WHERE created_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

/// Serialize vector to bytes (little-endian Float32)
fn serialize_vector(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for &v in vector {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    bytes
}

/// Deserialize vector from bytes (little-endian Float32)
fn deserialize_vector(bytes: &[u8]) -> Vec<f32> {
    let mut vector = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let bits = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        vector.push(f32::from_bits(bits));
    }
    vector
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_content() {
        let content = "Hello, world!";
        let hash = hash_content(content);
        assert_eq!(hash.len(), 64); // SHA256 = 32 bytes = 64 hex chars

        // Same content should produce same hash
        assert_eq!(hash, hash_content(content));

        // Different content should produce different hash
        assert_ne!(hash, hash_content("Hello, Rust!"));
    }

    #[test]
    fn test_vector_serialization() {
        let vector = vec![1.0, -2.5, 3.2, 0.0, -0.001];
        let bytes = serialize_vector(&vector);
        let recovered = deserialize_vector(&bytes);

        assert_eq!(vector.len(), recovered.len());
        for (a, b) in vector.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }
}
