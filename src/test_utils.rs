//! Test utilities and mock factories.
//!
//! This module provides shared testing infrastructure:
//! - Mock implementations for traits
//! - Test fixtures and factories
//! - Common test helpers
//!
//! Only compiled for tests (`#[cfg(test)]`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::error::{ModeError, StorageError};
use crate::traits::{
    CompletionConfig, CompletionResponse, Message, MockAnthropicClientTrait, MockStorageTrait,
    MockTimeProvider, Session, Thought, Usage,
};
use chrono::{DateTime, Utc};

/// Create a mock Anthropic client that returns a fixed response.
///
/// # Arguments
///
/// * `response` - The response content to return
/// * `input_tokens` - Input token count for usage
/// * `output_tokens` - Output token count for usage
///
/// # Example
///
/// ```ignore
/// let mock = mock_anthropic_success("Hello!", 10, 20);
/// let result = mock.complete(messages, config).await;
/// assert_eq!(result.unwrap().content, "Hello!");
/// ```
#[must_use]
pub fn mock_anthropic_success(
    response: impl Into<String>,
    input_tokens: u32,
    output_tokens: u32,
) -> MockAnthropicClientTrait {
    let response = response.into();
    let mut mock = MockAnthropicClientTrait::new();
    mock.expect_complete().returning(move |_msgs, _config| {
        Ok(CompletionResponse::new(
            response.clone(),
            Usage::new(input_tokens, output_tokens),
        ))
    });
    mock
}

/// Create a mock Anthropic client that returns an error.
///
/// # Arguments
///
/// * `error` - The error to return
///
/// # Example
///
/// ```ignore
/// let mock = mock_anthropic_error(ModeError::ApiUnavailable { message: "down".into() });
/// let result = mock.complete(messages, config).await;
/// assert!(result.is_err());
/// ```
#[must_use]
pub fn mock_anthropic_error(error: ModeError) -> MockAnthropicClientTrait {
    let mut mock = MockAnthropicClientTrait::new();
    mock.expect_complete()
        .returning(move |_msgs, _config| Err(error.clone()));
    mock
}

/// Create a mock storage that returns an existing session.
///
/// # Arguments
///
/// * `session_id` - The session ID
///
/// # Example
///
/// ```ignore
/// let mock = mock_storage_with_session("sess-123");
/// let result = mock.get_session("sess-123").await;
/// assert_eq!(result.unwrap().unwrap().id, "sess-123");
/// ```
#[must_use]
pub fn mock_storage_with_session(session_id: impl Into<String>) -> MockStorageTrait {
    let session_id = session_id.into();
    let mut mock = MockStorageTrait::new();
    let session_id_clone = session_id.clone();

    mock.expect_get_session().returning(move |id| {
        if id == session_id_clone {
            Ok(Some(Session::new(id)))
        } else {
            Ok(None)
        }
    });

    let session_id_clone2 = session_id.clone();
    mock.expect_get_or_create_session().returning(move |id| {
        Ok(Session::new(
            id.unwrap_or_else(|| session_id_clone2.clone()),
        ))
    });

    mock.expect_save_thought().returning(|_| Ok(()));
    mock.expect_get_thoughts().returning(|_| Ok(vec![]));

    mock
}

/// Create a mock storage that returns an error on get_session.
///
/// # Arguments
///
/// * `error` - The error to return
///
/// # Example
///
/// ```ignore
/// let mock = mock_storage_error(StorageError::ConnectionFailed { message: "down".into() });
/// let result = mock.get_session("test").await;
/// assert!(result.is_err());
/// ```
#[must_use]
pub fn mock_storage_error(error: StorageError) -> MockStorageTrait {
    let mut mock = MockStorageTrait::new();
    let error_clone = error.clone();
    mock.expect_get_session()
        .returning(move |_| Err(error_clone.clone()));
    mock.expect_get_or_create_session()
        .returning(move |_| Err(error.clone()));
    mock
}

/// Create a mock time provider that returns a fixed timestamp.
///
/// # Arguments
///
/// * `time` - The fixed time to return
///
/// # Example
///
/// ```ignore
/// let fixed_time = Utc::now();
/// let mock = mock_time(fixed_time);
/// assert_eq!(mock.now(), fixed_time);
/// ```
#[must_use]
pub fn mock_time(time: DateTime<Utc>) -> MockTimeProvider {
    let mut mock = MockTimeProvider::new();
    mock.expect_now().return_const(time);
    mock
}

/// Create a mock time provider from an ISO 8601 timestamp string.
///
/// # Arguments
///
/// * `timestamp` - ISO 8601 formatted timestamp string
///
/// # Panics
///
/// Panics if the timestamp string is invalid.
///
/// # Example
///
/// ```ignore
/// let mock = mock_time_str("2024-01-15T12:00:00Z");
/// ```
#[must_use]
pub fn mock_time_str(timestamp: &str) -> MockTimeProvider {
    let time = timestamp
        .parse::<DateTime<Utc>>()
        .expect("Invalid timestamp format");
    mock_time(time)
}

/// Create a test message.
#[must_use]
pub fn test_message(role: &str, content: &str) -> Message {
    Message::new(role, content)
}

/// Create a test user message.
#[must_use]
pub fn test_user_message(content: &str) -> Message {
    Message::user(content)
}

/// Create a test completion config.
#[must_use]
pub fn test_config() -> CompletionConfig {
    CompletionConfig::new()
        .with_max_tokens(1000)
        .with_temperature(0.7)
}

/// Create a test session.
#[must_use]
pub fn test_session(id: &str) -> Session {
    Session::new(id)
}

/// Create a test thought.
#[must_use]
pub fn test_thought(id: &str, session_id: &str, content: &str) -> Thought {
    Thought::new(id, session_id, content, "linear", 0.85)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{AnthropicClientTrait, StorageTrait, TimeProvider};
    use chrono::Datelike;

    #[tokio::test]
    async fn test_mock_anthropic_success() {
        let mock = mock_anthropic_success("Test response", 100, 50);
        let messages = vec![Message::user("Hello")];
        let config = CompletionConfig::new();

        let result = mock.complete(messages, config).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "Test response");
        assert_eq!(response.usage.input_tokens, 100);
        assert_eq!(response.usage.output_tokens, 50);
    }

    #[tokio::test]
    async fn test_mock_anthropic_error() {
        let mock = mock_anthropic_error(ModeError::ApiUnavailable {
            message: "Service down".to_string(),
        });
        let messages = vec![Message::user("Hello")];
        let config = CompletionConfig::new();

        let result = mock.complete(messages, config).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_mock_storage_with_session() {
        let mock = mock_storage_with_session("test-session");

        // Test get_session returns the session
        let result = mock.get_session("test-session").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        // Test get_session returns None for unknown session
        let result = mock.get_session("unknown").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_storage_error() {
        let mock = mock_storage_error(StorageError::ConnectionFailed {
            message: "Database down".to_string(),
        });

        let result = mock.get_session("test").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::ConnectionFailed { .. })));
    }

    #[test]
    fn test_mock_time() {
        let fixed_time = Utc::now() - chrono::Duration::days(1);
        let mock = mock_time(fixed_time);
        assert_eq!(mock.now(), fixed_time);
    }

    #[test]
    fn test_mock_time_str() {
        let mock = mock_time_str("2024-01-15T12:00:00Z");
        let now = mock.now();
        assert_eq!(now.year(), 2024);
        assert_eq!(now.month(), 1);
        assert_eq!(now.day(), 15);
    }

    #[test]
    fn test_test_message() {
        let msg = test_message("assistant", "Hello there");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Hello there");
    }

    #[test]
    fn test_test_user_message() {
        let msg = test_user_message("Hi");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hi");
    }

    #[test]
    fn test_test_config() {
        let config = test_config();
        assert_eq!(config.max_tokens, Some(1000));
        assert!((config.temperature.unwrap_or(0.0) - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_test_session() {
        let session = test_session("sess-123");
        assert_eq!(session.id, "sess-123");
    }

    #[test]
    fn test_test_thought() {
        let thought = test_thought("t-1", "sess-1", "Test content");
        assert_eq!(thought.id, "t-1");
        assert_eq!(thought.session_id, "sess-1");
        assert_eq!(thought.content, "Test content");
        assert_eq!(thought.mode, "linear");
        assert!((thought.confidence - 0.85).abs() < f64::EPSILON);
    }
}
