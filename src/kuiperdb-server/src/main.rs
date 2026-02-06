use actix_web::{middleware::Logger, web, App, HttpServer};
use actix_cors::Cors;
use actix_files as fs;
use anyhow::Result;
use kuiperdb_core::*;
use std::sync::Arc;
use tokio::sync::Mutex;

mod api;
mod telemetry;

#[actix_web::main]
async fn main() -> Result<()> {
    // Initialize OpenTelemetry tracing with file output
    let _guard = telemetry::init_telemetry()?;

    // Load configuration
    let config = config::Config::load("config.json").unwrap_or_else(|_| {
        tracing::warn!("Failed to load config.json, using defaults");
        config::Config::default()
    });

    tracing::info!("kuiperdb starting");
    tracing::info!("  Data directory: {}", config.data_dir);
    tracing::info!("  Port: {}", config.port);
    tracing::info!("  Embedding URL: {}", config.embedding_url);
    tracing::info!("  Embedding dimensions: {}", config.embedding_dimensions);
    tracing::info!("  Embedding workers: {}", config.num_embedding_workers);
    tracing::info!(
        "  Features: embedding={}, embedding_job={}, cache={}, index={}, hybrid={}",
        config.features.embedding,
        config.features.embedding_job,
        config.features.embedding_cache,
        config.features.vector_index,
        config.features.hybrid_search
    );
    tracing::info!(
        "  CORS: enabled={}, origins={:?}",
        config.cors.enabled,
        config.cors.allowed_origins
    );

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&config.data_dir)?;

    // Initialize document store
    let mut store = store::DocumentStore::new(config.data_dir.clone()).await?;
    let store_pool = store.get_global_pool().await?;

    // Configure vector indexing
    if config.features.vector_index {
        let index_config = index::IndexConfig {
            hnsw_m: config.vector_index.hnsw_m,
            hnsw_ef_construction: config.vector_index.hnsw_ef_construction,
            hnsw_ef_search: config.vector_index.hnsw_ef_search,
        };

        let enabled = match config.vector_index.mode.as_str() {
            "always" => true,
            "never" => false,
            "auto" => true, // Will auto-enable at threshold
            _ => true,
        };

        store.configure_indexing(enabled, config.vector_index.threshold, index_config);
        tracing::info!(
            "✓ Vector indexing configured (mode={}, threshold={})",
            config.vector_index.mode,
            config.vector_index.threshold
        );
    }

    tracing::info!("✓ Document store initialized");

    // Initialize embedder with cache
    let embedder: Option<Arc<embedder::OpenAIEmbedder>> = if config.features.embedding {
        let cache_opt = if config.features.embedding_cache {
            // Create embedding cache (10K memory entries, 30 days retention)
            let cache =
                cache::EmbeddingCache::new(store_pool.clone(), "default".to_string(), 10_000)
                    .await?;
            tracing::info!("✓ Embedding cache initialized (10K memory entries)");
            Some(Arc::new(cache))
        } else {
            None
        };

        let mut emb = embedder::OpenAIEmbedder::new(
            config.embedding_url.clone(),
            config.embedding_dimensions,
            config.insecure_skip_verify,
        )?;

        if let Some(cache) = cache_opt {
            emb = emb.with_cache(cache);
        }

        tracing::info!("✓ Embedder initialized");
        Some(Arc::new(emb))
    } else {
        None
    };

    // Start background embedding worker if enabled
    let _worker_handle = if config.features.embedding_job {
        if let Some(ref emb) = embedder {
            let worker = Arc::new(worker::BackgroundWorker::new(
                Arc::new(Mutex::new(store)),
                emb.clone(),
                Arc::new(config.clone()),
            ));

            let handle = worker.start();
            tracing::info!("✓ Background embedding worker started");
            Some(handle)
        } else {
            tracing::warn!("embedding_job enabled but embedding disabled");
            None
        }
    } else {
        None
    };

    // Create shared application state (note: store is duplicated for worker)
    let store_for_api = store::DocumentStore::new(config.data_dir.clone()).await?;
    let app_state = web::Data::new(api::AppState {
        store: Arc::new(Mutex::new(store_for_api)),
        embedder: embedder.clone().map(|e| e as Arc<dyn embedder::Embedder>),
        config: Arc::new(config.clone()),
    });

    tracing::info!("kuiperdb initialized successfully");

    // Start HTTP server
    let bind_addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("🚀 Starting HTTP server on {}", bind_addr);

    let cors_config = config.cors.clone();
    let server = HttpServer::new(move || {
        let mut cors = Cors::default();
        
        if cors_config.enabled {
            for origin in &cors_config.allowed_origins {
                cors = cors.allowed_origin(origin);
            }
            cors = cors
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec![
                    actix_web::http::header::AUTHORIZATION,
                    actix_web::http::header::ACCEPT,
                    actix_web::http::header::CONTENT_TYPE,
                ])
                .max_age(3600);
        }

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .configure(api::configure)
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind(&bind_addr)?
    .run();

    tracing::info!("Server running, press Ctrl+C to stop");

    // Run server and handle shutdown
    server.await?;

    tracing::info!("Shutting down telemetry...");
    telemetry::shutdown_telemetry();

    // Guard will be dropped here, flushing remaining logs
    drop(_guard);

    Ok(())
}
