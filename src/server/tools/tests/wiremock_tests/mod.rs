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
