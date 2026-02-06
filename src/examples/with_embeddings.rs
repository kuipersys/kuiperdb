//! Example with Real Embeddings using LM Studio
//!
//! This example demonstrates how to use kuiperdb with a local embedding server
//! like LM Studio running on your machine.
//!
//! Prerequisites:
//! 1. Start LM Studio
//! 2. Load an embedding model (e.g., nomic-embed-text)
//! 3. Start the local server (typically http://localhost:1234)
//!
//! Run with: cargo run --example with_embeddings

use kuiperdb_core::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 kuiperdb with Real Embeddings Example\n");

    // Initialize the DocumentStore
    let data_dir = "./data/embeddings_example".to_string();
    let mut store = store::DocumentStore::new(data_dir).await?;

    let db_name = "default";
    let table_name = "documents";
    store.ensure_table(db_name, table_name).await?;
    println!("✅ DocumentStore initialized\n");

    // Initialize the embedder pointing to LM Studio
    // LM Studio typically runs on http://localhost:1234
    // Adjust the URL if your LM Studio is on a different port
    // NOTE: Just use the base URL - the embedder will add /v1/embeddings
    let embedding_url = "http://localhost:1234";
    let embedding_dimensions = 768; // Common size for nomic-embed-text, adjust based on your model

    println!(
        "🔗 Connecting to embedding server at {}/v1/embeddings...",
        embedding_url
    );
    println!("   (Make sure LM Studio is running with an embedding model loaded)\n");

    let embedder = embedder::OpenAIEmbedder::new(
        embedding_url.to_string(),
        embedding_dimensions,
        false, // Don't skip TLS verification for localhost
    )?;

    // Test the embedder with a simple query
    print!("🧪 Testing embedder... ");
    match embedder.embed("test").await {
        Ok(vec) => {
            println!("✅ Success! (got {} dimensional vector)", vec.len());
            println!();
        }
        Err(e) => {
            println!("❌ Failed!");
            println!("\nError: {}", e);
            println!("\n⚠️  Make sure:");
            println!("   1. LM Studio is running");
            println!("   2. An embedding model is loaded (e.g., nomic-embed-text)");
            println!("   3. The local server is started (Server tab in LM Studio)");
            println!("   4. The port matches (default is 1234)\n");
            return Err(e);
        }
    }

    // Add documents with embeddings
    println!("📝 Adding documents with embeddings...\n");

    let documents = vec![
        (
            "Rust Programming",
            "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety.",
            vec!["rust", "programming", "systems"]
        ),
        (
            "Python Overview",
            "Python is an interpreted, high-level programming language known for its simplicity and extensive ecosystem.",
            vec!["python", "programming", "scripting"]
        ),
        (
            "Machine Learning",
            "Machine learning is a method of data analysis that automates analytical model building using algorithms.",
            vec!["ai", "ml", "data-science"]
        ),
        (
            "Web Development",
            "Web development involves building and maintaining websites using HTML, CSS, JavaScript, and backend technologies.",
            vec!["web", "frontend", "backend"]
        ),
        (
            "Database Systems",
            "Databases are organized collections of structured information or data stored electronically for efficient retrieval.",
            vec!["database", "storage", "sql"]
        ),
    ];

    for (title, content, tags) in documents {
        print!("  Adding: {}... ", title);

        // Generate embedding
        let embedding = embedder.embed(content).await?;

        // Create document with embedding
        let doc = models::Document {
            id: uuid::Uuid::new_v4().to_string(),
            db: db_name.to_string(),
            table: table_name.to_string(),
            content: content.to_string(),
            metadata: [("title".to_string(), serde_json::json!(title))].into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            vector: Some(embedding),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            is_embedded: true,
            vectorize: true,
            is_chunk: false,
            parent_id: None,
            chunk_index: None,
            token_count: None,
        };

        store.store_document(db_name, table_name, doc).await?;
        println!("✅");
    }

    println!("\n🔍 Performing semantic searches...\n");

    // Create a hybrid searcher
    let searcher = search::HybridSearcher::new();

    // Search 1: Programming languages
    println!("Query: 'fast compiled programming language'");
    let results = searcher
        .search(
            &mut store,
            Some(&embedder as &dyn embedder::Embedder),
            db_name,
            table_name,
            "fast compiled programming language",
            3,
        )
        .await?;

    display_search_results(&results);

    // Search 2: Data and AI
    println!("\nQuery: 'artificial intelligence and data analytics'");
    let results = searcher
        .search(
            &mut store,
            Some(&embedder as &dyn embedder::Embedder),
            db_name,
            table_name,
            "artificial intelligence and data analytics",
            3,
        )
        .await?;

    display_search_results(&results);

    // Search 3: Web technologies
    println!("\nQuery: 'building websites and user interfaces'");
    let results = searcher
        .search(
            &mut store,
            Some(&embedder as &dyn embedder::Embedder),
            db_name,
            table_name,
            "building websites and user interfaces",
            3,
        )
        .await?;

    display_search_results(&results);

    // Demonstrate vector-only search vs hybrid
    println!("\n📊 Comparing search types...\n");

    let query = "data storage and retrieval";

    // Vector search
    println!("Vector Search for: '{}'", query);
    let query_vec = embedder.embed(query).await?;
    let vector_results = store
        .search_vector(db_name, table_name, &query_vec, 3)
        .await?;
    println!("  Results:");
    for (i, (_id, _content, metadata, score, _, _, _)) in vector_results.iter().enumerate() {
        let title = metadata
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        println!("    {}. {} (similarity: {:.4})", i + 1, title, score);
    }

    // Fulltext search
    println!("\nFulltext Search for: '{}'", query);
    let fts_results = store.search_fts(db_name, table_name, query, 3).await?;
    println!("  Results:");
    for (i, (_id, _content, metadata, score, _, _, _)) in fts_results.iter().enumerate() {
        let title = metadata
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        println!("    {}. {} (FTS score: {:.4})", i + 1, title, score);
    }

    println!("\n✨ Example completed successfully!");
    println!("\n💡 Tips:");
    println!("   - Hybrid search combines vector similarity + fulltext for best results");
    println!("   - Vector search finds semantically similar content even with different words");
    println!("   - Fulltext search is faster for exact keyword matches");

    Ok(())
}

fn display_search_results(results: &[search::SearchResult]) {
    if results.is_empty() {
        println!("  No results found");
    } else {
        println!("  Results:");
        for (i, result) in results.iter().enumerate() {
            let title = result
                .metadata
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            println!("    {}. {} (score: {:.4})", i + 1, title, result.score);

            if let Some(vec_sim) = result.vector_similarity {
                print!("       Vector: {:.4}", vec_sim);
            }
            if let Some(fts_rank) = result.fts_rank {
                print!(" | FTS: {:.4}", fts_rank);
            }
            println!();
        }
    }
}
