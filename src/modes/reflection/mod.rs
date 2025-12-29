//! Reflection reasoning mode.
//!
//! This mode provides meta-cognitive evaluation and iterative refinement:
//! - `process`: Analyze and improve reasoning step-by-step
//! - `evaluate`: Comprehensive session-wide assessment

#![allow(clippy::missing_const_for_fn)]

mod parsing;
mod types;

pub use types::{
    EvaluateResponse, Improvement, Priority, ProcessResponse, ReasoningAnalysis, SessionAssessment,
};

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{get_prompt_for_mode, Operation, ReasoningMode};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

use parsing::{parse_analysis, parse_improvements, parse_session_assessment, parse_string_array};

/// Reflection reasoning mode.
///
/// Provides meta-cognitive evaluation and iterative improvement of reasoning.
pub struct ReflectionMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> ReflectionMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new reflection mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Process reasoning for improvement.
    ///
    /// Analyzes the reasoning and suggests improvements.
    ///
    /// # Arguments
    ///
    /// * `content` - The reasoning to analyze and improve
    /// * `session_id` - Optional session ID for context continuity
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn process(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<ProcessResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;
        let prompt = get_prompt_for_mode(ReasoningMode::Reflection, Some(&Operation::Process));

        let user_message = format!("{prompt}\n\nAnalyze and improve this reasoning:\n{content}");
        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(16384)
            .with_temperature(0.7)
            .with_deep_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        // Parse analysis
        let analysis = parse_analysis(&json)?;

        // Parse improvements
        let improvements = parse_improvements(&json)?;

        // Parse refined reasoning
        let refined_reasoning = json
            .get("refined_reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if refined_reasoning.is_empty() {
            return Err(ModeError::MissingField {
                field: "refined_reasoning".to_string(),
            });
        }

        // Parse confidence improvement
        let confidence_improvement = json
            .get("confidence_improvement")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);

        // Generate thought ID and save
        let thought_id = generate_thought_id();
        let thought = Thought::new(&thought_id, &session.id, content, "reflection", 0.8);
        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(ProcessResponse::new(
            thought_id,
            session.id,
            analysis,
            improvements,
            refined_reasoning,
            confidence_improvement,
        ))
    }

    /// Evaluate an entire session.
    ///
    /// Provides comprehensive assessment of all reasoning in the session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID to evaluate
    /// * `summary` - Optional summary of session content (if thoughts are not available)
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if API fails or parsing fails.
    pub async fn evaluate(
        &self,
        session_id: &str,
        summary: Option<&str>,
    ) -> Result<EvaluateResponse, ModeError> {
        let session = self
            .storage
            .get_session(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get session: {e}"),
            })?
            .ok_or_else(|| ModeError::MissingField {
                field: "session_id".to_string(),
            })?;

        // Get thoughts for context
        let thoughts = self.storage.get_thoughts(&session.id).await.map_err(|e| {
            ModeError::ApiUnavailable {
                message: format!("Failed to get thoughts: {e}"),
            }
        })?;

        // Build context from thoughts or use provided summary
        let context = if let Some(s) = summary {
            s.to_string()
        } else if thoughts.is_empty() {
            return Err(ModeError::InvalidValue {
                field: "session".to_string(),
                reason: "no thoughts to evaluate and no summary provided".to_string(),
            });
        } else {
            thoughts
                .iter()
                .map(|t| format!("[{}] {}", t.mode, t.content))
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        let prompt = get_prompt_for_mode(ReasoningMode::Reflection, Some(&Operation::Evaluate));
        let user_message = format!("{prompt}\n\nEvaluate this reasoning session:\n{context}");
        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(16384)
            .with_temperature(0.7)
            .with_deep_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        // Parse session assessment
        let session_assessment = parse_session_assessment(&json)?;

        // Parse string arrays
        let strongest_elements =
            parse_string_array(&json, "strongest_elements").ok_or_else(|| {
                ModeError::MissingField {
                    field: "strongest_elements".to_string(),
                }
            })?;

        let areas_for_improvement =
            parse_string_array(&json, "areas_for_improvement").ok_or_else(|| {
                ModeError::MissingField {
                    field: "areas_for_improvement".to_string(),
                }
            })?;

        let key_insights =
            parse_string_array(&json, "key_insights").ok_or_else(|| ModeError::MissingField {
                field: "key_insights".to_string(),
            })?;

        let recommendations = parse_string_array(&json, "recommendations").ok_or_else(|| {
            ModeError::MissingField {
                field: "recommendations".to_string(),
            }
        })?;

        let meta_observations = json
            .get("meta_observations")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Generate thought ID and save
        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            "Session evaluation",
            "reflection_evaluate",
            session_assessment.average(),
        );
        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        let mut response = EvaluateResponse::new(
            thought_id,
            session.id,
            session_assessment,
            strongest_elements,
            areas_for_improvement,
            key_insights,
            recommendations,
        );

        if let Some(meta) = meta_observations {
            response = response.with_meta_observations(meta);
        }

        Ok(response)
    }

    /// Get or create a session.
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

impl<S, C> std::fmt::Debug for ReflectionMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReflectionMode")
            .field("storage", &"<StorageTrait>")
            .field("client", &"<AnthropicClientTrait>")
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    fn mock_process_response() -> String {
        r#"{
            "analysis": {
                "strengths": ["Clear structure", "Good examples"],
                "weaknesses": ["Missing edge cases", "Limited scope"],
                "gaps": ["No error handling discussed"]
            },
            "improvements": [
                {
                    "issue": "Edge cases not covered",
                    "suggestion": "Add handling for null inputs",
                    "priority": "high"
                },
                {
                    "issue": "Limited scope",
                    "suggestion": "Expand to cover related topics",
                    "priority": "medium"
                }
            ],
            "refined_reasoning": "Improved version of the reasoning with edge cases and broader scope",
            "confidence_improvement": 0.15
        }"#
        .to_string()
    }

    fn mock_evaluate_response() -> String {
        r#"{
            "session_assessment": {
                "overall_quality": 0.85,
                "coherence": 0.9,
                "completeness": 0.75,
                "depth": 0.8
            },
            "strongest_elements": ["Logical flow", "Evidence-based claims"],
            "areas_for_improvement": ["Could explore alternatives", "More examples needed"],
            "key_insights": ["Core insight about the problem", "Novel approach identified"],
            "recommendations": ["Continue with approach A", "Validate with more data"],
            "meta_observations": "The reasoning process showed good metacognitive awareness"
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn test_reflection_process_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_process_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode.process("Test reasoning content", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert_eq!(response.analysis.strengths.len(), 2);
        assert_eq!(response.analysis.weaknesses.len(), 2);
        assert_eq!(response.improvements.len(), 2);
        assert!(!response.refined_reasoning.is_empty());
        assert!((response.confidence_improvement - 0.15).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_reflection_process_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode.process("", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "content"
        ));
    }

    #[tokio::test]
    async fn test_reflection_process_missing_analysis() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"improvements": [], "refined_reasoning": "test"}"#,
                Usage::new(50, 100),
            ))
        });

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "analysis"
        ));
    }

    #[tokio::test]
    async fn test_reflection_process_missing_refined_reasoning() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "analysis": {"strengths": [], "weaknesses": []},
                    "improvements": [],
                    "refined_reasoning": ""
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "refined_reasoning"
        ));
    }

    #[tokio::test]
    async fn test_reflection_process_api_error() {
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

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode.process("Content", None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_reflection_evaluate_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_session()
            .returning(|id| Ok(Some(Session::new(id))));
        mock_storage
            .expect_get_thoughts()
            .returning(|_| Ok(vec![Thought::new("t-1", "s-1", "Content", "linear", 0.8)]));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_evaluate_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode.evaluate("test-session", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert!((response.session_assessment.overall_quality - 0.85).abs() < f64::EPSILON);
        assert_eq!(response.strongest_elements.len(), 2);
        assert_eq!(response.key_insights.len(), 2);
        assert!(response.meta_observations.is_some());
    }

    #[tokio::test]
    async fn test_reflection_evaluate_with_summary() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_session()
            .returning(|id| Ok(Some(Session::new(id))));
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_evaluate_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode
            .evaluate("test-session", Some("Summary of the session"))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reflection_evaluate_session_not_found() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_session().returning(|_| Ok(None));

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode.evaluate("nonexistent", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "session_id"
        ));
    }

    #[tokio::test]
    async fn test_reflection_evaluate_no_thoughts_no_summary() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_session()
            .returning(|id| Ok(Some(Session::new(id))));
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode.evaluate("test-session", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::InvalidValue { field, reason })
            if field == "session" && reason.contains("no thoughts")
        ));
    }

    #[tokio::test]
    async fn test_reflection_evaluate_missing_session_assessment() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_session()
            .returning(|id| Ok(Some(Session::new(id))));
        mock_storage
            .expect_get_thoughts()
            .returning(|_| Ok(vec![Thought::new("t-1", "s-1", "Content", "linear", 0.8)]));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "strongest_elements": [],
                    "areas_for_improvement": [],
                    "key_insights": [],
                    "recommendations": []
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = ReflectionMode::new(mock_storage, mock_client);
        let result = mode.evaluate("test-session", None).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ModeError::MissingField { field }) if field == "session_assessment"
        ));
    }

    #[test]
    fn test_reflection_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = ReflectionMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("ReflectionMode"));
    }
}
