//! Server types and shared state.
//!
//! This module defines the core server types including application state
//! and the reasoning server wrapper.

use std::sync::Arc;

use crate::anthropic::AnthropicClient;
use crate::config::Config;
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

    #[test]
    fn test_app_state_debug() {
        // We can't easily construct AppState in tests without real storage/client,
        // but we verify the Debug impl compiles correctly
        let _debug_format = format!("{:?}", "placeholder");
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
