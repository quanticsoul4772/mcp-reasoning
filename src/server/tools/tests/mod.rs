use std::sync::Arc;

use rmcp::handler::server::ServerHandler;

use crate::server::requests::*;
use crate::server::responses::*;
use crate::server::tools::ReasoningServer;
use crate::server::types::AppState;

mod handler_gaps;
mod into_contents;
mod request_deser;
mod tool_coverage;
mod tool_methods;
mod tool_operations;
mod wiremock_tests;

// ============================================================================
// Shared Test Helpers
// ============================================================================

fn create_test_si_handle(
    storage: &crate::storage::SqliteStorage,
    metrics: Arc<crate::metrics::MetricsCollector>,
) -> crate::self_improvement::ManagerHandle {
    use crate::config::SelfImprovementConfig;
    use crate::self_improvement::{SelfImprovementManager, SelfImprovementStorage};
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

    let mut client = MockAnthropicClientTrait::new();
    client.expect_complete().returning(|_, _| {
        Ok(CompletionResponse::new(
            r#"{"summary": "Test", "confidence": 0.8, "actions": []}"#,
            Usage::new(100, 50),
        ))
    });

    let si_storage = Arc::new(SelfImprovementStorage::new(storage.pool.clone()));

    let (_manager, handle) = SelfImprovementManager::new(
        SelfImprovementConfig::default(),
        client,
        metrics,
        si_storage,
    );
    handle
}

fn create_test_server_sync() -> ReasoningServer {
    use crate::anthropic::{AnthropicClient, ClientConfig};
    use crate::config::{Config, SecretString};
    use crate::metrics::MetricsCollector;
    use crate::storage::SqliteStorage;

    let config = Config {
        api_key: SecretString::new("test-key"),
        database_path: ":memory:".to_string(),
        log_level: "info".to_string(),
        request_timeout_ms: 30000,
        request_timeout_deep_ms: 60000,
        request_timeout_maximum_ms: 120000,
        factory_timeout_ms: 30000,
        max_retries: 3,
        model: "claude-sonnet-4-20250514".to_string(),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let storage = rt.block_on(async { SqliteStorage::new_in_memory().await.unwrap() });

    let metrics = Arc::new(MetricsCollector::new());
    let si_handle = create_test_si_handle(&storage, metrics.clone());
    let client = AnthropicClient::new("test-key", ClientConfig::default()).unwrap();
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

async fn create_test_server() -> ReasoningServer {
    use crate::anthropic::{AnthropicClient, ClientConfig};
    use crate::config::{Config, SecretString};
    use crate::metrics::MetricsCollector;
    use crate::storage::SqliteStorage;

    let config = Config {
        api_key: SecretString::new("test-key"),
        database_path: ":memory:".to_string(),
        log_level: "info".to_string(),
        request_timeout_ms: 30000,
        request_timeout_deep_ms: 60000,
        request_timeout_maximum_ms: 120000,
        factory_timeout_ms: 30000,
        max_retries: 3,
        model: "claude-sonnet-4-20250514".to_string(),
    };

    let storage = SqliteStorage::new_in_memory().await.unwrap();

    let metrics = Arc::new(MetricsCollector::new());
    let si_handle = create_test_si_handle(&storage, metrics.clone());
    let client = AnthropicClient::new("test-key", ClientConfig::default()).unwrap();
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

// ============================================================================
// Basic Serialization & Schema Tests
// ============================================================================

#[test]
fn test_linear_response_serialize() {
    let response = LinearResponse {
        thought_id: "t1".to_string(),
        session_id: "s1".to_string(),
        content: "reasoning content".to_string(),
        confidence: 0.85,
        next_step: Some("continue".to_string()),
        metadata: None,
        next_call: None,
    };
    let json = serde_json::to_string(&response).expect("serialize");
    assert!(json.contains("thought_id"));
}

#[test]
fn test_linear_request_deserialize() {
    let json = r#"{"content": "test"}"#;
    let req: LinearRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.content, "test");
}

#[test]
fn test_all_response_types_implement_json_schema() {
    let _ = schemars::schema_for!(LinearResponse);
    let _ = schemars::schema_for!(TreeResponse);
    let _ = schemars::schema_for!(DivergentResponse);
    let _ = schemars::schema_for!(ReflectionResponse);
    let _ = schemars::schema_for!(CheckpointResponse);
    let _ = schemars::schema_for!(AutoResponse);
    let _ = schemars::schema_for!(GraphResponse);
    let _ = schemars::schema_for!(DetectResponse);
    let _ = schemars::schema_for!(DecisionResponse);
    let _ = schemars::schema_for!(EvidenceResponse);
    let _ = schemars::schema_for!(TimelineResponse);
    let _ = schemars::schema_for!(MctsResponse);
    let _ = schemars::schema_for!(CounterfactualResponse);
    let _ = schemars::schema_for!(PresetResponse);
    let _ = schemars::schema_for!(MetricsResponse);
}

#[test]
fn test_all_request_types_implement_json_schema() {
    let _ = schemars::schema_for!(LinearRequest);
    let _ = schemars::schema_for!(TreeRequest);
    let _ = schemars::schema_for!(DivergentRequest);
    let _ = schemars::schema_for!(ReflectionRequest);
    let _ = schemars::schema_for!(CheckpointRequest);
    let _ = schemars::schema_for!(AutoRequest);
    let _ = schemars::schema_for!(GraphRequest);
    let _ = schemars::schema_for!(DetectRequest);
    let _ = schemars::schema_for!(DecisionRequest);
    let _ = schemars::schema_for!(EvidenceRequest);
    let _ = schemars::schema_for!(TimelineRequest);
    let _ = schemars::schema_for!(MctsRequest);
    let _ = schemars::schema_for!(CounterfactualRequest);
    let _ = schemars::schema_for!(PresetRequest);
    let _ = schemars::schema_for!(MetricsRequest);
}

#[test]
fn test_reasoning_server_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ReasoningServer>();
}

#[test]
fn test_server_handler_get_info() {
    let server = create_test_server_sync();
    let info = server.get_info();
    assert!(info.capabilities.tools.is_some());
    assert!(info.instructions.is_some());
}

#[test]
fn test_reasoning_server_new() {
    let server = create_test_server_sync();
    // Just verify we can create a server without panicking
    let _ = &server.state;
}
