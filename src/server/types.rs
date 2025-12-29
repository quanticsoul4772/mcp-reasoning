//! Server types and shared state.
//!
//! This module defines the core server types including application state
//! and the reasoning server wrapper.

use std::sync::Arc;

use crate::anthropic::AnthropicClient;
use crate::config::Config;
use crate::metrics::MetricsCollector;
use crate::presets::PresetRegistry;
use crate::storage::SqliteStorage;

/// Shared application state for all tool handlers.
///
/// This struct holds the configured components that tools need
/// to perform reasoning operations.
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
}

impl AppState {
    /// Creates a new application state.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend
    /// * `client` - The Anthropic client
    /// * `config` - Server configuration
    #[must_use]
    pub fn new(storage: SqliteStorage, client: AnthropicClient, config: Config) -> Self {
        Self {
            storage: Arc::new(storage),
            client: Arc::new(client),
            config: Arc::new(config),
            metrics: Arc::new(MetricsCollector::new()),
            presets: Arc::new(PresetRegistry::new()),
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
mod tests {
    use super::*;
    use crate::anthropic::ClientConfig;
    use crate::config::SecretString;

    fn test_config() -> Config {
        Config {
            api_key: SecretString::new("test-key"),
            database_path: ":memory:".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    #[tokio::test]
    async fn test_app_state_new() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let client_config = ClientConfig::default();
        let client = AnthropicClient::new("test-key", client_config).unwrap();
        let config = test_config();

        let state = AppState::new(storage, client, config);

        // Verify all Arc wrappers are properly created
        assert!(Arc::strong_count(&state.storage) >= 1);
        assert!(Arc::strong_count(&state.client) >= 1);
        assert!(Arc::strong_count(&state.config) >= 1);
        assert!(Arc::strong_count(&state.metrics) >= 1);
        assert!(Arc::strong_count(&state.presets) >= 1);
    }

    #[tokio::test]
    async fn test_app_state_debug() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let client_config = ClientConfig::default();
        let client = AnthropicClient::new("test-key", client_config).unwrap();
        let config = test_config();

        let state = AppState::new(storage, client, config);
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

        let state1 = AppState::new(storage, client, config);
        let state2 = state1.clone();

        // Both should share the same Arc pointers
        assert!(Arc::ptr_eq(&state1.storage, &state2.storage));
        assert!(Arc::ptr_eq(&state1.client, &state2.client));
        assert!(Arc::ptr_eq(&state1.config, &state2.config));
        assert!(Arc::ptr_eq(&state1.metrics, &state2.metrics));
        assert!(Arc::ptr_eq(&state1.presets, &state2.presets));
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
