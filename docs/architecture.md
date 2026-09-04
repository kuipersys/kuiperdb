# Core architecture

`kuiperdb-core` owns only durable, model-independent database behavior:

```text
records ── metadata ── relationships
   │
   └── vectors ── named vector spaces
                        │
                        └── derived ANN indexes
                                  │
                                  └── filtered vector search
```

A record ID is an opaque string stored as the SQLite primary key. It is independent of row order, insertion order, vector contents, and HNSW node positions. A record may have one vector in each of zero or more spaces. Each space fixes dimensions, distance metric, normalization expectations, and index configuration; incompatible vectors and queries are rejected.

Payload and metadata are JSON values with no application meaning assigned by KuiperDB. Top-level metadata equality values are transactionally mirrored into an indexed lookup table for constrained retrieval. Relationships are directed edges between record IDs with an opaque `kind` and JSON metadata.

Applications own all preparation: parsing, segmentation, inference, batching, embedding caches, and model metadata. The optional HTTP server is a consumer of the embedded API and adds no storage semantics.
