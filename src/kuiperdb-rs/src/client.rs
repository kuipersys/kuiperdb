use crate::{ClientError, Result};
use kuiperdb_core::{Document, SearchResult};
use reqwest::Client as HttpClient;
use serde::Serialize;

/// KuiperDb REST API Client
pub struct Client {
    base_url: String,
    client: HttpClient,
}

#[derive(Serialize)]
struct AddDocumentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vectorize: Option<bool>,
}

#[derive(Serialize)]
struct SearchRequest {
    query: String,
    limit: usize,
}

impl Client {
    /// Create a new client connected to the given base URL
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: HttpClient::new(),
        }
    }

    /// Add a document to the database
    pub async fn add_document(
        &self,
        content: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<String> {
        self.add_document_with_options(None, content, metadata, None, None)
            .await
    }

    /// Add a document with full options including custom ID
    pub async fn add_document_with_options(
        &self,
        id: Option<String>,
        content: String,
        metadata: Option<serde_json::Value>,
        tags: Option<Vec<String>>,
        vectorize: Option<bool>,
    ) -> Result<String> {
        let url = format!("{}/documents", self.base_url);
        let req = AddDocumentRequest {
            id,
            content,
            metadata,
            tags,
            vectorize,
        };

        let response = self.client.post(&url).json(&req).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::Server {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let doc: Document = response.json().await?;
        Ok(doc.id)
    }

    /// Get a document by ID
    pub async fn get_document(&self, id: impl AsRef<str>) -> Result<Option<Document>> {
        let url = format!("{}/documents/{}", self.base_url, id.as_ref());

        let response = self.client.get(&url).send().await?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(ClientError::Server {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let doc: Document = response.json().await?;
        Ok(Some(doc))
    }

    /// Search for documents
    pub async fn search(&self, query: String, limit: usize) -> Result<Vec<SearchResult>> {
        let url = format!("{}/search", self.base_url);
        let req = SearchRequest { query, limit };

        let response = self.client.post(&url).json(&req).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::Server {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let results: Vec<SearchResult> = response.json().await?;
        Ok(results)
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, id: impl AsRef<str>) -> Result<()> {
        let url = format!("{}/documents/{}", self.base_url, id.as_ref());

        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::Server {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }

    /// Health check
    pub async fn health(&self) -> Result<()> {
        let url = format!("{}/health", self.base_url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::Server {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }
}
