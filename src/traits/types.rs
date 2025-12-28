//! Shared types for the traits module.
//!
//! This module defines the core data types used across the application:
//! - [`Message`]: API message structure
//! - [`CompletionConfig`]: Completion request configuration
//! - [`CompletionResponse`]: API response structure
//! - [`Usage`]: Token usage information
//! - [`Session`]: Reasoning session data
//! - [`Thought`]: Individual thought/reasoning step data

use chrono::{DateTime, Utc};

/// Message for API requests.
///
/// Represents a single message in a conversation with the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Role of the message sender (user, assistant, system).
    pub role: String,
    /// Content of the message.
    pub content: String,
}

impl Message {
    /// Create a new message.
    #[must_use]
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }

    /// Create a user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    /// Create an assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }

    /// Create a system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }
}

/// Completion configuration.
///
/// Configuration options for API completion requests.
#[derive(Debug, Clone, Default, PartialEq)]
// Cannot derive Eq: f32 temperature field does not implement Eq (IEEE 754 NaN != NaN)
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct CompletionConfig {
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Temperature for sampling (0.0 to 1.0).
    pub temperature: Option<f32>,
    /// System prompt to prepend.
    pub system_prompt: Option<String>,
}

impl CompletionConfig {
    /// Create a new completion config with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set max tokens.
    #[must_use]
    pub const fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature.
    #[must_use]
    pub const fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set system prompt.
    #[must_use]
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }
}

/// Token usage information.
///
/// Tracks the number of tokens used in a request/response.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Usage {
    /// Input tokens (prompt).
    pub input_tokens: u32,
    /// Output tokens (completion).
    pub output_tokens: u32,
}

impl Usage {
    /// Create new usage info.
    #[must_use]
    pub const fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    /// Total tokens used.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Completion response.
///
/// The response from an API completion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionResponse {
    /// Response content.
    pub content: String,
    /// Token usage.
    pub usage: Usage,
}

impl CompletionResponse {
    /// Create a new completion response.
    #[must_use]
    pub fn new(content: impl Into<String>, usage: Usage) -> Self {
        Self {
            content: content.into(),
            usage,
        }
    }
}

/// Session data.
///
/// Represents a reasoning session that groups related thoughts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Unique session identifier.
    pub id: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session with the current timestamp.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            created_at: Utc::now(),
        }
    }

    /// Create a session with a specific timestamp.
    #[must_use]
    pub fn with_timestamp(id: impl Into<String>, created_at: DateTime<Utc>) -> Self {
        Self {
            id: id.into(),
            created_at,
        }
    }
}

/// Thought data.
///
/// Represents a single reasoning step within a session.
#[derive(Debug, Clone, PartialEq)]
pub struct Thought {
    /// Unique thought identifier.
    pub id: String,
    /// Parent session identifier.
    pub session_id: String,
    /// Thought content.
    pub content: String,
    /// Reasoning mode used.
    pub mode: String,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl Thought {
    /// Create a new thought with the current timestamp.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        content: impl Into<String>,
        mode: impl Into<String>,
        confidence: f64,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            content: content.into(),
            mode: mode.into(),
            confidence,
            created_at: Utc::now(),
        }
    }

    /// Create a thought with a specific timestamp.
    #[must_use]
    pub fn with_timestamp(
        id: impl Into<String>,
        session_id: impl Into<String>,
        content: impl Into<String>,
        mode: impl Into<String>,
        confidence: f64,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            content: content.into(),
            mode: mode.into(),
            confidence,
            created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_impl_all;

    // Type Assertions
    assert_impl_all!(Message: Send, Sync, Clone, PartialEq, Eq);
    assert_impl_all!(CompletionConfig: Send, Sync, Clone, Default, PartialEq);
    assert_impl_all!(Usage: Send, Sync, Clone, Default, PartialEq, Eq);
    assert_impl_all!(CompletionResponse: Send, Sync, Clone, PartialEq, Eq);
    assert_impl_all!(Session: Send, Sync, Clone, PartialEq, Eq);
    assert_impl_all!(Thought: Send, Sync, Clone, PartialEq);

    // Message Tests
    #[test]
    fn test_message_new() {
        let msg = Message::new("user", "Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_message_user() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant("Hi there");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Hi there");
    }

    #[test]
    fn test_message_system() {
        let msg = Message::system("You are helpful");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "You are helpful");
    }

    #[test]
    fn test_message_clone() {
        let msg = Message::user("Hello");
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_message_debug() {
        let msg = Message::user("Hello");
        let debug = format!("{msg:?}");
        assert!(debug.contains("user"));
        assert!(debug.contains("Hello"));
    }

    // CompletionConfig Tests
    #[test]
    fn test_completion_config_default() {
        let config = CompletionConfig::default();
        assert!(config.max_tokens.is_none());
        assert!(config.temperature.is_none());
        assert!(config.system_prompt.is_none());
    }

    #[test]
    fn test_completion_config_new() {
        let config = CompletionConfig::new();
        assert!(config.max_tokens.is_none());
        assert!(config.temperature.is_none());
        assert!(config.system_prompt.is_none());
    }

    #[test]
    fn test_completion_config_with_max_tokens() {
        let config = CompletionConfig::new().with_max_tokens(1000);
        assert_eq!(config.max_tokens, Some(1000));
    }

    #[test]
    fn test_completion_config_with_temperature() {
        let config = CompletionConfig::new().with_temperature(0.7);
        assert!((config.temperature.unwrap_or(0.0) - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_completion_config_with_system_prompt() {
        let config = CompletionConfig::new().with_system_prompt("Be helpful");
        assert_eq!(config.system_prompt, Some("Be helpful".to_string()));
    }

    #[test]
    fn test_completion_config_builder_chain() {
        let config = CompletionConfig::new()
            .with_max_tokens(2000)
            .with_temperature(0.5)
            .with_system_prompt("System");
        assert_eq!(config.max_tokens, Some(2000));
        assert!((config.temperature.unwrap_or(0.0) - 0.5).abs() < f32::EPSILON);
        assert_eq!(config.system_prompt, Some("System".to_string()));
    }

    #[test]
    fn test_completion_config_clone() {
        let config = CompletionConfig::new().with_max_tokens(1000);
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    // Usage Tests
    #[test]
    fn test_usage_default() {
        let usage = Usage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
    }

    #[test]
    fn test_usage_new() {
        let usage = Usage::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    #[test]
    fn test_usage_total() {
        let usage = Usage::new(100, 50);
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn test_usage_total_zero() {
        let usage = Usage::default();
        assert_eq!(usage.total(), 0);
    }

    #[test]
    fn test_usage_clone() {
        let usage = Usage::new(100, 50);
        let cloned = usage.clone();
        assert_eq!(usage, cloned);
    }

    #[test]
    fn test_usage_debug() {
        let usage = Usage::new(100, 50);
        let debug = format!("{usage:?}");
        assert!(debug.contains("100"));
        assert!(debug.contains("50"));
    }

    // CompletionResponse Tests
    #[test]
    fn test_completion_response_new() {
        let response = CompletionResponse::new("Hello", Usage::new(10, 5));
        assert_eq!(response.content, "Hello");
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 5);
    }

    #[test]
    fn test_completion_response_clone() {
        let response = CompletionResponse::new("Hello", Usage::new(10, 5));
        let cloned = response.clone();
        assert_eq!(response, cloned);
    }

    #[test]
    fn test_completion_response_debug() {
        let response = CompletionResponse::new("Hello", Usage::new(10, 5));
        let debug = format!("{response:?}");
        assert!(debug.contains("Hello"));
        assert!(debug.contains("10"));
    }

    // Session Tests
    #[test]
    fn test_session_new() {
        let session = Session::new("sess-123");
        assert_eq!(session.id, "sess-123");
        let diff = Utc::now() - session.created_at;
        assert!(diff.num_seconds() < 1);
    }

    #[test]
    fn test_session_with_timestamp() {
        let timestamp = Utc::now() - chrono::Duration::hours(1);
        let session = Session::with_timestamp("sess-123", timestamp);
        assert_eq!(session.id, "sess-123");
        assert_eq!(session.created_at, timestamp);
    }

    #[test]
    fn test_session_clone() {
        let session = Session::new("sess-123");
        let cloned = session.clone();
        assert_eq!(session, cloned);
    }

    #[test]
    fn test_session_debug() {
        let session = Session::new("sess-123");
        let debug = format!("{session:?}");
        assert!(debug.contains("sess-123"));
    }

    // Thought Tests
    #[test]
    fn test_thought_new() {
        let thought = Thought::new("t-1", "sess-1", "Content", "linear", 0.85);
        assert_eq!(thought.id, "t-1");
        assert_eq!(thought.session_id, "sess-1");
        assert_eq!(thought.content, "Content");
        assert_eq!(thought.mode, "linear");
        assert!((thought.confidence - 0.85).abs() < f64::EPSILON);
        let diff = Utc::now() - thought.created_at;
        assert!(diff.num_seconds() < 1);
    }

    #[test]
    fn test_thought_with_timestamp() {
        let timestamp = Utc::now() - chrono::Duration::hours(1);
        let thought =
            Thought::with_timestamp("t-1", "sess-1", "Content", "linear", 0.85, timestamp);
        assert_eq!(thought.id, "t-1");
        assert_eq!(thought.created_at, timestamp);
    }

    #[test]
    fn test_thought_clone() {
        let thought = Thought::new("t-1", "sess-1", "Content", "linear", 0.85);
        let cloned = thought.clone();
        assert_eq!(thought, cloned);
    }

    #[test]
    fn test_thought_debug() {
        let thought = Thought::new("t-1", "sess-1", "Content", "linear", 0.85);
        let debug = format!("{thought:?}");
        assert!(debug.contains("t-1"));
        assert!(debug.contains("sess-1"));
        assert!(debug.contains("linear"));
    }
}
