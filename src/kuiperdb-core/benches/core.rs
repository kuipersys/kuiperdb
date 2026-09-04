use criterion::{criterion_group, criterion_main, Criterion};
use kuiperdb_core::{
    Database, DistanceMetric, NamedVector, Normalization, RecordInput, VectorQuery, VectorSpace,
};
use std::time::Duration;

fn benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bench.db");
    let database = runtime.block_on(Database::open(&path)).unwrap();
    runtime
        .block_on(database.create_vector_space(VectorSpace {
            name: "bench".into(),
            dimensions: 32,
            distance_metric: DistanceMetric::Cosine,
            normalization: Normalization::Unit,
            index: Default::default(),
        }))
        .unwrap();
    let records: Vec<_> = (0..1_000)
        .map(|number| RecordInput {
            id: Some(format!("record-{number}")),
            vectors: vec![NamedVector {
                space: "bench".into(),
                values: unit_vector(number, 32),
            }],
            ..Default::default()
        })
        .collect();
    runtime.block_on(database.put_records(records)).unwrap();
    runtime.block_on(database.rebuild_index("bench")).unwrap();
    let query = VectorQuery {
        space: "bench".into(),
        vector: unit_vector(42, 32),
        limit: 10,
        filter: Default::default(),
    };

    c.bench_function("ann_query_1k", |bencher| {
        bencher
            .to_async(&runtime)
            .iter(|| database.search(query.clone()))
    });
    c.bench_function("exact_query_1k", |bencher| {
        bencher
            .to_async(&runtime)
            .iter(|| database.search_exact(query.clone()))
    });
    c.bench_function("rebuild_index_1k", |bencher| {
        bencher
            .to_async(&runtime)
            .iter(|| database.rebuild_index("bench"))
    });
    let updates: Vec<_> = (0..100)
        .map(|number| RecordInput {
            id: Some(format!("record-{number}")),
            vectors: vec![NamedVector {
                space: "bench".into(),
                values: unit_vector(number + 1, 32),
            }],
            ..Default::default()
        })
        .collect();
    c.bench_function("bulk_upsert_100", |bencher| {
        bencher
            .to_async(&runtime)
            .iter(|| database.put_records(updates.clone()))
    });
    c.bench_function("concurrent_ann_queries_4", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            tokio::join!(
                database.search(query.clone()),
                database.search(query.clone()),
                database.search(query.clone()),
                database.search(query.clone())
            )
        })
    });
    c.bench_function("database_open", |bencher| {
        bencher.to_async(&runtime).iter(|| Database::open(&path))
    });
    c.bench_function("cold_index_open_1k", |bencher| {
        bencher.to_async(&runtime).iter(|| async {
            let reopened = Database::open(&path).await.unwrap();
            reopened.search(query.clone()).await
        })
    });
}

fn unit_vector(seed: usize, dimensions: usize) -> Vec<f32> {
    let mut values: Vec<f32> = (0..dimensions)
        .map(|position| (((seed + 1) * (position + 3)) % 97) as f32 + 1.0)
        .collect();
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    values.iter_mut().for_each(|value| *value /= norm);
    values
}

criterion_group! { name = benches; config = Criterion::default().measurement_time(Duration::from_secs(3)); targets = benchmark }
criterion_main!(benches);
