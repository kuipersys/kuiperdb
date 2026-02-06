# KuiperDb Examples

This directory contains example applications demonstrating how to use KuiperDb as an embedded library in your Rust applications.

## Overview

KuiperDb can be used in two ways:
1. **As an embedded library** - Integrate directly into your Rust application (shown here)
2. **As a REST API server** - Run as a service and connect via HTTP client

These examples focus on embedded usage, showing how to use `kuiperdb-core` directly in your code.

## Examples

### 1. Simple Embedded (`simple_embedded.rs`)

**Best for:** Beginners, quick proof-of-concept

A minimal example showing the basic workflow:
- Initialize a DocumentStore
- Add a document
- Perform a search

```bash
cargo run --example simple_embedded
```

**What you'll learn:**
- Basic DocumentStore setup
- Creating documents
- Simple hybrid search

---

### 2. Embedded App (`embedded_app.rs`)

**Best for:** Understanding core features

A comprehensive example covering the main features:
- Document creation with metadata and tags
- Different search types (vector, fulltext, hybrid)
- Document retrieval and deletion
- Working with chunks
- Cache and database statistics

```bash
cargo run --example embedded_app
```

**What you'll learn:**
- Full CRUD operations
- Search strategies and when to use each
- Metadata and tagging
- Performance monitoring with statistics

---

### 3. Advanced Embedded (`advanced_embedded.rs`)

**Best for:** Production applications, complex use cases

An advanced example demonstrating:
- Graph operations and relationships
- Advanced filtering
- Batch operations
- Semantic discovery
- Document updates and versioning
- Complete statistics and monitoring

```bash
cargo run --example advanced_embedded
```

**What you'll learn:**
- Building knowledge graphs
- Creating and querying document relationships
- Advanced search filters
- Performance optimization techniques
- Monitoring and observability

---

### 4. With Real Embeddings (`with_embeddings.rs`)

**Best for:** Understanding vector search with real embeddings

A practical example using a local embedding server (LM Studio):
- Connecting to an embedding API
- Generating real vector embeddings
- Semantic similarity search
- Comparing search strategies (vector vs fulltext vs hybrid)
- Working with high-dimensional vectors

```bash
# Prerequisites:
# 1. Start LM Studio (https://lmstudio.ai/)
# 2. Load an embedding model (e.g., nomic-embed-text)
# 3. Start the local server (Server tab)

cargo run --example with_embeddings
```

**📘 See [LM_STUDIO_SETUP.md](LM_STUDIO_SETUP.md) for detailed setup instructions**

**What you'll learn:**
- Setting up an embedder with real embedding models
- Vector similarity search in practice
- How hybrid search combines vector + fulltext
- Performance characteristics of different search types
- Working with embedding dimensions

---

## Quick Start

1. **Ensure you have Rust installed:**
   ```bash
   rustc --version
   ```

2. **Run any example:**
   ```bash
   # From the project root
   cargo run --example simple_embedded
   cargo run --example embedded_app
   cargo run --example advanced_embedded
   ```

3. **Check the generated data:**
   Each example creates its own data directory:
   - `./data/simple_example/`
   - `./data/example_embedded/`
   - `./data/advanced_example/`

## Common Patterns

### Initializing a Store

```rust
use KuiperDb_core::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut store = store::DocumentStore::new("./data".to_string()).await?;
    // ... use store
    Ok(())
}
```

### Adding Documents

```rust
use std::collections::HashMap;

let request = StoreDocumentRequest {
    id: None,  // Auto-generate ID
    content: "Your document content here".to_string(),
    metadata: HashMap::new(),
    tags: vec!["tag1".to_string()],
    vectorize: true,  // Enable vector embedding
};

let doc = store.store_document(request).await?;
```

### Searching

```rust
let request = SearchRequest {
    query: "search query".to_string(),
    search_type: SearchType::Hybrid,  // Vector + Fulltext
    limit: Some(10),
    filters: HashMap::new(),
    include_chunks: true,
    group_by_parent: false,
};

let results = store.search(request).await?;
```

### Search Types

- **`SearchType::Hybrid`** - Combines vector similarity and fulltext matching (recommended default)
- **`SearchType::Vector`** - Semantic similarity only (best for conceptual queries)
- **`SearchType::FullText`** - Keyword matching only (best for exact terms)

## Building Your Own App

To use KuiperDb in your own project:

1. **Add dependency to `Cargo.toml`:**
   ```toml
   [dependencies]
   kuiperdb-core = { path = "../kuiperdb-core" }
   tokio = { version = "1", features = ["full"] }
   anyhow = "1"
   ```

2. **Import and initialize:**
   ```rust
   use KuiperDb_core::*;
   
   #[tokio::main]
   async fn main() -> anyhow::Result<()> {
       let mut store = store::DocumentStore::new("./my_data".to_string()).await?;
       // Your code here
       Ok(())
   }
   ```

3. **Review the examples** to see different usage patterns and choose what fits your needs.

## Configuration

KuiperDb can be configured through `config.json` files. See the main [README.md](../README.md) for configuration options.

Key settings:
- **Embedding service URL** - Where to get vector embeddings
- **Vector dimensions** - Size of embedding vectors
- **HNSW parameters** - Index performance tuning
- **Chunking settings** - How to split large documents

## Troubleshooting

**Problem:** "Failed to connect to embedding service"
- **Solution:** Check your embedding service is running or configure a different embedder

**Problem:** "Database locked"
- **Solution:** Only one process can write to SQLite at a time. Ensure you're not running multiple instances.

**Problem:** "No search results"
- **Solution:** Documents may not be embedded yet. Check `doc.is_embedded` or wait for background worker.

## Next Steps

After running these examples:

1. **Read the API documentation** in `docs/`
2. **Explore the source code** in `kuiperdb-core/src/`
3. **Check out the REST API** by running the server: `cargo run --release`
4. **Build your own application** using these examples as templates

## Contributing

Found a bug in an example? Have an idea for a new example? Please open an issue or submit a PR!

## License

These examples are part of the KuiperDb project and share the same license. See [LICENSE](../LICENSE) for details.
