use kuiperdb_core::{
    Database, DistanceMetric, NamedVector, Normalization, RecordInput, VectorQuery, VectorSpace,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database = Database::open("./data/example.db").await?;
    database
        .create_vector_space(VectorSpace {
            name: "example".into(),
            dimensions: 3,
            distance_metric: DistanceMetric::Cosine,
            normalization: Normalization::Unit,
            index: Default::default(),
        })
        .await?;
    database
        .put_record(RecordInput {
            payload: Some(serde_json::json!({"text": "prepared by the application"})),
            vectors: vec![NamedVector {
                space: "example".into(),
                values: vec![1.0, 0.0, 0.0],
            }],
            ..Default::default()
        })
        .await?;
    let results = database
        .search(VectorQuery {
            space: "example".into(),
            vector: vec![1.0, 0.0, 0.0],
            limit: 5,
            filter: Default::default(),
        })
        .await?;
    println!("{results:#?}");
    Ok(())
}
