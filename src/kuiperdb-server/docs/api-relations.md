# kuiperdb-rust API Documentation: Document Relationships

## Overview
Create typed relationships between documents to build knowledge graphs. Supports N:N relationships with metadata.

**Base URL:** `http://localhost:8081`

---

## Create Relationship

Create a typed relationship between two documents.

**Endpoint:** `POST /db/{db_name}/relations`

### Path Parameters
- `db_name` (string) - Database name

### Request Body
```json
{
  "source_id": "doc-123",           // Required: Source document ID
  "target_id": "doc-456",           // Required: Target document ID
  "relation_type": "references",    // Required: Relationship type
  "metadata": {                     // Optional: Relationship metadata
    "page": 42,
    "context": "citation in introduction"
  }
}
```

### Response
**Status:** `201 Created`

```json
{
  "id": "rel-789",
  "source_id": "doc-123",
  "target_id": "doc-456",
  "relation_type": "references",
  "metadata": {
    "page": 42,
    "context": "citation in introduction"
  },
  "created_at": "2026-02-03T06:00:00Z"
}
```

### Relationship Types

Common relationship types (custom types allowed):
- `references` - Document A cites/references document B
- `contradicts` - Document A contradicts document B
- `supports` - Document A supports/agrees with document B
- `extends` - Document A extends/builds upon document B
- `summarizes` - Document A summarizes document B
- `translates` - Document A is translation of document B
- `version_of` - Document A is version of document B

### Unique Constraint

The combination of `(source_id, target_id, relation_type)` must be unique.

**Allowed:**
```json
{"source_id": "A", "target_id": "B", "relation_type": "references"}
{"source_id": "A", "target_id": "B", "relation_type": "supports"}
```

**Not Allowed (duplicate):**
```json
{"source_id": "A", "target_id": "B", "relation_type": "references"}
{"source_id": "A", "target_id": "B", "relation_type": "references"}
```

### Example
```bash
curl -X POST http://localhost:8081/db/mydb/relations \
  -H "Content-Type: application/json" \
  -d '{
    "source_id": "paper-2026",
    "target_id": "paper-2020",
    "relation_type": "references",
    "metadata": {"section": "related work", "page": 3}
  }'
```

---

## Get Relationship

Retrieve a specific relationship by ID.

**Endpoint:** `GET /db/{db_name}/relations/{relation_id}`

### Path Parameters
- `db_name` (string) - Database name
- `relation_id` (string) - Relationship ID

### Response
**Status:** `200 OK`

```json
{
  "id": "rel-789",
  "source_id": "doc-123",
  "target_id": "doc-456",
  "relation_type": "references",
  "metadata": {...},
  "created_at": "2026-02-03T06:00:00Z"
}
```

### Example
```bash
curl http://localhost:8081/db/mydb/relations/rel-789
```

---

## Delete Relationship

Delete a relationship.

**Endpoint:** `DELETE /db/{db_name}/relations/{relation_id}`

### Path Parameters
- `db_name` (string) - Database name
- `relation_id` (string) - Relationship ID

### Response
**Status:** `204 No Content`

### Example
```bash
curl -X DELETE http://localhost:8081/db/mydb/relations/rel-789
```

---

## Get Document Relationships

Get all relationships for a specific document (both incoming and outgoing).

**Endpoint:** `GET /db/{db_name}/documents/{doc_id}/relations`

### Path Parameters
- `db_name` (string) - Database name
- `doc_id` (string) - Document ID

### Response
**Status:** `200 OK`

```json
[
  {
    "id": "rel-1",
    "source_id": "doc-123",    // This document references another
    "target_id": "doc-456",
    "relation_type": "references",
    "metadata": {},
    "created_at": "..."
  },
  {
    "id": "rel-2",
    "source_id": "doc-789",    // Another document references this one
    "target_id": "doc-123",
    "relation_type": "cites",
    "metadata": {},
    "created_at": "..."
  }
]
```

### Example
```bash
curl http://localhost:8081/db/mydb/documents/doc-123/relations
```

**Use Cases:**
- Find all papers that cite this paper
- Find all papers this paper cites
- Build citation network visualization
- Discover related content

---

## Use Cases & Examples

### Citation Network

```bash
# Paper A references Paper B
curl -X POST http://localhost:8081/db/research/relations \
  -H "Content-Type: application/json" \
  -d '{
    "source_id": "paper-a",
    "target_id": "paper-b",
    "relation_type": "references",
    "metadata": {"citation_count": 1, "year": 2026}
  }'

# Get all citations for Paper B
curl http://localhost:8081/db/research/documents/paper-b/relations
```

---

### Knowledge Base Linking

```bash
# Link FAQ answer to detailed article
curl -X POST http://localhost:8081/db/kb/relations \
  -H "Content-Type: application/json" \
  -d '{
    "source_id": "faq-123",
    "target_id": "article-456",
    "relation_type": "links_to",
    "metadata": {"relevance": "high"}
  }'
```

---

### Version Control

```bash
# Mark document as version of another
curl -X POST http://localhost:8081/db/docs/relations \
  -H "Content-Type: application/json" \
  -d '{
    "source_id": "doc-v2",
    "target_id": "doc-v1",
    "relation_type": "version_of",
    "metadata": {"version": "2.0", "changes": "major update"}
  }'
```

---

### Contradiction Detection

```bash
# Mark conflicting information
curl -X POST http://localhost:8081/db/claims/relations \
  -H "Content-Type: application/json" \
  -d '{
    "source_id": "study-a",
    "target_id": "study-b",
    "relation_type": "contradicts",
    "metadata": {"field": "methodology", "severity": "high"}
  }'
```

---

## Configuration

Enable relationships in `config.json`:

```json
{
  "features": {
    "document_relations": true
  }
}
```

---

## Best Practices

1. **Use Standard Types:** Prefer common relation types for consistency
2. **Add Metadata:** Include context in relationship metadata
3. **Bidirectional Relationships:** Create both A→B and B→A if needed
4. **Clean Up:** Delete relationships when documents are deleted
5. **Avoid Cycles:** Unless intentional (graph algorithms can detect)

---

## Schema Details

### Database Table
```sql
CREATE TABLE document_relations (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    metadata TEXT,  -- JSON
    created_at TIMESTAMP NOT NULL,
    UNIQUE(source_id, target_id, relation_type)
);
```

### Indexes
```sql
CREATE INDEX idx_relations_source ON document_relations(source_id);
CREATE INDEX idx_relations_target ON document_relations(target_id);
CREATE INDEX idx_relations_type ON document_relations(relation_type);
```

**Performance:** Fast lookups by source, target, or type.

---

## Error Responses

### Duplicate Relationship
**Status:** `500 Internal Server Error`

```json
{
  "error": "failed to create relation",
  "message": "UNIQUE constraint failed: document_relations.source_id, document_relations.target_id, document_relations.relation_type"
}
```

**Solution:** Use different relation type or check existing relationships first.

---

### Document Not Found
Relationships don't enforce foreign key constraints on document IDs.

**Behavior:**
- Can create relationship to non-existent document
- Useful for forward references
- Graph traversal will skip missing documents

---

## Next Steps

See [Graph Operations API](api-graph.md) for:
- Graph traversal (BFS)
- Shortest path finding
- Cycle detection
- Graph statistics
