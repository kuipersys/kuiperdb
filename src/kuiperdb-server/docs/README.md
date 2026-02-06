# kuiperdb-rust API Documentation

Complete REST API reference for kuiperdb-rust, a high-performance document database with GPU embeddings, hybrid search, and knowledge graphs.

---

## Quick Start

### Start Server
```bash
cd kuiperdb-rust
cargo run --release
```

**Default:** `http://localhost:8081`

### Basic Example
```bash
# Create database
# (implicit - databases created on first document insert)

# Store document
curl -X POST http://localhost:8081/db/mydb/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Rust is a systems programming language",
    "metadata": {"author": "Alice", "year": 2026}
  }'

# Search
curl -X POST http://localhost:8081/db/mydb/documents/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "programming languages",
    "type": "hybrid"
  }'
```

---

## Features

✅ **Document Storage** - SQLite-backed with JSON metadata  
✅ **Full-Text Search** - FTS5 with BM25 ranking  
✅ **Vector Search** - GPU embeddings + HNSW index  
✅ **Hybrid Search** - RRF merging of BM25 + vector  
✅ **Embedding Cache** - 2-tier (memory + disk) with 11.2x speedup  
✅ **Background Worker** - Parallel async embedding (10 workers)  
✅ **Document Chunking** - Auto-split large docs (>512 tokens)  
✅ **Knowledge Graphs** - N:N relationships with graph algorithms  
✅ **Feature Toggles** - Enable/disable features via config  

---

## API Documentation by Feature

### Core Operations
- **[Documents API](api-documents.md)** - CRUD operations, health check
- **[Search API](api-search.md)** - Full-text, vector, hybrid search

### Advanced Features
- **[Chunking API](api-chunking.md)** - Auto-chunk large documents
- **[Relations API](api-relations.md)** - Create document relationships
- **[Graph API](api-graph.md)** - Graph traversal, pathfinding, stats

---

## Configuration

### config.json
```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8081
  },
  "database": {
    "base_path": "./data"
  },
  "embedding": {
    "api_url": "https://192.168.91.57/embed/v1/embeddings",
    "api_key": "",
    "model": "default",
    "dimensions": 2560,
    "timeout_seconds": 30
  },
  "features": {
    "embedding_job": true,
    "embedding_cache": true,
    "vector_index": true,
    "hybrid_search": true,
    "chunking": true,
    "document_relations": true
  },
  "worker": {
    "enabled": true,
    "interval_seconds": 10,
    "batch_size": 4,
    "num_workers": 10
  },
  "cache": {
    "memory_capacity": 10000,
    "ttl_seconds": 3600
  },
  "vector_index": {
    "enabled": true,
    "threshold": 1000,
    "m": 16,
    "ef_construction": 200,
    "ef_search": 100
  },
  "chunking": {
    "enabled": true,
    "token_threshold": 512,
    "chunk_size": 512,
    "chunk_overlap": 50,
    "strategy": "fixed_tokens"
  }
}
```

---

## Performance

### Benchmarks (Current System)

| Operation | Latency | Notes |
|-----------|---------|-------|
| Document store | ~5ms | Without embedding |
| GPU embedding | ~40ms | Single document |
| Cache hit | ~3.6ms | 11.2x faster than GPU |
| FTS5 search | ~5-10ms | 15 documents |
| Vector search (brute) | ~10-20ms | 15 documents |
| Vector search (HNSW) | ~200ms | 200K documents |
| Hybrid search | ~240ms | With curl |
| Background worker | 71 docs/sec | 10 workers, batch-4 |

**Note:** PowerShell adds ~1800ms overhead. Use curl for benchmarks.

---

## Architecture

```
Client
  ↓ HTTP
API Layer (actix-web)
  ↓
Search Orchestrator
  ├─ FTS5 (BM25)
  ├─ Vector Search (HNSW/brute-force)
  └─ RRF Merge
  ↓
Storage Layer (SQLite)
  ├─ Documents
  ├─ Embeddings (BLOB)
  ├─ FTS5 Index
  ├─ Document Relations
  └─ Embedding Cache
  ↓
Background Worker
  ├─ 10 parallel workers
  ├─ Batch-4 GPU calls
  └─ 2-tier cache check
  ↓
GPU Embedding (llama.cpp)
```

---

## Feature Toggles

Control features via `config.json`:

| Feature | Default | Description |
|---------|---------|-------------|
| `embedding_job` | true | Background embedding worker |
| `embedding_cache` | true | 2-tier embedding cache |
| `vector_index` | true | HNSW approximate NN search |
| `hybrid_search` | true | RRF merging of BM25 + vector |
| `chunking` | true | Auto-chunk large documents |
| `document_relations` | true | Knowledge graph relationships |

---

## Common Workflows

### 1. Simple Document Search
```bash
# Store
curl -X POST http://localhost:8081/db/kb/docs \
  -H "Content-Type: application/json" \
  -d '{"content": "AI is transforming healthcare"}'

# Search
curl -X POST http://localhost:8081/db/kb/docs/search \
  -H "Content-Type: application/json" \
  -d '{"query": "medical AI", "type": "hybrid"}'
```

---

### 2. RAG with Chunking
```bash
# Store large document (80K tokens)
curl -X POST http://localhost:8081/db/research/papers \
  -H "Content-Type: application/json" \
  -d '{
    "content": "... 80K token research paper ...",
    "metadata": {"title": "Deep Learning Survey"}
  }'

# Auto-chunked into ~160 pieces

# Search returns relevant 512-token chunks
curl -X POST http://localhost:8081/db/research/papers/search \
  -H "Content-Type: application/json" \
  -d '{"query": "convolutional networks", "type": "hybrid"}'
```

---

### 3. Citation Network
```bash
# Create relationships
curl -X POST http://localhost:8081/db/research/relations \
  -H "Content-Type: application/json" \
  -d '{
    "source_id": "paper-2026",
    "target_id": "paper-2020",
    "relation_type": "references"
  }'

# Traverse citation graph
curl -X POST http://localhost:8081/db/research/graph/traverse \
  -H "Content-Type: application/json" \
  -d '{"start_id": "paper-2026", "depth": 3}'

# Find citation path
curl "http://localhost:8081/db/research/graph/path?from=paper-a&to=paper-b"
```

---

## Error Handling

### Standard Error Response
```json
{
  "error": "error category",
  "message": "detailed error message"
}
```

### Common Status Codes

| Code | Meaning | Example |
|------|---------|---------|
| 200 | Success | Document retrieved |
| 201 | Created | Document stored |
| 204 | No Content | Document deleted |
| 400 | Bad Request | Invalid JSON |
| 404 | Not Found | Document doesn't exist |
| 500 | Internal Error | Database error |
| 501 | Not Implemented | Feature disabled |

---

## Best Practices

### 1. Use Feature Toggles
- Disable unused features for better performance
- Start simple, enable features as needed

### 2. Leverage Caching
- Cache hit rate: 50-80% typical
- Reduces GPU load dramatically
- Persistent across restarts

### 3. Chunk Large Documents
- Default 512 tokens works for most LLMs
- Adjust for your use case (256-2048)
- Enables precise search results

### 4. Use Async Embedding
- Default: `X-Client-Features: embed=async`
- Background worker handles embedding
- Faster document ingestion

### 5. Hybrid Search
- Best recall + precision balance
- Falls back to FTS if GPU unavailable
- ~240ms typical latency

### 6. Monitor Performance
- Check `/health` endpoint
- Watch for GPU errors
- Monitor cache hit rate

---

## Scaling Considerations

### Single Database
- **Good for:** 1K-100K documents
- **Search:** ~200ms with HNSW
- **Storage:** ~1-10GB

### Multiple Databases
- **Good for:** Multi-tenancy, isolation
- **Limit:** OS file handle limits (~1000 DBs)
- **Storage:** Each DB is separate .db file

### Large Scale (>1M documents)
- **HNSW index:** Handles millions efficiently
- **Chunking:** 200K docs → 32M chunks (still works)
- **SQLite:** Single-writer bottleneck
- **Future:** Consider PostgreSQL migration

---

## Troubleshooting

### Slow Searches
1. Check if HNSW enabled (`features.vector_index`)
2. Verify HNSW threshold reached (default: 1000 docs)
3. Monitor GPU embedding latency
4. Check PowerShell overhead (use curl instead)

### Embeddings Not Happening
1. Check `features.embedding_job` enabled
2. Verify GPU endpoint accessible
3. Check `worker.enabled` true
4. Look for errors in logs

### Cache Not Working
1. Check `features.embedding_cache` enabled
2. Verify cache directory writable
3. Monitor cache hit rate
4. Check memory limits

### Chunks Not Created
1. Check `features.chunking` enabled
2. Verify document exceeds `token_threshold`
3. Ensure `vectorize=true` in request
4. Check `chunking.enabled` true

---

## Support & Resources

### Documentation
- [Documents API](api-documents.md)
- [Search API](api-search.md)
- [Chunking API](api-chunking.md)
- [Relations API](api-relations.md)
- [Graph API](api-graph.md)

### Source Code
- GitHub: (your repo)
- Language: Rust
- License: (your license)

### Performance Reports
- `files/phase0_performance_report.md` - GPU benchmarks
- `files/phase2_completion_report.md` - Full implementation

---

## Version History

### Phase 2.5 (Current)
- ✅ HNSW vector index
- ✅ Feature toggles
- ✅ Comprehensive configuration

### Phase 2
- ✅ Embedding cache (2-tier)
- ✅ Parallel background worker
- ✅ Hybrid search (RRF)

### Phase 1
- ✅ Core API (CRUD)
- ✅ FTS5 full-text search
- ✅ Vector search (brute-force)
- ✅ GPU embeddings

### Phase 3 (Current)
- ✅ Document chunking
- ✅ Knowledge graphs
- ✅ Graph algorithms
- ✅ API documentation

---

## What's Next?

Completed features:
- ✅ All Phase 0-3 features implemented
- ✅ 17/17 unit tests passing
- ✅ Full API documentation
- ✅ Production-ready

Potential future enhancements:
- Cross-encoder reranking (when GPU supports it)
- Sparse vectors (SPLADE)
- Index persistence
- Graph visualization export
- Relationship weighting
- Advanced monitoring
