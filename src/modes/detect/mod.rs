//! Bias and fallacy detection mode.
//!
//! This mode provides two operations:
//! - `biases`: Detect cognitive biases in reasoning
//! - `fallacies`: Detect logical fallacies in arguments
//!
//! # Output Schema
//!
//! ## Biases Operation
//! - `biases_detected`: List of detected biases with evidence and severity
//! - `overall_assessment`: Summary including bias count and reasoning quality
//! - `debiased_version`: Corrected version of the argument
//!
//! ## Fallacies Operation
//! - `fallacies_detected`: List of logical fallacies with explanations
//! - `argument_structure`: Premises, conclusion, and validity assessment
//! - `overall_assessment`: Summary including fallacy count and argument strength

mod parsing;
mod types;

pub use types::{
    ArgumentStructure, ArgumentValidity, BiasAssessment, BiasSeverity, BiasesResponse,
    DetectedBias, DetectedFallacy, FallaciesResponse, FallacyAssessment, FallacyCategory,
};

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{detect_biases_prompt, detect_fallacies_prompt};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

use parsing::{
    parse_argument_structure, parse_bias_assessment, parse_biases, parse_fallacies,
    parse_fallacy_assessment,
};

// ============================================================================
// DetectMode
// ============================================================================

/// Bias and fallacy detection mode.
///
/// Provides operations to detect cognitive biases and logical fallacies
/// in reasoning and arguments.
///
/// # Example
///
/// ```
/// use mcp_reasoning::modes::{DetectMode, BiasSeverity, FallacyCategory};
/// use mcp_reasoning::doctest_helpers::{MockStorage, MockClient};
///
/// // Create a mode with mock dependencies
/// let mode = DetectMode::new(MockStorage::new(), MockClient::new());
///
/// // Severity and category enums can be used directly
/// assert!(matches!(BiasSeverity::High, BiasSeverity::High));
/// assert!(matches!(FallacyCategory::Informal, FallacyCategory::Informal));
/// ```
pub struct DetectMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> DetectMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new detect mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Detect cognitive biases in content.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to analyze for biases
    /// * `session_id` - Optional session ID for context continuity
    ///
    /// # Returns
    ///
    /// A [`BiasesResponse`] containing detected biases and assessment.
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if:
    /// - Content is empty
    /// - API call fails
    /// - Response parsing fails
    pub async fn biases(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<BiasesResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = detect_biases_prompt();
        let user_message = format!("{prompt}\n\nContent to analyze:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.3); // Lower temp for analytical tasks

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        // Parse biases_detected array
        let biases_detected = parse_biases(&json)?;

        // Parse overall_assessment
        let overall_assessment = parse_bias_assessment(&json)?;

        // Parse debiased_version
        let debiased_version = json
            .get("debiased_version")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ModeError::MissingField {
                field: "debiased_version".to_string(),
            })?
            .to_string();

        // Save thought
        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Bias detection: {} biases found", biases_detected.len()),
            "detect_biases",
            overall_assessment.reasoning_quality,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(BiasesResponse::new(
            thought_id,
            session.id,
            biases_detected,
            overall_assessment,
            debiased_version,
        ))
    }

    /// Detect logical fallacies in content.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to analyze for fallacies
    /// * `session_id` - Optional session ID for context continuity
    ///
    /// # Returns
    ///
    /// A [`FallaciesResponse`] containing detected fallacies and assessment.
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if:
    /// - Content is empty
    /// - API call fails
    /// - Response parsing fails
    pub async fn fallacies(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<FallaciesResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = detect_fallacies_prompt();
        let user_message = format!("{prompt}\n\nContent to analyze:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.3);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        // Parse fallacies_detected array
        let fallacies_detected = parse_fallacies(&json)?;

        // Parse argument_structure
        let argument_structure = parse_argument_structure(&json)?;

        // Parse overall_assessment
        let overall_assessment = parse_fallacy_assessment(&json)?;

        // Save thought
        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Fallacy detection: {} fallacies found",
                fallacies_detected.len()
            ),
            "detect_fallacies",
            overall_assessment.argument_strength,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(FallaciesResponse::new(
            thought_id,
            session.id,
            fallacies_detected,
            argument_structure,
            overall_assessment,
        ))
    }

    // ========================================================================
    // Private Helpers
    // ========================================================================

    async fn get_or_create_session(
        &self,
        session_id: Option<String>,
    ) -> Result<Session, ModeError> {
        self.storage
            .get_or_create_session(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get or create session: {e}"),
            })
    }
}

impl<S, C> std::fmt::Debug for DetectMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetectMode")
            .field("storage", &"<StorageTrait>")
            .field("client", &"<AnthropicClientTrait>")
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::error::StorageError;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    fn mock_biases_response() -> String {
        r#"{
            "biases_detected": [
                {
                    "bias": "Confirmation Bias",
                    "evidence": "Only citing supporting evidence",
                    "severity": "high",
                    "impact": "Ignores contradictory data",
                    "debiasing": "Seek disconfirming evidence"
                }
            ],
            "overall_assessment": {
                "bias_count": 1,
                "most_severe": "Confirmation Bias",
                "reasoning_quality": 0.6
            },
            "debiased_version": "A more balanced view..."
        }"#
        .to_string()
    }

    fn mock_fallacies_response() -> String {
        r#"{
            "fallacies_detected": [
                {
                    "fallacy": "Ad Hominem",
                    "category": "informal",
                    "passage": "You're wrong because you're stupid",
                    "explanation": "Attacks the person, not the argument",
                    "correction": "Address the argument's merits instead"
                }
            ],
            "argument_structure": {
                "premises": ["Premise 1", "Premise 2"],
                "conclusion": "The main conclusion",
                "validity": "invalid"
            },
            "overall_assessment": {
                "fallacy_count": 1,
                "argument_strength": 0.4,
                "most_critical": "Ad Hominem"
            }
        }"#
        .to_string()
    }

    // ========================================================================
    // Biases Operation Tests
    // ========================================================================

    #[tokio::test]
    async fn test_biases_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_biases_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode
            .biases("Some biased content", Some("test-session".to_string()))
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert_eq!(response.biases_detected.len(), 1);
        assert_eq!(response.biases_detected[0].bias, "Confirmation Bias");
        assert_eq!(response.biases_detected[0].severity, BiasSeverity::High);
        assert_eq!(response.overall_assessment.bias_count, 1);
        assert!((response.overall_assessment.reasoning_quality - 0.6).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_biases_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.biases("", None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_biases_api_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiUnavailable {
                message: "API error".to_string(),
            })
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.biases("Test content", None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_biases_invalid_json() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                "Not valid JSON",
                Usage::new(50, 100),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.biases("Test content", None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::JsonParseFailed { .. })));
    }

    #[tokio::test]
    async fn test_biases_missing_field() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        // Missing debiased_version
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"biases_detected": [], "overall_assessment": {"bias_count": 0, "most_severe": "None", "reasoning_quality": 0.9}}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.biases("Test content", None).await;

        assert!(result.is_err());
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "debiased_version")
        );
    }

    #[tokio::test]
    async fn test_biases_invalid_severity() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "biases_detected": [{"bias": "Test", "evidence": "E", "severity": "extreme", "impact": "I", "debiasing": "D"}],
                    "overall_assessment": {"bias_count": 1, "most_severe": "Test", "reasoning_quality": 0.5},
                    "debiased_version": "Fixed"
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.biases("Test content", None).await;

        assert!(result.is_err());
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "severity")
        );
    }

    #[tokio::test]
    async fn test_biases_invalid_reasoning_quality() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "biases_detected": [],
                    "overall_assessment": {"bias_count": 0, "most_severe": "None", "reasoning_quality": 1.5},
                    "debiased_version": "Fixed"
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.biases("Test content", None).await;

        assert!(result.is_err());
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "reasoning_quality")
        );
    }

    #[tokio::test]
    async fn test_biases_storage_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|_| {
            Err(StorageError::ConnectionFailed {
                message: "DB error".to_string(),
            })
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.biases("Test content", None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    // ========================================================================
    // Fallacies Operation Tests
    // ========================================================================

    #[tokio::test]
    async fn test_fallacies_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_fallacies_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode
            .fallacies("Some flawed argument", Some("test-session".to_string()))
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert_eq!(response.fallacies_detected.len(), 1);
        assert_eq!(response.fallacies_detected[0].fallacy, "Ad Hominem");
        assert_eq!(
            response.fallacies_detected[0].category,
            FallacyCategory::Informal
        );
        assert_eq!(
            response.argument_structure.validity,
            ArgumentValidity::Invalid
        );
        assert_eq!(response.overall_assessment.fallacy_count, 1);
    }

    #[tokio::test]
    async fn test_fallacies_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.fallacies("", None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_fallacies_invalid_category() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "fallacies_detected": [{"fallacy": "Test", "category": "unknown", "passage": "P", "explanation": "E", "correction": "C"}],
                    "argument_structure": {"premises": [], "conclusion": "C", "validity": "valid"},
                    "overall_assessment": {"fallacy_count": 1, "argument_strength": 0.5, "most_critical": "Test"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.fallacies("Test content", None).await;

        assert!(result.is_err());
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "category")
        );
    }

    #[tokio::test]
    async fn test_fallacies_invalid_validity() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "fallacies_detected": [],
                    "argument_structure": {"premises": [], "conclusion": "C", "validity": "maybe"},
                    "overall_assessment": {"fallacy_count": 0, "argument_strength": 0.8, "most_critical": "None"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.fallacies("Test content", None).await;

        assert!(result.is_err());
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "validity")
        );
    }

    #[tokio::test]
    async fn test_fallacies_invalid_argument_strength() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "fallacies_detected": [],
                    "argument_structure": {"premises": [], "conclusion": "C", "validity": "valid"},
                    "overall_assessment": {"fallacy_count": 0, "argument_strength": -0.1, "most_critical": "None"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.fallacies("Test content", None).await;

        assert!(result.is_err());
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "argument_strength")
        );
    }

    // ========================================================================
    // Response Type Tests
    // ========================================================================

    #[test]
    fn test_biases_response_new() {
        let response = BiasesResponse::new(
            "t-1",
            "s-1",
            vec![],
            BiasAssessment {
                bias_count: 0,
                most_severe: "None".to_string(),
                reasoning_quality: 0.9,
            },
            "Debiased",
        );
        assert_eq!(response.thought_id, "t-1");
        assert_eq!(response.session_id, "s-1");
        assert!(response.biases_detected.is_empty());
    }

    #[test]
    fn test_fallacies_response_new() {
        let response = FallaciesResponse::new(
            "t-1",
            "s-1",
            vec![],
            ArgumentStructure {
                premises: vec!["P1".to_string()],
                conclusion: "C".to_string(),
                validity: ArgumentValidity::Valid,
            },
            FallacyAssessment {
                fallacy_count: 0,
                argument_strength: 0.9,
                most_critical: "None".to_string(),
            },
        );
        assert_eq!(response.thought_id, "t-1");
        assert_eq!(
            response.argument_structure.validity,
            ArgumentValidity::Valid
        );
    }

    #[test]
    fn test_detect_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = DetectMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("DetectMode"));
    }
}
