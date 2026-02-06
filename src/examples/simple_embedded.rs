//! Simple Embedded Application Example
//!
//! A minimal example showing the basic usage of kuiperdb as an embedded library.
//!
//! Run with: cargo run --example simple_embedded

use kuiperdb_core::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Simple kuiperdb Embedded Example\n");

    // Create a DocumentStore
    let mut store = store::DocumentStore::new("./data/simple_example".to_string()).await?;
    println!("✅ Store initialized\n");

    // Database and table
    let db_name = "default";
    let table_name = "documents";

    // Ensure table exists
    store.ensure_table(db_name, table_name).await?;

    // Add a document - now much cleaner!
    let doc = store
        .add_simple_document(
            db_name,
            table_name,
            "The quick brown fox jumps over the lazy dog.",
        )
        .await?;

    println!("📝 Added document: {}", doc.id);

    // Retrieve it
    let retrieved = store.get_document(db_name, table_name, &doc.id).await?;
    println!("   Content: {}\n", retrieved.content);

    // Search for it (fulltext search)
    let results = store.search_fts(db_name, table_name, "fox", 5).await?;
    println!("🔍 Search results for 'fox':");
    for (i, (id, content, _metadata, score, _is_chunk, _parent, _chunk_idx)) in
        results.iter().enumerate()
    {
        println!("   {}. {} (score: {:.4})", i + 1, content, score);
        println!("      ID: {}", id);
    }

    Ok(())
}
