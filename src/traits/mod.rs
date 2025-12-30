//! Trait definitions for mockable dependencies.
//!
//! This module defines traits for:
//! - [`AnthropicClientTrait`]: API client abstraction
//! - [`StorageTrait`]: Database operations abstraction
//! - [`TimeProvider`]: Time abstraction for testing
//!
//! It also re-exports shared types from the `types` submodule.
//!
//! # Mocking
//!
//! All traits are annotated with `#[cfg_attr(test, mockall::automock)]`
//! which generates mock implementations automatically for testing.
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::traits::{TimeProvider, RealTimeProvider};
//!
//! let time_provider = RealTimeProvider;
//! let now = time_provider.now();
//! println!("Current time: {now}");
//! ```

mod types;

pub use types::{CompletionConfig, CompletionResponse, Message, Session, Thought, Usage};

// Re-export storage types needed by modes
pub use crate::storage::BranchStatus as StoredBranchStatus;
pub use crate::storage::{StoredBranch, StoredCheckpoint, StoredGraphEdge, StoredGraphNode};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::{ModeError, StorageError};

/// Anthropic API client trait for mocking.
///
/// This trait abstracts the Anthropic API client to allow for
/// dependency injection and testing with mock implementations.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AnthropicClientTrait: Send + Sync {
    /// Send a completion request to the API.
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation messages
    /// * `config` - Completion configuration options
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if the API call fails.
    async fn complete(
        &self,
        messages: Vec<Message>,
        config: CompletionConfig,
    ) -> Result<CompletionResponse, ModeError>;
}

/// Storage trait for mocking.
///
/// This trait abstracts database operations to allow for
/// dependency injection and testing with mock implementations.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait StorageTrait: Send + Sync {
    /// Get a session by ID.
    ///
    /// Returns `None` if the session doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_session(&self, id: &str) -> Result<Option<Session>, StorageError>;

    /// Get or create a session.
    ///
    /// If an ID is provided and exists, returns that session.
    /// If an ID is provided but doesn't exist, creates a new session with that ID.
    /// If no ID is provided, generates a new session with a UUID.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_or_create_session(&self, id: Option<String>) -> Result<Session, StorageError>;

    /// Save a thought to the database.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn save_thought(&self, thought: &Thought) -> Result<(), StorageError>;

    /// Get all thoughts for a session.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_thoughts(&self, session_id: &str) -> Result<Vec<Thought>, StorageError>;

    /// Save a checkpoint to the database.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn save_checkpoint(&self, checkpoint: &StoredCheckpoint) -> Result<(), StorageError>;

    /// Get a checkpoint by ID.
    ///
    /// Returns `None` if the checkpoint doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_checkpoint(&self, id: &str) -> Result<Option<StoredCheckpoint>, StorageError>;

    /// Get all checkpoints for a session.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_checkpoints(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredCheckpoint>, StorageError>;

    /// Save a branch to the database.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn save_branch(&self, branch: &StoredBranch) -> Result<(), StorageError>;

    /// Get a branch by ID.
    ///
    /// Returns `None` if the branch doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_branch(&self, id: &str) -> Result<Option<StoredBranch>, StorageError>;

    /// Get all branches for a session.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_branches(&self, session_id: &str) -> Result<Vec<StoredBranch>, StorageError>;

    /// Update a branch's status.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn update_branch_status(
        &self,
        id: &str,
        status: StoredBranchStatus,
    ) -> Result<(), StorageError>;

    /// Save a graph node to the database.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn save_graph_node(&self, node: &StoredGraphNode) -> Result<(), StorageError>;

    /// Get a graph node by ID.
    ///
    /// Returns `None` if the node doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_graph_node(&self, id: &str) -> Result<Option<StoredGraphNode>, StorageError>;

    /// Get all graph nodes for a session.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_graph_nodes(&self, session_id: &str)
        -> Result<Vec<StoredGraphNode>, StorageError>;

    /// Save a graph edge to the database.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn save_graph_edge(&self, edge: &StoredGraphEdge) -> Result<(), StorageError>;

    /// Get all graph edges for a session.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError`] if the database operation fails.
    async fn get_graph_edges(&self, session_id: &str)
        -> Result<Vec<StoredGraphEdge>, StorageError>;
}

/// Time provider trait for deterministic testing.
///
/// This trait abstracts time operations to allow for
/// deterministic testing by providing fixed timestamps.
#[cfg_attr(test, mockall::automock)]
pub trait TimeProvider: Send + Sync {
    /// Get the current time.
    fn now(&self) -> DateTime<Utc>;
}

/// Real time provider using system clock.
///
/// This is the production implementation that returns the actual current time.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealTimeProvider;

impl TimeProvider for RealTimeProvider {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
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
    clippy::unused_async,
    clippy::no_effect_underscore_binding
)]
mod tests {
    use super::*;
    use static_assertions::assert_impl_all;

    // Verify RealTimeProvider traits
    assert_impl_all!(RealTimeProvider: Send, Sync, Clone, Copy, Default);

    // RealTimeProvider Tests
    #[test]
    fn test_real_time_provider_default() {
        let provider = RealTimeProvider;
        let now = provider.now();
        let diff = Utc::now() - now;
        assert!(diff.num_seconds() < 1);
    }

    #[test]
    fn test_real_time_provider_now() {
        let provider = RealTimeProvider;
        let before = Utc::now();
        let now = provider.now();
        let after = Utc::now();
        assert!(now >= before);
        assert!(now <= after);
    }

    #[test]
    fn test_real_time_provider_clone() {
        let provider = RealTimeProvider;
        let _cloned = provider;
        // Copy works (no compile error)
    }

    #[test]
    fn test_real_time_provider_debug() {
        let provider = RealTimeProvider;
        let debug = format!("{provider:?}");
        assert!(debug.contains("RealTimeProvider"));
    }

    // Mock Verification Tests
    #[tokio::test]
    async fn test_mock_anthropic_client() {
        let mut mock = MockAnthropicClientTrait::new();
        mock.expect_complete().returning(|_msgs, _config| {
            Ok(CompletionResponse::new("Mock response", Usage::new(10, 20)))
        });

        let messages = vec![Message::user("Test")];
        let config = CompletionConfig::new();
        let result = mock.complete(messages, config).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "Mock response");
        assert_eq!(response.usage.total(), 30);
    }

    #[tokio::test]
    async fn test_mock_anthropic_client_error() {
        let mut mock = MockAnthropicClientTrait::new();
        mock.expect_complete().returning(|_msgs, _config| {
            Err(ModeError::ApiUnavailable {
                message: "Test error".to_string(),
            })
        });

        let messages = vec![Message::user("Test")];
        let config = CompletionConfig::new();
        let result = mock.complete(messages, config).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_mock_storage_get_session() {
        let mut mock = MockStorageTrait::new();
        mock.expect_get_session()
            .with(mockall::predicate::eq("sess-123"))
            .returning(|id| Ok(Some(Session::new(id))));

        let result = mock.get_session("sess-123").await;
        assert!(result.is_ok());
        let session = result.unwrap();
        assert!(session.is_some());
        assert_eq!(session.unwrap().id, "sess-123");
    }

    #[tokio::test]
    async fn test_mock_storage_get_session_not_found() {
        let mut mock = MockStorageTrait::new();
        mock.expect_get_session().returning(|_id| Ok(None));

        let result = mock.get_session("nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_storage_get_or_create_session() {
        let mut mock = MockStorageTrait::new();
        mock.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "generated-id".to_string()),
            ))
        });

        let result = mock
            .get_or_create_session(Some("sess-123".to_string()))
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "sess-123");
    }

    #[tokio::test]
    async fn test_mock_storage_get_or_create_session_no_id() {
        let mut mock = MockStorageTrait::new();
        mock.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "generated-id".to_string()),
            ))
        });

        let result = mock.get_or_create_session(None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "generated-id");
    }

    #[tokio::test]
    async fn test_mock_storage_save_thought() {
        let mut mock = MockStorageTrait::new();
        mock.expect_save_thought().returning(|_thought| Ok(()));

        let thought = Thought::new("t-1", "sess-1", "Content", "linear", 0.85);
        let result = mock.save_thought(&thought).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_get_thoughts() {
        let mut mock = MockStorageTrait::new();
        mock.expect_get_thoughts().returning(|session_id| {
            Ok(vec![
                Thought::new("t-1", session_id, "First", "linear", 0.8),
                Thought::new("t-2", session_id, "Second", "linear", 0.9),
            ])
        });

        let result = mock.get_thoughts("sess-123").await;
        assert!(result.is_ok());
        let thoughts = result.unwrap();
        assert_eq!(thoughts.len(), 2);
        assert_eq!(thoughts[0].id, "t-1");
        assert_eq!(thoughts[1].id, "t-2");
    }

    #[tokio::test]
    async fn test_mock_storage_error() {
        let mut mock = MockStorageTrait::new();
        mock.expect_get_session().returning(|_id| {
            Err(StorageError::ConnectionFailed {
                message: "Test error".to_string(),
            })
        });

        let result = mock.get_session("test").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::ConnectionFailed { .. })));
    }

    #[test]
    fn test_mock_time_provider() {
        let fixed_time = Utc::now() - chrono::Duration::days(1);
        let mut mock = MockTimeProvider::new();
        mock.expect_now().return_const(fixed_time);

        let result = mock.now();
        assert_eq!(result, fixed_time);
    }

    #[test]
    fn test_mock_time_provider_multiple_calls() {
        let time1 = Utc::now();
        let time2 = time1 + chrono::Duration::hours(1);

        let mut mock = MockTimeProvider::new();
        let mut seq = mockall::Sequence::new();
        mock.expect_now()
            .times(1)
            .in_sequence(&mut seq)
            .return_const(time1);
        mock.expect_now()
            .times(1)
            .in_sequence(&mut seq)
            .return_const(time2);

        assert_eq!(mock.now(), time1);
        assert_eq!(mock.now(), time2);
    }

    #[tokio::test]
    async fn test_mock_storage_save_checkpoint() {
        let mut mock = MockStorageTrait::new();
        mock.expect_save_checkpoint().returning(|_| Ok(()));

        let checkpoint = StoredCheckpoint::new("cp-1", "sess-1", "Test Checkpoint", "{}");
        let result = mock.save_checkpoint(&checkpoint).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_get_checkpoint() {
        let mut mock = MockStorageTrait::new();
        mock.expect_get_checkpoint()
            .with(mockall::predicate::eq("cp-1"))
            .returning(|id| {
                Ok(Some(StoredCheckpoint::new(
                    id,
                    "sess-1",
                    "Checkpoint",
                    "{}",
                )))
            });

        let result = mock.get_checkpoint("cp-1").await;
        assert!(result.is_ok());
        let checkpoint = result.unwrap();
        assert!(checkpoint.is_some());
        assert_eq!(checkpoint.unwrap().id, "cp-1");
    }

    #[tokio::test]
    async fn test_mock_storage_get_checkpoint_not_found() {
        let mut mock = MockStorageTrait::new();
        mock.expect_get_checkpoint().returning(|_| Ok(None));

        let result = mock.get_checkpoint("nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_storage_get_checkpoints() {
        let mut mock = MockStorageTrait::new();
        mock.expect_get_checkpoints().returning(|session_id| {
            Ok(vec![
                StoredCheckpoint::new("cp-1", session_id, "First", "{}"),
                StoredCheckpoint::new("cp-2", session_id, "Second", "{}"),
            ])
        });

        let result = mock.get_checkpoints("sess-123").await;
        assert!(result.is_ok());
        let checkpoints = result.unwrap();
        assert_eq!(checkpoints.len(), 2);
        assert_eq!(checkpoints[0].id, "cp-1");
        assert_eq!(checkpoints[1].id, "cp-2");
    }
}
