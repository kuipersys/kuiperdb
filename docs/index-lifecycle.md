# ANN index lifecycle

Persisted vectors and stable record IDs are authoritative. HNSW indexes are process-local derived state and can always be discarded or rebuilt without losing records.

Each vector space has a persistent monotonically increasing generation. SQLite triggers advance it on vector insertion, replacement, and deletion, including cascades from record deletion. Before an ANN query, KuiperDB compares the loaded generation with SQLite. A missing or stale index is rebuilt from a consistent set of persisted IDs and vectors. If writers change the generation during construction, construction retries; a half-built index is never published.

- Insert/update/delete: the committed vector change advances the generation. Other processes detect staleness on status checks or their next query.
- Bulk insertion: all changes commit together; generation changes are visible only after commit.
- Rebuild: constructed off to the side, generation checked, then atomically swapped into process memory.
- Reopen: no ANN state is trusted from a prior process; the first query rebuilds from SQLite.
- Interrupted write: SQLite rolls back authoritative changes, so no generation becomes visible.
- Interrupted rebuild: only unpublished process memory is lost.
- `drop_index`: removes derived state only.
- `update_index_config`: persists new HNSW tuning and advances the generation so every process invalidates old derived state.

Filtered queries currently use exhaustive search so metadata constraints are exact. Euclidean and dot-product spaces also use exhaustive search; HNSW acceleration currently applies to cosine spaces. This is an explicit performance distinction, not a change in retrieval semantics.
