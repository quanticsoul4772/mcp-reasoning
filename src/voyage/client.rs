//! Voyage AI HTTP client for embeddings and reranking.
//!
//! Mirrors [`crate::anthropic::AnthropicClient`]: a `reqwest` client with
//! exponential-backoff retry over a reused [`ClientConfig`]. The API key is
//! supplied by the caller (from `Config`/`SecretString`) and never logged.

#![allow(clippy::missing_errors_doc)]

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use super::types::{
    ContextualizedRequest, ContextualizedResponse, EmbeddingRequest, EmbeddingResponse,
    RerankRequest, RerankResponse, DEFAULT_CONTEXT_MODEL, DEFAULT_RERANK_MODEL,
    DEFAULT_VOYAGE_BASE_URL, DEFAULT_VOYAGE_MODEL,
};
use crate::anthropic::ClientConfig;
use crate::error::ModeError;
use crate::traits::EmbeddingProvider;

/// Maximum inputs/documents per Voyage request.
const MAX_BATCH: usize = 1000;

/// Client for the Voyage AI embeddings + reranking API.
#[derive(Debug)]
pub struct VoyageClient {
    client: Client,
    api_key: String,
    config: ClientConfig,
    model: String,
    rerank_model: String,
    context_model: String,
}

impl VoyageClient {
    /// Create a new Voyage client for `model`, using `config` for timeout/retry.
    ///
    /// The `base_url` on `config` defaults to the Anthropic URL; this overrides
    /// it to the Voyage endpoint unless the caller already set a Voyage URL.
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        config: ClientConfig,
    ) -> Result<Self, ModeError> {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to create Voyage HTTP client: {e}"),
            })?;
        // Point at Voyage unless the caller deliberately set a non-Anthropic URL.
        let mut config = config;
        if config.base_url == crate::anthropic::DEFAULT_BASE_URL {
            config.base_url = DEFAULT_VOYAGE_BASE_URL.to_string();
        }
        Ok(Self {
            client,
            api_key: api_key.into(),
            config,
            model: model.into(),
            rerank_model: DEFAULT_RERANK_MODEL.to_string(),
            context_model: DEFAULT_CONTEXT_MODEL.to_string(),
        })
    }

    /// Override the contextualized-embedding model (default `voyage-context-3`).
    #[must_use]
    pub fn with_context_model(mut self, model: impl Into<String>) -> Self {
        self.context_model = model.into();
        self
    }

    /// The contextualized-embedding model name.
    #[must_use]
    pub fn context_model(&self) -> &str {
        &self.context_model
    }

    /// Embed one document's ordered `chunks` with the contextualized model,
    /// returning the **mean-pooled** document vector (one vector summarizing the
    /// whole, with each chunk aware of its siblings).
    pub async fn embed_contextualized(
        &self,
        chunks: &[String],
        input_type: &str,
    ) -> Result<Vec<f32>, ModeError> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }
        let request = ContextualizedRequest {
            inputs: vec![chunks.to_vec()],
            model: self.context_model.clone(),
            input_type: Some(input_type.to_string()),
            output_dimension: None,
            output_dtype: None,
        };
        let resp: ContextualizedResponse = self
            .post_with_retry("contextualizedembeddings", &request)
            .await?;
        let doc = resp
            .data
            .into_iter()
            .next()
            .ok_or_else(|| ModeError::ParseError {
                message: "Voyage returned no contextualized document".to_string(),
            })?;
        mean_pool(doc.data.into_iter().map(|d| d.embedding)).ok_or_else(|| ModeError::ParseError {
            message: "Voyage returned no contextualized chunk embeddings".to_string(),
        })
    }

    /// Create a client with default config and the default embedding model.
    pub fn with_api_key(api_key: impl Into<String>) -> Result<Self, ModeError> {
        Self::new(api_key, DEFAULT_VOYAGE_MODEL, ClientConfig::default())
    }

    /// Override the reranking model (default `rerank-2.5`).
    #[must_use]
    pub fn with_rerank_model(mut self, model: impl Into<String>) -> Self {
        self.rerank_model = model.into();
        self
    }

    /// Embed a batch of texts with the given retrieval role.
    ///
    /// Returns one vector per input, ordered to match `inputs`.
    pub async fn embed(
        &self,
        inputs: Vec<String>,
        input_type: Option<&str>,
    ) -> Result<Vec<Vec<f32>>, ModeError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        if inputs.len() > MAX_BATCH {
            return Err(ModeError::InvalidValue {
                field: "input".to_string(),
                reason: format!("{} exceeds the {MAX_BATCH}-item batch limit", inputs.len()),
            });
        }
        let request = EmbeddingRequest {
            input: inputs,
            model: self.model.clone(),
            input_type: input_type.map(str::to_string),
            output_dimension: None,
            output_dtype: None,
        };
        let resp: EmbeddingResponse = self.post_with_retry("embeddings", &request).await?;
        // Restore caller order by the API-provided index.
        let mut indexed: Vec<(usize, Vec<f32>)> = resp
            .data
            .into_iter()
            .map(|d| (d.index, d.embedding))
            .collect();
        indexed.sort_by_key(|(i, _)| *i);
        Ok(indexed.into_iter().map(|(_, v)| v).collect())
    }

    /// Rerank `documents` against `query`, returning `(original_index, score)`
    /// pairs sorted by descending relevance.
    pub async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_k: Option<u32>,
    ) -> Result<Vec<(usize, f64)>, ModeError> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }
        let request = RerankRequest {
            query: query.to_string(),
            documents: documents.to_vec(),
            model: self.rerank_model.clone(),
            top_k,
        };
        let resp: RerankResponse = self.post_with_retry("rerank", &request).await?;
        Ok(resp
            .data
            .into_iter()
            .map(|r| (r.index, r.relevance_score))
            .collect())
    }

    /// POST `body` to `/{path}` with exponential-backoff retry on transient errors.
    async fn post_with_retry<B: serde::Serialize + Sync, R: serde::de::DeserializeOwned + Send>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R, ModeError> {
        let mut last_error = None;
        let mut delay = self.config.retry_delay_ms;
        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(delay)).await;
                delay *= 2;
            }
            match self.post_once(path, body).await {
                Ok(r) => return Ok(r),
                Err((e, retryable)) => {
                    if !retryable {
                        return Err(e);
                    }
                    tracing::warn!(error = %e, attempt, "Retryable Voyage error");
                    last_error = Some(e);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| ModeError::ApiUnavailable {
            message: "Unknown Voyage error after retries".to_string(),
        }))
    }

    /// A single POST attempt. The bool in the error indicates retryability.
    async fn post_once<B: serde::Serialize + Sync, R: serde::de::DeserializeOwned + Send>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R, (ModeError, bool)> {
        let url = format!("{}/{path}", self.config.base_url);
        let response = self
            .client
            .post(&url)
            .header("authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    (
                        ModeError::Timeout {
                            elapsed_ms: self.config.timeout_ms,
                        },
                        true,
                    )
                } else {
                    (
                        ModeError::ApiUnavailable {
                            message: format!("Voyage request failed: {e}"),
                        },
                        true,
                    )
                }
            })?;

        let status = response.status();
        if status.as_u16() == 401 {
            return Err((
                ModeError::ApiError {
                    message: "Voyage authentication failed (check VOYAGE_API_KEY)".to_string(),
                },
                false,
            ));
        }
        if status.as_u16() == 429 || status.as_u16() >= 500 {
            let body = response.text().await.unwrap_or_default();
            return Err((
                ModeError::ApiUnavailable {
                    message: format!("Voyage transient error {status}: {body}"),
                },
                true,
            ));
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err((
                ModeError::ApiError {
                    message: format!("Voyage error {status}: {body}"),
                },
                false,
            ));
        }
        response.json::<R>().await.map_err(|e| {
            (
                ModeError::ParseError {
                    message: format!("Failed to parse Voyage response: {e}"),
                },
                false,
            )
        })
    }
}

/// Element-wise mean of a set of equal-length vectors. Returns `None` if there
/// are no vectors; ignores the degenerate empty-vector case.
fn mean_pool(vectors: impl Iterator<Item = Vec<f32>>) -> Option<Vec<f32>> {
    let mut sum: Vec<f32> = Vec::new();
    let mut count = 0usize;
    for v in vectors {
        if sum.is_empty() {
            sum = v;
        } else if sum.len() == v.len() {
            for (s, x) in sum.iter_mut().zip(v.iter()) {
                *s += x;
            }
        } else {
            continue; // skip mismatched lengths defensively
        }
        count += 1;
    }
    if count == 0 || sum.is_empty() {
        return None;
    }
    let n = count as f32;
    for s in &mut sum {
        *s /= n;
    }
    Some(sum)
}

#[async_trait]
impl EmbeddingProvider for VoyageClient {
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, ModeError> {
        let mut out = self.embed(vec![text.to_string()], Some("query")).await?;
        out.pop().ok_or_else(|| ModeError::ParseError {
            message: "Voyage returned no embedding for query".to_string(),
        })
    }

    async fn embed_contextualized(
        &self,
        chunks: &[String],
        input_type: &str,
    ) -> Result<Vec<f32>, ModeError> {
        Self::embed_contextualized(self, chunks, input_type).await
    }

    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, ModeError> {
        self.embed(texts.to_vec(), Some("document")).await
    }

    async fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_k: Option<u32>,
    ) -> Result<Vec<(usize, f64)>, ModeError> {
        Self::rerank(self, query, documents, top_k).await
    }
}
