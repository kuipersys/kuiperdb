use crate::{ClientError, Result};
use kuiperdb_core::{
    IndexConfig, Record, RecordInput, Relation, RelationInput, SearchResult, VectorQuery,
    VectorSpace,
};
use reqwest::{Client as HttpClient, Response};

pub struct Client {
    base_url: String,
    client: HttpClient,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            client: HttpClient::new(),
        }
    }

    pub async fn create_vector_space(&self, space: &VectorSpace) -> Result<()> {
        check(
            self.client
                .post(format!("{}/spaces", self.base_url))
                .json(space)
                .send()
                .await?,
        )
        .await?;
        Ok(())
    }

    pub async fn vector_spaces(&self) -> Result<Vec<VectorSpace>> {
        Ok(check(
            self.client
                .get(format!("{}/spaces", self.base_url))
                .send()
                .await?,
        )
        .await?
        .json()
        .await?)
    }

    pub async fn update_index_config(&self, space: &str, config: &IndexConfig) -> Result<()> {
        check(
            self.client
                .put(format!("{}/spaces/{space}/index", self.base_url))
                .json(config)
                .send()
                .await?,
        )
        .await?;
        Ok(())
    }

    pub async fn put_record(&self, input: &RecordInput) -> Result<Record> {
        Ok(check(
            self.client
                .post(format!("{}/records", self.base_url))
                .json(input)
                .send()
                .await?,
        )
        .await?
        .json()
        .await?)
    }

    pub async fn put_records(&self, inputs: &[RecordInput]) -> Result<Vec<Record>> {
        Ok(check(
            self.client
                .post(format!("{}/records/bulk", self.base_url))
                .json(inputs)
                .send()
                .await?,
        )
        .await?
        .json()
        .await?)
    }

    pub async fn get_record(&self, id: &str) -> Result<Option<Record>> {
        let response = self
            .client
            .get(format!("{}/records/{id}", self.base_url))
            .send()
            .await?;
        if response.status().as_u16() == 404 {
            return Ok(None);
        }
        Ok(Some(check(response).await?.json().await?))
    }

    pub async fn delete_record(&self, id: &str) -> Result<bool> {
        let response = self
            .client
            .delete(format!("{}/records/{id}", self.base_url))
            .send()
            .await?;
        if response.status().as_u16() == 404 {
            return Ok(false);
        }
        check(response).await?;
        Ok(true)
    }

    pub async fn search(&self, query: &VectorQuery) -> Result<Vec<SearchResult>> {
        Ok(check(
            self.client
                .post(format!("{}/search", self.base_url))
                .json(query)
                .send()
                .await?,
        )
        .await?
        .json()
        .await?)
    }

    pub async fn create_relation(&self, input: &RelationInput) -> Result<Relation> {
        Ok(check(
            self.client
                .post(format!("{}/relations", self.base_url))
                .json(input)
                .send()
                .await?,
        )
        .await?
        .json()
        .await?)
    }

    pub async fn relations_for(&self, record_id: &str) -> Result<Vec<Relation>> {
        Ok(check(
            self.client
                .get(format!("{}/records/{record_id}/relations", self.base_url))
                .send()
                .await?,
        )
        .await?
        .json()
        .await?)
    }

    pub async fn health(&self) -> Result<()> {
        check(
            self.client
                .get(format!("{}/health", self.base_url))
                .send()
                .await?,
        )
        .await?;
        Ok(())
    }
}

async fn check(response: Response) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    Err(ClientError::Server {
        status: response.status().as_u16(),
        message: response.text().await.unwrap_or_default(),
    })
}
