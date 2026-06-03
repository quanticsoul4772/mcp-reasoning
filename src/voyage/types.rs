//! Request and response types for the Voyage AI REST API.
//!
//! Covers the `/embeddings`, `/rerank`, and `/contextualizedembeddings` endpoints.

use serde::{Deserialize, Serialize};

/// Default Voyage API base URL.
pub const DEFAULT_VOYAGE_BASE_URL: &str = "https://api.voyageai.com/v1";
/// Default embedding model.
pub const DEFAULT_VOYAGE_MODEL: &str = "voyage-4";
/// Default reranking model.
pub const DEFAULT_RERANK_MODEL: &str = "rerank-2.5";
/// Default contextualized-chunk embedding model.
pub const DEFAULT_CONTEXT_MODEL: &str = "voyage-context-3";

/// Request body for `POST /embeddings`.
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingRequest {
    /// Texts to embed (max 1000 per request).
    pub input: Vec<String>,
    /// Embedding model name.
    pub model: String,
    /// Retrieval role: `"query"` or `"document"` (asymmetric embeddings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>,
    /// Optional reduced output dimension (256/512/1024/2048).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimension: Option<u32>,
    /// Optional output dtype (`"float"`, `"int8"`, …) for quantization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dtype: Option<String>,
}

/// Response body for `POST /embeddings`.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResponse {
    /// One entry per input, each with its vector and original index.
    pub data: Vec<EmbeddingData>,
    /// Model that produced the embeddings.
    pub model: String,
    /// Token accounting.
    #[serde(default)]
    pub usage: Usage,
}

/// A single embedding result.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingData {
    /// The embedding vector.
    pub embedding: Vec<f32>,
    /// Index into the original `input` array.
    pub index: usize,
}

/// Token usage reported by the API.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Usage {
    /// Total tokens consumed by the request.
    #[serde(default)]
    pub total_tokens: u64,
}

/// Request body for `POST /rerank`.
#[derive(Debug, Clone, Serialize)]
pub struct RerankRequest {
    /// The query to rank documents against.
    pub query: String,
    /// Candidate documents (max 1000).
    pub documents: Vec<String>,
    /// Reranking model name.
    pub model: String,
    /// Optional cap on the number of results returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
}

/// Response body for `POST /rerank`.
#[derive(Debug, Clone, Deserialize)]
pub struct RerankResponse {
    /// Results sorted by descending relevance.
    pub data: Vec<RerankResult>,
    /// Model that produced the scores.
    pub model: String,
    /// Token accounting.
    #[serde(default)]
    pub usage: Usage,
}

/// A single rerank result.
#[derive(Debug, Clone, Deserialize)]
pub struct RerankResult {
    /// Index into the original `documents` array.
    pub index: usize,
    /// Cross-encoder relevance score (higher = more relevant).
    pub relevance_score: f64,
}

/// Request body for `POST /contextualizedembeddings`.
///
/// `inputs` is a list of documents, each a list of ordered chunks; the model
/// embeds the chunks of a document together so each chunk's vector carries
/// context from its siblings.
#[derive(Debug, Clone, Serialize)]
pub struct ContextualizedRequest {
    /// Documents, each split into ordered chunks.
    pub inputs: Vec<Vec<String>>,
    /// Contextualized model name.
    pub model: String,
    /// Retrieval role: `"query"` or `"document"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>,
    /// Optional reduced output dimension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimension: Option<u32>,
    /// Optional output dtype for quantization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dtype: Option<String>,
}

/// Response body for `POST /contextualizedembeddings`.
///
/// `data` has one entry per input document, each holding that document's
/// per-chunk embeddings.
#[derive(Debug, Clone, Deserialize)]
pub struct ContextualizedResponse {
    /// One entry per input document.
    pub data: Vec<ContextualizedDoc>,
    /// Model that produced the embeddings.
    pub model: String,
    /// Token accounting.
    #[serde(default)]
    pub usage: Usage,
}

/// Per-document contextualized embeddings.
#[derive(Debug, Clone, Deserialize)]
pub struct ContextualizedDoc {
    /// Per-chunk embeddings for this document.
    pub data: Vec<EmbeddingData>,
    /// Index into the original `inputs` array.
    pub index: usize,
}
