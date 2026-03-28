//! MCP Reasoning Server
//!
//! A Rust-based MCP server providing structured reasoning capabilities
//! via direct Anthropic Claude API calls.
//!
//! # Features
//!
//! - 15 consolidated reasoning tools (vs 40 in predecessor)
//! - Direct Anthropic API integration
//! - Extended thinking support with configurable budgets
//! - `SQLite` persistence for sessions and state
//! - Self-improvement 4-phase optimization loop
//!
//! # Quick Start
//!
//! ```bash
//! ANTHROPIC_API_KEY=sk-ant-xxx ./mcp-reasoning
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     stdin      ┌─────────────────┐
//! │ Claude Code │───────────────▶│   MCP Server    │──────▶ Anthropic API
//! │ or Desktop  │◀───────────────│     (Rust)      │
//! └─────────────┘     stdout     └────────┬────────┘
//!                                         │
//!                                         ▼
//!                                      SQLite
//! ```

// Enable the coverage attribute when running with nightly for llvm-cov exclusions
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
// Allowed pedantic lints for practical reasons
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)] // Many simple functions could be const but don't need to be
#![allow(clippy::cast_precision_loss)] // u64/usize to f64 for metrics is acceptable
#![allow(clippy::cast_possible_truncation)] // u128 milliseconds to u64 is safe in practice
#![allow(clippy::doc_markdown)] // Backticks in docs not required for all identifiers
#![allow(clippy::missing_errors_doc)] // Error documentation not required for all functions
#![allow(clippy::too_many_lines)] // Allow longer functions when logically coherent
#![allow(clippy::must_use_candidate)] // Not all getters need #[must_use]
#![allow(clippy::trivially_copy_pass_by_ref)] // Small types by ref is fine
#![allow(clippy::unused_self)] // Methods may have &self for future use
#![allow(clippy::match_same_arms)] // Explicit match arms can be clearer
#![allow(clippy::uninlined_format_args)] // format!("{}", x) is fine
#![allow(clippy::manual_let_else)] // if-let is sometimes clearer than let-else
#![allow(clippy::redundant_clone)] // Sometimes needed for borrow checker
#![allow(clippy::suboptimal_flops)] // Readable math over optimal
#![allow(clippy::assigning_clones)] // clone() is often clearer than clone_from()

pub mod agents;
pub mod anthropic;
pub mod config;
pub mod error;
pub mod metadata;
pub mod metrics;
pub mod modes;
pub mod presets;
pub mod prompts;
pub mod self_improvement;
pub mod server;
pub mod skills;
pub mod storage;
pub mod traits;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod test_utils;

/// Doctest helper module - provides mock implementations for documentation examples.
///
/// This module is hidden from documentation but available for doctests to use.
/// It provides simple mock implementations that don't require external resources.
///
/// Coverage is excluded because doctests run in a separate binary and don't contribute
/// to llvm-cov measurements for the main library.
#[doc(hidden)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub mod doctest_helpers {
    use crate::anthropic::StreamEvent;
    use crate::error::{ModeError, StorageError};
    use crate::traits::{
        AnthropicClientTrait, CompletionConfig, CompletionResponse, Message, Session, StorageTrait,
        Thought, TimeProvider, Usage,
    };
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use tokio::sync::mpsc;

    /// A simple mock storage for doctests that stores nothing but returns valid responses.
    #[derive(Debug, Clone, Default)]
    pub struct MockStorage;

    impl MockStorage {
        /// Create a new mock storage.
        #[must_use]
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl StorageTrait for MockStorage {
        async fn get_session(&self, id: &str) -> Result<Option<Session>, StorageError> {
            Ok(Some(Session::new(id)))
        }

        async fn get_or_create_session(
            &self,
            session_id: Option<String>,
        ) -> Result<Session, StorageError> {
            Ok(Session::new(
                session_id.unwrap_or_else(|| "mock-session".to_string()),
            ))
        }

        async fn save_thought(&self, _thought: &Thought) -> Result<(), StorageError> {
            Ok(())
        }

        async fn get_thoughts(&self, _session_id: &str) -> Result<Vec<Thought>, StorageError> {
            Ok(vec![])
        }

        async fn save_checkpoint(
            &self,
            _checkpoint: &crate::traits::StoredCheckpoint,
        ) -> Result<(), StorageError> {
            Ok(())
        }

        async fn get_checkpoint(
            &self,
            _id: &str,
        ) -> Result<Option<crate::traits::StoredCheckpoint>, StorageError> {
            Ok(None)
        }

        async fn get_checkpoints(
            &self,
            _session_id: &str,
        ) -> Result<Vec<crate::traits::StoredCheckpoint>, StorageError> {
            Ok(vec![])
        }

        async fn save_branch(
            &self,
            _branch: &crate::traits::StoredBranch,
        ) -> Result<(), StorageError> {
            Ok(())
        }

        async fn get_branch(
            &self,
            _id: &str,
        ) -> Result<Option<crate::traits::StoredBranch>, StorageError> {
            Ok(None)
        }

        async fn get_branches(
            &self,
            _session_id: &str,
        ) -> Result<Vec<crate::traits::StoredBranch>, StorageError> {
            Ok(vec![])
        }

        async fn update_branch_status(
            &self,
            _id: &str,
            _status: crate::traits::StoredBranchStatus,
        ) -> Result<(), StorageError> {
            Ok(())
        }

        async fn save_graph_node(
            &self,
            _node: &crate::traits::StoredGraphNode,
        ) -> Result<(), StorageError> {
            Ok(())
        }

        async fn get_graph_node(
            &self,
            _id: &str,
        ) -> Result<Option<crate::traits::StoredGraphNode>, StorageError> {
            Ok(None)
        }

        async fn get_graph_nodes(
            &self,
            _session_id: &str,
        ) -> Result<Vec<crate::traits::StoredGraphNode>, StorageError> {
            Ok(vec![])
        }

        async fn save_graph_edge(
            &self,
            _edge: &crate::traits::StoredGraphEdge,
        ) -> Result<(), StorageError> {
            Ok(())
        }

        async fn get_graph_edges(
            &self,
            _session_id: &str,
        ) -> Result<Vec<crate::traits::StoredGraphEdge>, StorageError> {
            Ok(vec![])
        }
    }

    /// A simple mock Anthropic client for doctests.
    #[derive(Debug, Clone)]
    pub struct MockClient {
        response: String,
    }

    impl Default for MockClient {
        fn default() -> Self {
            Self {
                response: r#"{"content": "mock response", "confidence": 0.85}"#.to_string(),
            }
        }
    }

    impl MockClient {
        /// Create a new mock client with a default response.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Create a mock client that returns a specific response.
        #[must_use]
        pub fn with_response(response: impl Into<String>) -> Self {
            Self {
                response: response.into(),
            }
        }
    }

    #[async_trait]
    impl AnthropicClientTrait for MockClient {
        async fn complete(
            &self,
            _messages: Vec<Message>,
            _config: CompletionConfig,
        ) -> Result<CompletionResponse, ModeError> {
            Ok(CompletionResponse::new(
                self.response.clone(),
                Usage::new(100, 50),
            ))
        }

        async fn complete_streaming(
            &self,
            _messages: Vec<Message>,
            _config: CompletionConfig,
        ) -> Result<mpsc::Receiver<Result<StreamEvent, ModeError>>, ModeError> {
            let (tx, rx) = mpsc::channel(32);
            let response = self.response.clone();

            // Spawn a task that sends a simple streaming sequence
            tokio::spawn(async move {
                let _ = tx
                    .send(Ok(StreamEvent::MessageStart {
                        message_id: "mock_msg".to_string(),
                    }))
                    .await;
                let _ = tx
                    .send(Ok(StreamEvent::TextDelta {
                        index: 0,
                        text: response,
                    }))
                    .await;
                let _ = tx
                    .send(Ok(StreamEvent::MessageStop {
                        stop_reason: "end_turn".to_string(),
                        usage: crate::anthropic::ApiUsage::new(100, 50),
                    }))
                    .await;
            });

            Ok(rx)
        }
    }

    /// A simple mock time provider for doctests.
    #[derive(Debug, Clone, Default)]
    pub struct MockTime;

    impl MockTime {
        /// Create a new mock time provider.
        #[must_use]
        pub fn new() -> Self {
            Self
        }
    }

    impl TimeProvider for MockTime {
        fn now(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    /// Run an async block in a doctest context.
    ///
    /// # Example
    ///
    /// ```
    /// use mcp_reasoning::doctest_helpers::block_on;
    ///
    /// let result = block_on(async {
    ///     42
    /// });
    /// assert_eq!(result, 42);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the tokio runtime cannot be created (only happens in catastrophic cases).
    #[allow(clippy::expect_used)]
    pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
            .block_on(f)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod doctest_helpers_tests {
    use super::doctest_helpers::*;
    use crate::traits::{
        AnthropicClientTrait, CompletionConfig, Message, StorageTrait, TimeProvider,
    };

    // ============================================================================
    // MockStorage Tests
    // ============================================================================

    #[test]
    fn test_mock_storage_new() {
        let storage = MockStorage::new();
        let debug = format!("{storage:?}");
        assert!(debug.contains("MockStorage"));
    }

    #[test]
    fn test_mock_storage_default() {
        let storage = MockStorage;
        let debug = format!("{storage:?}");
        assert!(debug.contains("MockStorage"));
    }

    #[test]
    fn test_mock_storage_clone() {
        let storage = MockStorage::new();
        let cloned = storage.clone();
        let debug = format!("{cloned:?}");
        assert!(debug.contains("MockStorage"));
    }

    #[tokio::test]
    async fn test_mock_storage_get_session() {
        let storage = MockStorage::new();
        let session = storage.get_session("test-id").await.unwrap();
        assert!(session.is_some());
        assert_eq!(session.unwrap().id, "test-id");
    }

    #[tokio::test]
    async fn test_mock_storage_get_or_create_session_with_id() {
        let storage = MockStorage::new();
        let session = storage
            .get_or_create_session(Some("my-session".to_string()))
            .await
            .unwrap();
        assert_eq!(session.id, "my-session");
    }

    #[tokio::test]
    async fn test_mock_storage_get_or_create_session_none() {
        let storage = MockStorage::new();
        let session = storage.get_or_create_session(None).await.unwrap();
        assert_eq!(session.id, "mock-session");
    }

    #[tokio::test]
    async fn test_mock_storage_save_thought() {
        let storage = MockStorage::new();
        let thought = crate::traits::Thought::new("t1", "s1", "content", "linear", 0.8);
        let result = storage.save_thought(&thought).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_get_thoughts() {
        let storage = MockStorage::new();
        let thoughts = storage.get_thoughts("s1").await.unwrap();
        assert!(thoughts.is_empty());
    }

    #[tokio::test]
    async fn test_mock_storage_save_checkpoint() {
        let storage = MockStorage::new();
        let checkpoint = crate::traits::StoredCheckpoint {
            id: "cp1".to_string(),
            session_id: "s1".to_string(),
            name: "test".to_string(),
            description: None,
            state: "{}".to_string(),
            created_at: chrono::Utc::now(),
        };
        let result = storage.save_checkpoint(&checkpoint).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_get_checkpoint() {
        let storage = MockStorage::new();
        let result = storage.get_checkpoint("cp1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_storage_get_checkpoints() {
        let storage = MockStorage::new();
        let result = storage.get_checkpoints("s1").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_mock_storage_save_branch() {
        let storage = MockStorage::new();
        let branch = crate::traits::StoredBranch {
            id: "b1".to_string(),
            session_id: "s1".to_string(),
            parent_branch_id: None,
            content: "branch content".to_string(),
            score: 0.9,
            status: crate::storage::BranchStatus::default(),
            created_at: chrono::Utc::now(),
        };
        let result = storage.save_branch(&branch).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_get_branch() {
        let storage = MockStorage::new();
        let result = storage.get_branch("b1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_storage_get_branches() {
        let storage = MockStorage::new();
        let result = storage.get_branches("s1").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_mock_storage_update_branch_status() {
        let storage = MockStorage::new();
        let result = storage
            .update_branch_status("b1", crate::storage::BranchStatus::default())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_save_graph_node() {
        let storage = MockStorage::new();
        let node = crate::traits::StoredGraphNode {
            id: "n1".to_string(),
            session_id: "s1".to_string(),
            content: "node content".to_string(),
            node_type: crate::storage::GraphNodeType::default(),
            score: Some(0.8),
            is_terminal: false,
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        let result = storage.save_graph_node(&node).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_get_graph_node() {
        let storage = MockStorage::new();
        let result = storage.get_graph_node("n1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_storage_get_graph_nodes() {
        let storage = MockStorage::new();
        let result = storage.get_graph_nodes("s1").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_mock_storage_save_graph_edge() {
        let storage = MockStorage::new();
        let edge = crate::traits::StoredGraphEdge {
            id: "e1".to_string(),
            session_id: "s1".to_string(),
            from_node_id: "n1".to_string(),
            to_node_id: "n2".to_string(),
            edge_type: crate::storage::GraphEdgeType::default(),
            created_at: chrono::Utc::now(),
        };
        let result = storage.save_graph_edge(&edge).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_get_graph_edges() {
        let storage = MockStorage::new();
        let result = storage.get_graph_edges("s1").await.unwrap();
        assert!(result.is_empty());
    }

    // ============================================================================
    // MockClient Tests
    // ============================================================================

    #[test]
    fn test_mock_client_new() {
        let client = MockClient::new();
        let debug = format!("{client:?}");
        assert!(debug.contains("MockClient"));
    }

    #[test]
    fn test_mock_client_default() {
        let client = MockClient::default();
        let debug = format!("{client:?}");
        assert!(debug.contains("mock response"));
    }

    #[test]
    fn test_mock_client_with_response() {
        let client = MockClient::with_response("custom response");
        let debug = format!("{client:?}");
        assert!(debug.contains("custom response"));
    }

    #[test]
    fn test_mock_client_clone() {
        let client = MockClient::new();
        let cloned = client.clone();
        let debug = format!("{cloned:?}");
        assert!(debug.contains("MockClient"));
    }

    #[tokio::test]
    async fn test_mock_client_complete() {
        let client = MockClient::new();
        let messages = vec![Message::user("Hello")];
        let config = CompletionConfig::new();
        let result = client.complete(messages, config).await.unwrap();
        assert_eq!(result.usage.input_tokens, 100);
        assert_eq!(result.usage.output_tokens, 50);
    }

    #[tokio::test]
    async fn test_mock_client_complete_with_custom_response() {
        let client = MockClient::with_response("custom");
        let messages = vec![Message::user("Hello")];
        let config = CompletionConfig::new();
        let result = client.complete(messages, config).await.unwrap();
        assert_eq!(result.content, "custom");
    }

    #[tokio::test]
    async fn test_mock_client_complete_streaming() {
        let client = MockClient::new();
        let messages = vec![Message::user("Hello")];
        let config = CompletionConfig::new();
        let mut rx = client.complete_streaming(messages, config).await.unwrap();

        // Receive the streaming events
        let event1 = rx.recv().await.unwrap().unwrap();
        assert!(matches!(
            event1,
            crate::anthropic::StreamEvent::MessageStart { .. }
        ));

        let event2 = rx.recv().await.unwrap().unwrap();
        assert!(matches!(
            event2,
            crate::anthropic::StreamEvent::TextDelta { .. }
        ));

        let event3 = rx.recv().await.unwrap().unwrap();
        assert!(matches!(
            event3,
            crate::anthropic::StreamEvent::MessageStop { .. }
        ));
    }

    // ============================================================================
    // MockTime Tests
    // ============================================================================

    #[test]
    fn test_mock_time_new() {
        let time = MockTime::new();
        let debug = format!("{time:?}");
        assert!(debug.contains("MockTime"));
    }

    #[test]
    fn test_mock_time_default() {
        let time = MockTime;
        let debug = format!("{time:?}");
        assert!(debug.contains("MockTime"));
    }

    #[test]
    fn test_mock_time_clone() {
        let time = MockTime::new();
        let cloned = time.clone();
        let _ = format!("{cloned:?}");
    }

    #[test]
    fn test_mock_time_now() {
        let time = MockTime::new();
        let now = time.now();
        let diff = chrono::Utc::now() - now;
        assert!(diff.num_seconds().abs() < 2);
    }

    // ============================================================================
    // block_on Tests
    // ============================================================================

    #[test]
    fn test_block_on_sync_value() {
        let result = block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_block_on_async_operation() {
        let result = block_on(async {
            let storage = MockStorage::new();
            storage.get_session("test").await.unwrap()
        });
        assert!(result.is_some());
    }
}
