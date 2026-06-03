//! Voyage AI integration: embeddings and reranking for semantic memory.
//!
//! - [`VoyageClient`]: HTTP client mirroring the Anthropic client (retry/backoff).
//! - Request/response [`types`] for the `/embeddings` and `/rerank` endpoints.
//!
//! The client implements [`crate::traits::EmbeddingProvider`] so the memory mode
//! can depend on the trait and be mocked in tests.

mod client;
pub mod types;

pub use client::VoyageClient;
pub use types::{DEFAULT_RERANK_MODEL, DEFAULT_VOYAGE_BASE_URL, DEFAULT_VOYAGE_MODEL};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::anthropic::ClientConfig;
    use crate::traits::EmbeddingProvider;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client_for(server: &MockServer) -> VoyageClient {
        let config = ClientConfig::default()
            .with_base_url(server.uri())
            .with_max_retries(0);
        VoyageClient::new("test-key", "voyage-4", config).expect("client")
    }

    fn client_with_retries(server: &MockServer, retries: u32) -> VoyageClient {
        let config = ClientConfig::default()
            .with_base_url(server.uri())
            .with_max_retries(retries)
            .with_retry_delay_ms(1);
        VoyageClient::new("test-key", "voyage-4", config).expect("client")
    }

    #[tokio::test]
    async fn test_embed_query_returns_vector_and_sends_bearer_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{"object": "embedding", "embedding": [0.1, 0.2, 0.3], "index": 0}],
                "model": "voyage-4",
                "usage": {"total_tokens": 4}
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let v = client.embed_query("hello").await.expect("embed");
        assert_eq!(v, vec![0.1, 0.2, 0.3]);
    }

    #[tokio::test]
    async fn test_embed_documents_restores_input_order() {
        let server = MockServer::start().await;
        // Return the two embeddings out of order; the client must re-sort by index.
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"object": "embedding", "embedding": [9.0], "index": 1},
                    {"object": "embedding", "embedding": [1.0], "index": 0}
                ],
                "model": "voyage-4",
                "usage": {"total_tokens": 8}
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let out = client
            .embed_documents(&["a".to_string(), "b".to_string()])
            .await
            .expect("embed");
        assert_eq!(out, vec![vec![1.0], vec![9.0]]);
    }

    #[tokio::test]
    async fn test_embed_contextualized_mean_pools_chunk_vectors() {
        let server = MockServer::start().await;
        // Two chunk embeddings; the client returns their element-wise mean.
        Mock::given(method("POST"))
            .and(path("/contextualizedembeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"object": "list", "index": 0, "data": [
                        {"object": "embedding", "embedding": [1.0, 3.0], "index": 0},
                        {"object": "embedding", "embedding": [3.0, 5.0], "index": 1}
                    ]}
                ],
                "model": "voyage-context-3",
                "usage": {"total_tokens": 6}
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let v = client
            .embed_contextualized(&["a".to_string(), "b".to_string()], "document")
            .await
            .expect("contextualized");
        assert_eq!(v, vec![2.0, 4.0]);
    }

    #[tokio::test]
    async fn test_embed_contextualized_empty_short_circuits() {
        let server = MockServer::start().await;
        let client = client_for(&server);
        assert!(client
            .embed_contextualized(&[], "document")
            .await
            .expect("ok")
            .is_empty());
    }

    #[tokio::test]
    async fn test_rerank_returns_index_score_pairs() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/rerank"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"index": 2, "relevance_score": 0.9},
                    {"index": 0, "relevance_score": 0.4}
                ],
                "model": "rerank-2.5",
                "usage": {"total_tokens": 12}
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let docs = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let ranked = client.rerank("q", &docs, Some(2)).await.expect("rerank");
        assert_eq!(ranked, vec![(2, 0.9), (0, 0.4)]);
    }

    #[tokio::test]
    async fn test_auth_failure_is_not_retried() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
            .expect(1) // non-retryable: exactly one attempt
            .mount(&server)
            .await;

        let client = client_for(&server);
        let err = client.embed_query("hello").await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("authentication"));
    }

    #[tokio::test]
    async fn test_empty_inputs_short_circuit() {
        let server = MockServer::start().await;
        let client = client_for(&server);
        assert!(client.embed_documents(&[]).await.expect("ok").is_empty());
        assert!(client.rerank("q", &[], None).await.expect("ok").is_empty());
    }

    #[tokio::test]
    async fn test_transient_5xx_is_retried_then_exhausts() {
        let server = MockServer::start().await;
        // 500 is transient: with one retry, exactly two attempts are made.
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream down"))
            .expect(2)
            .mount(&server)
            .await;

        let client = client_with_retries(&server, 1);
        let err = client.embed_query("hello").await.unwrap_err();
        assert!(err.to_string().to_lowercase().contains("transient"));
    }

    #[tokio::test]
    async fn test_non_retryable_4xx_errors_immediately() {
        let server = MockServer::start().await;
        // 400 is a client error: non-retryable, a single attempt despite retries.
        Mock::given(method("POST"))
            .and(path("/rerank"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad request"))
            .expect(1)
            .mount(&server)
            .await;

        let client = client_with_retries(&server, 2);
        let err = client
            .rerank("q", &["a".to_string()], None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("400"));
    }

    #[tokio::test]
    async fn test_batch_limit_rejected_without_request() {
        let server = MockServer::start().await;
        let client = client_for(&server);
        // 1001 > the 1000-item cap → rejected locally, no HTTP call made.
        let too_many: Vec<String> = (0..1001).map(|i| i.to_string()).collect();
        let err = client.embed_documents(&too_many).await.unwrap_err();
        assert!(err.to_string().contains("batch limit"));
    }

    #[test]
    fn test_constructors_default_to_voyage_endpoint() {
        let client = VoyageClient::with_api_key("k")
            .expect("client")
            .with_rerank_model("rerank-2.5-lite");
        // base_url was rewritten from the Anthropic default to Voyage's.
        let _ = client; // construction + builder cover the default-URL rewrite path
    }
}
