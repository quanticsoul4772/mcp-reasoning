//! Bias and fallacy detection mode.
//!
//! This mode provides three operations:
//! - `biases`: Detect cognitive biases in reasoning
//! - `fallacies`: Detect logical fallacies in arguments
//! - `knowledge_gaps`: Find absent information that could change the conclusion
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
//!
//! ## Knowledge Gaps Operation
//! - `gaps`: List of absent information items with category and investigation steps
//! - `unchallenged_assumptions`: Premises taken as given without verification
//! - `overall_assessment`: Summary including gap count and completeness score

mod parsing;
mod types;

pub use types::{
    ArgumentStructure, ArgumentValidity, BiasAssessment, BiasSeverity, BiasesResponse,
    DetectedBias, DetectedFallacy, FallaciesResponse, FallacyAssessment, FallacyCategory,
    FallacySeverity, GapCategory, KnowledgeGap, KnowledgeGapAssessment, KnowledgeGapsResponse,
};

use std::fmt::Write as _;

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{detect_biases_prompt, detect_fallacies_prompt, detect_knowledge_gaps_prompt};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

use parsing::{
    parse_argument_structure, parse_bias_assessment, parse_biases, parse_fallacies,
    parse_fallacy_assessment, parse_knowledge_gap_assessment, parse_knowledge_gaps,
    parse_unchallenged_assumptions,
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

        let has_prior_session = session_id.is_some();
        let session = self.get_or_create_session(session_id).await?;

        let prompt = detect_biases_prompt();
        let user_message = self
            .build_user_message(prompt, content, &session.id, has_prior_session)
            .await;

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(16384)
            .with_temperature(0.3) // Lower temp for analytical tasks
            .with_deep_thinking();

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

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

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

        let has_prior_session = session_id.is_some();
        let session = self.get_or_create_session(session_id).await?;

        let prompt = detect_fallacies_prompt();
        let user_message = self
            .build_user_message(prompt, content, &session.id, has_prior_session)
            .await;

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(16384)
            .with_temperature(0.3)
            .with_deep_thinking();

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

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        Ok(FallaciesResponse::new(
            thought_id,
            session.id,
            fallacies_detected,
            argument_structure,
            overall_assessment,
        ))
    }

    /// Detect knowledge gaps — absent information that could change the conclusion.
    ///
    /// Finds "unknown unknowns": missing data, unchecked assumptions, unexplored
    /// domains, and questions the reasoning never poses. Distinct from bias detection
    /// (cognitive distortions) and fallacy detection (logical errors): this identifies
    /// what is **absent**, not what is flawed.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to analyze for knowledge gaps
    /// * `session_id` - Optional session ID for context continuity
    ///
    /// # Returns
    ///
    /// A [`KnowledgeGapsResponse`] containing detected gaps and assessment.
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if:
    /// - Content is empty
    /// - API call fails
    /// - Response parsing fails
    pub async fn knowledge_gaps(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<KnowledgeGapsResponse, ModeError> {
        validate_content(content)?;

        let has_prior_session = session_id.is_some();
        let session = self.get_or_create_session(session_id).await?;

        let prompt = detect_knowledge_gaps_prompt();
        let user_message = self
            .build_user_message(prompt, content, &session.id, has_prior_session)
            .await;

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(16384)
            .with_temperature(0.3)
            .with_deep_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        // Parse gaps array
        let gaps = parse_knowledge_gaps(&json)?;

        // Parse unchallenged assumptions (optional — returns empty vec if absent)
        let unchallenged_assumptions = parse_unchallenged_assumptions(&json);

        // Parse overall_assessment
        let overall_assessment = parse_knowledge_gap_assessment(&json)?;

        // Save thought
        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Knowledge gap detection: {} gaps found", gaps.len()),
            "detect_knowledge_gaps",
            overall_assessment.completeness_score,
        );

        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Storage write failed — reasoning result preserved, thought not persisted");
        }

        Ok(KnowledgeGapsResponse::new(
            thought_id,
            session.id,
            gaps,
            unchallenged_assumptions,
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

    /// Load recent prior thoughts for a session as a context block, so a
    /// follow-up detection (e.g. "now check the same argument for fallacies")
    /// can build on earlier findings. A lookup failure proceeds without history.
    async fn load_prior_context(&self, session_id: &str) -> String {
        let thoughts = match self.storage.get_thoughts(session_id).await {
            Ok(thoughts) => thoughts,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to load prior thoughts — proceeding without session context"
                );
                return String::new();
            }
        };

        if thoughts.is_empty() {
            return String::new();
        }

        let start = thoughts.len().saturating_sub(MAX_CONTEXT_THOUGHTS);
        let mut block = String::from("Previous reasoning in this session (oldest to newest):\n");
        for (idx, thought) in thoughts[start..].iter().enumerate() {
            let content = truncate_chars(&thought.content, MAX_CONTEXT_THOUGHT_CHARS);
            let _ = writeln!(
                block,
                "{}. [{}, confidence {:.2}] {content}",
                idx + 1,
                thought.mode,
                thought.confidence,
            );
        }
        block
    }

    /// Build the user message for an operation, prepending session history when
    /// an existing session was referenced and has prior reasoning.
    async fn build_user_message(
        &self,
        prompt: &str,
        content: &str,
        session_id: &str,
        has_prior_session: bool,
    ) -> String {
        let prior_context = if has_prior_session {
            self.load_prior_context(session_id).await
        } else {
            String::new()
        };

        if prior_context.is_empty() {
            format!("{prompt}\n\nContent to analyze:\n{content}")
        } else {
            format!("{prompt}\n\n{prior_context}\nContent to analyze:\n{content}")
        }
    }
}

/// Maximum number of prior thoughts to include as session context.
const MAX_CONTEXT_THOUGHTS: usize = 5;
/// Maximum characters per prior thought when building the context block.
const MAX_CONTEXT_THOUGHT_CHARS: usize = 600;

/// Truncate a string to at most `max` characters (char-safe), appending an
/// ellipsis when truncated.
fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{truncated}…")
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
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
                    "confidence": 0.85,
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
                    "severity": "high",
                    "confidence": 0.9,
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
        // A referenced session triggers a prior-thoughts lookup for context.
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

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
        assert!((response.biases_detected[0].confidence - 0.85).abs() < f64::EPSILON);
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
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

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
                conclusion_altering_biases: String::new(),
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

    // ========================================================================
    // Knowledge Gaps Operation Tests
    // ========================================================================

    fn mock_knowledge_gaps_response() -> String {
        r#"{
            "gaps": [
                {
                    "gap": "Market size data",
                    "category": "missing_data",
                    "confidence": 0.8,
                    "impact": "Could invalidate the market opportunity claim",
                    "would_change_conclusion": "yes",
                    "investigation": "Check industry reports for TAM"
                }
            ],
            "unchallenged_assumptions": [
                "Customers will adopt the new feature",
                "Competitors will not respond"
            ],
            "overall_assessment": {
                "gap_count": 1,
                "most_critical": "Market size data",
                "completeness_score": 0.4
            }
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn test_knowledge_gaps_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let response_json = mock_knowledge_gaps_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode
            .knowledge_gaps("Some reasoning content", Some("test-session".to_string()))
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert_eq!(response.gaps.len(), 1);
        assert_eq!(response.gaps[0].gap, "Market size data");
        assert!(matches!(
            response.gaps[0].category,
            GapCategory::MissingData
        ));
        assert_eq!(response.gaps[0].would_change_conclusion, "yes");
        assert_eq!(response.unchallenged_assumptions.len(), 2);
        assert_eq!(response.overall_assessment.gap_count, 1);
        assert!((response.overall_assessment.completeness_score - 0.4).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_knowledge_gaps_empty_content_returns_error() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.knowledge_gaps("", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_knowledge_gaps_storage_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|_| {
            Err(StorageError::SessionNotFound {
                session_id: "test-session".to_string(),
            })
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.knowledge_gaps("Some content", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_knowledge_gaps_invalid_completeness_score() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "gaps": [],
                    "unchallenged_assumptions": [],
                    "overall_assessment": {
                        "gap_count": 0,
                        "most_critical": "None",
                        "completeness_score": 1.5
                    }
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.knowledge_gaps("Test content", None).await;
        assert!(result.is_err());
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "completeness_score")
        );
    }

    #[tokio::test]
    async fn test_biases_injects_prior_session_context() {
        use crate::traits::Thought;

        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("ctx-session")));
        mock_storage.expect_get_thoughts().returning(|_| {
            Ok(vec![Thought::new(
                "t-prev",
                "ctx-session",
                "Earlier: the argument leaned heavily on anecdotal evidence",
                "detect_biases",
                0.7,
            )])
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_biases_response();
        // The prompt sent to the API must carry the prior finding forward.
        mock_client
            .expect_complete()
            .withf(|messages, _| {
                messages.first().is_some_and(|m| {
                    m.content.contains("Previous reasoning in this session")
                        && m.content.contains("anecdotal evidence")
                })
            })
            .returning(move |_, _| {
                Ok(CompletionResponse::new(
                    response_json.clone(),
                    Usage::new(100, 200),
                ))
            });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode
            .biases("Re-examine the argument", Some("ctx-session".to_string()))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_biases_new_session_skips_history_lookup() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        // No session_id → get_thoughts must NOT be called (no expectation set).
        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("fresh")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_biases_response();
        mock_client
            .expect_complete()
            .withf(|messages, _| {
                messages
                    .first()
                    .is_some_and(|m| !m.content.contains("Previous reasoning in this session"))
            })
            .returning(move |_, _| {
                Ok(CompletionResponse::new(
                    response_json.clone(),
                    Usage::new(100, 200),
                ))
            });

        let mode = DetectMode::new(mock_storage, mock_client);
        let result = mode.biases("Fresh content", None).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_truncate_chars() {
        assert_eq!(truncate_chars("short", 10), "short");
        assert_eq!(truncate_chars("12345", 5), "12345");
        assert_eq!(truncate_chars("123456789", 4), "1234…");
    }

    #[test]
    fn test_changes_conclusion_parsed_from_biases() {
        let json: serde_json::Value = serde_json::from_str(&mock_biases_with_changes()).unwrap();
        let biases = parse_biases(&json).unwrap();
        assert_eq!(biases[0].changes_conclusion, "yes");
        let assessment = parse_bias_assessment(&json).unwrap();
        assert_eq!(assessment.conclusion_altering_biases, "Confirmation Bias");
    }

    fn mock_biases_with_changes() -> String {
        r#"{
            "biases_detected": [
                {
                    "bias": "Confirmation Bias",
                    "evidence": "Only citing supporting evidence",
                    "severity": "high",
                    "confidence": 0.85,
                    "changes_conclusion": "yes",
                    "impact": "Ignores contradictory data",
                    "debiasing": "Seek disconfirming evidence"
                }
            ],
            "overall_assessment": {
                "bias_count": 1,
                "most_severe": "Confirmation Bias",
                "conclusion_altering_biases": "Confirmation Bias",
                "reasoning_quality": 0.6
            },
            "debiased_version": "Balanced."
        }"#
        .to_string()
    }
}
