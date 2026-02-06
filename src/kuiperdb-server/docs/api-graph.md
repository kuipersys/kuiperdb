# kuiperdb-rust API Documentation: Graph Operations

## Overview
Perform graph algorithms on document relationships: traversal, pathfinding, cycle detection, and statistics.

**Base URL:** `http://localhost:8081`

---

## Graph Traversal (BFS)

Traverse the document graph starting from a node using breadth-first search.

**Endpoint:** `POST /db/{db_name}/graph/traverse`

### Path Parameters
- `db_name` (string) - Database name

### Request Body
```json
{
  "start_id": "doc-123",              // Required: Starting document ID
  "depth": 3,                         // Optional: Max depth (default: 3)
  "relation_types": [                 // Optional: Filter by relation types
    "references",
    "supports"
  ]
}
```

### Response
**Status:** `200 OK`

```json
{
  "document_ids": [
    "doc-123",    // Depth 0 (start)
    "doc-456",    // Depth 1
    "doc-789",    // Depth 2
    "doc-999"     // Depth 3
  ],
  "relations": [
    {
      "id": "rel-1",
      "source_id": "doc-123",
      "target_id": "doc-456",
      "relation_type": "references",
      "metadata": {},
      "created_at": "..."
    },
    ...
  ],
  "depth_map": {
    "doc-123": 0,
    "doc-456": 1,
    "doc-789": 2,
    "doc-999": 3
  }
}
```

### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `document_ids` | array | All documents reached in traversal order |
| `relations` | array | All relationships traversed |
| `depth_map` | object | Document ID → depth from start |

### Example: Basic Traversal
```bash
curl -X POST http://localhost:8081/db/research/graph/traverse \
  -H "Content-Type: application/json" \
  -d '{
    "start_id": "paper-2026",
    "depth": 2
  }'
```

**Result:** Finds all documents within 2 hops of the starting paper.

---

### Example: Filtered Traversal
```bash
curl -X POST http://localhost:8081/db/research/graph/traverse \
  -H "Content-Type: application/json" \
  -d '{
    "start_id": "paper-2026",
    "depth": 3,
    "relation_types": ["references", "cites"]
  }'
```

**Result:** Follows only "references" and "cites" relationships, ignoring others.

---

## Shortest Path

Find the shortest path between two documents.

**Endpoint:** `GET /db/{db_name}/graph/path?from={from_id}&to={to_id}`

### Query Parameters
- `from` (string, required) - Source document ID
- `to` (string, required) - Target document ID

### Response (Path Found)
**Status:** `200 OK`

```json
{
  "path": [
    "doc-A",
    "doc-B",
    "doc-C",
    "doc-D"
  ],
  "relations": [
    {
      "id": "rel-1",
      "source_id": "doc-A",
      "target_id": "doc-B",
      "relation_type": "references",
      ...
    },
    ...
  ],
  "total_weight": 3     // Number of hops
}
```

### Response (No Path)
**Status:** `404 Not Found`

```json
{
  "error": "no path found"
}
```

### Example
```bash
curl "http://localhost:8081/db/research/graph/path?from=paper-2020&to=paper-2026"
```

**Use Cases:**
- Citation chain discovery
- Influence tracing
- Relationship strength analysis
- Knowledge graph navigation

---

## Graph Statistics

Get statistics about the entire document graph.

**Endpoint:** `GET /db/{db_name}/graph/stats`

### Path Parameters
- `db_name` (string) - Database name

### Response
**Status:** `200 OK`

```json
{
  "node_count": 1523,           // Total documents in graph
  "edge_count": 4891,           // Total relationships
  "has_cycles": true,           // Whether graph contains cycles
  "in_degrees": {               // Incoming edge counts per document
    "doc-123": 5,               // 5 documents point to doc-123
    "doc-456": 12,
    ...
  },
  "out_degrees": {              // Outgoing edge counts per document
    "doc-123": 8,               // doc-123 points to 8 documents
    "doc-456": 3,
    ...
  }
}
```

### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| `node_count` | integer | Number of unique documents in graph |
| `edge_count` | integer | Number of relationships |
| `has_cycles` | boolean | Whether graph contains cycles |
| `in_degrees` | object | Document ID → count of incoming edges |
| `out_degrees` | object | Document ID → count of outgoing edges |

### Example
```bash
curl http://localhost:8081/db/research/graph/stats
```

---

## Use Cases & Examples

### Citation Network Analysis

```bash
# Find all papers cited by a landmark paper
curl -X POST http://localhost:8081/db/research/graph/traverse \
  -H "Content-Type: application/json" \
  -d '{
    "start_id": "landmark-paper",
    "depth": 1,
    "relation_types": ["references"]
  }'

# Find most-cited papers
curl http://localhost:8081/db/research/graph/stats
# Look for high in_degree values
```

---

### Knowledge Base Navigation

```bash
# Find related articles
curl -X POST http://localhost:8081/db/kb/graph/traverse \
  -H "Content-Type: application/json" \
  -d '{
    "start_id": "article-123",
    "depth": 2,
    "relation_types": ["relates_to", "similar_to"]
  }'
```

---

### Influence Tracing

```bash
# How did idea A influence idea B?
curl "http://localhost:8081/db/ideas/graph/path?from=idea-a&to=idea-b"

# Result shows citation chain:
# idea-a → paper-1 → paper-2 → idea-b
```

---

### Cycle Detection

```bash
# Check if there are circular references
curl http://localhost:8081/db/docs/graph/stats

# If has_cycles=true, investigate with traversal
curl -X POST http://localhost:8081/db/docs/graph/traverse \
  -H "Content-Type: application/json" \
  -d '{
    "start_id": "suspect-doc",
    "depth": 10
  }'
```

---

## Algorithms

### BFS Traversal
- **Algorithm:** Breadth-First Search
- **Complexity:** O(V + E) where V=documents, E=relationships
- **Properties:**
  - Visits nodes level by level
  - Finds documents at minimum depth first
  - Respects depth limit
  - Can filter by relationship type

---

### Shortest Path
- **Algorithm:** Dijkstra's algorithm
- **Complexity:** O((V + E) log V) with binary heap
- **Properties:**
  - Finds shortest path (minimum hops)
  - All edges have weight 1
  - Returns None if no path exists
  - Works on directed graph

---

### Cycle Detection
- **Algorithm:** petgraph's `is_cyclic_directed`
- **Complexity:** O(V + E)
- **Properties:**
  - Detects if any cycles exist
  - Boolean result (true/false)
  - Works on entire graph

---

## Performance

### Typical Performance

| Operation | Graph Size | Time |
|-----------|------------|------|
| BFS Traversal | 1K nodes, depth 3 | ~10-50ms |
| BFS Traversal | 10K nodes, depth 3 | ~50-200ms |
| Shortest Path | 1K nodes | ~20-100ms |
| Shortest Path | 10K nodes | ~100-500ms |
| Graph Stats | 10K nodes, 30K edges | ~50-100ms |
| Cycle Detection | 10K nodes | ~50-200ms |

### Optimization Tips

1. **Limit Depth:** Use depth limit in traversal (default: 3)
2. **Filter Relations:** Specify `relation_types` to reduce search space
3. **Cache Stats:** Graph stats are expensive, cache results
4. **Index Relations:** Ensure indexes exist on source_id, target_id

---

## Configuration

Enable graph operations in `config.json`:

```json
{
  "features": {
    "document_relations": true
  }
}
```

**Note:** Graph operations require the `document_relations` feature.

---

## Limitations

1. **In-Memory:** Graph is built in memory for algorithms
   - Large graphs (>100K relationships) may use significant RAM
   - Consider pagination or subgraph queries for very large graphs

2. **No Weights:** Currently all edges have weight 1
   - Future: Support edge weights for weighted shortest path

3. **Directed Graph:** Relationships are directional
   - A→B is different from B→A
   - Create both if bidirectional relationship needed

4. **No Persistence:** Graph structure rebuilt on each request
   - Future: Cache graph in memory for repeated operations

---

## Error Responses

### Feature Disabled
**Status:** `501 Not Implemented`

```json
{
  "error": "document_relations feature is disabled"
}
```

**Solution:** Enable `features.document_relations` in config.json

---

### Node Not Found
**Status:** `200 OK` (empty result)

```json
{
  "document_ids": [],
  "relations": [],
  "depth_map": {}
}
```

**Cause:** Starting document doesn't exist or has no relationships

---

## Best Practices

1. **Start Small:** Test with small depth values first
2. **Filter Relations:** Use `relation_types` to focus search
3. **Cache Results:** Graph stats are expensive, cache when possible
4. **Visualize:** Export graph data for visualization tools
5. **Monitor Performance:** Large graphs may need optimization
6. **Document Cycles:** If cycles expected, document them
7. **Limit Depth:** Unbounded traversal can be expensive

---

## Integration Examples

### Build Citation Network Visualization

```javascript
// Fetch graph data
const response = await fetch('http://localhost:8081/db/research/graph/traverse', {
  method: 'POST',
  headers: {'Content-Type': 'application/json'},
  body: JSON.stringify({
    start_id: 'paper-123',
    depth: 2
  })
});

const {document_ids, relations, depth_map} = await response.json();

// Convert to vis.js format
const nodes = document_ids.map(id => ({
  id,
  label: id,
  level: depth_map[id]
}));

const edges = relations.map(rel => ({
  from: rel.source_id,
  to: rel.target_id,
  label: rel.relation_type
}));

// Render with vis.js or d3.js
```

---

### Find Influential Papers

```bash
# Get graph stats
curl http://localhost:8081/db/research/graph/stats > stats.json

# Parse in_degrees to find most-cited papers
jq '.in_degrees | to_entries | sort_by(.value) | reverse | .[0:10]' stats.json
```

---

## Future Enhancements

Potential future features (not currently implemented):

1. **Weighted Edges:** Support edge weights for importance
2. **PageRank:** Calculate document importance
3. **Community Detection:** Find clusters in graph
4. **Graph Export:** Export to DOT, GraphML formats
5. **Subgraph Queries:** Query specific subgraphs
6. **Graph Caching:** Keep graph in memory for performance
