# SQLite multi-process contract

Every `Database::open` configures SQLite WAL mode, foreign keys, normal synchronous mode, a five-second busy timeout, and a bounded connection pool. Callers may customize the timeout and pool size with `OpenOptions`.

- Concurrent readers do not block a writer under WAL during normal operation.
- SQLite serializes competing writers. KuiperDB keeps record/vector batches within one short transaction and relies on the busy timeout rather than spinning.
- A record plus all supplied vectors is committed atomically. Validation happens before the transaction. A failed or interrupted transaction is rolled back by SQLite.
- Committed state survives process crashes according to SQLite WAL guarantees. Opening the database again initializes any missing schema objects and reads the authoritative records and vectors.
- A bulk input is one transaction. Very large batches increase writer hold time; applications should choose bounded batches when several processes write concurrently.

The automated suite opens independent pools against the same file to cover competing writers, reopening, and cross-handle index invalidation.

## Conditional atomic batches

`Database::commit_batch(expected_revision, WriteBatch)` combines record upserts,
relation inserts, and record deletions in one SQLite transaction, in that order.
Relations can reference records created in the batch. Deletions run last and
cascade to vectors, indexed metadata, and incoming/outgoing relations. Missing
deletion targets are no-ops. Record replacement and omitted-vector retention match
`put_records`. Supply explicit IDs for records referenced by new relations.

For a read/modify/write operation:

1. Read `Database::revision()` before reading the records or relations you need.
2. Prepare a batch from those reads.
3. Call `commit_batch(Some(revision), batch)`.
4. On `Ok(false)`, reread the revision and state and prepare a new batch. Bound
   retries according to the application's contention policy.

The transaction reserves the SQLite writer before checking the revision, so two
writers cannot both commit against the same stale token. `None` disables the
condition; `Ok(true)` means the batch committed. Validation can fail before the
revision check. An empty batch checks the condition without advancing the token.
Do slow work such as model inference before preparing the operation.

The revision is an opaque equality token shared across handles and processes,
not a transaction count: one batch can advance it multiple times. Triggers cover
record, vector, and relation inserts, updates, and deletes, including writes by
older clients after the schema has been initialized. Vector-space configuration,
derived metadata-table edits, and process-local index changes are outside this
token's contract. Unrelated tracked writes can cause conflicts.

Opening an existing v0.2 database adds the revision table and triggers without
rewriting records or vectors. Older clients can continue to write, but must adopt
the conditional batch API to protect their own read/modify/write operations.
The revision does not turn multiple read calls into a snapshot.

Failure before commit rolls back the entire batch, its revision changes, and
vector index generations. Cancellation while commit is in flight has an unknown
outcome to the caller; this API does not deduplicate retries of committed writes.
