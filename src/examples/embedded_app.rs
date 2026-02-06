//! Example Embedded Application using kuiperdb-core
//!
//! This example demonstrates how to use kuiperdb as an embedded library
//! in your own Rust application. It shows:
//! - Creating and configuring a DocumentStore
//! - Adding documents with metadata
//! - Retrieving documents
//! - Performing searches (fulltext and vector)
//!
//! Run with: cargo run --example embedded_app

use kuiperdb_core::*;
use serde_json::json;
use std::collections::HashMap;

/// Type alias for search result tuples
type SearchResultTuple = (
    String,
    String,
    HashMap<String, serde_json::Value>,
    f64,
    bool,
    Option<String>,
    Option<i32>,
);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Starting kuiperdb Embedded Application Example\n");

    // 1. Initialize the DocumentStore
    println!("📦 Initializing DocumentStore...");
    let data_dir = "./data/example_embedded".to_string();
    let mut store = store::DocumentStore::new(data_dir).await?;

    // Database and table names
    let db_name = "default";
    let table_name = "documents";

    // Ensure table exists
    store.ensure_table(db_name, table_name).await?;
    println!("✅ DocumentStore initialized\n");

    // 2. Add some sample documents
    println!("📝 Adding sample documents...");

    let doc1 = store.add_document(db_name, table_name, StoreDocumentRequest {
        id: None,
        content: "Rust is a systems programming language focused on safety, speed, and concurrency.".to_string(),
        metadata: [("category".to_string(), json!("programming")),
                   ("language".to_string(), json!("rust")),
                   ("difficulty".to_string(), json!("intermediate"))].into(),
        tags: vec!["rust".to_string(), "programming".to_string()],
        vectorize: true,
    }).await?;
    println!("  Added document: {}", doc1.id);

    let doc2 = store.add_document(db_name, table_name, StoreDocumentRequest {
        id: None,
        content: "Python is a high-level programming language known for its simplicity and readability.".to_string(),
        metadata: [("category".to_string(), json!("programming")),
                   ("language".to_string(), json!("python")),
                   ("difficulty".to_string(), json!("beginner"))].into(),
        tags: vec!["python".to_string(), "programming".to_string()],
        vectorize: true,
    }).await?;
    println!("  Added document: {}", doc2.id);

    let doc3 = store
        .add_document(
            db_name,
            table_name,
            StoreDocumentRequest {
                id: None,
                content:
                    "Machine learning is a subset of AI that enables systems to learn from data."
                        .to_string(),
                metadata: [
                    ("category".to_string(), json!("ai")),
                    ("topic".to_string(), json!("machine learning")),
                    ("difficulty".to_string(), json!("advanced")),
                ]
                .into(),
                tags: vec!["ai".to_string(), "ml".to_string()],
                vectorize: true,
            },
        )
        .await?;
    println!("  Added document: {}", doc3.id);

    let doc4 = store
        .add_document(
            db_name,
            table_name,
            StoreDocumentRequest {
                id: None,
                content: "Vector databases store and retrieve data based on semantic similarity."
                    .to_string(),
                metadata: [
                    ("category".to_string(), json!("database")),
                    ("type".to_string(), json!("vector")),
                    ("difficulty".to_string(), json!("intermediate")),
                ]
                .into(),
                tags: vec!["database".to_string(), "vector".to_string()],
                vectorize: true,
            },
        )
        .await?;
    println!("  Added document: {}\n", doc4.id);

    // 3. Retrieve a specific document
    println!("🔍 Retrieving document by ID...");
    let retrieved_doc = store.get_document(db_name, table_name, &doc1.id).await?;
    println!(
        "  Found: {} (created: {})",
        retrieved_doc.id, retrieved_doc.created_at
    );
    println!("  Content: {}", retrieved_doc.content);
    println!("  Embedded: {}\n", retrieved_doc.is_embedded);

    // 4. Perform fulltext search
    println!("🔎 Performing fulltext searches...\n");

    println!("  Fulltext search for 'programming':");
    let fts_results = store
        .search_fts(db_name, table_name, "programming", 10)
        .await?;
    display_results(&fts_results);

    println!("  Fulltext search for 'database':");
    let fts_results2 = store
        .search_fts(db_name, table_name, "database", 10)
        .await?;
    display_results(&fts_results2);

    // Get relations for a document
    let relations = store.get_document_relations(db_name, &doc1.id).await?;
    println!("  Relations from {}: {}", doc1.id, relations.len());
    for rel in &relations {
        println!(
            "    - {} -> {} ({})",
            rel.source_id, rel.target_id, rel.relation_type
        );
    }
    println!();

    // 6. List all databases
    println!("📋 Available databases:");
    let databases = store.list_databases().await?;
    for db in &databases {
        println!("  - {}", db);
    }
    println!();

    // 7. List tables in database
    println!("📋 Available tables in '{}':", db_name);
    let tables = store.list_tables(db_name).await?;
    for table in &tables {
        println!("  - {}", table);
    }
    println!();

    println!("✨ Example completed successfully!");

    Ok(())
}

/// Display search results in a formatted way
fn display_results(results: &[SearchResultTuple]) {
    if results.is_empty() {
        println!("    No results found");
    } else {
        for (i, (id, content, _metadata, score, _is_chunk, _parent_id, _chunk_index)) in
            results.iter().enumerate()
        {
            println!("    {}. Score: {:.4} | ID: {}", i + 1, score, id);
            let preview = content.chars().take(60).collect::<String>();
            println!(
                "       {}{}",
                preview,
                if content.len() > 60 { "..." } else { "" }
            );
        }
    }
    println!();
}
