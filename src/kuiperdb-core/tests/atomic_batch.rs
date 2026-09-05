use anyhow::Result;
use kuiperdb_core::{
    Database, DistanceMetric, NamedVector, Normalization, RecordInput, RelationInput, VectorQuery,
    VectorSpace, WriteBatch,
};

fn record(id: &str) -> RecordInput {
    RecordInput {
        id: Some(id.into()),
        payload: None,
        metadata: Default::default(),
        vectors: vec![],
    }
}

async fn create_space(db: &Database) -> Result<()> {
    db.create_vector_space(VectorSpace {
        name: "features".into(),
        dimensions: 2,
        distance_metric: DistanceMetric::Cosine,
        normalization: Normalization::Unit,
        index: Default::default(),
    })
    .await
}

fn vector_record(id: &str) -> RecordInput {
    RecordInput {
        vectors: vec![NamedVector {
            space: "features".into(),
            values: vec![1.0, 0.0],
        }],
        metadata: [("group".into(), serde_json::json!("example"))].into(),
        ..record(id)
    }
}

fn relation(source: &str, target: &str) -> RelationInput {
    RelationInput {
        id: None,
        source_id: source.into(),
        target_id: target.into(),
        kind: "related".into(),
        metadata: Default::default(),
    }
}

#[tokio::test]
async fn reopening_a_pre_revision_database_preserves_existing_records() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("db");
    let db = Database::open(&path).await?;
    db.put_record(record("existing")).await?;
    drop(db);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&path))
        .await?;
    // Reproduce the pinned schema before the additive revision extension.
    sqlx::raw_sql(
        "DROP TRIGGER records_revision_insert;
        DROP TRIGGER records_revision_update;
        DROP TRIGGER records_revision_delete;
        DROP TRIGGER relations_revision_insert;
        DROP TRIGGER relations_revision_update;
        DROP TRIGGER relations_revision_delete;
        DROP TRIGGER vectors_revision_insert;
        DROP TRIGGER vectors_revision_update;
        DROP TRIGGER vectors_revision_delete;",
    )
    .execute(&pool)
    .await?;
    sqlx::query("DROP TABLE store_revision")
        .execute(&pool)
        .await?;
    pool.close().await;
    let upgraded = Database::open(&path).await?;
    assert!(upgraded.get_record("existing").await?.is_some());
    let revision = upgraded.revision().await?;
    assert!(
        upgraded
            .commit_batch(
                Some(revision),
                WriteBatch {
                    records: vec![record("new")],
                    ..Default::default()
                }
            )
            .await?
    );
    assert!(upgraded.get_record("existing").await?.is_some());
    assert!(upgraded.get_record("new").await?.is_some());
    Ok(())
}

#[tokio::test]
async fn relation_failure_rolls_back_records_and_revision() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let db = Database::open(dir.path().join("db")).await?;
    create_space(&db).await?;
    db.put_record(vector_record("existing")).await?;
    let before = db.revision().await?;
    let index_before = db.index_status("features").await?;
    let batch = WriteBatch {
        records: vec![vector_record("observation"), record("memory")],
        relations: vec![
            RelationInput {
                id: None,
                source_id: "observation".into(),
                target_id: "memory".into(),
                kind: "supports".into(),
                metadata: Default::default(),
            },
            RelationInput {
                id: None,
                source_id: "memory".into(),
                target_id: "missing".into(),
                kind: "related".into(),
                metadata: Default::default(),
            },
        ],
        delete_records: vec!["existing".into()],
    };
    assert!(db.commit_batch(Some(before), batch).await.is_err());
    assert_eq!(db.revision().await?, before);
    assert!(db.get_record("memory").await?.is_none());
    assert!(db.get_record("observation").await?.is_none());
    assert!(db.all_relations().await?.is_empty());
    assert!(db.get_record("existing").await?.is_some());
    assert!(db.get_vector("observation", "features").await?.is_none());
    assert_eq!(
        db.index_status("features").await?.database_generation,
        index_before.database_generation
    );
    Ok(())
}

#[tokio::test]
async fn mixed_batch_commits_and_deletions_cascade_with_fresh_indexes() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("db");
    let db = Database::open(&path).await?;
    create_space(&db).await?;
    db.put_record(vector_record("old")).await?;
    db.put_record(record("retained")).await?;
    db.create_relation(relation("old", "retained")).await?;
    let reader = Database::open(&path).await?;
    let query = || VectorQuery {
        space: "features".into(),
        vector: vec![1.0, 0.0],
        limit: 10,
        filter: Default::default(),
    };
    assert_eq!(reader.search(query()).await?[0].record.id, "old");
    let revision = db.revision().await?;
    assert!(
        db.commit_batch(
            Some(revision),
            WriteBatch {
                records: vec![vector_record("new")],
                relations: vec![relation("new", "retained"), relation("new", "old")],
                delete_records: vec!["old".into(), "missing".into()],
            },
        )
        .await?
    );
    assert!(db.revision().await? > revision);
    assert!(db.get_record("old").await?.is_none());
    assert!(db.get_vector("old", "features").await?.is_none());
    let metadata_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM record_metadata WHERE record_id = 'old'")
            .fetch_one(db.pool())
            .await?;
    assert_eq!(metadata_rows, 0);
    let relations = db.all_relations().await?;
    assert_eq!(relations.len(), 1);
    assert_eq!(relations[0].source_id, "new");
    assert_eq!(relations[0].target_id, "retained");
    let found = reader.search(query()).await?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].record.id, "new");
    drop(db);
    let reopened = Database::open(&path).await?;
    assert!(reopened.get_record("new").await?.is_some());
    assert!(reopened.revision().await? > revision);
    Ok(())
}

#[tokio::test]
async fn existing_mutation_apis_advance_the_revision() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let db = Database::open(dir.path().join("db")).await?;
    create_space(&db).await?;
    let mut previous = db.revision().await?;
    db.put_record(vector_record("a")).await?;
    let mut current = db.revision().await?;
    assert!(current > previous);
    previous = current;
    db.put_record(record("a")).await?;
    current = db.revision().await?;
    assert!(current > previous);
    // Omitted vectors retain put_records semantics.
    assert!(db.get_vector("a", "features").await?.is_some());
    previous = current;
    db.delete_vector("a", "features").await?;
    current = db.revision().await?;
    assert!(current > previous);
    db.put_record(record("b")).await?;
    previous = db.revision().await?;
    let edge = db.create_relation(relation("a", "b")).await?;
    current = db.revision().await?;
    assert!(current > previous);
    previous = current;
    db.delete_relation(&edge.id).await?;
    current = db.revision().await?;
    assert!(current > previous);
    previous = current;
    db.delete_record("a").await?;
    current = db.revision().await?;
    assert!(current > previous);
    assert!(
        db.commit_batch(Some(current), WriteBatch::default())
            .await?
    );
    assert_eq!(db.revision().await?, current);
    assert!(
        !db.commit_batch(Some(previous), WriteBatch::default())
            .await?
    );
    assert_eq!(db.revision().await?, current);
    Ok(())
}

#[tokio::test]
async fn process_revision_writer() -> Result<()> {
    let Some(path) = std::env::var_os("KUIPERDB_BATCH_TEST_PATH") else {
        return Ok(());
    };
    let db = Database::open(path).await?;
    db.put_record(record("other-process")).await?;
    Ok(())
}

#[tokio::test]
async fn another_process_invalidates_a_prepared_batch() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("db");
    let db = Database::open(&path).await?;
    let revision = db.revision().await?;
    let child = tokio::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "process_revision_writer", "--nocapture"])
        .env("KUIPERDB_BATCH_TEST_PATH", &path)
        .output()
        .await?;
    assert!(child.status.success(), "{child:?}");
    assert!(db.get_record("other-process").await?.is_some());
    assert!(
        !db.commit_batch(
            Some(revision),
            WriteBatch {
                records: vec![record("stale")],
                ..Default::default()
            },
        )
        .await?
    );
    assert!(db.get_record("stale").await?.is_none());
    Ok(())
}

#[tokio::test]
async fn competing_handle_rejects_stale_batch_without_deleting_anything() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("db");
    let a = Database::open(&path).await?;
    let b = Database::open(&path).await?;
    a.put_record(record("original")).await?;
    let before = a.revision().await?;
    b.put_record(record("other-writer")).await?;
    assert!(
        !a.commit_batch(
            Some(before),
            WriteBatch {
                records: vec![record("stale")],
                delete_records: vec!["original".into()],
                ..Default::default()
            }
        )
        .await?
    );
    assert!(a.get_record("stale").await?.is_none());
    assert!(a.get_record("original").await?.is_some());
    Ok(())
}

#[tokio::test]
async fn concurrent_batches_with_the_same_revision_have_one_winner() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("db");
    let a = Database::open(&path).await?;
    let b = Database::open(&path).await?;
    a.put_record(record("shared")).await?;
    let revision = a.revision().await?;
    let batch = |id: &str| WriteBatch {
        records: vec![record(id)],
        relations: vec![relation(id, "shared")],
        ..Default::default()
    };
    let (left, right) = tokio::join!(
        a.commit_batch(Some(revision), batch("left")),
        b.commit_batch(Some(revision), batch("right")),
    );
    let left = left?;
    let right = right?;
    assert_ne!(left, right);
    assert_eq!(a.get_record("left").await?.is_some(), left);
    assert_eq!(a.get_record("right").await?.is_some(), right);
    let relations = a.all_relations().await?;
    assert_eq!(relations.len(), 1);
    assert_eq!(relations[0].source_id, if left { "left" } else { "right" });
    Ok(())
}
