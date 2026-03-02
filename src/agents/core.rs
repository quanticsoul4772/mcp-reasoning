//! Agent core shared dependencies.
//!
//! Follows the `ModeCore` composition pattern: Arc-wrapped shared deps.

use std::sync::Arc;

use crate::anthropic::AnthropicClient;
use crate::storage::SqliteStorage;

/// Shared dependencies for agent operations.
///
/// Provides access to storage and the Anthropic client via Arc references.
/// Used by `AgentExecutor`, `TeamCoordinator`, and other agent components.
#[derive(Clone)]
pub struct AgentCore {
    storage: Arc<SqliteStorage>,
    client: Arc<AnthropicClient>,
}

impl AgentCore {
    /// Create a new agent core with shared dependencies.
    #[must_use]
    pub fn new(storage: Arc<SqliteStorage>, client: Arc<AnthropicClient>) -> Self {
        Self { storage, client }
    }

    /// Get a reference to the storage backend.
    #[must_use]
    pub fn storage(&self) -> &SqliteStorage {
        &self.storage
    }

    /// Get a reference to the Anthropic client.
    #[must_use]
    pub fn client(&self) -> &AnthropicClient {
        &self.client
    }

    /// Get an Arc clone of the storage.
    #[must_use]
    pub fn storage_arc(&self) -> Arc<SqliteStorage> {
        Arc::clone(&self.storage)
    }

    /// Get an Arc clone of the client.
    #[must_use]
    pub fn client_arc(&self) -> Arc<AnthropicClient> {
        Arc::clone(&self.client)
    }
}

impl std::fmt::Debug for AgentCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentCore").finish_non_exhaustive()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::anthropic::ClientConfig;

    #[tokio::test]
    async fn test_agent_core_new() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client =
            AnthropicClient::new("test-key", ClientConfig::default()).expect("create client");
        let core = AgentCore::new(Arc::new(storage), Arc::new(client));

        assert!(format!("{core:?}").contains("AgentCore"));
    }

    #[tokio::test]
    async fn test_agent_core_clone() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client =
            AnthropicClient::new("test-key", ClientConfig::default()).expect("create client");
        let core1 = AgentCore::new(Arc::new(storage), Arc::new(client));
        let core2 = core1.clone();

        // Both should share the same Arc pointers
        assert!(Arc::ptr_eq(&core1.storage_arc(), &core2.storage_arc()));
        assert!(Arc::ptr_eq(&core1.client_arc(), &core2.client_arc()));
    }

    #[tokio::test]
    async fn test_agent_core_storage_access() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client =
            AnthropicClient::new("test-key", ClientConfig::default()).expect("create client");
        let core = AgentCore::new(Arc::new(storage), Arc::new(client));

        let _ = core.storage();
        let arc = core.storage_arc();
        assert!(Arc::strong_count(&arc) >= 1);
    }

    #[tokio::test]
    async fn test_agent_core_client_access() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client =
            AnthropicClient::new("test-key", ClientConfig::default()).expect("create client");
        let core = AgentCore::new(Arc::new(storage), Arc::new(client));

        let _ = core.client();
        let arc = core.client_arc();
        assert!(Arc::strong_count(&arc) >= 1);
    }
}
