use crate::index::{IndexMap, VectorIndex};
use crate::models::*;
use crate::search::distance;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow, SqliteSynchronous,
};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS records (
    id TEXT PRIMARY KEY,
    payload TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS vector_spaces (
    name TEXT PRIMARY KEY,
    dimensions INTEGER NOT NULL CHECK (dimensions > 0),
    distance_metric TEXT NOT NULL,
    normalization TEXT NOT NULL,
    index_config TEXT NOT NULL,
    generation INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS vectors (
    record_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
    space TEXT NOT NULL REFERENCES vector_spaces(name) ON DELETE CASCADE,
    values_blob BLOB NOT NULL,
    PRIMARY KEY (record_id, space)
);
CREATE TABLE IF NOT EXISTS record_metadata (
    record_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value_json TEXT NOT NULL,
    PRIMARY KEY (record_id, key)
);
CREATE TABLE IF NOT EXISTS relations (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
    target_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_vectors_space ON vectors(space);
CREATE INDEX IF NOT EXISTS idx_record_metadata_lookup ON record_metadata(key, value_json, record_id);
CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id);
CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target_id);
CREATE TRIGGER IF NOT EXISTS vectors_generation_insert AFTER INSERT ON vectors BEGIN
  UPDATE vector_spaces SET generation = generation + 1 WHERE name = NEW.space;
END;
CREATE TRIGGER IF NOT EXISTS vectors_generation_update AFTER UPDATE ON vectors BEGIN
  UPDATE vector_spaces SET generation = generation + 1 WHERE name = NEW.space;
  UPDATE vector_spaces SET generation = generation + 1 WHERE name = OLD.space AND OLD.space <> NEW.space;
END;
CREATE TRIGGER IF NOT EXISTS vectors_generation_delete AFTER DELETE ON vectors BEGIN
  UPDATE vector_spaces SET generation = generation + 1 WHERE name = OLD.space;
END;
PRAGMA user_version = 1;
CREATE TABLE IF NOT EXISTS store_revision (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    revision INTEGER NOT NULL
);
INSERT OR IGNORE INTO store_revision VALUES (1, 0);
CREATE TRIGGER IF NOT EXISTS records_revision_insert AFTER INSERT ON records BEGIN
  UPDATE store_revision SET revision = revision + 1;
END;
CREATE TRIGGER IF NOT EXISTS records_revision_update AFTER UPDATE ON records BEGIN
  UPDATE store_revision SET revision = revision + 1;
END;
CREATE TRIGGER IF NOT EXISTS records_revision_delete AFTER DELETE ON records BEGIN
  UPDATE store_revision SET revision = revision + 1;
END;
CREATE TRIGGER IF NOT EXISTS relations_revision_insert AFTER INSERT ON relations BEGIN
  UPDATE store_revision SET revision = revision + 1;
END;
CREATE TRIGGER IF NOT EXISTS relations_revision_update AFTER UPDATE ON relations BEGIN
  UPDATE store_revision SET revision = revision + 1;
END;
CREATE TRIGGER IF NOT EXISTS relations_revision_delete AFTER DELETE ON relations BEGIN
  UPDATE store_revision SET revision = revision + 1;
END;
CREATE TRIGGER IF NOT EXISTS vectors_revision_insert AFTER INSERT ON vectors BEGIN
  UPDATE store_revision SET revision = revision + 1;
END;
CREATE TRIGGER IF NOT EXISTS vectors_revision_update AFTER UPDATE ON vectors BEGIN
  UPDATE store_revision SET revision = revision + 1;
END;
CREATE TRIGGER IF NOT EXISTS vectors_revision_delete AFTER DELETE ON vectors BEGIN
  UPDATE store_revision SET revision = revision + 1;
END;
"#;

/// Record upserts, relation inserts, then record deletions in one transaction.
///
/// Record inputs have the same replacement and vector-retention semantics as
/// [`Database::put_records`]. Relations can refer to records created in this batch.
/// Deletions run last and cascade to vectors, metadata, and incident relations;
/// deleting a missing record is a no-op. Supply explicit IDs when referring to
/// new records or relations, since [`Database::commit_batch`] returns only a bool.
#[derive(Default)]
pub struct WriteBatch {
    pub records: Vec<RecordInput>,
    pub relations: Vec<RelationInput>,
    pub delete_records: Vec<String>,
}

/// A cloneable handle to one embedded KuiperDB database.
///
/// SQLite is authoritative. ANN indexes are process-local derived state and are
/// automatically rebuilt when their persistent generation changes.
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
    indexes: Arc<RwLock<IndexMap>>,
}

impl Database {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with(path, OpenOptions::default()).await
    }

    pub async fn open_with(path: impl AsRef<Path>, settings: OpenOptions) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            tokio::fs::create_dir_all(parent).await?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(settings.busy_timeout);
        let pool = SqlitePoolOptions::new()
            .max_connections(settings.max_connections)
            .connect_with(options)
            .await
            .with_context(|| format!("failed to open KuiperDB at {}", path.display()))?;
        sqlx::raw_sql(SCHEMA).execute(&pool).await?;
        Ok(Self {
            pool,
            indexes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn create_vector_space(&self, space: VectorSpace) -> Result<()> {
        validate_space(&space)?;
        if let Some(existing) = self.get_vector_space(&space.name).await? {
            if existing == space {
                return Ok(());
            }
            anyhow::bail!(
                "vector space '{}' already exists with a different contract",
                space.name
            );
        }
        sqlx::query(
            "INSERT INTO vector_spaces(name, dimensions, distance_metric, normalization, index_config) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&space.name)
        .bind(space.dimensions as i64)
        .bind(space.distance_metric.as_str())
        .bind(space.normalization.as_str())
        .bind(serde_json::to_string(&space.index)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_vector_space(&self, name: &str) -> Result<Option<VectorSpace>> {
        let row = sqlx::query(
            "SELECT name, dimensions, distance_metric, normalization, index_config FROM vector_spaces WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        row.map(vector_space_from_row).transpose()
    }

    pub async fn list_vector_spaces(&self) -> Result<Vec<VectorSpace>> {
        sqlx::query("SELECT name, dimensions, distance_metric, normalization, index_config FROM vector_spaces ORDER BY name")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(vector_space_from_row)
            .collect()
    }

    /// Changes ANN tuning without changing the mathematical vector-space contract.
    /// Advancing the generation invalidates indexes loaded by every process.
    pub async fn update_index_config(
        &self,
        space_name: &str,
        config: crate::IndexConfig,
    ) -> Result<()> {
        validate_index_config(&config)?;
        let result = sqlx::query(
            "UPDATE vector_spaces SET index_config = ?, generation = generation + 1 WHERE name = ?",
        )
        .bind(serde_json::to_string(&config)?)
        .bind(space_name)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            anyhow::bail!("unknown vector space '{space_name}'");
        }
        self.indexes.write().await.remove(space_name);
        Ok(())
    }

    /// Removes a vector space and its vectors; records and relations are retained.
    pub async fn delete_vector_space(&self, name: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM vector_spaces WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await?;
        self.indexes.write().await.remove(name);
        Ok(result.rows_affected() != 0)
    }

    pub async fn put_record(&self, input: RecordInput) -> Result<Record> {
        let mut records = self.put_records(vec![input]).await?;
        Ok(records.remove(0))
    }

    /// Atomically upserts a batch. Existing vectors in spaces omitted from an input are retained.
    pub async fn put_records(&self, inputs: Vec<RecordInput>) -> Result<Vec<Record>> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::with_capacity(inputs.len());
        let inputs = inputs
            .into_iter()
            .map(|mut input| {
                let id = input.id.get_or_insert_with(|| Uuid::new_v4().to_string());
                ids.push(id.clone());
                input
            })
            .collect();
        self.commit_batch(
            None,
            WriteBatch {
                records: inputs,
                ..Default::default()
            },
        )
        .await?;
        let mut records = Vec::with_capacity(ids.len());
        for id in ids {
            records.push(
                self.get_record(&id)
                    .await?
                    .context("record disappeared after commit")?,
            );
        }
        Ok(records)
    }

    /// Persistent token advanced by record, vector, and relation mutations.
    ///
    /// Treat this as an opaque equality token, not a transaction count. It can
    /// advance several times per commit and is shared by all database handles.
    /// Vector-space configuration and process-local index changes are not tracked.
    pub async fn revision(&self) -> Result<i64> {
        Ok(
            sqlx::query_scalar("SELECT revision FROM store_revision WHERE singleton = 1")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Atomically apply a batch, optionally conditioned on [`Self::revision`].
    ///
    /// Read the revision before reading the state used to prepare the batch.
    /// `Some(revision)` returns `Ok(false)` without writes if that token is stale;
    /// reread state and prepare a fresh batch before retrying. `None` applies the
    /// batch unconditionally. `Ok(true)` means committed, including an empty batch.
    /// Validation errors can be returned before the revision is checked.
    ///
    /// SQLite serializes writers across handles and processes. Errors before
    /// commit roll back all changes, including revision and index generations.
    /// Cancellation before commit rolls back through SQLx's transaction; once
    /// commit is in flight, cancellation does not establish whether it committed.
    pub async fn commit_batch(
        &self,
        expected_revision: Option<i64>,
        batch: WriteBatch,
    ) -> Result<bool> {
        let spaces = self.space_map().await?;
        let mut prepared = Vec::with_capacity(batch.records.len());
        for input in batch.records {
            let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
            if id.is_empty() {
                anyhow::bail!("record id cannot be empty");
            }
            let mut seen = HashSet::new();
            for vector in &input.vectors {
                if !seen.insert(&vector.space) {
                    anyhow::bail!(
                        "record '{id}' supplies vector space '{}' more than once",
                        vector.space
                    );
                }
                let space = spaces
                    .get(&vector.space)
                    .with_context(|| format!("unknown vector space '{}'", vector.space))?;
                validate_vector(space, &vector.values)?;
            }
            prepared.push((id, input.payload, input.metadata, input.vectors));
        }

        let mut transaction = self.pool.begin().await?;
        // Obtain the SQLite writer reservation before reading the revision.
        // This also avoids a deferred read-to-write upgrade race across pools.
        sqlx::query("UPDATE store_revision SET revision = revision WHERE singleton = 1")
            .execute(&mut *transaction)
            .await?;
        let revision: i64 =
            sqlx::query_scalar("SELECT revision FROM store_revision WHERE singleton = 1")
                .fetch_one(&mut *transaction)
                .await?;
        if expected_revision.is_some_and(|expected| expected != revision) {
            transaction.rollback().await?;
            return Ok(false);
        }
        let now = Utc::now();
        for (id, payload, metadata, vectors) in &prepared {
            sqlx::query(
                "INSERT INTO records(id, payload, metadata, created_at, updated_at) VALUES (?, ?, ?, ?, ?) \
                 ON CONFLICT(id) DO UPDATE SET payload=excluded.payload, metadata=excluded.metadata, updated_at=excluded.updated_at",
            )
            .bind(id)
            .bind(payload.as_ref().map(serde_json::to_string).transpose()?)
            .bind(serde_json::to_string(metadata)?)
            .bind(now)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
            sqlx::query("DELETE FROM record_metadata WHERE record_id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            for (key, value) in metadata {
                sqlx::query(
                    "INSERT INTO record_metadata(record_id, key, value_json) VALUES (?, ?, ?)",
                )
                .bind(id)
                .bind(key)
                .bind(serde_json::to_string(value)?)
                .execute(&mut *transaction)
                .await?;
            }
            for vector in vectors {
                sqlx::query(
                    "INSERT INTO vectors(record_id, space, values_blob) VALUES (?, ?, ?) \
                     ON CONFLICT(record_id, space) DO UPDATE SET values_blob=excluded.values_blob",
                )
                .bind(id)
                .bind(&vector.space)
                .bind(serialize_vector(&vector.values))
                .execute(&mut *transaction)
                .await?;
            }
        }
        for relation in batch.relations {
            let id = relation.id.unwrap_or_else(|| Uuid::new_v4().to_string());
            if id.is_empty() || relation.kind.is_empty() {
                anyhow::bail!("relation id and kind cannot be empty");
            }
            sqlx::query("INSERT INTO relations(id, source_id, target_id, kind, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(id).bind(relation.source_id).bind(relation.target_id)
                .bind(relation.kind).bind(serde_json::to_string(&relation.metadata)?)
                .bind(now).execute(&mut *transaction).await?;
        }
        for id in batch.delete_records {
            sqlx::query("DELETE FROM records WHERE id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn get_record(&self, id: &str) -> Result<Option<Record>> {
        sqlx::query(
            "SELECT id, payload, metadata, created_at, updated_at FROM records WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .map(|row| record_from_row(&row))
        .transpose()
    }

    pub async fn get_vector(&self, record_id: &str, space: &str) -> Result<Option<Vec<f32>>> {
        let blob: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT values_blob FROM vectors WHERE record_id = ? AND space = ?")
                .bind(record_id)
                .bind(space)
                .fetch_optional(&self.pool)
                .await?;
        blob.map(|blob| deserialize_vector(&blob)).transpose()
    }

    pub async fn list_records(&self, limit: usize, offset: usize) -> Result<Vec<Record>> {
        sqlx::query("SELECT id, payload, metadata, created_at, updated_at FROM records ORDER BY id LIMIT ? OFFSET ?")
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| record_from_row(&row))
            .collect()
    }

    pub async fn delete_record(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM records WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() != 0)
    }

    pub async fn delete_vector(&self, record_id: &str, space: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM vectors WHERE record_id = ? AND space = ?")
            .bind(record_id)
            .bind(space)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() != 0)
    }

    /// Vector-first nearest-neighbor retrieval. Filters are exact metadata equality constraints.
    pub async fn search(&self, query: VectorQuery) -> Result<Vec<SearchResult>> {
        let space = self
            .get_vector_space(&query.space)
            .await?
            .with_context(|| format!("unknown vector space '{}'", query.space))?;
        validate_vector(&space, &query.vector)?;
        if query.limit == 0 {
            return Ok(Vec::new());
        }

        // Filtering must be correct even when ANN candidate expansion would be insufficient.
        if !query.filter.is_empty()
            || !space.index.enabled
            || space.distance_metric != DistanceMetric::Cosine
        {
            return self.search_exact_with_space(&space, &query).await;
        }

        self.ensure_fresh_index(&space).await?;
        let neighbors = {
            let indexes = self.indexes.read().await;
            indexes
                .get(&space.name)
                .context("index was not loaded")?
                .search(&query.vector, query.limit)
        };
        let mut results = Vec::with_capacity(neighbors.len());
        for (id, ann_distance) in neighbors {
            if let Some(record) = self.get_record(&id).await? {
                results.push(SearchResult {
                    record,
                    distance: ann_distance,
                });
            }
        }
        Ok(results)
    }

    /// Exhaustive retrieval, useful for correctness checks and filtered queries.
    pub async fn search_exact(&self, query: VectorQuery) -> Result<Vec<SearchResult>> {
        let space = self
            .get_vector_space(&query.space)
            .await?
            .with_context(|| format!("unknown vector space '{}'", query.space))?;
        validate_vector(&space, &query.vector)?;
        self.search_exact_with_space(&space, &query).await
    }

    async fn search_exact_with_space(
        &self,
        space: &VectorSpace,
        query: &VectorQuery,
    ) -> Result<Vec<SearchResult>> {
        let mut statement = QueryBuilder::<Sqlite>::new(
            "SELECT r.id, r.payload, r.metadata, r.created_at, r.updated_at, v.values_blob \
             FROM vectors v JOIN records r ON r.id = v.record_id WHERE v.space = ",
        );
        statement.push_bind(&space.name);
        if !query.filter.is_empty() {
            statement.push(" AND v.record_id IN (SELECT record_id FROM record_metadata WHERE ");
            for (position, (key, value)) in query.filter.equals.iter().enumerate() {
                if position != 0 {
                    statement.push(" OR ");
                }
                statement
                    .push("(key = ")
                    .push_bind(key)
                    .push(" AND value_json = ")
                    .push_bind(serde_json::to_string(value)?)
                    .push(")");
            }
            statement
                .push(" GROUP BY record_id HAVING COUNT(*) = ")
                .push_bind(query.filter.equals.len() as i64)
                .push(")");
        }
        let rows = statement.build().fetch_all(&self.pool).await?;
        let mut results = Vec::new();
        for row in rows {
            let record = record_from_row(&row)?;
            if !query.filter.matches(&record.metadata) {
                continue;
            }
            let blob: Vec<u8> = row.try_get("values_blob")?;
            let vector = deserialize_vector(&blob)?;
            results.push(SearchResult {
                distance: distance(space.distance_metric, &query.vector, &vector),
                record,
            });
        }
        results.sort_by(|left, right| {
            left.distance
                .total_cmp(&right.distance)
                .then_with(|| left.record.id.cmp(&right.record.id))
        });
        results.truncate(query.limit);
        Ok(results)
    }

    /// Rebuilds derived ANN state and atomically swaps it into use when complete.
    pub async fn rebuild_index(&self, space_name: &str) -> Result<IndexStatus> {
        let space = self
            .get_vector_space(space_name)
            .await?
            .with_context(|| format!("unknown vector space '{space_name}'"))?;
        if !space.index.enabled {
            anyhow::bail!("indexing is disabled for vector space '{space_name}'");
        }
        if space.distance_metric != DistanceMetric::Cosine {
            anyhow::bail!(
                "HNSW currently supports cosine vector spaces; exact search remains available"
            );
        }
        self.build_current_index(&space).await?;
        self.index_status(space_name).await
    }

    /// Drops only process-local derived state. Persisted records and vectors are untouched.
    pub async fn drop_index(&self, space_name: &str) {
        self.indexes.write().await.remove(space_name);
    }

    pub async fn index_status(&self, space_name: &str) -> Result<IndexStatus> {
        let database_generation = self.generation(space_name).await?;
        let indexes = self.indexes.read().await;
        let loaded = indexes.get(space_name);
        let loaded_generation = loaded.map(VectorIndex::generation);
        Ok(IndexStatus {
            space: space_name.to_owned(),
            database_generation,
            loaded_generation,
            indexed_vectors: loaded.map_or(0, VectorIndex::len),
            stale: loaded_generation.is_some_and(|generation| generation != database_generation),
        })
    }

    pub async fn create_relation(&self, input: RelationInput) -> Result<Relation> {
        let relation = Relation {
            id: input.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            source_id: input.source_id,
            target_id: input.target_id,
            kind: input.kind,
            metadata: input.metadata,
            created_at: Utc::now(),
        };
        if relation.id.is_empty() || relation.kind.is_empty() {
            anyhow::bail!("relation id and kind cannot be empty");
        }
        sqlx::query("INSERT INTO relations(id, source_id, target_id, kind, metadata, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&relation.id)
            .bind(&relation.source_id)
            .bind(&relation.target_id)
            .bind(&relation.kind)
            .bind(serde_json::to_string(&relation.metadata)?)
            .bind(relation.created_at)
            .execute(&self.pool)
            .await?;
        Ok(relation)
    }

    pub async fn delete_relation(&self, id: &str) -> Result<bool> {
        Ok(sqlx::query("DELETE FROM relations WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            != 0)
    }

    pub async fn relations_for(&self, record_id: &str) -> Result<Vec<Relation>> {
        sqlx::query("SELECT id, source_id, target_id, kind, metadata, created_at FROM relations WHERE source_id = ? OR target_id = ? ORDER BY created_at, id")
            .bind(record_id)
            .bind(record_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(relation_from_row)
            .collect()
    }

    pub async fn all_relations(&self) -> Result<Vec<Relation>> {
        sqlx::query("SELECT id, source_id, target_id, kind, metadata, created_at FROM relations ORDER BY created_at, id")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(relation_from_row)
            .collect()
    }

    async fn ensure_fresh_index(&self, space: &VectorSpace) -> Result<()> {
        let generation = self.generation(&space.name).await?;
        if self
            .indexes
            .read()
            .await
            .get(&space.name)
            .is_some_and(|index| index.generation() == generation)
        {
            return Ok(());
        }
        self.build_current_index(space).await
    }

    async fn build_current_index(&self, space: &VectorSpace) -> Result<()> {
        // If another process writes during a rebuild, retry rather than publishing stale state.
        for _ in 0..3 {
            let generation = self.generation(&space.name).await?;
            let vectors = self.load_vectors(&space.name).await?;
            let index = VectorIndex::build(space.dimensions, &space.index, generation, vectors)?;
            if self.generation(&space.name).await? == generation {
                self.indexes.write().await.insert(space.name.clone(), index);
                return Ok(());
            }
        }
        anyhow::bail!(
            "vector space '{}' changed repeatedly while rebuilding its index",
            space.name
        )
    }

    async fn load_vectors(&self, space: &str) -> Result<Vec<(String, Vec<f32>)>> {
        let rows = sqlx::query(
            "SELECT record_id, values_blob FROM vectors WHERE space = ? ORDER BY record_id",
        )
        .bind(space)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("record_id")?;
                let blob: Vec<u8> = row.try_get("values_blob")?;
                Ok((id, deserialize_vector(&blob)?))
            })
            .collect()
    }

    async fn generation(&self, space: &str) -> Result<i64> {
        sqlx::query_scalar("SELECT generation FROM vector_spaces WHERE name = ?")
            .bind(space)
            .fetch_optional(&self.pool)
            .await?
            .with_context(|| format!("unknown vector space '{space}'"))
    }

    async fn space_map(&self) -> Result<HashMap<String, VectorSpace>> {
        Ok(self
            .list_vector_spaces()
            .await?
            .into_iter()
            .map(|space| (space.name.clone(), space))
            .collect())
    }
}

fn validate_space(space: &VectorSpace) -> Result<()> {
    if space.name.is_empty() {
        anyhow::bail!("vector space name cannot be empty");
    }
    if space.dimensions == 0 {
        anyhow::bail!("vector space dimensions must be greater than zero");
    }
    validate_index_config(&space.index)?;
    Ok(())
}

fn validate_index_config(config: &crate::IndexConfig) -> Result<()> {
    if !(2..=64).contains(&config.hnsw_m) {
        anyhow::bail!("hnsw_m must be between 2 and 64");
    }
    if config.hnsw_ef_construction == 0 || config.hnsw_ef_search == 0 {
        anyhow::bail!("HNSW ef values must be greater than zero");
    }
    Ok(())
}

fn validate_vector(space: &VectorSpace, values: &[f32]) -> Result<()> {
    if values.len() != space.dimensions {
        anyhow::bail!(
            "vector for space '{}' has {} dimensions; expected {}",
            space.name,
            values.len(),
            space.dimensions
        );
    }
    if values.iter().any(|value| !value.is_finite()) {
        anyhow::bail!("vectors must contain only finite values");
    }
    if space.normalization == Normalization::Unit {
        let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
        if (norm - 1.0).abs() > 1e-3 {
            anyhow::bail!("vector for space '{}' must have unit norm", space.name);
        }
    }
    Ok(())
}

fn serialize_vector(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn deserialize_vector(blob: &[u8]) -> Result<Vec<f32>> {
    if !blob.len().is_multiple_of(4) {
        anyhow::bail!("corrupt persisted vector: byte length is not divisible by four");
    }
    Ok(blob
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
        .collect())
}

fn vector_space_from_row(row: SqliteRow) -> Result<VectorSpace> {
    let dimensions: i64 = row.try_get("dimensions")?;
    Ok(VectorSpace {
        name: row.try_get("name")?,
        dimensions: usize::try_from(dimensions).context("invalid persisted vector dimensions")?,
        distance_metric: DistanceMetric::parse(row.try_get("distance_metric")?)?,
        normalization: Normalization::parse(row.try_get("normalization")?)?,
        index: serde_json::from_str(row.try_get("index_config")?)?,
    })
}

fn record_from_row(row: &SqliteRow) -> Result<Record> {
    let payload: Option<String> = row.try_get("payload")?;
    let metadata: String = row.try_get("metadata")?;
    Ok(Record {
        id: row.try_get("id")?,
        payload: payload
            .map(|value| serde_json::from_str(&value))
            .transpose()?,
        metadata: serde_json::from_str(&metadata)?,
        created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
        updated_at: row.try_get::<DateTime<Utc>, _>("updated_at")?,
    })
}

fn relation_from_row(row: SqliteRow) -> Result<Relation> {
    let metadata: String = row.try_get("metadata")?;
    Ok(Relation {
        id: row.try_get("id")?,
        source_id: row.try_get("source_id")?,
        target_id: row.try_get("target_id")?,
        kind: row.try_get("kind")?,
        metadata: serde_json::from_str(&metadata)?,
        created_at: row.try_get("created_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn database() -> (tempfile::TempDir, Database) {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(directory.path().join("test.db"))
            .await
            .unwrap();
        database
            .create_vector_space(VectorSpace {
                name: "semantic".into(),
                dimensions: 3,
                distance_metric: DistanceMetric::Cosine,
                normalization: Normalization::Unit,
                index: Default::default(),
            })
            .await
            .unwrap();
        (directory, database)
    }

    fn input(id: &str, vector: [f32; 3], category: &str) -> RecordInput {
        RecordInput {
            id: Some(id.into()),
            payload: Some(json!({"text": id})),
            metadata: HashMap::from([("category".into(), json!(category))]),
            vectors: vec![NamedVector {
                space: "semantic".into(),
                values: vector.to_vec(),
            }],
        }
    }

    #[tokio::test]
    async fn exact_search_and_filtering_are_correct() {
        let (_directory, database) = database().await;
        database
            .put_records(vec![
                input("a", [1.0, 0.0, 0.0], "x"),
                input("b", [0.0, 1.0, 0.0], "y"),
            ])
            .await
            .unwrap();
        let results = database
            .search(VectorQuery {
                space: "semantic".into(),
                vector: vec![1.0, 0.0, 0.0],
                limit: 10,
                filter: MetadataFilter {
                    equals: HashMap::from([("category".into(), json!("x"))]),
                },
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].record.id, "a");
        assert!(results[0].distance.abs() < 1e-6);
    }

    #[tokio::test]
    async fn updates_deletes_and_rebuilds_keep_stable_ids() {
        let (_directory, database) = database().await;
        database
            .put_record(input("stable", [1.0, 0.0, 0.0], "old"))
            .await
            .unwrap();
        database.rebuild_index("semantic").await.unwrap();
        database
            .put_record(input("stable", [0.0, 1.0, 0.0], "new"))
            .await
            .unwrap();
        assert!(database.index_status("semantic").await.unwrap().stale);
        let results = database
            .search(VectorQuery {
                space: "semantic".into(),
                vector: vec![0.0, 1.0, 0.0],
                limit: 1,
                filter: Default::default(),
            })
            .await
            .unwrap();
        assert_eq!(results[0].record.id, "stable");
        assert!(!database.index_status("semantic").await.unwrap().stale);
        database.drop_index("semantic").await;
        assert!(database.get_record("stable").await.unwrap().is_some());
        assert!(database.delete_record("stable").await.unwrap());
        assert!(database
            .search(VectorQuery {
                space: "semantic".into(),
                vector: vec![0.0, 1.0, 0.0],
                limit: 1,
                filter: Default::default()
            })
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn separate_process_handles_detect_stale_indexes_and_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared.db");
        let first = Database::open(&path).await.unwrap();
        first
            .create_vector_space(VectorSpace {
                name: "semantic".into(),
                dimensions: 3,
                distance_metric: DistanceMetric::Cosine,
                normalization: Normalization::Unit,
                index: Default::default(),
            })
            .await
            .unwrap();
        first
            .put_record(input("a", [1.0, 0.0, 0.0], "x"))
            .await
            .unwrap();
        first.rebuild_index("semantic").await.unwrap();
        let second = Database::open(&path).await.unwrap();
        second
            .put_record(input("b", [0.0, 1.0, 0.0], "x"))
            .await
            .unwrap();
        assert!(first.index_status("semantic").await.unwrap().stale);
        let found = first
            .search(VectorQuery {
                space: "semantic".into(),
                vector: vec![0.0, 1.0, 0.0],
                limit: 1,
                filter: Default::default(),
            })
            .await
            .unwrap();
        assert_eq!(found[0].record.id, "b");
        drop(first);
        let reopened = Database::open(&path).await.unwrap();
        assert_eq!(reopened.get_record("a").await.unwrap().unwrap().id, "a");
    }

    #[tokio::test]
    async fn competing_writers_complete_under_wal() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("writers.db");
        let first = Database::open(&path).await.unwrap();
        first
            .create_vector_space(VectorSpace {
                name: "semantic".into(),
                dimensions: 3,
                distance_metric: DistanceMetric::Cosine,
                normalization: Normalization::Unit,
                index: Default::default(),
            })
            .await
            .unwrap();
        let second = Database::open(&path).await.unwrap();
        let left = tokio::spawn(async move {
            for n in 0..25 {
                first
                    .put_record(input(&format!("left-{n}"), [1.0, 0.0, 0.0], "x"))
                    .await
                    .unwrap();
            }
        });
        let right = tokio::spawn(async move {
            for n in 0..25 {
                second
                    .put_record(input(&format!("right-{n}"), [0.0, 1.0, 0.0], "x"))
                    .await
                    .unwrap();
            }
        });
        left.await.unwrap();
        right.await.unwrap();
        let reopened = Database::open(&path).await.unwrap();
        assert_eq!(reopened.list_records(100, 0).await.unwrap().len(), 50);
    }

    #[tokio::test]
    async fn invalid_batch_rolls_back_before_writing() {
        let (_directory, database) = database().await;
        let error = database
            .put_records(vec![
                input("ok", [1.0, 0.0, 0.0], "x"),
                input("bad", [2.0, 0.0, 0.0], "x"),
            ])
            .await
            .unwrap_err();
        assert!(error.to_string().contains("unit norm"));
        assert!(database.get_record("ok").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn vector_space_contracts_and_metrics_are_enforced() {
        let (_directory, database) = database().await;
        let invalid = input("wrong-dimensions", [1.0, 0.0, 0.0], "x");
        let mut invalid = invalid;
        invalid.vectors[0].values.pop();
        assert!(database
            .put_record(invalid)
            .await
            .unwrap_err()
            .to_string()
            .contains("dimensions"));

        database
            .create_vector_space(VectorSpace {
                name: "euclidean".into(),
                dimensions: 2,
                distance_metric: DistanceMetric::Euclidean,
                normalization: Normalization::None,
                index: Default::default(),
            })
            .await
            .unwrap();
        database
            .put_records(vec![
                RecordInput {
                    id: Some("near".into()),
                    vectors: vec![NamedVector {
                        space: "euclidean".into(),
                        values: vec![1.0, 1.0],
                    }],
                    ..Default::default()
                },
                RecordInput {
                    id: Some("far".into()),
                    vectors: vec![NamedVector {
                        space: "euclidean".into(),
                        values: vec![9.0, 9.0],
                    }],
                    ..Default::default()
                },
            ])
            .await
            .unwrap();
        let result = database
            .search(VectorQuery {
                space: "euclidean".into(),
                vector: vec![0.0, 0.0],
                limit: 1,
                filter: Default::default(),
            })
            .await
            .unwrap();
        assert_eq!(result[0].record.id, "near");
    }

    #[tokio::test]
    async fn index_configuration_changes_invalidate_derived_state() {
        let (_directory, database) = database().await;
        database
            .put_record(input("a", [1.0, 0.0, 0.0], "x"))
            .await
            .unwrap();
        database.rebuild_index("semantic").await.unwrap();
        let generation = database
            .index_status("semantic")
            .await
            .unwrap()
            .database_generation;
        database
            .update_index_config(
                "semantic",
                crate::IndexConfig {
                    hnsw_ef_search: 200,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let status = database.index_status("semantic").await.unwrap();
        assert!(status.database_generation > generation);
        assert_eq!(status.loaded_generation, None);
        database
            .search(VectorQuery {
                space: "semantic".into(),
                vector: vec![1.0, 0.0, 0.0],
                limit: 1,
                filter: Default::default(),
            })
            .await
            .unwrap();
        assert!(!database.index_status("semantic").await.unwrap().stale);
    }

    #[tokio::test]
    async fn hnsw_recall_tracks_exact_neighbors() {
        let (_directory, database) = database().await;
        let records = (0..200)
            .map(|number| {
                let angle = number as f32 * std::f32::consts::TAU / 200.0;
                input(
                    &format!("point-{number}"),
                    [angle.cos(), angle.sin(), 0.0],
                    "x",
                )
            })
            .collect();
        database.put_records(records).await.unwrap();
        let query = VectorQuery {
            space: "semantic".into(),
            vector: vec![0.0, 1.0, 0.0],
            limit: 10,
            filter: Default::default(),
        };
        let exact: HashSet<_> = database
            .search_exact(query.clone())
            .await
            .unwrap()
            .into_iter()
            .map(|result| result.record.id)
            .collect();
        let approximate = database.search(query).await.unwrap();
        let recalled = approximate
            .iter()
            .filter(|result| exact.contains(&result.record.id))
            .count();
        assert!(recalled >= 9, "Recall@10 was {recalled}/10");
    }

    #[tokio::test]
    async fn process_writer_helper() {
        let Ok(path) = std::env::var("KUIPERDB_TEST_WRITER_PATH") else {
            return;
        };
        let mode = std::env::var("KUIPERDB_TEST_WRITER_MODE").unwrap();
        let database = Database::open(path).await.unwrap();
        if mode == "crash" {
            let mut connection = database.pool().acquire().await.unwrap();
            sqlx::query("BEGIN IMMEDIATE")
                .execute(&mut *connection)
                .await
                .unwrap();
            sqlx::query("INSERT INTO records(id, metadata, created_at, updated_at) VALUES ('uncommitted', '{}', datetime('now'), datetime('now'))")
                .execute(&mut *connection).await.unwrap();
            std::process::exit(17);
        }
        for number in 0..20 {
            database
                .put_record(input(
                    &format!("{mode}-{number}"),
                    [1.0, 0.0, 0.0],
                    "process",
                ))
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn competing_process_writers_and_crash_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("processes.db");
        let database = Database::open(&path).await.unwrap();
        database
            .create_vector_space(VectorSpace {
                name: "semantic".into(),
                dimensions: 3,
                distance_metric: DistanceMetric::Cosine,
                normalization: Normalization::Unit,
                index: Default::default(),
            })
            .await
            .unwrap();
        let executable = std::env::current_exe().unwrap();
        let spawn = |mode: &str| {
            std::process::Command::new(&executable)
                .args([
                    "--exact",
                    "store::tests::process_writer_helper",
                    "--nocapture",
                ])
                .env("KUIPERDB_TEST_WRITER_PATH", &path)
                .env("KUIPERDB_TEST_WRITER_MODE", mode)
                .spawn()
                .unwrap()
        };
        let left = spawn("left");
        let right = spawn("right");
        assert!(left.wait_with_output().unwrap().status.success());
        assert!(right.wait_with_output().unwrap().status.success());
        assert_eq!(database.list_records(100, 0).await.unwrap().len(), 40);

        let crashed = spawn("crash").wait_with_output().unwrap();
        assert_eq!(crashed.status.code(), Some(17));
        let reopened = Database::open(&path).await.unwrap();
        assert!(reopened.get_record("uncommitted").await.unwrap().is_none());
        reopened
            .put_record(input("after-crash", [1.0, 0.0, 0.0], "process"))
            .await
            .unwrap();
    }
}
