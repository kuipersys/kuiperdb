# KuiperDB

KuiperDB is a model-agnostic embedded vector database built on SQLite. Applications give it prepared records and vectors; KuiperDB provides stable identities, named vector spaces, indexed metadata filtering, relationships, nearest-neighbor retrieval, persistence, and derived HNSW indexes.

Parsing, chunking, model inference, embedding caches, model management, and HTTP behavior are deliberately outside `kuiperdb-core`.

## Workspace

- `kuiperdb-core`: the embedded storage engine. It has no HTTP, model, tokenizer, cache, or worker dependencies.
- `kuiperdb-server`: an optional Actix HTTP adapter over the same embedded API.
- `kuiperdb-rs`: a client for that HTTP adapter.

## Embedded usage

```rust
use kuiperdb_core::{
    Database, DistanceMetric, NamedVector, Normalization, RecordInput, VectorQuery, VectorSpace,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = Database::open("./data/app.db").await?;
    db.create_vector_space(VectorSpace {
        name: "image-features".into(),
        dimensions: 3,
        distance_metric: DistanceMetric::Cosine,
        normalization: Normalization::Unit,
        index: Default::default(),
    }).await?;

    db.put_record(RecordInput {
        id: Some("asset-42".into()),
        payload: Some(serde_json::json!({"uri": "images/42.png"})),
        metadata: [("tenant".into(), serde_json::json!("acme"))].into(),
        vectors: vec![NamedVector {
            space: "image-features".into(),
            values: vec![1.0, 0.0, 0.0],
        }],
    }).await?;

    let nearest = db.search(VectorQuery {
        space: "image-features".into(),
        vector: vec![1.0, 0.0, 0.0],
        limit: 10,
        filter: Default::default(),
    }).await?;
    println!("{nearest:#?}");
    Ok(())
}
```

## Server

The server uses `KUIPERDB_PATH` (default `./data/kuiper.db`) and `KUIPERDB_BIND` (default `0.0.0.0:17001`).

```bash
cargo run --release -p kuiperdb-server
```

Its primitive search endpoint is `POST /search` and accepts a named space plus a vector. It never embeds query text.

## Correctness and benchmarks

```bash
cargo test --workspace --all-targets
cargo bench -p kuiperdb-core
```

The tests exercise vector compatibility, exact and filtered retrieval, record updates and deletion, index rebuilds, reopen behavior, WAL readers/writers, cross-handle stale-index detection, and atomic batch failure. Benchmarks separate ANN query, exact query, index construction, and database-open time.

See [architecture](docs/architecture.md), [SQLite concurrency](docs/concurrency.md), and [index lifecycle](docs/index-lifecycle.md) for the behavioral contracts.
