use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use tracing::{error, info};

use crate::config::Config;
use crate::embedder::{Embedder, OpenAIEmbedder};
use crate::store::DocumentStore;

/// Background worker that processes non-embedded documents
pub struct BackgroundWorker {
    store: Arc<Mutex<DocumentStore>>,
    embedder: Arc<OpenAIEmbedder>,
    config: Arc<Config>,
    shutdown: Arc<tokio::sync::Notify>,
}

impl BackgroundWorker {
    pub fn new(
        store: Arc<Mutex<DocumentStore>>,
        embedder: Arc<OpenAIEmbedder>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            store,
            embedder,
            config,
            shutdown: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Start the background worker
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Background embedding worker started");

            let mut interval = time::interval(Duration::from_secs(10));
            interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = self.shutdown.notified() => {
                        info!("Background embedding worker stopped");
                        break;
                    }
                    _ = interval.tick() => {
                        if let Err(e) = self.process_non_embedded_documents().await {
                            error!("Error processing non-embedded documents: {}", e);
                        }
                    }
                }
            }
        })
    }

    /// Stop the background worker
    pub fn stop(&self) {
        self.shutdown.notify_one();
    }

    /// Process non-embedded documents across all databases and tables
    async fn process_non_embedded_documents(&self) -> anyhow::Result<()> {
        info!("Embedding worker: checking for non-embedded documents...");

        // Get all databases
        let databases = {
            let store = self.store.lock().await;
            store.list_databases().await?
        };

        info!(
            "Embedding worker: found {} databases to check",
            databases.len()
        );

        let max_documents = self.config.num_embedding_workers as i32;
        let mut total_processed = 0;

        // Process each database
        for db_name in databases {
            if total_processed >= max_documents {
                break;
            }

            // Get all tables in this database
            let tables = {
                let mut store = self.store.lock().await;
                store.list_tables(&db_name).await?
            };

            info!(
                "Embedding worker: found {} tables in database '{}'",
                tables.len(),
                db_name
            );

            // Process each table
            for table_name in tables {
                if total_processed >= max_documents {
                    break;
                }

                let remaining = max_documents - total_processed;
                info!(
                    "Embedding worker: checking table '{}.{}' for up to {} documents",
                    db_name, table_name, remaining
                );

                // Get non-embedded documents
                let docs = {
                    let mut store = self.store.lock().await;
                    store
                        .get_non_embedded_documents(&db_name, &table_name, remaining)
                        .await?
                };

                if docs.is_empty() {
                    continue;
                }

                info!(
                    "Embedding worker: found {} non-embedded documents in table '{}.{}'",
                    docs.len(),
                    db_name,
                    table_name
                );

                // Process documents in parallel batches
                let batch_size = 4; // Optimal from benchmarks
                let num_workers = self.config.num_embedding_workers.min(docs.len());

                let mut handles = vec![];
                for chunk in docs.chunks(batch_size) {
                    if handles.len() >= num_workers {
                        // Wait for a worker to finish
                        if let Some(handle) = handles.pop() {
                            handle.await??;
                        }
                    }

                    let store = self.store.clone();
                    let embedder = self.embedder.clone();
                    let db_name = db_name.clone();
                    let table_name = table_name.clone();
                    let chunk = chunk.to_vec();

                    let handle = tokio::spawn(async move {
                        // Embed all documents in this batch
                        let mut vectors = Vec::new();
                        for doc in &chunk {
                            let vector = embedder.embed(&doc.content).await?;
                            vectors.push(vector);
                        }

                        // Update each document with its vector
                        for (doc, vector) in chunk.iter().zip(vectors.iter()) {
                            let mut store = store.lock().await;
                            store
                                .update_document_vector(&db_name, &table_name, &doc.id, vector)
                                .await?;
                        }

                        Ok::<usize, anyhow::Error>(chunk.len())
                    });

                    handles.push(handle);
                }

                // Wait for remaining workers
                for handle in handles {
                    let processed = handle.await??;
                    total_processed += processed as i32;
                }
            }
        }

        if total_processed > 0 {
            info!(
                "Background embedding: processed {} documents",
                total_processed
            );
        } else {
            info!("Embedding worker: no documents to process");
        }

        Ok(())
    }
}

impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        self.stop();
    }
}
