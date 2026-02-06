# kuiperdb

A lightweight, embeddable vector database built with Rust and SQLite. KuiperDb provides both a library and REST API for storing documents with vector embeddings and performing semantic similarity searches, making it ideal for local development, testing, embedded applications, or distributed environments.

## Overview

KuiperDb combines SQLite's reliability with HNSW vector indexing to create a minimal yet powerful document store. The project features:

- **Vector Search**: Store and query documents using high-dimensional embeddings (configurable dimensions)
- **HNSW Indexing**: Fast approximate nearest neighbor search with configurable parameters
- **Dual Usage Modes**:
  - **Embedded**: Use as a Rust library for direct integration
  - **REST API**: Run as a server and connect via HTTP client
- **Flexible Embedding**: Support for external embedding services with caching
- **Background Workers**: Asynchronous embedding generation and processing
- **Graph Operations**: Document relationship tracking and graph queries
- **Feature Flags**: Configurable features for different deployment scenarios

## Architecture

KuiperDb is structured as a Rust workspace with three crates:

- **kuiperdb-core** - Core library with storage, indexing, search, and embedding logic
- **kuiperdb-client** - HTTP client for connecting to KuiperDb REST API
- **kuiperdb** - Main binary and server runtime (can also be used as library)

## Quick Start

### As Library (Embedded)

```rust
use kuiperdb_core::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut store = store::DocumentStore::new("./data".to_string()).await?;
    
    // Add document
    let doc_id = store.add_document(/* ... */).await?;
    
    // Search
    let results = store.search(/* ... */).await?;
    
    Ok(())
}
```

### As Server + Client

```bash
# Start server
cargo run --release

# In another terminal/application
```

```rust
use kuiperdb_client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new("http://localhost:8080");
    
    // Add document
    let doc_id = client.add_document(
        "Hello world".to_string(),
        None
    ).await?;
    
    // Search
    let results = client.search("greeting".to_string(), 10).await?;
    
    Ok(())
}
```

## Building

```bash
# Build entire workspace
cargo build --release

# Run server
cargo run --release
```

For detailed build instructions, cross-compilation, CI/CD, and development commands, see [BUILD.md](BUILD.md).

---

## Disclaimer

*This project was generated with the assistance of Claude Sonnet 4.5 via GitHub Copilot in VS Code.*

*I include these tools for transparency and provenance tracking in case this is ever useful to others in the future.*

| Tool                | Version | Date       |
| ------------------- | ------- | ---------- |
| VsCode              | 1.107.1 | 2026.01.07 |
| github.copilot-chat | 0.35.3  | 2026.01.07 |
| Claude Sonnet       | 4.5     | 2026.01.07 |