use std::sync::Arc;

use wiremock::MockServer;

use crate::server::tools::ReasoningServer;
use crate::server::types::AppState;

mod analysis;
mod basic_coverage;
mod confidence;
mod core_tools;
mod detect_extra;
mod extended;
mod graph_coverage;
mod handler_success;
mod more_tests;
mod streaming_coverage;
mod temporal;
mod temporal_coverage;

// ============================================================================
// Shared Wiremock Helpers
// ============================================================================

fn anthropic_response(text: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "msg_test_123",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": text}],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 100, "output_tokens": 50}
    })
}

/// Build an Anthropic SSE (`text/event-stream`) body that streams `text` as a
/// single text delta. Use with `ResponseTemplate::set_body_string` to exercise
/// the handlers that call `complete_streaming` (mcts, counterfactual).
fn anthropic_sse_response(text: &str) -> String {
    let start = serde_json::json!({
        "type": "message_start",
        "message": {"id": "msg_test", "stop_reason": null}
    });
    let delta = serde_json::json!({
        "type": "content_block_delta",
        "index": 0,
        "delta": {"type": "text_delta", "text": text}
    });
    // The accumulator only flushes the text buffer on content_block_stop.
    let block_stop = serde_json::json!({"type": "content_block_stop", "index": 0});
    // message_stop carries no `message` object (RawMessage.id is required).
    let stop = serde_json::json!({"type": "message_stop"});
    format!("data: {start}\n\ndata: {delta}\n\ndata: {block_stop}\n\ndata: {stop}\n\n")
}

async fn create_mocked_server(mock_server: &MockServer) -> ReasoningServer {
    use crate::anthropic::{AnthropicClient, ClientConfig};
    use crate::config::{Config, SecretString};
    use crate::metrics::MetricsCollector;
    use crate::storage::SqliteStorage;

    let config = Config {
        api_key: SecretString::new("test-key"),
        database_path: ":memory:".to_string(),
        log_level: "info".to_string(),
        request_timeout_ms: 5000,
        request_timeout_deep_ms: 60000,
        request_timeout_maximum_ms: 120000,
        factory_timeout_ms: 30000,
        max_retries: 0,
        model: "claude-sonnet-4-20250514".to_string(),
        voyage_api_key: None,
        voyage_model: "voyage-4".to_string(),
        high_confidence_threshold: 0.75,
        reflection_quality_threshold: 0.8,
        mcts_quality_threshold: 0.5,
        graph_prune_threshold: 0.3,
    };

    let storage = SqliteStorage::new_in_memory().await.unwrap();
    let metrics = Arc::new(MetricsCollector::new());
    let si_handle = super::create_test_si_handle(&storage, metrics.clone());
    let client_config = ClientConfig::default()
        .with_base_url(mock_server.uri())
        .with_max_retries(0)
        .with_timeout_ms(5000);
    let client = AnthropicClient::new("test-key", client_config).unwrap();
    let metadata_builder = crate::metadata::MetadataBuilder::new(
        Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
            storage.clone(),
        ))),
        Arc::new(crate::metadata::PresetIndex::build()),
        30000,
    );
    let (progress_tx, _rx) = tokio::sync::broadcast::channel(100);
    let state = AppState::new(
        storage,
        client,
        config,
        metrics,
        si_handle,
        metadata_builder,
        progress_tx,
    );
    ReasoningServer::new(Arc::new(state))
}
