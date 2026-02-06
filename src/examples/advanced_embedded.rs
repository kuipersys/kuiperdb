//! Advanced Embedded Application Example
//!
//! Demonstrates advanced features of kuiperdb including:
//! - Graph operations and relationships
//! - Document retrieval and management
//! - Batch operations
//! - Multiple databases and tables
//!
//! Run with: cargo run --example advanced_embedded

use chrono::Utc;
use kuiperdb_core::*;
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Advanced kuiperdb Embedded Application\n");

    // Initialize with configuration
    let data_dir = "./data/advanced_example".to_string();
    let mut store = store::DocumentStore::new(data_dir).await?;
    println!("✅ DocumentStore initialized\n");

    let db_name = "default";
    let table_name = "documents";
    store.ensure_table(db_name, table_name).await?;

    // ===== Part 1: Build a Knowledge Base =====
    println!("📚 Building knowledge base...\n");

    let rust_doc = store.add_document(db_name, table_name, StoreDocumentRequest {
        id: None,
        content: "Rust is a systems programming language that emphasizes memory safety without garbage collection.".to_string(),
        metadata: [("title".to_string(), json!("Rust Programming Language")),
                   ("type".to_string(), json!("language")),
                   ("paradigm".to_string(), json!("systems"))].into(),
        tags: vec!["rust".to_string(), "programming".to_string()],
        vectorize: true,
    }).await?;

    let cargo_doc = store.add_document(db_name, table_name, StoreDocumentRequest {
        id: None,
        content: "Cargo is Rust's package manager and build system, making dependency management easy.".to_string(),
        metadata: [("title".to_string(), json!("Cargo Build System")),
                   ("type".to_string(), json!("tool")),
                   ("for".to_string(), json!("rust"))].into(),
        tags: vec!["rust".to_string(), "build-tool".to_string()],
        vectorize: true,
    }).await?;

    let tokio_doc = store.add_document(db_name, table_name, StoreDocumentRequest {
        id: None,
        content: "Tokio is an asynchronous runtime for Rust, enabling high-performance async I/O applications.".to_string(),
        metadata: [("title".to_string(), json!("Tokio Runtime")),
                   ("type".to_string(), json!("library")),
                   ("category".to_string(), json!("async"))].into(),
        tags: vec!["rust".to_string(), "async".to_string(), "library".to_string()],
        vectorize: true,
    }).await?;

    let python_doc = store
        .add_document(
            db_name,
            table_name,
            StoreDocumentRequest {
                id: None,
                content:
                    "Python is a high-level, interpreted programming language with dynamic typing."
                        .to_string(),
                metadata: [
                    ("title".to_string(), json!("Python Language")),
                    ("type".to_string(), json!("language")),
                    ("paradigm".to_string(), json!("multi")),
                ]
                .into(),
                tags: vec!["python".to_string(), "programming".to_string()],
                vectorize: true,
            },
        )
        .await?;

    println!("  Created {} documents\n", 4);

    // ===== Part 2: Create Relationships =====
    println!("🔗 Creating document relationships...\n");

    // Cargo is related to Rust
    let rel1 = models::DocumentRelation {
        id: Uuid::new_v4().to_string(),
        source_id: cargo_doc.id.clone(),
        target_id: rust_doc.id.clone(),
        relation_type: "tool_for".to_string(),
        metadata: Default::default(),
        created_at: Utc::now(),
    };
    store.create_relation(db_name, rel1).await?;
    println!("  ✓ cargo -> rust (tool_for)");

    // Tokio is a library for Rust
    let rel2 = models::DocumentRelation {
        id: Uuid::new_v4().to_string(),
        source_id: tokio_doc.id.clone(),
        target_id: rust_doc.id.clone(),
        relation_type: "library_for".to_string(),
        metadata: Default::default(),
        created_at: Utc::now(),
    };
    store.create_relation(db_name, rel2).await?;
    println!("  ✓ tokio -> rust (library_for)");

    // Rust and Python are similar (both programming languages)
    let rel3 = models::DocumentRelation {
        id: Uuid::new_v4().to_string(),
        source_id: rust_doc.id.clone(),
        target_id: python_doc.id.clone(),
        relation_type: "similar_to".to_string(),
        metadata: [(
            "reason".to_string(),
            json!("both are programming languages"),
        )]
        .into(),
        created_at: Utc::now(),
    };
    store.create_relation(db_name, rel3).await?;
    println!("  ✓ rust <-> python (similar_to)\n");

    // ===== Part 3: Query the Graph =====
    println!("🌐 Graph queries...\n");

    // Find all relations from Rust document
    let rust_relations = store.get_document_relations(db_name, &rust_doc.id).await?;
    println!(
        "  Relations involving Rust document: {}",
        rust_relations.len()
    );
    for rel in &rust_relations {
        println!(
            "    - {} -> {} ({})",
            rel.source_id, rel.target_id, rel.relation_type
        );
    }
    println!();

    // Get all relations in the database
    let all_relations = store.get_all_relations(db_name).await?;
    println!("  Total relations in database: {}", all_relations.len());

    // Calculate graph statistics
    use kuiperdb_core::graph::DocumentGraph;
    let graph = DocumentGraph::new();
    let graph_stats = graph.statistics(&all_relations);

    println!("  Graph Statistics:");
    println!("    Node count: {}", graph_stats.node_count);
    println!("    Edge count: {}", graph_stats.edge_count);
    println!("    Has cycles: {}", graph_stats.has_cycles);
    println!();

    // ===== Part 4: Graph Traversal =====
    println!("🔄 Graph traversal...\n");

    let traversal_result = graph.traverse_bfs(&rust_doc.id, &all_relations, 2, None)?;
    println!(
        "  Documents reachable from Rust (depth 2): {}",
        traversal_result.document_ids.len()
    );
    for doc_id in &traversal_result.document_ids {
        let depth = traversal_result.depth_map.get(doc_id).unwrap_or(&0);
        println!("    - {} (depth: {})", doc_id, depth);
    }
    println!();

    // ===== Part 5: Shortest Path =====
    println!("🛣️  Finding shortest path...\n");

    if let Some(path) = graph.shortest_path(&cargo_doc.id, &python_doc.id, &all_relations)? {
        println!("  Shortest path from Cargo to Python:");
        println!("    Path: {} nodes", path.path.len());
        for (i, node) in path.path.iter().enumerate() {
            println!("      {}. {}", i + 1, node);
        }
        println!("    Total weight: {}", path.total_weight);
    } else {
        println!("  No path found from Cargo to Python");
    }
    println!();

    // ===== Part 6: Fulltext Search =====
    println!("🔍 Fulltext search...\n");

    let search_results = store.search_fts(db_name, table_name, "async", 5).await?;
    println!("  Search results for 'async':");
    for (i, (id, content, metadata, score, _is_chunk, _parent, _chunk_idx)) in
        search_results.iter().enumerate()
    {
        let title = metadata
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        println!("    {}. {} (score: {:.4})", i + 1, title, score);
        println!("       {}", content.chars().take(80).collect::<String>());
        println!("       ID: {}", id);
    }
    println!();

    // ===== Part 7: Batch Retrieval =====
    println!("📦 Batch document retrieval...\n");

    let doc_ids = vec![&rust_doc.id, &cargo_doc.id, &tokio_doc.id];
    println!("  Checking {} documents:", doc_ids.len());

    for id in doc_ids {
        let doc = store.get_document(db_name, table_name, id).await?;
        let title = doc
            .metadata
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        println!("    ✓ {} - embedded: {}", title, doc.is_embedded);
    }
    println!();

    // ===== Part 8: Multiple Databases/Tables =====
    println!("🗄️  Working with multiple databases...\n");

    println!("  Available databases:");
    let databases = store.list_databases().await?;
    for db in &databases {
        println!("    - {}", db);
    }
    println!();

    println!("  Available tables in '{}':", db_name);
    let tables = store.list_tables(db_name).await?;
    for table in &tables {
        println!("    - {}", table);
    }
    println!();

    println!("✨ Advanced example completed!");

    Ok(())
}
