//! Evidence evaluation mode.
//!
//! This mode provides two operations:
//! - `assess`: Evaluate source credibility and evidence quality
//! - `probabilistic`: Perform Bayesian belief updating
//!
//! # Output Schema
//!
//! ## Assess Operation
//! - `evidence_pieces`: List of evidence with credibility and quality scores
//! - `overall_assessment`: Summary including gaps and weaknesses
//! - `confidence_in_conclusion`: Overall confidence score
//!
//! ## Probabilistic Operation
//! - `hypothesis`: The hypothesis being evaluated
//! - `prior`/`posterior`: Probability distributions with explanations
//! - `belief_update`: Direction and magnitude of belief change

mod parsing;
mod types;

pub use types::{
    AssessResponse, BeliefDirection, BeliefMagnitude, BeliefUpdate, Credibility, EvidenceAnalysis,
    EvidencePiece, EvidenceQuality, OverallEvidenceAssessment, Posterior, Prior,
    ProbabilisticResponse, SourceType,
};

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{evidence_assess_prompt, evidence_probabilistic_prompt};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

use parsing::{
    parse_belief_update, parse_confidence, parse_evidence_analysis, parse_evidence_pieces,
    parse_overall_assessment, parse_posterior, parse_prior,
};

// ============================================================================
// EvidenceMode
// ============================================================================

/// Evidence evaluation mode.
///
/// Provides operations to assess evidence quality and perform Bayesian updates.
pub struct EvidenceMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> EvidenceMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new evidence mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Assess evidence quality and credibility.
    ///
    /// # Arguments
    ///
    /// * `content` - The evidence to assess
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn assess(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<AssessResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = evidence_assess_prompt();
        let user_message = format!("{prompt}\n\nEvidence to assess:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(16384)
            .with_temperature(0.3)
            .with_deep_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let evidence_pieces = parse_evidence_pieces(&json)?;
        let overall_assessment = parse_overall_assessment(&json)?;
        let confidence = parse_confidence(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Evidence assessment: {} pieces, confidence {:.2}",
                evidence_pieces.len(),
                confidence
            ),
            "evidence_assess",
            confidence,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(AssessResponse::new(
            thought_id,
            session.id,
            evidence_pieces,
            overall_assessment,
            confidence,
        ))
    }

    /// Perform Bayesian probability update.
    ///
    /// # Arguments
    ///
    /// * `content` - The hypothesis and evidence to analyze
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn probabilistic(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<ProbabilisticResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = evidence_probabilistic_prompt();
        let user_message = format!("{prompt}\n\nHypothesis and evidence:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(16384)
            .with_temperature(0.3)
            .with_deep_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let hypothesis = json
            .get("hypothesis")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ModeError::MissingField {
                field: "hypothesis".to_string(),
            })?
            .to_string();

        let prior = parse_prior(&json)?;
        let evidence_analysis = parse_evidence_analysis(&json)?;
        let posterior = parse_posterior(&json)?;
        let belief_update = parse_belief_update(&json)?;

        let sensitivity = json
            .get("sensitivity")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ModeError::MissingField {
                field: "sensitivity".to_string(),
            })?
            .to_string();

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Bayesian update: prior {:.2} -> posterior {:.2}",
                prior.probability, posterior.probability
            ),
            "evidence_probabilistic",
            posterior.probability,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(ProbabilisticResponse::new(
            thought_id,
            session.id,
            hypothesis,
            prior,
            evidence_analysis,
            posterior,
            belief_update,
            sensitivity,
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

impl<S, C> std::fmt::Debug for EvidenceMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvidenceMode")
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

    fn mock_assess_response() -> String {
        r#"{
            "evidence_pieces": [
                {
                    "summary": "Research paper on topic",
                    "source_type": "primary",
                    "credibility": {
                        "expertise": 0.9,
                        "objectivity": 0.8,
                        "corroboration": 0.7,
                        "recency": 0.9,
                        "overall": 0.83
                    },
                    "quality": {
                        "relevance": 0.9,
                        "strength": 0.8,
                        "representativeness": 0.7,
                        "overall": 0.8
                    }
                }
            ],
            "overall_assessment": {
                "evidential_support": 0.8,
                "key_strengths": ["Strong primary source"],
                "key_weaknesses": ["Limited sample size"],
                "gaps": ["Need replication"]
            },
            "confidence_in_conclusion": 0.75
        }"#
        .to_string()
    }

    fn mock_probabilistic_response() -> String {
        r#"{
            "hypothesis": "The treatment is effective",
            "prior": {
                "probability": 0.3,
                "basis": "Limited prior evidence"
            },
            "evidence_analysis": [
                {
                    "evidence": "Clinical trial results",
                    "likelihood_if_true": 0.9,
                    "likelihood_if_false": 0.1,
                    "bayes_factor": 9.0
                }
            ],
            "posterior": {
                "probability": 0.79,
                "calculation": "Applied Bayes theorem with strong evidence"
            },
            "belief_update": {
                "direction": "increase",
                "magnitude": "strong",
                "interpretation": "The evidence strongly supports the hypothesis"
            },
            "sensitivity": "Moderately sensitive to prior assumptions"
        }"#
        .to_string()
    }

    // ========================================================================
    // Assess Tests
    // ========================================================================

    #[tokio::test]
    async fn test_assess_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_assess_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode
            .assess("Evidence to assess", Some("test-session".to_string()))
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert_eq!(response.evidence_pieces.len(), 1);
        assert_eq!(response.evidence_pieces[0].source_type, SourceType::Primary);
        assert!((response.confidence_in_conclusion - 0.75).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_assess_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode.assess("", None).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_assess_api_error() {
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

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode.assess("Test", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_assess_invalid_source_type() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "evidence_pieces": [{"summary": "S", "source_type": "unknown", "credibility": {"expertise": 0.5, "objectivity": 0.5, "corroboration": 0.5, "recency": 0.5, "overall": 0.5}, "quality": {"relevance": 0.5, "strength": 0.5, "representativeness": 0.5, "overall": 0.5}}],
                    "overall_assessment": {"evidential_support": 0.5, "key_strengths": [], "key_weaknesses": [], "gaps": []},
                    "confidence_in_conclusion": 0.5
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode.assess("Test", None).await;

        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "source_type")
        );
    }

    #[tokio::test]
    async fn test_assess_invalid_confidence() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "evidence_pieces": [],
                    "overall_assessment": {"evidential_support": 0.5, "key_strengths": [], "key_weaknesses": [], "gaps": []},
                    "confidence_in_conclusion": 1.5
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode.assess("Test", None).await;

        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "confidence_in_conclusion")
        );
    }

    #[tokio::test]
    async fn test_assess_storage_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|_| {
            Err(StorageError::ConnectionFailed {
                message: "DB error".to_string(),
            })
        });

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode.assess("Test", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    // ========================================================================
    // Probabilistic Tests
    // ========================================================================

    #[tokio::test]
    async fn test_probabilistic_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_probabilistic_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode
            .probabilistic("Hypothesis and evidence", Some("test-session".to_string()))
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.hypothesis, "The treatment is effective");
        assert!((response.prior.probability - 0.3).abs() < f64::EPSILON);
        assert!((response.posterior.probability - 0.79).abs() < f64::EPSILON);
        assert_eq!(response.belief_update.direction, BeliefDirection::Increase);
        assert_eq!(response.belief_update.magnitude, BeliefMagnitude::Strong);
    }

    #[tokio::test]
    async fn test_probabilistic_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode.probabilistic("", None).await;

        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_probabilistic_invalid_direction() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "hypothesis": "H",
                    "prior": {"probability": 0.5, "basis": "B"},
                    "evidence_analysis": [],
                    "posterior": {"probability": 0.5, "calculation": "C"},
                    "belief_update": {"direction": "sideways", "magnitude": "strong", "interpretation": "I"},
                    "sensitivity": "S"
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode.probabilistic("Test", None).await;

        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "direction")
        );
    }

    #[tokio::test]
    async fn test_probabilistic_invalid_magnitude() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "hypothesis": "H",
                    "prior": {"probability": 0.5, "basis": "B"},
                    "evidence_analysis": [],
                    "posterior": {"probability": 0.5, "calculation": "C"},
                    "belief_update": {"direction": "increase", "magnitude": "huge", "interpretation": "I"},
                    "sensitivity": "S"
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = EvidenceMode::new(mock_storage, mock_client);
        let result = mode.probabilistic("Test", None).await;

        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "magnitude")
        );
    }

    // ========================================================================
    // Response Type Tests
    // ========================================================================

    #[test]
    fn test_evidence_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = EvidenceMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("EvidenceMode"));
    }

    #[test]
    fn test_assess_response_new() {
        let response = AssessResponse::new(
            "t-1",
            "s-1",
            vec![],
            OverallEvidenceAssessment {
                evidential_support: 0.8,
                key_strengths: vec![],
                key_weaknesses: vec![],
                gaps: vec![],
            },
            0.75,
        );
        assert_eq!(response.thought_id, "t-1");
    }

    #[test]
    fn test_probabilistic_response_new() {
        let response = ProbabilisticResponse::new(
            "t-1",
            "s-1",
            "Hypothesis",
            Prior {
                probability: 0.5,
                basis: "Base".to_string(),
            },
            vec![],
            Posterior {
                probability: 0.8,
                calculation: "Calc".to_string(),
            },
            BeliefUpdate {
                direction: BeliefDirection::Increase,
                magnitude: BeliefMagnitude::Strong,
                interpretation: "Interp".to_string(),
            },
            "Sensitivity",
        );
        assert_eq!(response.hypothesis, "Hypothesis");
    }
}
