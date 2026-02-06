# kuiperdb-rust API Documentation: Core Document Operations

## Overview
Core CRUD (Create, Read, Update, Delete) operations for documents in kuiperdb-rust.

**Base URL:** `http://localhost:8081`

---

## Store Document

Create or update a document in the database.

**Endpoint:** `POST /db/{db_name}/{table_name}`

### Path Parameters
- `db_name` (string) - Database name
- `table_name` (string) - Collection/table name

### Request Headers
- `Content-Type: application/json`
- `X-Client-Features` (optional) - Control embedding behavior
  - `embed=async` - Skip synchronous embedding, let background worker handle it
  - Default: Synchronous embedding if enabled
- `Accept` (optional) - Control response metadata
  - `application/json;metadata=none` - Return only ID
  - `application/json;metadata=minimal` - Return ID + timestamps
  - `application/json;metadata=full` - Return full document (default)

### Request Body
```json
{
  "id": "doc-123",              // Optional: Auto-generated UUID if not provided
  "content": "Document text",   // Required: The actual document content
  "metadata": {                 // Optional: JSON metadata
    "author": "John Doe",
    "source": "research_paper"
  },
  "tags": ["ml", "ai"],         // Optional: Array of tags
  "vectorize": true             // Optional: Enable/disable embedding (default: true)
}
```

### Response (Full Metadata)
**Status:** `201 Created`

```json
{
  "id": "doc-123",
  "db": "mydb",
  "table": "documents",
  "content": "Document text",
  "metadata": {
    "author": "John Doe",
    "source": "research_paper"
  },
  "tags": ["ml", "ai"],
  "vector": [0.1, 0.2, ...],    // Present if embedded
  "created_at": "2026-02-03T06:00:00Z",
  "updated_at": "2026-02-03T06:00:00Z",
  "is_embedded": true,
  "vectorize": true,
  "is_chunk": false,
  "parent_id": null,
  "chunk_index": null,
  "token_count": 42
}
```

### Auto-Chunking Behavior

If `features.chunking` is enabled and document exceeds `chunking.token_threshold`:

1. **Parent document** is stored with `vectorize=false`
2. **Chunks** are automatically created and stored as separate documents:
   - Each chunk has `is_chunk=true`
   - Each chunk has `parent_id` pointing to parent
   - Each chunk has `chunk_index` for ordering
   - Each chunk has `vectorize=true` (will be embedded)

**Example:** 80,000 token document with 512 token threshold:
- 1 parent document (not embedded)
- ~160 child chunks (each embedded)

### Examples

#### Basic Document
```bash
curl -X POST http://localhost:8081/db/mydb/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Machine learning is a subset of artificial intelligence.",
    "metadata": {"topic": "AI"},
    "tags": ["ml", "ai"]
  }'
```

#### Document with Async Embedding
```bash
curl -X POST http://localhost:8081/db/mydb/documents \
  -H "Content-Type: application/json" \
  -H "X-Client-Features: embed=async" \
  -d '{
    "content": "Very long document...",
    "vectorize": true
  }'
```

---

## Get Document

Retrieve a document by its ID.

**Endpoint:** `GET /db/{db_name}/{table_name}/{doc_id}`

### Path Parameters
- `db_name` (string) - Database name
- `table_name` (string) - Collection/table name
- `doc_id` (string) - Document ID

### Response
**Status:** `200 OK`

```json
{
  "id": "doc-123",
  "content": "Document text",
  "metadata": {...},
  "tags": [...],
  "created_at": "2026-02-03T06:00:00Z",
  "updated_at": "2026-02-03T06:00:00Z",
  "is_embedded": true,
  "vectorize": true,
  "is_chunk": false,
  "parent_id": null,
  "chunk_index": null,
  "token_count": 42
}
```

### Example
```bash
curl http://localhost:8081/db/mydb/documents/doc-123
```

---

## Delete Document

Delete a document by its ID.

**Endpoint:** `DELETE /db/{db_name}/{table_name}/{doc_id}`

### Response
**Status:** `204 No Content`

### Cascading Deletes
When deleting a parent document:
- All child chunks are automatically deleted
- All relationships where this document is source or target are deleted

### Example
```bash
curl -X DELETE http://localhost:8081/db/mydb/documents/doc-123
```

---

## Health Check

Check if the service is running.

**Endpoint:** `GET /health`

### Response
```json
{
  "status": "ok",
  "timestamp": "2026-02-03T06:00:00Z"
}
```
