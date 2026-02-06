use actix_web::{web, HttpRequest, HttpResponse, Result as ActixResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use kuiperdb_core::config::Config;
use kuiperdb_core::embedder::Embedder;
use kuiperdb_core::models::{
    Document, ErrorResponse, SearchRequest, SearchResponse, StoreDocumentRequest,
};
use kuiperdb_core::store::DocumentStore;

/// Shared application state
pub struct AppState {
    pub store: Arc<Mutex<DocumentStore>>,
    pub embedder: Option<Arc<dyn Embedder>>,
    pub config: Arc<Config>,
}

/// Log file information
#[derive(Serialize)]
pub struct LogFileInfo {
    pub name: String,
    pub size: u64,
    pub date: String,
    pub number: u32,
}

/// Log files response
#[derive(Serialize)]
pub struct LogFilesResponse {
    pub files: Vec<LogFileInfo>,
    pub total_size: u64,
    pub total_count: usize,
}

/// Log analysis response
#[derive(Serialize)]
pub struct LogAnalysisResponse {
    pub date: String,
    pub total_entries: usize,
    pub by_level: std::collections::HashMap<String, usize>,
    pub by_target: Vec<(String, usize)>,
    pub api_operations: Vec<(String, usize)>,
    pub errors: Vec<String>,
}

/// Log cleanup request
#[derive(Deserialize)]
pub struct LogCleanupRequest {
    pub days_to_keep: Option<u32>,
}

/// Store a document
/// POST /db/{db_name}/{table_name}
#[tracing::instrument(skip(path, req, state, http_req))]
pub async fn store_document(
    path: web::Path<(String, String)>,
    req: web::Json<StoreDocumentRequest>,
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let (db_name, table_name) = path.into_inner();
    tracing::debug!(db = %db_name, table = %table_name, "Storing document");

    if req.content.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "content is required".to_string(),
            message: None,
        }));
    }

    // Use the cleaner add_document API
    let mut store = state.store.lock().await;
    let mut doc = match store
        .add_document(&db_name, &table_name, req.0.clone())
        .await
    {
        Ok(doc) => doc,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "failed to store document".to_string(),
                message: Some(e.to_string()),
            }));
        }
    };

    // Check if sync embedding is requested
    if state.config.features.embedding {
        if let Some(embedder) = &state.embedder {
            // Parse X-Client-Features header
            let client_features =
                parse_client_features(http_req.headers().get("X-Client-Features"));

            let should_embed = !client_features.contains_key("embed")
                || client_features.get("embed").map(|v| v.as_str()) != Some("async");

            if should_embed {
                match embedder.embed(&req.content).await {
                    Ok(vector) => {
                        doc.vector = Some(vector);
                        doc.is_embedded = true;
                    }
                    Err(e) => {
                        return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "embedding failed".to_string(),
                            message: Some(e.to_string()),
                        }));
                    }
                }
            }
        }
    }

    // Handle chunking if enabled and document exceeds threshold
    let mut chunks_to_store = Vec::new();
    if state.config.features.chunking && state.config.chunking.enabled && doc.vectorize {
        use kuiperdb_core::chunking::{Chunker, FixedTokenChunker, MarkdownChunker};

        // Select chunker based on strategy
        let use_markdown = state.config.chunking.strategy.as_str() == "markdown";
        
        // Count tokens and chunk if needed
        if use_markdown {
            if let Ok(chunker) = MarkdownChunker::new() {
                if let Ok(token_count) = chunker.count_tokens(&doc.content) {
                    doc.token_count = Some(token_count as i32);

                    if token_count > state.config.chunking.token_threshold {
                        tracing::info!(
                            "Document {} has {} tokens, chunking with markdown strategy...",
                            doc.id,
                            token_count
                        );
                        doc.vectorize = false;

                        if let Ok(chunk_texts) = chunker.chunk(
                            &doc.content,
                            state.config.chunking.chunk_size,
                            state.config.chunking.chunk_overlap,
                        ) {
                            for (idx, chunk_text) in chunk_texts.iter().enumerate() {
                                let chunk_doc = Document {
                                    id: Uuid::new_v4().to_string(),
                                    db: db_name.clone(),
                                    table: table_name.clone(),
                                    content: chunk_text.clone(),
                                    metadata: doc.metadata.clone(),
                                    tags: doc.tags.clone(),
                                    vector: None,
                                    created_at: Utc::now(),
                                    updated_at: Utc::now(),
                                    is_embedded: false,
                                    vectorize: true,
                                    is_chunk: true,
                                    parent_id: Some(doc.id.clone()),
                                    chunk_index: Some(idx as i32),
                                    token_count: chunker
                                        .count_tokens(chunk_text)
                                        .ok()
                                        .map(|c| c as i32),
                                    is_vectorized: false,
                                };
                                chunks_to_store.push(chunk_doc);
                            }

                            tracing::info!(
                                "Created {} chunks for document {}",
                                chunks_to_store.len(),
                                doc.id
                            );
                        }
                    }
                }
            }
        } else {
            // Use fixed token chunker
            if let Ok(chunker) = FixedTokenChunker::new() {
                if let Ok(token_count) = chunker.count_tokens(&doc.content) {
                    doc.token_count = Some(token_count as i32);

                    if token_count > state.config.chunking.token_threshold {
                        tracing::info!(
                            "Document {} has {} tokens, chunking with fixed token strategy...",
                            doc.id,
                            token_count
                        );
                        doc.vectorize = false;

                        if let Ok(chunk_texts) = chunker.chunk(
                            &doc.content,
                            state.config.chunking.chunk_size,
                            state.config.chunking.chunk_overlap,
                        ) {
                            for (idx, chunk_text) in chunk_texts.iter().enumerate() {
                                let chunk_doc = Document {
                                    id: Uuid::new_v4().to_string(),
                                    db: db_name.clone(),
                                    table: table_name.clone(),
                                    content: chunk_text.clone(),
                                    metadata: doc.metadata.clone(),
                                    tags: doc.tags.clone(),
                                    vector: None,
                                    created_at: Utc::now(),
                                    updated_at: Utc::now(),
                                    is_embedded: false,
                                    vectorize: true,
                                    is_chunk: true,
                                    parent_id: Some(doc.id.clone()),
                                    chunk_index: Some(idx as i32),
                                    token_count: chunker
                                        .count_tokens(chunk_text)
                                        .ok()
                                        .map(|c| c as i32),
                                    is_vectorized: false,
                                };
                                chunks_to_store.push(chunk_doc);
                            }

                            tracing::info!(
                                "Created {} chunks for document {}",
                                chunks_to_store.len(),
                                doc.id
                            );
                        }
                    }
                }
            }
        }
    }

    // Update parent document with new settings
    if let Err(e) = store
        .store_document(&db_name, &table_name, doc.clone())
        .await
    {
        return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
            error: "failed to store document".to_string(),
            message: Some(e.to_string()),
        }));
    }

    // Store chunks
    for chunk in chunks_to_store {
        if let Err(e) = store.store_document(&db_name, &table_name, chunk).await {
            tracing::warn!("Failed to store chunk: {}", e);
        }
    }

    // Parse metadata level from Accept header
    let metadata_level = parse_metadata_level(http_req.headers().get("Accept"));

    match metadata_level.as_str() {
        "none" => Ok(HttpResponse::Created().json(serde_json::json!({
            "id": doc.id
        }))),
        "minimal" => Ok(HttpResponse::Created().json(serde_json::json!({
            "id": doc.id,
            "created_at": doc.created_at,
            "updated_at": doc.updated_at,
            "is_embedded": doc.is_embedded
        }))),
        _ => Ok(HttpResponse::Created().json(doc)),
    }
}

/// Get a document by ID
/// GET /db/{db_name}/{table_name}/{doc_id}
#[tracing::instrument(skip(path, state))]
pub async fn get_document(
    path: web::Path<(String, String, String)>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, table_name, doc_id) = path.into_inner();
    tracing::debug!(db = %db_name, table = %table_name, doc_id = %doc_id, "Getting document");

    let mut store = state.store.lock().await;
    match store.get_document(&db_name, &table_name, &doc_id).await {
        Ok(doc) => {
            tracing::debug!("Document retrieved successfully");
            Ok(HttpResponse::Ok().json(doc))
        }
        Err(e) => {
            tracing::warn!("Document not found: {}", e);
            Ok(HttpResponse::NotFound().json(ErrorResponse {
                error: "document not found".to_string(),
                message: None,
            }))
        }
    }
}

/// Delete a document
/// DELETE /db/{db_name}/{table_name}/{doc_id}
#[tracing::instrument(skip(path, state))]
pub async fn delete_document(
    path: web::Path<(String, String, String)>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, table_name, doc_id) = path.into_inner();
    tracing::debug!(db = %db_name, table = %table_name, doc_id = %doc_id, "Deleting document");

    let mut store = state.store.lock().await;
    match store
        .delete_document_by_id(&db_name, &table_name, &doc_id)
        .await
    {
        Ok(_) => {
            tracing::info!("Document deleted successfully");
            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            tracing::error!("Failed to delete document: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "failed to delete document".to_string(),
                message: Some(e.to_string()),
            }))
        }
    }
}

/// DELETE /db/{db_name}/{table_name}
pub async fn delete_table(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, table_name) = path.into_inner();
    tracing::debug!(db = %db_name, table = %table_name, "Deleting table");

    let mut store = state.store.lock().await;
    match store.delete_table(&db_name, &table_name).await {
        Ok(_) => {
            tracing::info!("Table deleted successfully");
            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            tracing::error!("Failed to delete table: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "failed to delete table".to_string(),
                message: Some(e.to_string()),
            }))
        }
    }
}

/// DELETE /db/{db_name}
pub async fn delete_database(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let db_name = path.into_inner();
    tracing::debug!(db = %db_name, "Deleting database");

    let mut store = state.store.lock().await;
    match store.delete_database(&db_name).await {
        Ok(_) => {
            tracing::info!("Database deleted successfully");
            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            tracing::error!("Failed to delete database: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "failed to delete database".to_string(),
                message: Some(e.to_string()),
            }))
        }
    }
}

/// Search documents
/// POST /db/{db_name}/{table_name}/search
#[tracing::instrument(skip(path, req, state))]
pub async fn search(
    path: web::Path<(String, String)>,
    req: web::Json<SearchRequest>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, table_name) = path.into_inner();
    tracing::debug!(
        db = %db_name,
        table = %table_name,
        query_len = req.query.len(),
        limit = req.limit.unwrap_or(10),
        "Searching documents"
    );

    let mut store = state.store.lock().await;
    let searcher = kuiperdb_core::search::HybridSearcher::new();

    let results = searcher
        .search(
            &mut store,
            state.embedder.as_deref(),
            &db_name,
            &table_name,
            &req.query,
            req.limit.unwrap_or(10),
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Search failed: {}", e)))?;

    let total = results.len();
    let response = SearchResponse {
        results,
        query: req.query.clone(),
        search_type: req.search_type,
        db: db_name,
        total,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Health check
/// GET /health
pub async fn health() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "timestamp": Utc::now()
    })))
}

/// List databases
/// GET /db
pub async fn list_databases(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let store = state.store.lock().await;
    let databases = store.list_databases().await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to list databases: {}", e))
    })?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "databases": databases.iter().map(|name| serde_json::json!({"name": name})).collect::<Vec<_>>()
    })))
}

/// List tables in a database
/// GET /db/{db_name}/tables
pub async fn list_tables(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let db_name = path.into_inner();
    let mut store = state.store.lock().await;
    
    let tables = store.list_tables(&db_name).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to list tables: {}", e))
    })?;
    
    // Filter out system/internal tables
    let user_tables: Vec<String> = tables
        .into_iter()
        .filter(|name| !name.ends_with("_fts") && name != "document_relations")
        .collect();
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "tables": user_tables.iter().map(|name| serde_json::json!({"name": name})).collect::<Vec<_>>()
    })))
}

/// List documents in a table (roots only - documents without parent_id)
/// GET /db/{db_name}/{table_name}/documents
pub async fn list_documents(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, table_name) = path.into_inner();
    let mut store = state.store.lock().await;
    
    // Get all documents (not just non-embedded ones)
    let all_docs = store.get_all_documents(&db_name, &table_name, 1000).await
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Query error: {}", e)))?;
    
    // Filter for root documents (no parent_id)
    let root_docs: Vec<_> = all_docs.into_iter()
        .filter(|doc| doc.parent_id.is_none() || doc.parent_id.as_ref().map(|s| s.is_empty()).unwrap_or(true))
        .take(100)
        .collect();
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "documents": root_docs
    })))
}

/// Parse X-Client-Features header
/// Format: "feature1=value; feature2=value; feature3"
fn parse_client_features(
    header: Option<&actix_web::http::header::HeaderValue>,
) -> std::collections::HashMap<String, String> {
    let mut features = std::collections::HashMap::new();

    if let Some(value) = header {
        if let Ok(header_str) = value.to_str() {
            for part in header_str.split(';') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }

                if let Some(idx) = part.find('=') {
                    let key = part[..idx].trim().to_string();
                    let val = part[idx + 1..].trim().to_string();
                    features.insert(key, val);
                } else {
                    features.insert(part.to_string(), String::new());
                }
            }
        }
    }

    features
}

/// Parse Accept header for metadata level
/// Supports: application/json;metadata=minimal|full|none
fn parse_metadata_level(header: Option<&actix_web::http::header::HeaderValue>) -> String {
    if let Some(value) = header {
        if let Ok(header_str) = value.to_str() {
            for part in header_str.split(';').skip(1) {
                let part = part.trim();
                if part.starts_with("metadata=") {
                    let level = part.trim_start_matches("metadata=").trim_matches('"');
                    if matches!(level, "full" | "none" | "minimal") {
                        return level.to_string();
                    }
                }
            }
        }
    }

    "minimal".to_string()
}

// ===== Document Relations Endpoints =====

/// Create a document relation
/// POST /db/{db_name}/relations
pub async fn create_relation(
    path: web::Path<String>,
    req: web::Json<kuiperdb_core::models::CreateRelationRequest>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let db_name = path.into_inner();

    if !state.config.features.document_relations {
        return Ok(HttpResponse::NotImplemented().json(ErrorResponse {
            error: "document_relations feature is disabled".to_string(),
            message: None,
        }));
    }

    let relation = kuiperdb_core::models::DocumentRelation {
        id: Uuid::new_v4().to_string(),
        source_id: req.source_id.clone(),
        target_id: req.target_id.clone(),
        relation_type: req.relation_type.clone(),
        metadata: req.metadata.clone(),
        created_at: Utc::now(),
    };

    let mut store = state.store.lock().await;
    match store.create_relation(&db_name, relation.clone()).await {
        Ok(_) => Ok(HttpResponse::Created().json(relation)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ErrorResponse {
            error: "failed to create relation".to_string(),
            message: Some(e.to_string()),
        })),
    }
}

/// Get a relation by ID
/// GET /db/{db_name}/relations/{relation_id}
pub async fn get_relation(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, relation_id) = path.into_inner();

    let mut store = state.store.lock().await;
    match store.get_relation(&db_name, &relation_id).await {
        Ok(relation) => Ok(HttpResponse::Ok().json(relation)),
        Err(_) => Ok(HttpResponse::NotFound().json(ErrorResponse {
            error: "relation not found".to_string(),
            message: None,
        })),
    }
}

/// Delete a relation
/// DELETE /db/{db_name}/relations/{relation_id}
pub async fn delete_relation(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, relation_id) = path.into_inner();

    let mut store = state.store.lock().await;
    match store.delete_relation(&db_name, &relation_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ErrorResponse {
            error: "failed to delete relation".to_string(),
            message: Some(e.to_string()),
        })),
    }
}

/// Get all relations for a document
/// GET /db/{db_name}/documents/{doc_id}/relations
pub async fn get_document_relations(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, doc_id) = path.into_inner();

    let mut store = state.store.lock().await;
    match store.get_document_relations(&db_name, &doc_id).await {
        Ok(relations) => Ok(HttpResponse::Ok().json(relations)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ErrorResponse {
            error: "failed to get relations".to_string(),
            message: Some(e.to_string()),
        })),
    }
}

/// Graph traversal
/// POST /db/{db_name}/graph/traverse
pub async fn graph_traverse(
    path: web::Path<String>,
    req: web::Json<kuiperdb_core::models::GraphTraversalRequest>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let db_name = path.into_inner();

    if !state.config.features.document_relations {
        return Ok(HttpResponse::NotImplemented().json(ErrorResponse {
            error: "document_relations feature is disabled".to_string(),
            message: None,
        }));
    }

    use kuiperdb_core::graph::DocumentGraph;

    let mut store = state.store.lock().await;
    let relations = store.get_all_relations(&db_name).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to get relations: {}", e))
    })?;

    let graph = DocumentGraph::new();
    let filter = if req.relation_types.is_empty() {
        None
    } else {
        Some(req.relation_types.as_slice())
    };

    let result = graph
        .traverse_bfs(&req.start_id, &relations, req.depth, filter)
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Traversal failed: {}", e))
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "document_ids": result.document_ids,
        "relations": result.relations,
        "depth_map": result.depth_map,
    })))
}

/// Get shortest path between two documents
/// GET /db/{db_name}/graph/path?from={from_id}&to={to_id}
pub async fn graph_shortest_path(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let db_name = path.into_inner();

    let from_id = query
        .get("from")
        .ok_or_else(|| actix_web::error::ErrorBadRequest("'from' parameter required"))?;

    let to_id = query
        .get("to")
        .ok_or_else(|| actix_web::error::ErrorBadRequest("'to' parameter required"))?;

    use kuiperdb_core::graph::DocumentGraph;

    let mut store = state.store.lock().await;
    let relations = store.get_all_relations(&db_name).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to get relations: {}", e))
    })?;

    let graph = DocumentGraph::new();
    let result = graph
        .shortest_path(from_id, to_id, &relations)
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Path finding failed: {}", e))
        })?;

    match result {
        Some(path) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "path": path.path,
            "relations": path.relations,
            "total_weight": path.total_weight,
        }))),
        None => Ok(HttpResponse::NotFound().json(ErrorResponse {
            error: "no path found".to_string(),
            message: None,
        })),
    }
}

/// Get graph statistics
/// GET /db/{db_name}/graph/stats
pub async fn graph_statistics(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let db_name = path.into_inner();

    use kuiperdb_core::graph::DocumentGraph;

    let mut store = state.store.lock().await;
    let relations = store.get_all_relations(&db_name).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to get relations: {}", e))
    })?;

    let graph = DocumentGraph::new();
    let stats = graph.statistics(&relations);

    Ok(HttpResponse::Ok().json(stats))
}

// ===== Chunking Endpoints =====

/// Get chunks for a parent document
/// GET /db/{db_name}/{table_name}/{doc_id}/chunks
pub async fn get_chunks(
    path: web::Path<(String, String, String)>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, table_name, doc_id) = path.into_inner();

    let mut store = state.store.lock().await;
    match store.get_chunks(&db_name, &table_name, &doc_id).await {
        Ok(chunks) => Ok(HttpResponse::Ok().json(chunks)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(ErrorResponse {
            error: "failed to get chunks".to_string(),
            message: Some(e.to_string()),
        })),
    }
}

/// Force re-chunk a document
/// POST /db/{db_name}/{table_name}/{doc_id}/rechunk
pub async fn rechunk_document(
    path: web::Path<(String, String, String)>,
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let (db_name, table_name, doc_id) = path.into_inner();

    if !state.config.features.chunking || !state.config.chunking.enabled {
        return Ok(HttpResponse::NotImplemented().json(ErrorResponse {
            error: "chunking feature is disabled".to_string(),
            message: None,
        }));
    }

    // Get original document
    let mut store = state.store.lock().await;
    let doc = store
        .get_document(&db_name, &table_name, &doc_id)
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(format!("Document not found: {}", e)))?;

    // Delete existing chunks
    store
        .delete_chunks(&db_name, &table_name, &doc_id)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to delete chunks: {}", e))
        })?;

    // Re-chunk
    use kuiperdb_core::chunking::{Chunker, FixedTokenChunker};

    let chunker = FixedTokenChunker::new().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to create chunker: {}", e))
    })?;

    let chunks_texts = chunker
        .chunk(
            &doc.content,
            state.config.chunking.chunk_size,
            state.config.chunking.chunk_overlap,
        )
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Chunking failed: {}", e))
        })?;

    let mut created_chunks = Vec::new();
    for (idx, chunk_text) in chunks_texts.iter().enumerate() {
        let chunk_doc = Document {
            id: Uuid::new_v4().to_string(),
            db: db_name.clone(),
            table: table_name.clone(),
            content: chunk_text.clone(),
            metadata: doc.metadata.clone(),
            tags: doc.tags.clone(),
            vector: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_embedded: false,
            vectorize: true,
            is_chunk: true,
            parent_id: Some(doc.id.clone()),
            chunk_index: Some(idx as i32),
            token_count: chunker.count_tokens(chunk_text).ok().map(|c| c as i32),
            is_vectorized: false,
        };

        store
            .store_document(&db_name, &table_name, chunk_doc.clone())
            .await
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!("Failed to store chunk: {}", e))
            })?;

        created_chunks.push(chunk_doc);
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "chunks_created": created_chunks.len(),
        "chunks": created_chunks,
    })))
}

/// List log files
/// GET /logs
pub async fn list_logs() -> ActixResult<HttpResponse> {
    use std::fs;
    use std::path::Path;

    let log_dir = Path::new("./logs");
    if !log_dir.exists() {
        return Ok(HttpResponse::Ok().json(LogFilesResponse {
            files: vec![],
            total_size: 0,
            total_count: 0,
        }));
    }

    let mut files = Vec::new();
    let mut total_size = 0u64;

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("kuiperdb.") && name.contains(".log") {
                        // Parse: kuiperdb.log.2026-02-04 (current file) or kuiperdb.log.2026-02-04-0 (rotated)
                        // rolling-file creates: kuiperdb.log (current) and kuiperdb.log.YYYY-MM-DD (rotated daily)
                        let date: String;
                        let number: u32;
                        
                        if name == "kuiperdb.log" {
                            // Current active log file
                            date = chrono::Utc::now().format("%Y-%m-%d").to_string();
                            number = 0;
                        } else if name.starts_with("kuiperdb.log.") {
                            // Rotated file: kuiperdb.log.2026-02-04
                            let date_str = &name[10..]; // Skip "kuiperdb.log."
                            date = date_str.to_string();
                            number = 0;
                        } else {
                            continue;
                        }

                        total_size += metadata.len();
                        files.push(LogFileInfo {
                            name: name.to_string(),
                            size: metadata.len(),
                            date,
                            number,
                        });
                    }
                }
            }
        }
    }

    files.sort_by(|a, b| a.date.cmp(&b.date).then(a.number.cmp(&b.number)));

    Ok(HttpResponse::Ok().json(LogFilesResponse {
        total_count: files.len(),
        total_size,
        files,
    }))
}

/// View log file content
/// GET /logs/{filename}
pub async fn view_log(path: web::Path<String>) -> ActixResult<HttpResponse> {
    use std::fs;
    use std::path::Path;

    let filename = path.into_inner();

    // Validate filename to prevent path traversal
    if filename.contains("..") || filename.contains("/") || filename.contains("\\") {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "invalid filename".to_string(),
            message: Some("filename cannot contain path separators".to_string()),
        }));
    }

    let log_path = Path::new("./logs").join(&filename);

    if !log_path.exists() {
        return Ok(HttpResponse::NotFound().json(ErrorResponse {
            error: "log file not found".to_string(),
            message: None,
        }));
    }

    match fs::read_to_string(&log_path) {
        Ok(content) => {
            // Return as JSON array of log entries
            let entries: Vec<serde_json::Value> = content
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect();

            Ok(HttpResponse::Ok().json(entries))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(ErrorResponse {
            error: "failed to read log file".to_string(),
            message: Some(e.to_string()),
        })),
    }
}

/// Analyze logs for a specific date
/// GET /logs/analyze/{date}
pub async fn analyze_logs(path: web::Path<String>) -> ActixResult<HttpResponse> {
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;

    let date = path.into_inner();

    // Validate date format (yyyy-MM-dd)
    if date.len() != 10 || !date.chars().nth(4).map(|c| c == '-').unwrap_or(false) {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "invalid date format".to_string(),
            message: Some("expected format: yyyy-MM-dd".to_string()),
        }));
    }

    let log_dir = Path::new("./logs");

    let mut entries: Vec<serde_json::Value> = Vec::new();

    if let Ok(dir_entries) = fs::read_dir(log_dir) {
        for entry in dir_entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                // Check if it's the current log or a rotated log for the specified date
                let is_match = if name == "kuiperdb.log" {
                    // Current log file - check if it matches today's date
                    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                    date == today
                } else if name.starts_with("kuiperdb.log.") {
                    // Rotated log file: kuiperdb.log.2026-02-04
                    let log_date = &name[10..]; // Skip "kuiperdb.log."
                    log_date == date
                } else {
                    false
                };

                if is_match {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        for line in content.lines() {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                entries.push(json);
                            }
                        }
                    }
                }
            }
        }
    }

    if entries.is_empty() {
        return Ok(HttpResponse::NotFound().json(ErrorResponse {
            error: "no log entries found".to_string(),
            message: Some(format!("no logs for date: {}", date)),
        }));
    }

    // Analyze
    let mut by_level: HashMap<String, usize> = HashMap::new();
    let mut by_target: HashMap<String, usize> = HashMap::new();
    let mut api_ops: HashMap<String, usize> = HashMap::new();
    let mut errors: Vec<String> = Vec::new();

    for entry in &entries {
        if let Some(level) = entry.get("level").and_then(|v| v.as_str()) {
            *by_level.entry(level.to_string()).or_insert(0) += 1;

            if level == "ERROR" {
                if let Some(msg) = entry
                    .get("fields")
                    .and_then(|f| f.get("message"))
                    .and_then(|m| m.as_str())
                {
                    errors.push(msg.to_string());
                }
            }
        }

        if let Some(target) = entry.get("target").and_then(|v| v.as_str()) {
            *by_target.entry(target.to_string()).or_insert(0) += 1;
        }

        if let Some(span) = entry
            .get("span")
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str())
        {
            *api_ops.entry(span.to_string()).or_insert(0) += 1;
        }
    }

    let mut by_target_vec: Vec<(String, usize)> = by_target.into_iter().collect();
    by_target_vec.sort_by(|a, b| b.1.cmp(&a.1));
    by_target_vec.truncate(10); // Top 10

    let mut api_ops_vec: Vec<(String, usize)> = api_ops.into_iter().collect();
    api_ops_vec.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(HttpResponse::Ok().json(LogAnalysisResponse {
        date,
        total_entries: entries.len(),
        by_level,
        by_target: by_target_vec,
        api_operations: api_ops_vec,
        errors: errors.into_iter().take(10).collect(), // First 10 errors
    }))
}

/// Cleanup old log files
/// POST /logs/cleanup
pub async fn cleanup_logs(req: web::Json<LogCleanupRequest>) -> ActixResult<HttpResponse> {
    use chrono::{NaiveDate, Utc};
    use std::fs;
    use std::path::Path;

    let days_to_keep = req.days_to_keep.unwrap_or(30);
    let cutoff_date = Utc::now().naive_utc().date() - chrono::Duration::days(days_to_keep as i64);

    let log_dir = Path::new("./logs");
    let mut deleted_files = Vec::new();
    let mut deleted_size = 0u64;

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("kuiperdb.log.") {
                    // Parse date from filename: kuiperdb.log.2026-02-04
                    let date_str = &name[10..]; // Skip "kuiperdb.log."

                    if let Ok(file_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        if file_date < cutoff_date {
                            if let Ok(metadata) = entry.metadata() {
                                deleted_size += metadata.len();
                                deleted_files.push(name.to_string());
                                let _ = fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "deleted_files": deleted_files,
        "deleted_count": deleted_files.len(),
        "deleted_size": deleted_size,
        "days_to_keep": days_to_keep,
    })))
}

/// Configure routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/db")
            .route("", web::get().to(list_databases))
            // Specific named routes MUST come before generic patterns
            // Relations endpoints
            .route("/{db_name}/relations", web::post().to(create_relation))
            .route(
                "/{db_name}/relations/{relation_id}",
                web::get().to(get_relation),
            )
            .route(
                "/{db_name}/relations/{relation_id}",
                web::delete().to(delete_relation),
            )
            .route(
                "/{db_name}/documents/{doc_id}/relations",
                web::get().to(get_document_relations),
            )
            // Graph endpoints
            .route("/{db_name}/graph/traverse", web::post().to(graph_traverse))
            .route("/{db_name}/graph/path", web::get().to(graph_shortest_path))
            .route("/{db_name}/graph/stats", web::get().to(graph_statistics))
            // Table-specific routes
            .route("/{db_name}/tables", web::get().to(list_tables))
            .route("/{db_name}/{table_name}/documents", web::get().to(list_documents))
            .route("/{db_name}/{table_name}/search", web::post().to(search))
            .route(
                "/{db_name}/{table_name}/{doc_id}/chunks",
                web::get().to(get_chunks),
            )
            .route(
                "/{db_name}/{table_name}/{doc_id}/rechunk",
                web::post().to(rechunk_document),
            )
            .route(
                "/{db_name}/{table_name}/{doc_id}",
                web::get().to(get_document),
            )
            .route(
                "/{db_name}/{table_name}/{doc_id}",
                web::delete().to(delete_document),
            )
            .route("/{db_name}/{table_name}", web::post().to(store_document))
            .route(
                "/{db_name}/{table_name}",
                web::delete().to(delete_table),
            )
            // Database deletion - MUST be last for /{db_name}
            .route(
                "/{db_name}",
                web::delete().to(delete_database),
            ),
    )
    .service(
        web::scope("/logs")
            .route("", web::get().to(list_logs))
            .route("/analyze/{date}", web::get().to(analyze_logs))
            .route("/cleanup", web::post().to(cleanup_logs))
            .route("/{filename}", web::get().to(view_log)),
    )
    .route("/health", web::get().to(health));
}
