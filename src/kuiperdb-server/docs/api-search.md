# kuiperdb-rust API Documentation: Search Operations

## Overview
Search documents using full-text search (FTS5), vector similarity search, or hybrid search combining both.

**Base URL:** `http://localhost:8081`

---

## Search Documents

Search for documents using various search types.

**Endpoint:** `POST /db/{db_name}/{table_name}/search`

### Request Body
```json
{
  "query": "machine learning algorithms",  // Required
  "type": "hybrid",                       // Optional: "vector", "fulltext", or "hybrid" (default)
  "limit": 10,                            // Optional: Max results (default: 10)
  "include_chunks": true,                 // Optional: Include chunks (default: true)
  "group_by_parent": false                // Optional: Group chunks (default: false)
}
```

### Search Types

| Type | Description | Requirements |
|------|-------------|--------------|
| `fulltext` | FTS5 keyword search | None |
| `vector` | Semantic similarity search | Embeddings enabled |
| `hybrid` | BM25 + Vector with RRF fusion | Embeddings enabled |

### Response
**Status:** `200 OK`

```json
{
  "results": [
    {
      "id": "chunk-456",
      "content": "... 512-token chunk about ML algorithms ...",
      "metadata": {"source": "research_paper"},
      "score": 0.95,
      "fts_rank": -2.3,
      "vector_similarity": 0.87,
      "is_chunk": true,
      "parent_id": "doc-123",
      "chunk_index": 42
    }
  ],
  "query": "machine learning algorithms",
  "type": "hybrid",
  "db": "mydb",
  "total": 1
}
```

---

## Search Examples

### Full-Text Search
```bash
curl -X POST http://localhost:8081/db/mydb/documents/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "machine learning",
    "type": "fulltext",
    "limit": 5
  }'
```

**Performance:** ~5-20ms

---

### Vector Search
```bash
curl -X POST http://localhost:8081/db/mydb/documents/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "deep neural networks",
    "type": "vector",
    "limit": 10
  }'
```

**Performance:**
- HNSW (>1000 docs): ~20-200ms
- Brute-force (<1000 docs): ~50-500ms

---

### Hybrid Search (Recommended)
```bash
curl -X POST http://localhost:8081/db/mydb/documents/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "convolutional neural networks for image recognition",
    "type": "hybrid",
    "limit": 20
  }'
```

**Performance:** ~240-500ms

**How It Works:**
1. Runs FTS5 search for top 2×limit results
2. Runs vector search for top 2×limit results
3. Merges using Reciprocal Rank Fusion (RRF)
4. Returns top `limit` results

---

## FTS5 Query Syntax

### Phrase Search
```bash
curl -X POST http://localhost:8081/db/mydb/documents/search \
  -H "Content-Type: application/json" \
  -d '{"query": "\"machine learning\"", "type": "fulltext"}'
```

### Boolean Operators
```bash
# AND
{"query": "machine AND learning", "type": "fulltext"}

# OR
{"query": "machine OR neural", "type": "fulltext"}

# NOT
{"query": "machine NOT networks", "type": "fulltext"}

# Prefix matching
{"query": "algo*", "type": "fulltext"}
```

---

## HNSW Vector Index

Auto-activates at 1000 documents for faster vector search.

### Performance

| Document Count | Method | Time |
|----------------|--------|------|
| < 1000 | Brute-force | ~50-500ms |
| ≥ 1000 | HNSW | ~20-200ms |
| 200K | HNSW | ~200ms |

### Configuration
```json
{
  "vector_index": {
    "mode": "auto",
    "threshold": 1000,
    "hnsw_m": 16,
    "hnsw_ef_construction": 200,
    "hnsw_ef_search": 100
  }
}
```

---

## Chunk-Aware Search

When documents are chunked, search returns individual 512-token chunks.

### Benefits
- **Precision:** Shows exact match location
- **RAG-Friendly:** Returns snippets for LLMs
- **Context:** Parent metadata via `parent_id`

### Retrieving Parent
```bash
curl http://localhost:8081/db/mydb/documents/{parent_id}
```

---

## Best Practices

1. **Default to Hybrid:** Best results for most queries
2. **Use FTS for Keywords:** When you know exact terms
3. **Use Vector for Concepts:** When searching by meaning
4. **Limit Appropriately:** 10-20 results usually sufficient
5. **Handle Chunks:** Display chunk with parent context
6. **Monitor Performance:** HNSW for large datasets
