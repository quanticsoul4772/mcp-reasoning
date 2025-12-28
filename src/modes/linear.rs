//! Linear reasoning mode.
//!
//! This mode provides single-pass sequential reasoning with confidence scoring.
//! It processes input content step-by-step and provides a logical continuation.
//!
//! # Output Schema
//!
//! The mode produces a JSON response with:
//! - `analysis`: The detailed step-by-step analysis
//! - `confidence`: A score from 0.0 to 1.0
//! - `next_step`: Suggested next step for further exploration

#![allow(clippy::missing_const_for_fn)]

use serde::{Deserialize, Serialize};

use crate::error::ModeError;
#[cfg(test)]
use crate::modes::generate_session_id;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{get_prompt_for_mode, ReasoningMode};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

/// Response from linear reasoning mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinearResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// The reasoning analysis/continuation.
    pub content: String,
    /// Model's confidence in the reasoning (0.0-1.0).
    pub confidence: f64,
    /// Suggested next reasoning step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_step: Option<String>,
}

impl LinearResponse {
    /// Create a new linear response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        content: impl Into<String>,
        confidence: f64,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            content: content.into(),
            confidence,
            next_step: None,
        }
    }

    /// Add a next step suggestion.
    #[must_use]
    pub fn with_next_step(mut self, next_step: impl Into<String>) -> Self {
        self.next_step = Some(next_step.into());
        self
    }
}

/// Linear reasoning mode.
///
/// Processes content using sequential step-by-step analysis.
///
/// # Example
///
/// ```ignore
/// use mcp_reasoning::modes::LinearMode;
///
/// let mode = LinearMode::new(storage, client);
/// let response = mode.process("Analyze this problem", None, None).await?;
/// println!("Analysis: {}", response.content);
/// println!("Confidence: {}", response.confidence);
/// ```
pub struct LinearMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> LinearMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new linear mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Process content using linear reasoning.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to analyze
    /// * `session_id` - Optional session ID for context continuity
    /// * `min_confidence` - Optional minimum confidence threshold
    ///
    /// # Returns
    ///
    /// A [`LinearResponse`] containing the analysis, confidence score,
    /// and suggested next step.
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if:
    /// - Content is empty
    /// - API call fails
    /// - Response parsing fails
    /// - Storage operation fails
    pub async fn process(
        &self,
        content: &str,
        session_id: Option<String>,
        min_confidence: Option<f64>,
    ) -> Result<LinearResponse, ModeError> {
        // Validate input
        validate_content(content)?;

        // Get or create session
        let session = self.get_or_create_session(session_id).await?;

        // Build the prompt
        let prompt = get_prompt_for_mode(ReasoningMode::Linear, None);
        let user_message = format!("{prompt}\n\nContent to analyze:\n{content}");

        // Call the API
        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.7);

        let response = self.client.complete(messages, config).await?;

        // Parse the response
        let json = extract_json(&response.content)?;

        let analysis = json
            .get("analysis")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModeError::MissingField {
                field: "analysis".to_string(),
            })?
            .to_string();

        let confidence = json
            .get("confidence")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| ModeError::MissingField {
                field: "confidence".to_string(),
            })?;

        // Validate confidence
        if !(0.0..=1.0).contains(&confidence) {
            return Err(ModeError::InvalidValue {
                field: "confidence".to_string(),
                reason: format!("must be between 0.0 and 1.0, got {confidence}"),
            });
        }

        // Check minimum confidence if specified
        if let Some(min) = min_confidence {
            if confidence < min {
                return Err(ModeError::InvalidValue {
                    field: "confidence".to_string(),
                    reason: format!("confidence {confidence} is below minimum threshold {min}"),
                });
            }
        }

        let next_step = json
            .get("next_step")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Generate thought ID and save
        let thought_id = generate_thought_id();
        let thought = Thought::new(&thought_id, &session.id, &analysis, "linear", confidence);

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        // Build response
        let mut response = LinearResponse::new(&thought_id, &session.id, analysis, confidence);
        if let Some(step) = next_step {
            response = response.with_next_step(step);
        }

        Ok(response)
    }

    /// Get or create a session.
    async fn get_or_create_session(
        &self,
        session_id: Option<String>,
    ) -> Result<Session, ModeError> {
        let session = self
            .storage
            .get_or_create_session(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get or create session: {e}"),
            })?;
        Ok(session)
    }
}

impl<S, C> std::fmt::Debug for LinearMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinearMode")
            .field("storage", &"<StorageTrait>")
            .field("client", &"<AnthropicClientTrait>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StorageError;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    fn mock_json_response(analysis: &str, confidence: f64, next_step: Option<&str>) -> String {
        match next_step {
            Some(step) => format!(
                r#"{{"analysis": "{}", "confidence": {}, "next_step": "{}"}}"#,
                analysis, confidence, step
            ),
            None => format!(
                r#"{{"analysis": "{}", "confidence": {}}}"#,
                analysis, confidence
            ),
        }
    }

    #[tokio::test]
    async fn test_linear_process_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        // Setup storage expectations
        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        // Setup client expectations
        let response_json = mock_json_response(
            "Step 1: Analyze the problem. Step 2: Consider solutions.",
            0.85,
            Some("Explore solution A in detail"),
        );
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode
            .process("Test content", Some("test-session".to_string()), None)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert!(response.content.contains("Step 1"));
        assert!((response.confidence - 0.85).abs() < f64::EPSILON);
        assert_eq!(
            response.next_step,
            Some("Explore solution A in detail".to_string())
        );
    }

    #[tokio::test]
    async fn test_linear_process_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("", None, None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_linear_process_whitespace_only_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("   \n\t  ", None, None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_linear_process_api_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiUnavailable {
                message: "API unavailable".to_string(),
            })
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_linear_process_creates_session() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        // Session should be created with generated ID when none provided
        mock_storage
            .expect_get_or_create_session()
            .withf(|id| id.is_none())
            .returning(|_| Ok(Session::new(generate_session_id())));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_json_response("Analysis result", 0.9, None);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        // Session ID should be a valid UUID
        assert!(!response.session_id.is_empty());
    }

    #[tokio::test]
    async fn test_linear_process_with_session() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .withf(|id| id.as_ref().map(|s| s.as_str()) == Some("existing-session"))
            .returning(|id| Ok(Session::new(id.unwrap())));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_json_response("Continued analysis", 0.8, None);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(75, 150),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode
            .process("Test content", Some("existing-session".to_string()), None)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "existing-session");
    }

    #[tokio::test]
    async fn test_linear_process_invalid_json_response() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                "This is not JSON",
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::JsonParseFailed { .. })));
    }

    #[tokio::test]
    async fn test_linear_process_missing_analysis_field() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        // Response missing 'analysis' field
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"confidence": 0.8}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "analysis"));
    }

    #[tokio::test]
    async fn test_linear_process_missing_confidence_field() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        // Response missing 'confidence' field
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"analysis": "Some analysis"}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "confidence"));
    }

    #[tokio::test]
    async fn test_linear_process_invalid_confidence_too_high() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        // Confidence > 1.0
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"analysis": "Some analysis", "confidence": 1.5}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_err());
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "confidence")
        );
    }

    #[tokio::test]
    async fn test_linear_process_invalid_confidence_negative() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        // Confidence < 0.0
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"analysis": "Some analysis", "confidence": -0.1}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_err());
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "confidence")
        );
    }

    #[tokio::test]
    async fn test_linear_process_below_min_confidence() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        let response_json = mock_json_response("Low confidence analysis", 0.5, None);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        // Require minimum confidence of 0.7
        let result = mode.process("Test content", None, Some(0.7)).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::InvalidValue { field, reason })
            if field == "confidence" && reason.contains("below minimum threshold")
        ));
    }

    #[tokio::test]
    async fn test_linear_process_meets_min_confidence() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_json_response("High confidence analysis", 0.85, None);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        // Require minimum confidence of 0.7
        let result = mode.process("Test content", None, Some(0.7)).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!((response.confidence - 0.85).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_linear_process_storage_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|_| {
            Err(StorageError::ConnectionFailed {
                message: "Database unavailable".to_string(),
            })
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_linear_process_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT INTO thoughts".to_string(),
                message: "Insert failed".to_string(),
            })
        });

        let response_json = mock_json_response("Analysis", 0.8, None);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_linear_process_json_in_code_block() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        // Response wrapped in markdown code block
        let response_json = r#"Here's my analysis:
```json
{"analysis": "Code block analysis", "confidence": 0.9, "next_step": "Continue"}
```"#;
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.to_string(),
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "Code block analysis");
        assert!((response.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(response.next_step, Some("Continue".to_string()));
    }

    #[tokio::test]
    async fn test_linear_process_without_next_step() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_json_response("Analysis without next step", 0.75, None);
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(50, 100),
            ))
        });

        let mode = LinearMode::new(mock_storage, mock_client);
        let result = mode.process("Test content", None, None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.next_step.is_none());
    }

    // LinearResponse tests
    #[test]
    fn test_linear_response_new() {
        let response = LinearResponse::new("t-1", "s-1", "Content", 0.8);
        assert_eq!(response.thought_id, "t-1");
        assert_eq!(response.session_id, "s-1");
        assert_eq!(response.content, "Content");
        assert!((response.confidence - 0.8).abs() < f64::EPSILON);
        assert!(response.next_step.is_none());
    }

    #[test]
    fn test_linear_response_with_next_step() {
        let response =
            LinearResponse::new("t-1", "s-1", "Content", 0.8).with_next_step("Next action");
        assert_eq!(response.next_step, Some("Next action".to_string()));
    }

    #[test]
    fn test_linear_response_serialize() {
        let response = LinearResponse::new("t-1", "s-1", "Content", 0.8).with_next_step("Next");
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("thought_id"));
        assert!(json.contains("session_id"));
        assert!(json.contains("content"));
        assert!(json.contains("confidence"));
        assert!(json.contains("next_step"));
    }

    #[test]
    fn test_linear_response_serialize_without_next_step() {
        let response = LinearResponse::new("t-1", "s-1", "Content", 0.8);
        let json = serde_json::to_string(&response).unwrap();
        // next_step should be omitted when None
        assert!(!json.contains("next_step"));
    }

    #[test]
    fn test_linear_response_deserialize() {
        let json = r#"{"thought_id":"t-1","session_id":"s-1","content":"Content","confidence":0.8,"next_step":"Next"}"#;
        let response: LinearResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.thought_id, "t-1");
        assert_eq!(response.session_id, "s-1");
        assert_eq!(response.content, "Content");
        assert!((response.confidence - 0.8).abs() < f64::EPSILON);
        assert_eq!(response.next_step, Some("Next".to_string()));
    }

    #[test]
    fn test_linear_response_clone() {
        let response = LinearResponse::new("t-1", "s-1", "Content", 0.8);
        let cloned = response.clone();
        assert_eq!(response, cloned);
    }

    #[test]
    fn test_linear_response_debug() {
        let response = LinearResponse::new("t-1", "s-1", "Content", 0.8);
        let debug = format!("{response:?}");
        assert!(debug.contains("LinearResponse"));
        assert!(debug.contains("t-1"));
        assert!(debug.contains("s-1"));
    }

    #[test]
    fn test_linear_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = LinearMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("LinearMode"));
        assert!(debug.contains("StorageTrait"));
        assert!(debug.contains("AnthropicClientTrait"));
    }
}
