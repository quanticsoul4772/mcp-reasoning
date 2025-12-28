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

pub mod anthropic;
pub mod config;
pub mod error;
pub mod metrics;
pub mod modes;
pub mod presets;
pub mod prompts;
pub mod self_improvement;
pub mod server;
pub mod storage;
pub mod traits;

#[cfg(test)]
mod test_utils;

/// Doctest helper module - provides mock implementations for documentation examples.
///
/// This module is hidden from documentation but available for doctests to use.
/// It provides simple mock implementations that don't require external resources.
#[doc(hidden)]
pub mod doctest_helpers {
    use crate::error::{ModeError, StorageError};
    use crate::traits::{
        AnthropicClientTrait, CompletionConfig, CompletionResponse, Message, Session, StorageTrait,
        Thought, TimeProvider, Usage,
    };
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};

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
