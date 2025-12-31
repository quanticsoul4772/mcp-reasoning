//! Server types and shared state.
//!
//! This module defines the core server types including application state
//! and the reasoning server wrapper.

use std::sync::Arc;

use crate::anthropic::AnthropicClient;
use crate::config::Config;
use crate::metadata::MetadataBuilder;
use crate::metrics::MetricsCollector;
use crate::presets::PresetRegistry;
use crate::self_improvement::ManagerHandle;
use crate::storage::SqliteStorage;

/// Shared application state for all tool handlers.
///
/// This struct holds the configured components that tools need
/// to perform reasoning operations.
///
/// # Self-Improvement System
///
/// The `self_improvement` field provides access to the self-improvement system.
/// Self-improvement is ALWAYS enabled - it is a core feature, not optional.
#[derive(Clone)]
pub struct AppState {
    /// Storage backend for sessions and thoughts.
    pub storage: Arc<SqliteStorage>,
    /// Anthropic client for LLM calls.
    pub client: Arc<AnthropicClient>,
    /// Server configuration.
    pub config: Arc<Config>,
    /// Metrics collector for tracking tool usage.
    pub metrics: Arc<MetricsCollector>,
    /// Preset registry for workflow presets.
    pub presets: Arc<PresetRegistry>,
    /// Self-improvement manager handle.
    ///
    /// This handle allows MCP tools to interact with the self-improvement system.
    /// Self-improvement is ALWAYS enabled - it is a core feature.
    pub self_improvement: Arc<ManagerHandle>,
    /// Metadata builder for enriching tool responses.
    pub metadata_builder: Arc<MetadataBuilder>,
}

impl AppState {
    /// Creates a new application state.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend
    /// * `client` - The Anthropic client
    /// * `config` - Server configuration
    /// * `metrics` - Shared metrics collector (used by both tools and self-improvement)
    /// * `self_improvement` - Self-improvement manager handle
    /// * `metadata_builder` - Metadata builder for tool responses
    #[must_use]
    pub fn new(
        storage: SqliteStorage,
        client: AnthropicClient,
        config: Config,
        metrics: Arc<MetricsCollector>,
        self_improvement: ManagerHandle,
        metadata_builder: MetadataBuilder,
    ) -> Self {
        Self {
            storage: Arc::new(storage),
            client: Arc::new(client),
            config: Arc::new(config),
            metrics,
            presets: Arc::new(PresetRegistry::new()),
            self_improvement: Arc::new(self_improvement),
            metadata_builder: Arc::new(metadata_builder),
        }
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::unused_async
)]
mod tests {
    use super::*;
    use crate::anthropic::ClientConfig;
    use crate::config::{SecretString, SelfImprovementConfig};
    use crate::self_improvement::{SelfImprovementManager, SelfImprovementStorage};
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

    fn test_config() -> Config {
        Config {
            api_key: SecretString::new("test-key"),
            database_path: ":memory:".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            request_timeout_deep_ms: 60000,
            request_timeout_maximum_ms: 120000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    fn mock_response(content: &str) -> CompletionResponse {
        CompletionResponse::new(content, Usage::new(100, 50))
    }

    fn create_mock_client() -> MockAnthropicClientTrait {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(
                r#"{"summary": "Test", "confidence": 0.8, "actions": []}"#,
            ))
        });
        client
    }

    async fn create_test_handle(
        metrics: Arc<MetricsCollector>,
        storage: &SqliteStorage,
    ) -> ManagerHandle {
        let si_config = SelfImprovementConfig::default();
        let si_client = create_mock_client();
        let si_storage = Arc::new(SelfImprovementStorage::new(storage.pool.clone()));

        let (_manager, handle) =
            SelfImprovementManager::new(si_config, si_client, metrics, si_storage);
        handle
    }

    #[tokio::test]
    async fn test_app_state_new() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let client_config = ClientConfig::default();
        let client = AnthropicClient::new("test-key", client_config).unwrap();
        let config = test_config();
        let metrics = Arc::new(MetricsCollector::new());
        let si_handle = create_test_handle(metrics.clone(), &storage).await;
        let metadata_builder = crate::metadata::MetadataBuilder::new(
            Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
                storage.clone(),
            ))),
            Arc::new(crate::metadata::PresetIndex::build()),
            30000,
        );

        let state = AppState::new(
            storage,
            client,
            config,
            metrics,
            si_handle,
            metadata_builder,
        );

        // Verify all Arc wrappers are properly created
        assert!(Arc::strong_count(&state.storage) >= 1);
        assert!(Arc::strong_count(&state.client) >= 1);
        assert!(Arc::strong_count(&state.config) >= 1);
        assert!(Arc::strong_count(&state.metrics) >= 1);
        assert!(Arc::strong_count(&state.presets) >= 1);
        assert!(Arc::strong_count(&state.self_improvement) >= 1);
    }

    #[tokio::test]
    async fn test_app_state_debug() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let client_config = ClientConfig::default();
        let client = AnthropicClient::new("test-key", client_config).unwrap();
        let config = test_config();
        let metrics = Arc::new(MetricsCollector::new());
        let si_handle = create_test_handle(metrics.clone(), &storage).await;
        let metadata_builder = crate::metadata::MetadataBuilder::new(
            Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
                storage.clone(),
            ))),
            Arc::new(crate::metadata::PresetIndex::build()),
            30000,
        );

        let state = AppState::new(
            storage,
            client,
            config,
            metrics,
            si_handle,
            metadata_builder,
        );
        let debug = format!("{:?}", state);

        assert!(debug.contains("AppState"));
        assert!(debug.contains("config"));
    }

    #[tokio::test]
    async fn test_app_state_clone() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let client_config = ClientConfig::default();
        let client = AnthropicClient::new("test-key", client_config).unwrap();
        let config = test_config();
        let metrics = Arc::new(MetricsCollector::new());
        let si_handle = create_test_handle(metrics.clone(), &storage).await;
        let metadata_builder = crate::metadata::MetadataBuilder::new(
            Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
                storage.clone(),
            ))),
            Arc::new(crate::metadata::PresetIndex::build()),
            30000,
        );

        let state1 = AppState::new(
            storage,
            client,
            config,
            metrics,
            si_handle,
            metadata_builder,
        );
        let state2 = state1.clone();

        // Both should share the same Arc pointers
        assert!(Arc::ptr_eq(&state1.storage, &state2.storage));
        assert!(Arc::ptr_eq(&state1.client, &state2.client));
        assert!(Arc::ptr_eq(&state1.config, &state2.config));
        assert!(Arc::ptr_eq(&state1.metrics, &state2.metrics));
        assert!(Arc::ptr_eq(&state1.presets, &state2.presets));
        assert!(Arc::ptr_eq(
            &state1.self_improvement,
            &state2.self_improvement
        ));
    }

    #[test]
    fn test_app_state_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<AppState>();
    }

    #[test]
    fn test_app_state_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppState>();
    }
}
