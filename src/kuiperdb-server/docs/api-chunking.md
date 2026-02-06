# kuiperdb-rust API Documentation: Document Chunking

## Overview
Automatically chunk large documents into smaller, searchable pieces. Great for RAG (Retrieval-Augmented Generation) and precise search.

**Base URL:** `http://localhost:8081`

---

## How Chunking Works

### Automatic Chunking

When you store a document:
1. Token count is calculated using tiktoken (OpenAI tokenizer)
2. If `token_count > threshold` (default: 512) AND `vectorize=true`:
   - Parent document stored with `vectorize=false`
   - Document split into chunks of `chunk_size` tokens (default: 512)
   - Each chunk stored as separate document
   - Chunks have `is_chunk=true`, `parent_id`, `chunk_index`
   - Chunks automatically embedded by background worker

### Example

**Input:** 80,000 token document  
**Threshold:** 512 tokens  
**Chunk Size:** 512 tokens  
**Overlap:** 50 tokens  

**Result:**
- 1 parent document (not embedded, full content preserved)
- ~160 child chunks (each embedded independently)

---

## Configuration

Enable chunking in `config.json`:

```json
{
  "features": {
    "chunking": true
  },
  "chunking": {
    "enabled": true,
    "token_threshold": 512,     // Auto-chunk if > N tokens
    "chunk_size": 512,          // Target chunk size
    "chunk_overlap": 50,        // Overlap between chunks (preserves context)
    "strategy": "fixed_tokens"  // "fixed_tokens" or "custom"
  }
}
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `token_threshold` | 512 | Documents with more tokens get chunked |
| `chunk_size` | 512 | Target size for each chunk |
| `chunk_overlap` | 50 | Token overlap between chunks (for context preservation) |
| `strategy` | "fixed_tokens" | Chunking algorithm ("fixed_tokens" or "custom") |

---

## API Endpoints

### Get Chunks for Document

Retrieve all chunks for a parent document.

**Endpoint:** `GET /db/{db_name}/{table_name}/{doc_id}/chunks`

### Path Parameters
- `db_name` (string) - Database name
- `table_name` (string) - Collection/table name
- `doc_id` (string) - Parent document ID

### Response
**Status:** `200 OK`

```json
[
  {
    "id": "chunk-1",
    "content": "First 512-token chunk...",
    "is_chunk": true,
    "parent_id": "doc-123",
    "chunk_index": 0,
    "token_count": 512,
    "is_embedded": true,
    "vector": [...]
  },
  {
    "id": "chunk-2",
    "content": "Second 512-token chunk...",
    "is_chunk": true,
    "parent_id": "doc-123",
    "chunk_index": 1,
    "token_count": 512,
    "is_embedded": true,
    "vector": [...]
  }
]
```

### Example
```bash
curl http://localhost:8081/db/mydb/documents/doc-123/chunks
```

---

### Re-chunk Document

Force re-chunking of a document with current configuration.

**Endpoint:** `POST /db/{db_name}/{table_name}/{doc_id}/rechunk`

### Path Parameters
- `db_name` (string) - Database name
- `table_name` (string) - Collection/table name
- `doc_id` (string) - Document ID to re-chunk

### Behavior
1. Deletes all existing chunks for this document
2. Re-chunks using current `chunking` configuration
3. Creates new chunks with fresh IDs
4. Returns created chunks

### Response
**Status:** `200 OK`

```json
{
  "chunks_created": 160,
  "chunks": [
    {
      "id": "new-chunk-1",
      "content": "...",
      "is_chunk": true,
      "parent_id": "doc-123",
      "chunk_index": 0
    },
    ...
  ]
}
```

### Example
```bash
curl -X POST http://localhost:8081/db/mydb/documents/doc-123/rechunk
```

**Use Cases:**
- Changed chunking configuration
- Want different chunk size or overlap
- Re-process after fixing document content

---

## Storing Documents with Chunking

### Example: Large Document

```bash
curl -X POST http://localhost:8081/db/mydb/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "... 80,000 token research paper here ...",
    "metadata": {"title": "Deep Learning Survey", "year": 2026},
    "vectorize": true
  }'
```

**What Happens:**
1. Document stored with ID `doc-123`
2. Token count: 80,000 tokens (exceeds 512 threshold)
3. Parent doc: `vectorize` set to `false`
4. 160 chunks created automatically
5. Background worker embeds all 160 chunks
6. Search operates on chunks (not 80K token blob)

**Response:**
```json
{
  "id": "doc-123",
  "token_count": 80000,
  "vectorize": false,      // Parent not embedded
  "is_chunk": false,
  "parent_id": null,
  "created_at": "..."
}
```

---

### Example: Small Document (No Chunking)

```bash
curl -X POST http://localhost:8081/db/mydb/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Short document about ML.",
    "vectorize": true
  }'
```

**What Happens:**
1. Token count: ~5 tokens (below 512 threshold)
2. Stored as single document
3. Embedded directly
4. No chunks created

**Response:**
```json
{
  "id": "doc-456",
  "token_count": 5,
  "vectorize": true,       // Will be embedded
  "is_chunk": false,
  "parent_id": null,
  "is_embedded": true,
  "vector": [...]
}
```

---

## Search Behavior with Chunks

### Search Returns Chunks

When searching chunked documents, results contain individual chunks:

```bash
curl -X POST http://localhost:8081/db/mydb/documents/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "convolutional neural networks",
    "type": "hybrid"
  }'
```

**Response:**
```json
{
  "results": [
    {
      "id": "chunk-42",
      "content": "Convolutional neural networks (CNNs) are...",
      "score": 0.95,
      "is_chunk": true,
      "parent_id": "doc-123",
      "chunk_index": 42
    }
  ]
}
```

### Benefits

1. **Precision:** Returns exact 512-token section that matched
2. **RAG-Friendly:** Perfect for feeding to LLMs (not 80K tokens!)
3. **Fast:** Searches 160 chunks, not one giant document
4. **Context:** Can retrieve parent or surrounding chunks if needed

### Retrieving Context

```bash
# Get the matching chunk
curl http://localhost:8081/db/mydb/documents/chunk-42

# Get the parent document (full context)
curl http://localhost:8081/db/mydb/documents/doc-123

# Get all chunks for parent (full chunked version)
curl http://localhost:8081/db/mydb/documents/doc-123/chunks
```

---

## Chunk Overlap

Chunks overlap by default (50 tokens) to preserve context at boundaries.

### Without Overlap
```
Chunk 1: [tokens 0-512)
Chunk 2: [tokens 512-1024)  ❌ May split important phrase
```

### With 50-Token Overlap
```
Chunk 1: [tokens 0-512)
Chunk 2: [tokens 462-974)   ✅ Phrase preserved in both
```

**Benefits:**
- Important phrases at chunk boundaries appear in multiple chunks
- Better search recall (won't miss boundary matches)
- LLMs get more context

**Tradeoff:**
- ~10% more chunks (50/512 = ~10% overlap)
- Slightly more storage and compute

---

## Performance

### Chunking Performance
- **Token counting:** ~10-50ms per document (one-time)
- **Chunking 80K doc:** ~1000ms (one-time, creates 160 chunks)
- **Per-chunk storage:** ~5ms

### Search Performance
- **Without chunking:** Search 1 giant doc → imprecise, returns 80K tokens
- **With chunking:** Search 160 chunks → precise, returns 512-token snippet
- **HNSW scaling:** Works great with chunks (200K chunks supported)

---

## Best Practices

1. **Default Threshold (512):** Works well for most use cases
2. **Adjust for Use Case:**
   - RAG for LLMs: 512-768 tokens (fits context window)
   - Fine-grained search: 256-512 tokens
   - Coarse search: 1024-2048 tokens
3. **Use Overlap:** Keep default 50 tokens for context preservation
4. **Async Embedding:** Use `X-Client-Features: embed=async` for bulk imports
5. **Monitor Chunks:** Check `/db/{db}/{table}/{id}/chunks` to see chunking results
6. **Re-chunk Sparingly:** Only when config changes or content fixes needed

---

## Disabling Chunking

### Per-Document
```bash
curl -X POST http://localhost:8081/db/mydb/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Large document but I want it whole",
    "vectorize": false     // Disable embedding (and thus chunking)
  }'
```

### Globally
In `config.json`:
```json
{
  "features": {
    "chunking": false   // Disable chunking feature entirely
  }
}
```

Or:
```json
{
  "chunking": {
    "enabled": false    // Disable while keeping feature toggle on
  }
}
```

---

## Troubleshooting

### Chunks Not Being Created

**Check:**
1. Is `features.chunking` enabled?
2. Is `chunking.enabled` true?
3. Does document exceed `token_threshold`?
4. Is `vectorize=true` in the request?

### Chunks Not Being Embedded

**Check:**
1. Is `features.embedding_job` enabled?
2. Is background worker running?
3. Check logs for embedding errors

### Too Many/Few Chunks

**Adjust `chunk_size`:**
- Too many chunks → increase `chunk_size`
- Too few chunks → decrease `chunk_size`

**Typical values:** 256, 512, 768, 1024 tokens
