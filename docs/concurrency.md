# SQLite multi-process contract

Every `Database::open` configures SQLite WAL mode, foreign keys, normal synchronous mode, a five-second busy timeout, and a bounded connection pool. Callers may customize the timeout and pool size with `OpenOptions`.

- Concurrent readers do not block a writer under WAL during normal operation.
- SQLite serializes competing writers. KuiperDB keeps record/vector batches within one short transaction and relies on the busy timeout rather than spinning.
- A record plus all supplied vectors is committed atomically. Validation happens before the transaction. A failed or interrupted transaction is rolled back by SQLite.
- Committed state survives process crashes according to SQLite WAL guarantees. Opening the database again initializes any missing schema objects and reads the authoritative records and vectors.
- A bulk input is one transaction. Very large batches increase writer hold time; applications should choose bounded batches when several processes write concurrently.

The automated suite opens independent pools against the same file to cover competing writers, reopening, and cross-handle index invalidation.
