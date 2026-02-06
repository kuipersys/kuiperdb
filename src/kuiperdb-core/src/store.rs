use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{sqlite::SqlitePool, Row};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::index::{IndexConfig, VectorIndex};
use crate::models::Document;

pub struct DocumentStore {
    base_dir: String,
    pools: HashMap<String, SqlitePool>,
    global_pool: Option<SqlitePool>,
    /// Per-database vector indexes
    indexes: HashMap<String, Arc<VectorIndex>>,
    /// Index configuration
    index_config: IndexConfig,
    /// Whether to use vector indexing
    use_indexing: bool,
    /// Auto-enable threshold (document count)
    index_threshold: usize,
}

impl DocumentStore {
    pub async fn new(base_dir: String) -> Result<Self> {
        tokio::fs::create_dir_all(&base_dir)
            .await
            .context("Failed to create base directory")?;

        // Create global pool for cache
        let global_db_path = format!("{}/global.db", base_dir);
        let global_pool = SqlitePool::connect(&format!("sqlite://{}?mode=rwc", global_db_path))
            .await
            .context("Failed to connect to global database")?;

        Ok(Self {
            base_dir,
            pools: HashMap::new(),
            global_pool: Some(global_pool),
            indexes: HashMap::new(),
            index_config: IndexConfig::default(),
            use_indexing: false,
            index_threshold: 1000,
        })
    }

    /// Configure vector indexing
    pub fn configure_indexing(&mut self, enabled: bool, threshold: usize, config: IndexConfig) {
        self.use_indexing = enabled;
        self.index_threshold = threshold;
        self.index_config = config.clone();
        tracing::info!(
            "Vector indexing configured: enabled={}, threshold={}, m={}, ef_construction={}, ef_search={}",
            enabled, threshold, config.hnsw_m, config.hnsw_ef_construction, config.hnsw_ef_search
        );
    }

    /// Get global pool for cache
    pub async fn get_global_pool(&self) -> Result<SqlitePool> {
        self.global_pool
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Global pool not initialized"))
    }

    /// Get or create database pool for a specific database
    pub async fn get_pool(&mut self, db_id: &str) -> Result<&SqlitePool> {
        if !self.pools.contains_key(db_id) {
            let db_path = format!("{}/{}.db", self.base_dir, db_id);
            let connection_string = format!("sqlite://{}?mode=rwc", db_path);
            let pool = SqlitePool::connect(&connection_string)
                .await
                .context("Failed to connect to database")?;

            // Enable foreign keys
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&pool)
                .await?;

            self.pools.insert(db_id.to_string(), pool);
        }

        Ok(self.pools.get(db_id).unwrap())
    }

    /// Ensure a table exists
    pub async fn ensure_table(&mut self, db_id: &str, table_name: &str) -> Result<()> {
        if !is_valid_table_name(table_name) {
            anyhow::bail!("Invalid table name: {}", table_name);
        }

        let pool = self.get_pool(db_id).await?;

        // Create main documents table
        let create_table = format!(
            r#"
            CREATE TABLE IF NOT EXISTS "{}" (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                metadata TEXT,
                tags TEXT,
                vector BLOB,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                is_embedded INTEGER DEFAULT 0,
                vectorize INTEGER DEFAULT 1,
                is_chunk INTEGER DEFAULT 0,
                parent_id TEXT DEFAULT NULL,
                chunk_index INTEGER DEFAULT NULL,
                token_count INTEGER DEFAULT NULL,
                is_vectorized INTEGER DEFAULT 0,
                FOREIGN KEY (parent_id) REFERENCES "{}"(id) ON DELETE CASCADE
            )
        "#,
            table_name, table_name
        );

        sqlx::query(&create_table).execute(pool).await?;

        // Create FTS5 virtual table
        let create_fts = format!(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS "{}_fts" USING fts5(
                id UNINDEXED,
                content,
                content='{}',
                content_rowid='rowid'
            )
        "#,
            table_name, table_name
        );

        sqlx::query(&create_fts).execute(pool).await?;

        // Create triggers - re-get pool to work around borrow checker
        self.create_fts_triggers_for_table(db_id, table_name)
            .await?;

        // Create indexes - re-get pool again
        let pool = self.get_pool(db_id).await?;
        let indexes = vec![
            format!(
                r#"CREATE INDEX IF NOT EXISTS idx_{}_created_at ON "{}"(created_at)"#,
                table_name, table_name
            ),
            format!(
                r#"CREATE INDEX IF NOT EXISTS idx_{}_updated_at ON "{}"(updated_at)"#,
                table_name, table_name
            ),
            format!(
                r#"CREATE INDEX IF NOT EXISTS idx_{}_embedded ON "{}"(is_embedded)"#,
                table_name, table_name
            ),
            format!(
                r#"CREATE INDEX IF NOT EXISTS idx_{}_parent ON "{}"(parent_id) WHERE parent_id IS NOT NULL"#,
                table_name, table_name
            ),
            format!(
                r#"CREATE INDEX IF NOT EXISTS idx_{}_chunks ON "{}"(is_chunk, parent_id) WHERE is_chunk = 1"#,
                table_name, table_name
            ),
        ];

        for index_sql in indexes {
            sqlx::query(&index_sql).execute(pool).await?;
        }

        // Create document_relations table (shared for all tables in this db)
        self.create_relations_table(db_id).await?;

        Ok(())
    }

    async fn create_relations_table(&mut self, db_id: &str) -> Result<()> {
        let pool = self.get_pool(db_id).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS document_relations (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                metadata TEXT,
                created_at DATETIME NOT NULL
            )
        "#,
        )
        .execute(pool)
        .await?;

        // Create indexes
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_relations_source ON document_relations(source_id)
        "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_relations_target ON document_relations(target_id)
        "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_relations_type ON document_relations(relation_type)
        "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_relations_unique
            ON document_relations(source_id, target_id, relation_type)
        "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn create_fts_triggers_for_table(&mut self, db_id: &str, table_name: &str) -> Result<()> {
        let pool = self.get_pool(db_id).await?;
        // Insert trigger
        let insert_trigger = format!(
            r#"
            CREATE TRIGGER IF NOT EXISTS {}_ai AFTER INSERT ON "{}" BEGIN
                INSERT INTO "{}_fts"(rowid, id, content)
                VALUES (new.rowid, new.id, new.content);
            END
        "#,
            table_name, table_name, table_name
        );

        // Delete trigger
        let delete_trigger = format!(
            r#"
            CREATE TRIGGER IF NOT EXISTS {}_ad AFTER DELETE ON "{}" BEGIN
                DELETE FROM "{}_fts" WHERE rowid = old.rowid;
            END
        "#,
            table_name, table_name, table_name
        );

        // Update trigger
        let update_trigger = format!(
            r#"
            CREATE TRIGGER IF NOT EXISTS {}_au AFTER UPDATE ON "{}" BEGIN
                UPDATE "{}_fts" SET content = new.content WHERE rowid = old.rowid;
            END
        "#,
            table_name, table_name, table_name
        );

        sqlx::query(&insert_trigger).execute(pool).await?;
        sqlx::query(&delete_trigger).execute(pool).await?;
        sqlx::query(&update_trigger).execute(pool).await?;

        Ok(())
    }

    /// Store a document
    pub async fn store_document(
        &mut self,
        db_id: &str,
        table_name: &str,
        doc: Document,
    ) -> Result<()> {
        self.ensure_table(db_id, table_name).await?;
        let pool = self.get_pool(db_id).await?;

        // Serialize metadata
        let metadata_json = serde_json::to_string(&doc.metadata)?;

        // Serialize tags
        let tags_str = doc.tags.join(",");

        // Serialize vector
        let (vector_bytes, is_embedded, is_vectorized) = if let Some(ref vector) = doc.vector {
            (Some(serialize_vector(vector)), 1, 1)
        } else {
            (None, 0, 0)
        };

        // Calculate token count if not already set (estimate: 1 token per 4 characters)
        let token_count = doc.token_count.unwrap_or_else(|| {
            (doc.content.len() as f32 / 4.0).ceil() as i32
        });

        let query = format!(
            r#"
            INSERT INTO "{}" (id, content, metadata, tags, vector, created_at, updated_at, is_embedded, vectorize, is_chunk, parent_id, chunk_index, token_count, is_vectorized)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                content = excluded.content,
                metadata = excluded.metadata,
                tags = excluded.tags,
                vector = excluded.vector,
                updated_at = excluded.updated_at,
                is_embedded = excluded.is_embedded,
                vectorize = excluded.vectorize,
                is_chunk = excluded.is_chunk,
                parent_id = excluded.parent_id,
                chunk_index = excluded.chunk_index,
                token_count = excluded.token_count,
                is_vectorized = excluded.is_vectorized
        "#,
            table_name
        );

        sqlx::query(&query)
            .bind(&doc.id)
            .bind(&doc.content)
            .bind(&metadata_json)
            .bind(&tags_str)
            .bind(&vector_bytes)
            .bind(doc.created_at)
            .bind(doc.updated_at)
            .bind(is_embedded)
            .bind(if doc.vectorize { 1 } else { 0 })
            .bind(if doc.is_chunk { 1 } else { 0 })
            .bind(&doc.parent_id)
            .bind(doc.chunk_index)
            .bind(token_count)
            .bind(is_vectorized)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Get a document by ID
    pub async fn get_document(
        &mut self,
        db_id: &str,
        table_name: &str,
        id: &str,
    ) -> Result<Document> {
        let pool = self.get_pool(db_id).await?;

        let query = format!(
            r#"
            SELECT id, content, metadata, tags, vector, created_at, updated_at, is_embedded,
                   vectorize, is_chunk, parent_id, chunk_index, token_count, is_vectorized
            FROM "{}"
            WHERE id = ?
        "#,
            table_name
        );

        let row = sqlx::query(&query)
            .bind(id)
            .fetch_one(pool)
            .await
            .context("Document not found")?;

        let metadata_json: String = row.get("metadata");
        let metadata: HashMap<String, serde_json::Value> =
            serde_json::from_str(&metadata_json).unwrap_or_default();

        let tags_str: String = row.get("tags");
        let tags: Vec<String> = if tags_str.is_empty() {
            Vec::new()
        } else {
            tags_str.split(',').map(String::from).collect()
        };

        let vector_bytes: Option<Vec<u8>> = row.get("vector");
        let vector = vector_bytes.map(|bytes| deserialize_vector(&bytes));

        let is_embedded: i32 = row.get("is_embedded");
        let vectorize: i32 = row.get("vectorize");
        let is_chunk: i32 = row.get("is_chunk");
        let is_vectorized: i32 = row.get("is_vectorized");

        Ok(Document {
            id: row.get("id"),
            db: db_id.to_string(),
            table: table_name.to_string(),
            content: row.get("content"),
            metadata,
            tags,
            vector,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            is_embedded: is_embedded == 1,
            vectorize: vectorize == 1,
            is_chunk: is_chunk == 1,
            parent_id: row.get("parent_id"),
            chunk_index: row.get("chunk_index"),
            token_count: row.get("token_count"),
            is_vectorized: is_vectorized == 1,
        })
    }

    /// Get documents that need embedding
    pub async fn get_non_embedded_documents(
        &mut self,
        db_id: &str,
        table_name: &str,
        limit: i32,
    ) -> Result<Vec<Document>> {
        let pool = self.get_pool(db_id).await?;

        let query = format!(
            r#"
            SELECT id, content, metadata, tags, vector, created_at, updated_at, is_embedded, vectorize, is_chunk, parent_id, chunk_index, token_count, is_vectorized
            FROM "{}"
            WHERE is_embedded = 0 AND vectorize = 1
            ORDER BY created_at ASC
            LIMIT ?
        "#,
            table_name
        );

        let rows = sqlx::query(&query).bind(limit).fetch_all(pool).await?;

        let mut documents = Vec::new();
        for row in rows {
            let metadata_json: String = row.get("metadata");
            let metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&metadata_json).unwrap_or_default();

            let tags_str: String = row.get("tags");
            let tags: Vec<String> = if tags_str.is_empty() {
                Vec::new()
            } else {
                tags_str.split(',').map(String::from).collect()
            };

            let vectorize: i32 = row.get("vectorize");
            let is_chunk: i32 = row.get("is_chunk");
            let is_vectorized: i32 = row.get("is_vectorized");

            documents.push(Document {
                id: row.get("id"),
                db: db_id.to_string(),
                table: table_name.to_string(),
                content: row.get("content"),
                metadata,
                tags,
                vector: None,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                is_embedded: false,
                vectorize: vectorize == 1,
                is_chunk: is_chunk == 1,
                parent_id: row.get("parent_id"),
                chunk_index: row.get("chunk_index"),
                token_count: row.get("token_count"),
                is_vectorized: is_vectorized == 1,
            });
        }

        Ok(documents)
    }

    /// Get all documents (embedded or not) - for listing endpoints
    pub async fn get_all_documents(
        &mut self,
        db_id: &str,
        table_name: &str,
        limit: i32,
    ) -> Result<Vec<Document>> {
        let pool = self.get_pool(db_id).await?;

        let query = format!(
            r#"
            SELECT id, content, metadata, tags, vector, created_at, updated_at, is_embedded, vectorize, is_chunk, parent_id, chunk_index, token_count, is_vectorized
            FROM "{}"
            ORDER BY created_at ASC
            LIMIT ?
        "#,
            table_name
        );

        let rows = sqlx::query(&query).bind(limit).fetch_all(pool).await?;

        let mut documents = Vec::new();
        for row in rows {
            let metadata_json: String = row.get("metadata");
            let metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&metadata_json).unwrap_or_default();

            let tags_str: String = row.get("tags");
            let tags: Vec<String> = if tags_str.is_empty() {
                Vec::new()
            } else {
                tags_str.split(',').map(String::from).collect()
            };

            let vector_bytes: Option<Vec<u8>> = row.get("vector");
            let vector = vector_bytes.map(|bytes| deserialize_vector(&bytes));

            let is_embedded: i32 = row.get("is_embedded");
            let vectorize: i32 = row.get("vectorize");
            let is_chunk: i32 = row.get("is_chunk");
            let is_vectorized: i32 = row.get("is_vectorized");

            documents.push(Document {
                id: row.get("id"),
                db: db_id.to_string(),
                table: table_name.to_string(),
                content: row.get("content"),
                metadata,
                tags,
                vector,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                is_embedded: is_embedded == 1,
                vectorize: vectorize == 1,
                is_chunk: is_chunk == 1,
                parent_id: row.get("parent_id"),
                chunk_index: row.get("chunk_index"),
                token_count: row.get("token_count"),
                is_vectorized: is_vectorized == 1,
            });
        }

        Ok(documents)
    }

    /// Update document vector
    pub async fn update_document_vector(
        &mut self,
        db_id: &str,
        table_name: &str,
        doc_id: &str,
        vector: &[f32],
    ) -> Result<()> {
        let pool = self.get_pool(db_id).await?;
        let vector_bytes = serialize_vector(vector);

        let query = format!(
            r#"
            UPDATE "{}"
            SET vector = ?, is_embedded = 1, is_vectorized = 1, updated_at = ?
            WHERE id = ?
        "#,
            table_name
        );

        sqlx::query(&query)
            .bind(&vector_bytes)
            .bind(Utc::now())
            .bind(doc_id)
            .execute(pool)
            .await?;

        // Add to vector index if it exists
        if self.use_indexing {
            let index_key = format!("{}:{}", db_id, table_name);
            if let Some(index) = self.indexes.get(&index_key) {
                index.add(doc_id.to_string(), vector.to_vec())?;
            }
        }

        Ok(())
    }

    /// Build HNSW index for a table
    async fn build_index(&mut self, db_id: &str, table_name: &str) -> Result<()> {
        let pool = self.get_pool(db_id).await?;

        tracing::info!("Building HNSW index for {}.{}", db_id, table_name);

        // Fetch all vectors
        let sql = format!(
            r#"
            SELECT id, vector FROM "{}"
            WHERE is_embedded = 1 AND vector IS NOT NULL
        "#,
            table_name
        );

        let rows = sqlx::query(&sql).fetch_all(pool).await?;

        if rows.is_empty() {
            tracing::warn!("No vectors to index for {}.{}", db_id, table_name);
            return Ok(());
        }

        let mut documents = Vec::new();
        let mut dimensions = 0;

        for row in rows {
            let id: String = row.get("id");
            let vector_bytes: Vec<u8> = row.get("vector");
            let vector = deserialize_vector(&vector_bytes);
            if dimensions == 0 {
                dimensions = vector.len();
            }
            documents.push((id, vector));
        }

        // Create and build index
        let index = Arc::new(VectorIndex::new(dimensions, self.index_config.clone()));

        index.build(documents)?;

        let index_key = format!("{}:{}", db_id, table_name);
        self.indexes.insert(index_key, index);

        tracing::info!("HNSW index built for {}.{}", db_id, table_name);

        Ok(())
    }

    /// List all databases
    pub async fn list_databases(&self) -> Result<Vec<String>> {
        let data_dir = Path::new("data");
        let mut databases = Vec::new();

        if !data_dir.exists() {
            return Ok(databases);
        }

        let entries = std::fs::read_dir(data_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "db" {
                        if let Some(stem) = path.file_stem() {
                            let db_name = stem.to_string_lossy().to_string();
                            // Skip global.db
                            if db_name != "global" {
                                databases.push(db_name);
                            }
                        }
                    }
                }
            }
        }

        databases.sort();
        Ok(databases)
    }

    /// List all tables in a database
    pub async fn list_tables(&mut self, db_id: &str) -> Result<Vec<String>> {
        let pool = self.get_pool(db_id).await?;

        let rows = sqlx::query(
            r#"
            SELECT name FROM sqlite_master
            WHERE type = 'table'
            AND name NOT LIKE 'sqlite_%'
            AND name NOT LIKE '%_fts'
            AND name NOT LIKE '%_config'
            AND name NOT LIKE '%_data'
            AND name NOT LIKE '%_idx'
            AND name NOT LIKE '%_docsize'
            ORDER BY name
            "#,
        )
        .fetch_all(pool)
        .await?;

        let tables: Vec<String> = rows.iter().map(|row| row.get("name")).collect();

        Ok(tables)
    }

    /// FTS5 full-text search
    pub async fn search_fts(
        &mut self,
        db_id: &str,
        table_name: &str,
        query: &str,
        limit: usize,
    ) -> Result<
        Vec<(
            String,
            String,
            HashMap<String, serde_json::Value>,
            f64,
            bool,
            Option<String>,
            Option<i32>,
        )>,
    > {
        let pool = self.get_pool(db_id).await?;

        let _fts_table = format!("{}_fts", table_name);
        let sql = format!(
            r#"
            SELECT d.id, d.content, d.metadata, fts.rank, d.is_chunk, d.parent_id, d.chunk_index
            FROM "{0}_fts" AS fts
            JOIN "{0}" AS d ON fts.rowid = d.rowid
            WHERE fts.content MATCH ?
            ORDER BY fts.rank
            LIMIT ?
        "#,
            table_name
        );

        let rows = sqlx::query(&sql)
            .bind(query)
            .bind(limit as i64)
            .fetch_all(pool)
            .await?;

        let mut results = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let content: String = row.get("content");
            let metadata_json: String = row.get("metadata");
            let rank: f64 = row.get("rank");
            let is_chunk: i32 = row.get("is_chunk");
            let parent_id: Option<String> = row.get("parent_id");
            let chunk_index: Option<i32> = row.get("chunk_index");

            let metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&metadata_json).unwrap_or_default();

            results.push((
                id,
                content,
                metadata,
                rank,
                is_chunk == 1,
                parent_id,
                chunk_index,
            ));
        }

        Ok(results)
    }

    /// Vector similarity search (cosine distance)
    /// Uses HNSW index if available and enabled, otherwise falls back to brute-force
    pub async fn search_vector(
        &mut self,
        db_id: &str,
        table_name: &str,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<
        Vec<(
            String,
            String,
            HashMap<String, serde_json::Value>,
            f64,
            bool,
            Option<String>,
            Option<i32>,
        )>,
    > {
        // Check if we should use HNSW index
        let use_index = self.should_use_index(db_id, table_name).await?;

        if use_index {
            return self
                .search_vector_with_index(db_id, table_name, query_vector, limit)
                .await;
        }

        // Fall back to brute-force
        self.search_vector_brute_force(db_id, table_name, query_vector, limit)
            .await
    }

    /// Check if we should use HNSW index
    async fn should_use_index(&mut self, db_id: &str, table_name: &str) -> Result<bool> {
        if !self.use_indexing {
            return Ok(false);
        }

        // Get document count
        let pool = self.get_pool(db_id).await?;
        let count_query = format!(
            r#"
            SELECT COUNT(*) as count FROM "{}" WHERE is_embedded = 1
        "#,
            table_name
        );

        let row = sqlx::query(&count_query).fetch_one(pool).await?;
        let count: i64 = row.get("count");

        Ok(count as usize >= self.index_threshold)
    }

    /// Search using HNSW index
    async fn search_vector_with_index(
        &mut self,
        db_id: &str,
        table_name: &str,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<
        Vec<(
            String,
            String,
            HashMap<String, serde_json::Value>,
            f64,
            bool,
            Option<String>,
            Option<i32>,
        )>,
    > {
        let index_key = format!("{}:{}", db_id, table_name);

        // Build index if not exists
        if !self.indexes.contains_key(&index_key) {
            self.build_index(db_id, table_name).await?;
        }

        // Search using index
        let index = self
            .indexes
            .get(&index_key)
            .ok_or_else(|| anyhow::anyhow!("Index not found"))?;

        let neighbors = index.search(query_vector, limit)?;

        // Fetch document details
        let pool = self.get_pool(db_id).await?;
        let mut results = Vec::new();

        for (doc_id, similarity) in neighbors {
            let query = format!(
                r#"
                SELECT id, content, metadata, is_chunk, parent_id, chunk_index FROM "{}"
                WHERE id = ?
            "#,
                table_name
            );

            if let Ok(row) = sqlx::query(&query).bind(&doc_id).fetch_one(pool).await {
                let content: String = row.get("content");
                let metadata_json: String = row.get("metadata");
                let metadata: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&metadata_json).unwrap_or_default();
                let is_chunk: i32 = row.get("is_chunk");
                let parent_id: Option<String> = row.get("parent_id");
                let chunk_index: Option<i32> = row.get("chunk_index");

                results.push((
                    doc_id,
                    content,
                    metadata,
                    similarity as f64,
                    is_chunk == 1,
                    parent_id,
                    chunk_index,
                ));
            }
        }

        tracing::debug!("HNSW search returned {} results", results.len());
        Ok(results)
    }

    /// Brute-force vector search
    async fn search_vector_brute_force(
        &mut self,
        db_id: &str,
        table_name: &str,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<
        Vec<(
            String,
            String,
            HashMap<String, serde_json::Value>,
            f64,
            bool,
            Option<String>,
            Option<i32>,
        )>,
    > {
        let pool = self.get_pool(db_id).await?;

        let sql = format!(
            r#"
            SELECT id, content, metadata, vector, is_chunk, parent_id, chunk_index
            FROM "{}"
            WHERE is_embedded = 1 AND vector IS NOT NULL
        "#,
            table_name
        );

        let rows = sqlx::query(&sql).fetch_all(pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let content: String = row.get("content");
            let metadata_json: String = row.get("metadata");
            let vector_bytes: Vec<u8> = row.get("vector");
            let is_chunk: i32 = row.get("is_chunk");
            let parent_id: Option<String> = row.get("parent_id");
            let chunk_index: Option<i32> = row.get("chunk_index");

            let metadata: HashMap<String, serde_json::Value> =
                serde_json::from_str(&metadata_json).unwrap_or_default();

            // Deserialize vector
            let doc_vector = deserialize_vector(&vector_bytes);

            // Calculate cosine similarity
            let similarity = cosine_similarity(query_vector, &doc_vector);

            results.push((
                id,
                content,
                metadata,
                similarity,
                is_chunk == 1,
                parent_id,
                chunk_index,
            ));
        }

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap());

        // Return top results
        results.truncate(limit);

        Ok(results)
    }

    // ===== Document Relations Methods =====

    /// Create a document relation
    pub async fn create_relation(
        &mut self,
        db_id: &str,
        relation: crate::models::DocumentRelation,
    ) -> Result<()> {
        let pool = self.get_pool(db_id).await?;

        let metadata_json = serde_json::to_string(&relation.metadata)?;

        sqlx::query(r#"
            INSERT INTO document_relations (id, source_id, target_id, relation_type, metadata, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
        "#)
        .bind(&relation.id)
        .bind(&relation.source_id)
        .bind(&relation.target_id)
        .bind(&relation.relation_type)
        .bind(&metadata_json)
        .bind(relation.created_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get relation by ID
    pub async fn get_relation(
        &mut self,
        db_id: &str,
        relation_id: &str,
    ) -> Result<crate::models::DocumentRelation> {
        let pool = self.get_pool(db_id).await?;

        let row = sqlx::query(
            r#"
            SELECT id, source_id, target_id, relation_type, metadata, created_at
            FROM document_relations
            WHERE id = ?
        "#,
        )
        .bind(relation_id)
        .fetch_one(pool)
        .await?;

        let metadata_json: String = row.get("metadata");
        let metadata = serde_json::from_str(&metadata_json).unwrap_or_default();

        Ok(crate::models::DocumentRelation {
            id: row.get("id"),
            source_id: row.get("source_id"),
            target_id: row.get("target_id"),
            relation_type: row.get("relation_type"),
            metadata,
            created_at: row.get("created_at"),
        })
    }

    /// Delete a relation
    pub async fn delete_relation(&mut self, db_id: &str, relation_id: &str) -> Result<()> {
        let pool = self.get_pool(db_id).await?;

        sqlx::query(
            r#"
            DELETE FROM document_relations WHERE id = ?
        "#,
        )
        .bind(relation_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get all relations for a document
    pub async fn get_document_relations(
        &mut self,
        db_id: &str,
        doc_id: &str,
    ) -> Result<Vec<crate::models::DocumentRelation>> {
        let pool = self.get_pool(db_id).await?;

        let rows = sqlx::query(
            r#"
            SELECT id, source_id, target_id, relation_type, metadata, created_at
            FROM document_relations
            WHERE source_id = ? OR target_id = ?
        "#,
        )
        .bind(doc_id)
        .bind(doc_id)
        .fetch_all(pool)
        .await?;

        let mut relations = Vec::new();
        for row in rows {
            let metadata_json: String = row.get("metadata");
            let metadata = serde_json::from_str(&metadata_json).unwrap_or_default();

            relations.push(crate::models::DocumentRelation {
                id: row.get("id"),
                source_id: row.get("source_id"),
                target_id: row.get("target_id"),
                relation_type: row.get("relation_type"),
                metadata,
                created_at: row.get("created_at"),
            });
        }

        Ok(relations)
    }

    /// Get all relations in database (for graph operations)
    pub async fn get_all_relations(
        &mut self,
        db_id: &str,
    ) -> Result<Vec<crate::models::DocumentRelation>> {
        let pool = self.get_pool(db_id).await?;

        let rows = sqlx::query(
            r#"
            SELECT id, source_id, target_id, relation_type, metadata, created_at
            FROM document_relations
        "#,
        )
        .fetch_all(pool)
        .await?;

        let mut relations = Vec::new();
        for row in rows {
            let metadata_json: String = row.get("metadata");
            let metadata = serde_json::from_str(&metadata_json).unwrap_or_default();

            relations.push(crate::models::DocumentRelation {
                id: row.get("id"),
                source_id: row.get("source_id"),
                target_id: row.get("target_id"),
                relation_type: row.get("relation_type"),
                metadata,
                created_at: row.get("created_at"),
            });
        }

        Ok(relations)
    }

    // ===== Chunking Methods =====

    /// Get all chunks for a parent document
    pub async fn get_chunks(
        &mut self,
        db_id: &str,
        table_name: &str,
        parent_id: &str,
    ) -> Result<Vec<Document>> {
        let pool = self.get_pool(db_id).await?;

        let query = format!(
            r#"
            SELECT id, content, metadata, tags, vector, created_at, updated_at, is_embedded, 
                   vectorize, is_chunk, parent_id, chunk_index, token_count, is_vectorized
            FROM "{}"
            WHERE parent_id = ? AND is_chunk = 1
            ORDER BY chunk_index ASC
        "#,
            table_name
        );

        let rows = sqlx::query(&query).bind(parent_id).fetch_all(pool).await?;

        let mut documents = Vec::new();
        for row in rows {
            let metadata_json: String = row.get("metadata");
            let metadata = serde_json::from_str(&metadata_json).unwrap_or_default();

            let tags_str: String = row.get("tags");
            let tags: Vec<String> = if tags_str.is_empty() {
                Vec::new()
            } else {
                tags_str.split(',').map(String::from).collect()
            };

            let vector_bytes: Option<Vec<u8>> = row.get("vector");
            let vector = vector_bytes.map(|bytes| deserialize_vector(&bytes));

            let is_embedded: i32 = row.get("is_embedded");
            let vectorize: i32 = row.get("vectorize");
            let is_chunk: i32 = row.get("is_chunk");
            let is_vectorized: i32 = row.get("is_vectorized");

            documents.push(Document {
                id: row.get("id"),
                db: db_id.to_string(),
                table: table_name.to_string(),
                content: row.get("content"),
                metadata,
                tags,
                vector,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                is_embedded: is_embedded == 1,
                vectorize: vectorize == 1,
                is_chunk: is_chunk == 1,
                parent_id: row.get("parent_id"),
                chunk_index: row.get("chunk_index"),
                token_count: row.get("token_count"),
                is_vectorized: is_vectorized == 1,
            });
        }

        Ok(documents)
    }

    /// Delete all chunks for a parent document
    pub async fn delete_chunks(
        &mut self,
        db_id: &str,
        table_name: &str,
        parent_id: &str,
    ) -> Result<()> {
        let pool = self.get_pool(db_id).await?;

        let query = format!(
            r#"
            DELETE FROM "{}"
            WHERE parent_id = ? AND is_chunk = 1
        "#,
            table_name
        );

        sqlx::query(&query).bind(parent_id).execute(pool).await?;

        Ok(())
    }

    /// Convenience method to add a document from a StoreDocumentRequest
    /// This provides a cleaner API for adding documents without manually constructing Document structs
    pub async fn add_document(
        &mut self,
        db_id: &str,
        table_name: &str,
        request: crate::models::StoreDocumentRequest,
    ) -> Result<Document> {
        use uuid::Uuid;

        let doc_id = request.id.unwrap_or_else(|| Uuid::new_v4().to_string());

        let doc = Document {
            id: doc_id.clone(),
            db: db_id.to_string(),
            table: table_name.to_string(),
            content: request.content,
            metadata: request.metadata,
            tags: request.tags,
            vector: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_embedded: false,
            vectorize: request.vectorize,
            is_chunk: false,
            parent_id: None,
            chunk_index: None,
            token_count: None,
            is_vectorized: false,
        };

        self.store_document(db_id, table_name, doc.clone()).await?;
        Ok(doc)
    }

    /// Convenience method to add a simple document with just content
    pub async fn add_simple_document(
        &mut self,
        db_id: &str,
        table_name: &str,
        content: impl Into<String>,
    ) -> Result<Document> {
        let request = crate::models::StoreDocumentRequest {
            id: None,
            content: content.into(),
            metadata: std::collections::HashMap::new(),
            tags: Vec::new(),
            vectorize: true,
        };
        self.add_document(db_id, table_name, request).await
    }

    /// Convenience method to delete a document by ID
    pub async fn delete_document_by_id(
        &mut self,
        db_id: &str,
        table_name: &str,
        doc_id: &str,
    ) -> Result<()> {
        let pool = self.get_pool(db_id).await?;

        // First, check if this is a child document (has parent_id)
        let check_query = format!(
            r#"SELECT parent_id FROM "{}" WHERE id = ?"#,
            table_name
        );
        
        let result: Option<(Option<String>,)> = sqlx::query_as(&check_query)
            .bind(doc_id)
            .fetch_optional(pool)
            .await?;
        
        if let Some((Some(_parent_id),)) = result {
            return Err(anyhow::anyhow!(
                "Cannot delete child document. Delete the parent document instead, which will cascade delete all children."
            ));
        }

        // Document is a parent or standalone - proceed with deletion
        // CASCADE will automatically delete children
        let query = format!(r#"DELETE FROM "{}" WHERE id = ?"#, table_name);
        sqlx::query(&query).bind(doc_id).execute(pool).await?;

        Ok(())
    }

    pub async fn delete_table(&mut self, db_id: &str, table_name: &str) -> Result<()> {
        if !is_valid_table_name(table_name) {
            anyhow::bail!("Invalid table name: {}", table_name);
        }

        let pool = self.get_pool(db_id).await?;

        // Drop the main table
        let drop_table = format!(r#"DROP TABLE IF EXISTS "{}""#, table_name);
        sqlx::query(&drop_table).execute(pool).await?;

        // Drop the FTS table
        let drop_fts = format!(r#"DROP TABLE IF EXISTS "{}_fts""#, table_name);
        sqlx::query(&drop_fts).execute(pool).await?;

        Ok(())
    }

    pub async fn delete_database(&mut self, db_id: &str) -> Result<()> {
        // Remove pool from cache
        self.pools.remove(db_id);

        // Delete the database file
        let db_path = format!("{}/{}.db", self.base_dir, db_id);
        if std::path::Path::new(&db_path).exists() {
            std::fs::remove_file(&db_path)
                .context("Failed to delete database file")?;
        }

        Ok(())
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

/// Deserialize vector from bytes
fn deserialize_vector(bytes: &[u8]) -> Vec<f32> {
    let mut vector = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        vector.push(value);
    }
    vector
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }

    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot_product += (a[i] * b[i]) as f64;
        norm_a += (a[i] * a[i]) as f64;
        norm_b += (b[i] * b[i]) as f64;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}

/// Validate table name (alphanumeric and underscores only)
fn is_valid_table_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}
