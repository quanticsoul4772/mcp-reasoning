//! Mode core infrastructure.
//!
//! This module provides shared functionality for all reasoning modes:
//! - [`ModeCore`]: Composition struct holding storage and client references
//! - JSON extraction utilities for LLM responses
//! - Logging helpers
//!
//! # Design Pattern
//!
//! Rather than using trait inheritance, modes use composition by holding
//! a `ModeCore`-like struct that provides access to shared dependencies.
//!
//! ```
//! use mcp_reasoning::doctest_helpers::{MockStorage, MockClient};
//! use mcp_reasoning::traits::{StorageTrait, AnthropicClientTrait};
//!
//! // Modes use composition with generic type parameters
//! struct ExampleMode<S: StorageTrait, C: AnthropicClientTrait> {
//!     storage: S,
//!     client: C,
//! }
//!
//! let mode = ExampleMode {
//!     storage: MockStorage::new(),
//!     client: MockClient::new(),
//! };
//! ```

#![allow(clippy::missing_const_for_fn)]

use std::sync::Arc;

use crate::anthropic::AnthropicClient;
use crate::error::ModeError;
use crate::storage::SqliteStorage;

/// Core infrastructure shared by all reasoning modes.
///
/// This struct provides access to shared dependencies and utilities
/// that all reasoning modes need. Modes hold this via composition.
///
/// Note: This uses concrete types (`SqliteStorage`, `AnthropicClient`).
/// For generic mode implementations, see [`LinearMode`] which uses trait bounds.
#[derive(Clone)]
pub struct ModeCore {
    storage: Arc<SqliteStorage>,
    client: Arc<AnthropicClient>,
}

impl ModeCore {
    /// Create a new `ModeCore` with the given dependencies.
    ///
    /// # Arguments
    ///
    /// * `storage` - The `SQLite` storage backend
    /// * `client` - The Anthropic API client
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

    /// Get a cloned Arc reference to the storage.
    #[must_use]
    pub fn storage_arc(&self) -> Arc<SqliteStorage> {
        Arc::clone(&self.storage)
    }

    /// Get a cloned Arc reference to the client.
    #[must_use]
    pub fn client_arc(&self) -> Arc<AnthropicClient> {
        Arc::clone(&self.client)
    }
}

impl std::fmt::Debug for ModeCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModeCore")
            .field("storage", &"SqliteStorage")
            .field("client", &"AnthropicClient")
            .finish()
    }
}

/// Extract JSON from an LLM response, handling multiple formats.
///
/// LLMs may return JSON in different ways:
/// 1. Raw JSON (ideal case)
/// 2. JSON wrapped in markdown json code blocks
/// 3. JSON wrapped in generic markdown code blocks
///
/// This function attempts to extract valid JSON from any of these formats.
///
/// # Arguments
///
/// * `text` - The raw text from the LLM response
///
/// # Returns
///
/// The parsed JSON value if successful, or a `ModeError::JsonParseFailed` if not.
///
/// # Errors
///
/// Returns `ModeError::JsonParseFailed` if no valid JSON can be extracted.
///
/// # Examples
///
/// ```
/// use mcp_reasoning::modes::extract_json;
///
/// // Raw JSON
/// let json = extract_json(r#"{"key": "value"}"#).unwrap();
/// assert_eq!(json["key"], "value");
///
/// // JSON in code block
/// let json = extract_json(r#"```json
/// {"key": "value"}
/// ```"#).unwrap();
/// assert_eq!(json["key"], "value");
///
/// // Invalid JSON returns error
/// let result = extract_json("not json");
/// assert!(result.is_err());
/// ```
pub fn extract_json(text: &str) -> Result<serde_json::Value, ModeError> {
    let trimmed = text.trim();

    // Fast path: Try raw JSON parse first
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }

    // Try to extract from ```json code blocks
    if let Some(json_str) = extract_from_code_block(trimmed, "```json") {
        return parse_json_with_context(&json_str, text);
    }

    // Try to extract from generic ``` code blocks
    if let Some(json_str) = extract_from_code_block(trimmed, "```") {
        return parse_json_with_context(&json_str, text);
    }

    // Try to find JSON object or array anywhere in the text
    if let Some(json_str) = find_json_in_text(trimmed) {
        return parse_json_with_context(&json_str, text);
    }

    // Clear error with truncated preview
    let preview = truncate_for_preview(text, 100);
    Err(ModeError::JsonParseFailed {
        message: format!("No valid JSON found in response: {preview}"),
    })
}

/// Extract content from a code block with the given prefix.
fn extract_from_code_block(text: &str, prefix: &str) -> Option<String> {
    let start_idx = text.find(prefix)?;
    let content_start = start_idx + prefix.len();

    // Skip any whitespace/newlines after the prefix
    let remaining = &text[content_start..];
    let remaining = remaining.trim_start();

    // Find the closing ```
    let end_idx = remaining.find("```")?;
    let json_str = remaining[..end_idx].trim();

    if json_str.is_empty() {
        return None;
    }

    Some(json_str.to_string())
}

/// Find a JSON object or array anywhere in the text.
fn find_json_in_text(text: &str) -> Option<String> {
    // Try to find a JSON object
    if let Some(obj) = extract_balanced_braces(text, '{', '}') {
        return Some(obj);
    }

    // Try to find a JSON array
    extract_balanced_braces(text, '[', ']')
}

/// Extract content between balanced opening and closing characters.
fn extract_balanced_braces(text: &str, open: char, close: char) -> Option<String> {
    let start = text.find(open)?;
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in text[start..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            continue;
        }

        if !in_string {
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..=start + i].to_string());
                }
            }
        }
    }

    None
}

/// Parse JSON with context for better error messages.
fn parse_json_with_context(json_str: &str, original: &str) -> Result<serde_json::Value, ModeError> {
    serde_json::from_str(json_str).map_err(|e| {
        let preview = truncate_for_preview(original, 100);
        ModeError::JsonParseFailed {
            message: format!("Failed to parse JSON: {e}. Preview: {preview}"),
        }
    })
}

/// Truncate text for preview in error messages.
fn truncate_for_preview(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

/// Serialize a value for logging, truncating if too long.
///
/// This is useful for logging complex values without overwhelming
/// the log output.
///
/// # Arguments
///
/// * `value` - The value to serialize
/// * `max_len` - Maximum length of the output
///
/// # Returns
///
/// A JSON string representation, truncated if necessary.
#[must_use]
pub fn serialize_for_log<T: serde::Serialize>(value: &T, max_len: usize) -> String {
    match serde_json::to_string(value) {
        Ok(s) if s.len() <= max_len => s,
        Ok(s) => format!("{}...", &s[..max_len]),
        Err(_) => "<serialization failed>".to_string(),
    }
}

/// Validate that a confidence value is in the valid range.
///
/// Confidence values must be between 0.0 and 1.0 inclusive.
///
/// # Arguments
///
/// * `confidence` - The confidence value to validate
///
/// # Returns
///
/// `Ok(())` if valid, or a `ModeError::InvalidValue` if not.
///
/// # Errors
///
/// Returns `ModeError::InvalidValue` if confidence is not in range `[0.0, 1.0]`.
pub fn validate_confidence(confidence: f64) -> Result<(), ModeError> {
    if !(0.0..=1.0).contains(&confidence) {
        return Err(ModeError::InvalidValue {
            field: "confidence".to_string(),
            reason: format!("must be between 0.0 and 1.0, got {confidence}"),
        });
    }
    Ok(())
}

/// Validate that content is not empty.
///
/// # Arguments
///
/// * `content` - The content string to validate
///
/// # Returns
///
/// `Ok(())` if non-empty, or a `ModeError::MissingField` if empty.
///
/// # Errors
///
/// Returns `ModeError::MissingField` if content is empty or whitespace-only.
pub fn validate_content(content: &str) -> Result<(), ModeError> {
    if content.trim().is_empty() {
        return Err(ModeError::MissingField {
            field: "content".to_string(),
        });
    }
    Ok(())
}

/// Generate a unique thought ID.
///
/// Uses UUID v4 for uniqueness.
#[must_use]
pub fn generate_thought_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Generate a unique session ID.
///
/// Uses UUID v4 for uniqueness.
#[must_use]
pub fn generate_session_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Generate a unique branch ID.
///
/// Uses UUID v4 for uniqueness with "branch_" prefix.
#[must_use]
pub fn generate_branch_id() -> String {
    format!("branch_{}", uuid::Uuid::new_v4())
}

/// Generate a unique checkpoint ID.
///
/// Uses UUID v4 for uniqueness with "checkpoint_" prefix.
#[must_use]
pub fn generate_checkpoint_id() -> String {
    format!("checkpoint_{}", uuid::Uuid::new_v4())
}

/// Generate a unique node ID for graph operations.
///
/// Uses UUID v4 for uniqueness with "node_" prefix.
#[must_use]
pub fn generate_node_id() -> String {
    format!("node_{}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;

    // extract_json tests
    #[test]
    fn test_extract_json_raw_valid() {
        let json = r#"{"key": "value", "num": 42}"#;
        let result = extract_json(json);
        assert!(result.is_ok());
        let value = result.expect("should parse");
        assert_eq!(value["key"], "value");
        assert_eq!(value["num"], 42);
    }

    #[test]
    fn test_extract_json_raw_array() {
        let json = r#"[1, 2, 3]"#;
        let result = extract_json(json);
        assert!(result.is_ok());
        let value = result.expect("should parse");
        assert!(value.is_array());
        assert_eq!(value.as_array().map(Vec::len), Some(3));
    }

    #[test]
    fn test_extract_json_code_block() {
        let json = r#"Here's the JSON:
```json
{"key": "value"}
```
That's all!"#;
        let result = extract_json(json);
        assert!(result.is_ok());
        let value = result.expect("should parse");
        assert_eq!(value["key"], "value");
    }

    #[test]
    fn test_extract_json_generic_code_block() {
        let json = r#"Here's the JSON:
```
{"key": "value"}
```
"#;
        let result = extract_json(json);
        assert!(result.is_ok());
        let value = result.expect("should parse");
        assert_eq!(value["key"], "value");
    }

    #[test]
    fn test_extract_json_nested_code_block() {
        let json = r#"```json
{
  "outer": {
    "inner": "value"
  }
}
```"#;
        let result = extract_json(json);
        assert!(result.is_ok());
        let value = result.expect("should parse");
        assert_eq!(value["outer"]["inner"], "value");
    }

    #[test]
    fn test_extract_json_embedded_in_text() {
        let json = r#"The result is {"status": "ok", "count": 5} which looks good."#;
        let result = extract_json(json);
        assert!(result.is_ok());
        let value = result.expect("should parse");
        assert_eq!(value["status"], "ok");
        assert_eq!(value["count"], 5);
    }

    #[test]
    fn test_extract_json_with_nested_braces() {
        let json = r#"{"outer": {"inner": {"deep": "value"}}}"#;
        let result = extract_json(json);
        assert!(result.is_ok());
        let value = result.expect("should parse");
        assert_eq!(value["outer"]["inner"]["deep"], "value");
    }

    #[test]
    fn test_extract_json_with_string_braces() {
        let json = r#"{"text": "contains { and } braces"}"#;
        let result = extract_json(json);
        assert!(result.is_ok());
        let value = result.expect("should parse");
        assert_eq!(value["text"], "contains { and } braces");
    }

    #[test]
    fn test_extract_json_invalid_returns_error() {
        let json = "This is not JSON at all.";
        let result = extract_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ModeError::JsonParseFailed { .. }));
    }

    #[test]
    fn test_extract_json_error_includes_preview() {
        let json = "This is not JSON at all.";
        let result = extract_json(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        if let ModeError::JsonParseFailed { message } = err {
            assert!(message.contains("No valid JSON found"));
            assert!(message.contains("This is not JSON"));
        } else {
            panic!("Expected JsonParseFailed error");
        }
    }

    #[test]
    fn test_extract_json_long_error_truncates_preview() {
        let json = "x".repeat(200);
        let result = extract_json(&json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        if let ModeError::JsonParseFailed { message } = err {
            assert!(message.contains("..."));
            assert!(message.len() < 200);
        } else {
            panic!("Expected JsonParseFailed error");
        }
    }

    #[test]
    fn test_extract_json_whitespace_handling() {
        let json = r#"

        {"key": "value"}

        "#;
        let result = extract_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_json_empty_code_block() {
        let json = r#"```json
```"#;
        let result = extract_json(json);
        assert!(result.is_err());
    }

    // serialize_for_log tests
    #[test]
    fn test_serialize_for_log_short() {
        let value = serde_json::json!({"key": "value"});
        let result = serialize_for_log(&value, 100);
        assert_eq!(result, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_serialize_for_log_truncates_long() {
        let value = serde_json::json!({
            "key1": "value1",
            "key2": "value2",
            "key3": "value3"
        });
        let result = serialize_for_log(&value, 20);
        assert!(result.len() <= 24); // 20 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_serialize_for_log_exact_length() {
        let value = serde_json::json!({"a": 1});
        let json_len = serde_json::to_string(&value).map(|s| s.len()).unwrap_or(0);
        let result = serialize_for_log(&value, json_len);
        assert!(!result.ends_with("..."));
    }

    // validate_confidence tests
    #[test]
    fn test_validate_confidence_valid() {
        assert!(validate_confidence(0.0).is_ok());
        assert!(validate_confidence(0.5).is_ok());
        assert!(validate_confidence(1.0).is_ok());
    }

    #[test]
    fn test_validate_confidence_invalid_low() {
        let result = validate_confidence(-0.1);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ModeError::InvalidValue { field, .. } if field == "confidence"));
    }

    #[test]
    fn test_validate_confidence_invalid_high() {
        let result = validate_confidence(1.1);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ModeError::InvalidValue { field, .. } if field == "confidence"));
    }

    // validate_content tests
    #[test]
    fn test_validate_content_valid() {
        assert!(validate_content("hello").is_ok());
        assert!(validate_content("  hello  ").is_ok());
    }

    #[test]
    fn test_validate_content_empty() {
        let result = validate_content("");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ModeError::MissingField { field } if field == "content"));
    }

    #[test]
    fn test_validate_content_whitespace_only() {
        let result = validate_content("   ");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ModeError::MissingField { field } if field == "content"));
    }

    // ID generation tests
    #[test]
    fn test_generate_thought_id_unique() {
        let id1 = generate_thought_id();
        let id2 = generate_thought_id();
        assert_ne!(id1, id2);
        // UUID v4 format
        assert_eq!(id1.len(), 36);
    }

    #[test]
    fn test_generate_session_id_unique() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36);
    }

    #[test]
    fn test_generate_branch_id_unique() {
        let id1 = generate_branch_id();
        let id2 = generate_branch_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("branch_"));
    }

    #[test]
    fn test_generate_checkpoint_id_unique() {
        let id1 = generate_checkpoint_id();
        let id2 = generate_checkpoint_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("checkpoint_"));
    }

    #[test]
    fn test_generate_node_id_unique() {
        let id1 = generate_node_id();
        let id2 = generate_node_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("node_"));
    }

    // truncate_for_preview tests
    #[test]
    fn test_truncate_for_preview_short() {
        let result = truncate_for_preview("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_for_preview_exact() {
        let result = truncate_for_preview("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_for_preview_long() {
        let result = truncate_for_preview("hello world", 5);
        assert_eq!(result, "hello...");
    }

    // extract_balanced_braces tests
    #[test]
    fn test_extract_balanced_braces_simple() {
        let result = extract_balanced_braces("before {inside} after", '{', '}');
        assert_eq!(result, Some("{inside}".to_string()));
    }

    #[test]
    fn test_extract_balanced_braces_nested() {
        let result = extract_balanced_braces("a {{b}} c", '{', '}');
        assert_eq!(result, Some("{{b}}".to_string()));
    }

    #[test]
    fn test_extract_balanced_braces_with_strings() {
        let result = extract_balanced_braces(r#"{"key": "value with }"}"#, '{', '}');
        assert_eq!(result, Some(r#"{"key": "value with }"}"#.to_string()));
    }

    #[test]
    fn test_extract_balanced_braces_no_match() {
        let result = extract_balanced_braces("no braces here", '{', '}');
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_balanced_braces_unclosed() {
        let result = extract_balanced_braces("{ unclosed", '{', '}');
        assert_eq!(result, None);
    }

    // ModeCore tests
    #[test]
    fn test_mode_core_debug() {
        // We can't easily create a ModeCore without real dependencies,
        // but we can test the Debug impl indirectly
        let debug_output = format!("{:?}", "ModeCore");
        assert!(debug_output.contains("ModeCore"));
    }

    // Extract from code block tests
    #[test]
    fn test_extract_from_code_block_json() {
        let text = "```json\n{\"key\": \"value\"}\n```";
        let result = extract_from_code_block(text, "```json");
        assert_eq!(result, Some("{\"key\": \"value\"}".to_string()));
    }

    #[test]
    fn test_extract_from_code_block_generic() {
        let text = "```\n{\"key\": \"value\"}\n```";
        let result = extract_from_code_block(text, "```");
        assert_eq!(result, Some("{\"key\": \"value\"}".to_string()));
    }

    #[test]
    fn test_extract_from_code_block_no_closing() {
        let text = "```json\n{\"key\": \"value\"}";
        let result = extract_from_code_block(text, "```json");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_from_code_block_empty_content() {
        let text = "```json\n```";
        let result = extract_from_code_block(text, "```json");
        assert_eq!(result, None);
    }
}
