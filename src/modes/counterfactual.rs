//! Counterfactual causal analysis mode.
//!
//! This mode provides Pearl's Ladder causal analysis:
//! 1. Association (Seeing): What correlates with what?
//! 2. Intervention (Doing): What happens if we change X?
//! 3. Counterfactual (Imagining): What if X had been different?
//!
//! # Output Schema
//!
//! - `causal_question`: The question with ladder rung classification
//! - `causal_model`: Nodes, edges, and confounders
//! - `analysis`: Three-level analysis (association, intervention, counterfactual)
//! - `conclusions`: Causal claims with strength and caveats

#![allow(clippy::missing_const_for_fn)]

use serde::{Deserialize, Serialize};

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::counterfactual_prompt;
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

// ============================================================================
// Response Types
// ============================================================================

/// The three rungs of Pearl's Ladder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LadderRung {
    /// Seeing/observing - P(Y|X).
    Association,
    /// Doing/intervening - P(Y|do(X)).
    Intervention,
    /// Imagining/counterfactual - What if X had been different?
    Counterfactual,
}

/// Variables involved in the causal question.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CausalVariables {
    /// The hypothesized cause.
    pub cause: String,
    /// The outcome of interest.
    pub effect: String,
    /// What change we're considering.
    pub intervention: String,
}

/// The causal question being analyzed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CausalQuestion {
    /// Clear statement of the question.
    pub statement: String,
    /// Which rung of the ladder is relevant.
    pub ladder_rung: LadderRung,
    /// Variables involved.
    pub variables: CausalVariables,
}

/// Type of causal edge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EdgeType {
    /// Direct causal link.
    Direct,
    /// Mediated through another variable.
    Mediated,
    /// Confounded relationship.
    Confounded,
}

/// A causal edge in the model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CausalEdge {
    /// Source variable.
    pub from: String,
    /// Target variable.
    pub to: String,
    /// Type of relationship.
    #[serde(rename = "type")]
    pub edge_type: EdgeType,
}

/// The causal model (DAG).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CausalModel {
    /// Variable names.
    pub nodes: Vec<String>,
    /// Causal edges.
    pub edges: Vec<CausalEdge>,
    /// Variables that affect both cause and effect.
    pub confounders: Vec<String>,
}

/// Association level analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssociationLevel {
    /// Observed correlation.
    pub observed_correlation: f64,
    /// Interpretation of the association.
    pub interpretation: String,
}

/// Intervention level analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InterventionLevel {
    /// Estimated causal effect.
    pub causal_effect: f64,
    /// How the intervention would work.
    pub mechanism: String,
}

/// Counterfactual level analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CounterfactualLevel {
    /// The alternative scenario.
    pub scenario: String,
    /// What would have happened.
    pub outcome: String,
    /// Confidence in counterfactual.
    pub confidence: f64,
}

/// Three-level causal analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CausalAnalysis {
    /// Rung 1: Association.
    pub association_level: AssociationLevel,
    /// Rung 2: Intervention.
    pub intervention_level: InterventionLevel,
    /// Rung 3: Counterfactual.
    pub counterfactual_level: CounterfactualLevel,
}

/// Strength of causal claim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CausalStrength {
    /// Strong causal evidence.
    Strong,
    /// Moderate causal evidence.
    Moderate,
    /// Weak causal evidence.
    Weak,
}

/// Conclusions from causal analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CausalConclusions {
    /// Clear statement of causal relationship.
    pub causal_claim: String,
    /// Strength of evidence.
    pub strength: CausalStrength,
    /// Important qualifications.
    pub caveats: Vec<String>,
    /// What this means for decisions.
    pub actionable_insight: String,
}

/// Response from counterfactual analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CounterfactualResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// The causal question being analyzed.
    pub causal_question: CausalQuestion,
    /// The causal model (DAG).
    pub causal_model: CausalModel,
    /// Three-level analysis.
    pub analysis: CausalAnalysis,
    /// Conclusions and recommendations.
    pub conclusions: CausalConclusions,
}

impl CounterfactualResponse {
    /// Create a new counterfactual response.
    #[must_use]
    // All args are semantic components of causal analysis; builder pattern would obscure structure
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        causal_question: CausalQuestion,
        causal_model: CausalModel,
        analysis: CausalAnalysis,
        conclusions: CausalConclusions,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            causal_question,
            causal_model,
            analysis,
            conclusions,
        }
    }
}

// ============================================================================
// CounterfactualMode
// ============================================================================

/// Counterfactual causal analysis mode.
///
/// Applies Pearl's Ladder for causal reasoning.
pub struct CounterfactualMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> CounterfactualMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new counterfactual mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Perform counterfactual causal analysis.
    ///
    /// # Arguments
    ///
    /// * `content` - The causal question to analyze
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn analyze(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<CounterfactualResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = counterfactual_prompt();
        let user_message = format!("{prompt}\n\nCausal question to analyze:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(32768)
            .with_temperature(0.3)
            .with_maximum_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let causal_question = Self::parse_causal_question(&json)?;
        let causal_model = Self::parse_causal_model(&json)?;
        let analysis = Self::parse_analysis(&json)?;
        let conclusions = Self::parse_conclusions(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Counterfactual analysis: {} ({})",
                causal_question.statement,
                match causal_question.ladder_rung {
                    LadderRung::Association => "association",
                    LadderRung::Intervention => "intervention",
                    LadderRung::Counterfactual => "counterfactual",
                }
            ),
            "counterfactual",
            analysis.counterfactual_level.confidence,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(CounterfactualResponse::new(
            thought_id,
            session.id,
            causal_question,
            causal_model,
            analysis,
            conclusions,
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

    fn parse_causal_question(json: &serde_json::Value) -> Result<CausalQuestion, ModeError> {
        let q = json
            .get("causal_question")
            .ok_or_else(|| ModeError::MissingField {
                field: "causal_question".to_string(),
            })?;

        let statement = Self::get_str(q, "statement")?;
        let ladder_str = Self::get_str(q, "ladder_rung")?;

        let ladder_rung = match ladder_str.to_lowercase().as_str() {
            "association" => LadderRung::Association,
            "intervention" => LadderRung::Intervention,
            "counterfactual" => LadderRung::Counterfactual,
            _ => {
                return Err(ModeError::InvalidValue {
                    field: "ladder_rung".to_string(),
                    reason: format!(
                        "must be association, intervention, or counterfactual, got {ladder_str}"
                    ),
                })
            }
        };

        let vars = q.get("variables").ok_or_else(|| ModeError::MissingField {
            field: "variables".to_string(),
        })?;

        let variables = CausalVariables {
            cause: Self::get_str(vars, "cause")?,
            effect: Self::get_str(vars, "effect")?,
            intervention: Self::get_str(vars, "intervention")?,
        };

        Ok(CausalQuestion {
            statement,
            ladder_rung,
            variables,
        })
    }

    fn parse_causal_model(json: &serde_json::Value) -> Result<CausalModel, ModeError> {
        let m = json
            .get("causal_model")
            .ok_or_else(|| ModeError::MissingField {
                field: "causal_model".to_string(),
            })?;

        let nodes = Self::get_string_array(m, "nodes")?;

        let edges_array = m
            .get("edges")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| ModeError::MissingField {
                field: "edges".to_string(),
            })?;

        let edges: Result<Vec<_>, _> = edges_array
            .iter()
            .map(|e| {
                let from = Self::get_str(e, "from")?;
                let to = Self::get_str(e, "to")?;
                let type_str = Self::get_str(e, "type")?;

                let edge_type = match type_str.to_lowercase().as_str() {
                    "direct" => EdgeType::Direct,
                    "mediated" => EdgeType::Mediated,
                    "confounded" => EdgeType::Confounded,
                    _ => {
                        return Err(ModeError::InvalidValue {
                            field: "type".to_string(),
                            reason: format!(
                                "must be direct, mediated, or confounded, got {type_str}"
                            ),
                        })
                    }
                };

                Ok(CausalEdge {
                    from,
                    to,
                    edge_type,
                })
            })
            .collect();

        let confounders = Self::get_string_array(m, "confounders")?;

        Ok(CausalModel {
            nodes,
            edges: edges?,
            confounders,
        })
    }

    fn parse_analysis(json: &serde_json::Value) -> Result<CausalAnalysis, ModeError> {
        let a = json
            .get("analysis")
            .ok_or_else(|| ModeError::MissingField {
                field: "analysis".to_string(),
            })?;

        let assoc = a
            .get("association_level")
            .ok_or_else(|| ModeError::MissingField {
                field: "association_level".to_string(),
            })?;

        let association_level = AssociationLevel {
            observed_correlation: Self::get_f64(assoc, "observed_correlation")?,
            interpretation: Self::get_str(assoc, "interpretation")?,
        };

        let interv = a
            .get("intervention_level")
            .ok_or_else(|| ModeError::MissingField {
                field: "intervention_level".to_string(),
            })?;

        let intervention_level = InterventionLevel {
            causal_effect: Self::get_f64(interv, "causal_effect")?,
            mechanism: Self::get_str(interv, "mechanism")?,
        };

        let cf = a
            .get("counterfactual_level")
            .ok_or_else(|| ModeError::MissingField {
                field: "counterfactual_level".to_string(),
            })?;

        let confidence = Self::get_f64(cf, "confidence")?;
        if !(0.0..=1.0).contains(&confidence) {
            return Err(ModeError::InvalidValue {
                field: "confidence".to_string(),
                reason: format!("must be between 0.0 and 1.0, got {confidence}"),
            });
        }

        let counterfactual_level = CounterfactualLevel {
            scenario: Self::get_str(cf, "scenario")?,
            outcome: Self::get_str(cf, "outcome")?,
            confidence,
        };

        Ok(CausalAnalysis {
            association_level,
            intervention_level,
            counterfactual_level,
        })
    }

    fn parse_conclusions(json: &serde_json::Value) -> Result<CausalConclusions, ModeError> {
        let c = json
            .get("conclusions")
            .ok_or_else(|| ModeError::MissingField {
                field: "conclusions".to_string(),
            })?;

        let causal_claim = Self::get_str(c, "causal_claim")?;
        let strength_str = Self::get_str(c, "strength")?;

        let strength = match strength_str.to_lowercase().as_str() {
            "strong" => CausalStrength::Strong,
            "moderate" => CausalStrength::Moderate,
            "weak" => CausalStrength::Weak,
            _ => {
                return Err(ModeError::InvalidValue {
                    field: "strength".to_string(),
                    reason: format!("must be strong, moderate, or weak, got {strength_str}"),
                })
            }
        };

        let caveats = Self::get_string_array(c, "caveats")?;
        let actionable_insight = Self::get_str(c, "actionable_insight")?;

        Ok(CausalConclusions {
            causal_claim,
            strength,
            caveats,
            actionable_insight,
        })
    }

    // ========================================================================
    // Utility Helpers
    // ========================================================================

    fn get_str(json: &serde_json::Value, field: &str) -> Result<String, ModeError> {
        json.get(field)
            .and_then(serde_json::Value::as_str)
            .map(String::from)
            .ok_or_else(|| ModeError::MissingField {
                field: field.to_string(),
            })
    }

    fn get_f64(json: &serde_json::Value, field: &str) -> Result<f64, ModeError> {
        json.get(field)
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| ModeError::MissingField {
                field: field.to_string(),
            })
    }

    fn get_string_array(json: &serde_json::Value, field: &str) -> Result<Vec<String>, ModeError> {
        Ok(json
            .get(field)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| ModeError::MissingField {
                field: field.to_string(),
            })?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect())
    }
}

impl<S, C> std::fmt::Debug for CounterfactualMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CounterfactualMode")
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

    fn mock_counterfactual_response() -> String {
        r#"{
            "causal_question": {
                "statement": "Would the patient have survived without treatment?",
                "ladder_rung": "counterfactual",
                "variables": {
                    "cause": "Treatment",
                    "effect": "Survival",
                    "intervention": "Withholding treatment"
                }
            },
            "causal_model": {
                "nodes": ["Treatment", "Survival", "Disease Severity"],
                "edges": [
                    {"from": "Treatment", "to": "Survival", "type": "direct"},
                    {"from": "Disease Severity", "to": "Survival", "type": "direct"}
                ],
                "confounders": ["Disease Severity"]
            },
            "analysis": {
                "association_level": {
                    "observed_correlation": 0.7,
                    "interpretation": "Strong positive correlation between treatment and survival"
                },
                "intervention_level": {
                    "causal_effect": 0.5,
                    "mechanism": "Treatment reduces inflammation"
                },
                "counterfactual_level": {
                    "scenario": "If the patient had not received treatment",
                    "outcome": "Probability of survival would be 0.3 instead of 0.8",
                    "confidence": 0.75
                }
            },
            "conclusions": {
                "causal_claim": "Treatment likely saved the patient",
                "strength": "moderate",
                "caveats": ["Unknown genetic factors", "Small sample size"],
                "actionable_insight": "Treatment should be prioritized for similar cases"
            }
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn test_analyze_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_counterfactual_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode
            .analyze(
                "Would the treatment have helped?",
                Some("test-session".to_string()),
            )
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert_eq!(
            response.causal_question.ladder_rung,
            LadderRung::Counterfactual
        );
        assert_eq!(response.conclusions.strength, CausalStrength::Moderate);
        assert!((response.analysis.counterfactual_level.confidence - 0.75).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_analyze_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("", None).await;

        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_analyze_api_error() {
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

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("Test", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_analyze_invalid_ladder_rung() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "causal_question": {"statement": "S", "ladder_rung": "unknown", "variables": {"cause": "C", "effect": "E", "intervention": "I"}},
                    "causal_model": {"nodes": [], "edges": [], "confounders": []},
                    "analysis": {"association_level": {"observed_correlation": 0.5, "interpretation": "I"}, "intervention_level": {"causal_effect": 0.5, "mechanism": "M"}, "counterfactual_level": {"scenario": "S", "outcome": "O", "confidence": 0.5}},
                    "conclusions": {"causal_claim": "C", "strength": "moderate", "caveats": [], "actionable_insight": "A"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("Test", None).await;

        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "ladder_rung")
        );
    }

    #[tokio::test]
    async fn test_analyze_invalid_edge_type() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "causal_question": {"statement": "S", "ladder_rung": "association", "variables": {"cause": "C", "effect": "E", "intervention": "I"}},
                    "causal_model": {"nodes": ["A"], "edges": [{"from": "A", "to": "B", "type": "unknown"}], "confounders": []},
                    "analysis": {"association_level": {"observed_correlation": 0.5, "interpretation": "I"}, "intervention_level": {"causal_effect": 0.5, "mechanism": "M"}, "counterfactual_level": {"scenario": "S", "outcome": "O", "confidence": 0.5}},
                    "conclusions": {"causal_claim": "C", "strength": "moderate", "caveats": [], "actionable_insight": "A"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("Test", None).await;

        assert!(matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "type"));
    }

    #[tokio::test]
    async fn test_analyze_invalid_confidence() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "causal_question": {"statement": "S", "ladder_rung": "association", "variables": {"cause": "C", "effect": "E", "intervention": "I"}},
                    "causal_model": {"nodes": [], "edges": [], "confounders": []},
                    "analysis": {"association_level": {"observed_correlation": 0.5, "interpretation": "I"}, "intervention_level": {"causal_effect": 0.5, "mechanism": "M"}, "counterfactual_level": {"scenario": "S", "outcome": "O", "confidence": 1.5}},
                    "conclusions": {"causal_claim": "C", "strength": "moderate", "caveats": [], "actionable_insight": "A"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("Test", None).await;

        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "confidence")
        );
    }

    #[tokio::test]
    async fn test_analyze_storage_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|_| {
            Err(StorageError::ConnectionFailed {
                message: "DB error".to_string(),
            })
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("Test", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    // Type tests
    #[test]
    fn test_ladder_rung_serialize() {
        assert_eq!(
            serde_json::to_string(&LadderRung::Association).unwrap(),
            "\"association\""
        );
        assert_eq!(
            serde_json::to_string(&LadderRung::Intervention).unwrap(),
            "\"intervention\""
        );
        assert_eq!(
            serde_json::to_string(&LadderRung::Counterfactual).unwrap(),
            "\"counterfactual\""
        );
    }

    #[test]
    fn test_edge_type_serialize() {
        assert_eq!(
            serde_json::to_string(&EdgeType::Direct).unwrap(),
            "\"direct\""
        );
        assert_eq!(
            serde_json::to_string(&EdgeType::Mediated).unwrap(),
            "\"mediated\""
        );
    }

    #[test]
    fn test_causal_strength_serialize() {
        assert_eq!(
            serde_json::to_string(&CausalStrength::Strong).unwrap(),
            "\"strong\""
        );
    }

    #[test]
    fn test_counterfactual_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("CounterfactualMode"));
    }

    #[test]
    fn test_counterfactual_response_new() {
        let response = CounterfactualResponse::new(
            "t-1",
            "s-1",
            CausalQuestion {
                statement: "Test".to_string(),
                ladder_rung: LadderRung::Counterfactual,
                variables: CausalVariables {
                    cause: "C".to_string(),
                    effect: "E".to_string(),
                    intervention: "I".to_string(),
                },
            },
            CausalModel {
                nodes: vec![],
                edges: vec![],
                confounders: vec![],
            },
            CausalAnalysis {
                association_level: AssociationLevel {
                    observed_correlation: 0.5,
                    interpretation: "I".to_string(),
                },
                intervention_level: InterventionLevel {
                    causal_effect: 0.5,
                    mechanism: "M".to_string(),
                },
                counterfactual_level: CounterfactualLevel {
                    scenario: "S".to_string(),
                    outcome: "O".to_string(),
                    confidence: 0.7,
                },
            },
            CausalConclusions {
                causal_claim: "Claim".to_string(),
                strength: CausalStrength::Moderate,
                caveats: vec![],
                actionable_insight: "Insight".to_string(),
            },
        );
        assert_eq!(response.thought_id, "t-1");
    }

    #[tokio::test]
    async fn test_analyze_with_association_ladder_rung() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "causal_question": {"statement": "Is X correlated with Y?", "ladder_rung": "association", "variables": {"cause": "X", "effect": "Y", "intervention": "observe"}},
                    "causal_model": {"nodes": ["X", "Y"], "edges": [], "confounders": []},
                    "analysis": {"association_level": {"observed_correlation": 0.8, "interpretation": "Strong correlation"}, "intervention_level": {"causal_effect": 0.0, "mechanism": "Unknown"}, "counterfactual_level": {"scenario": "N/A", "outcome": "N/A", "confidence": 0.5}},
                    "conclusions": {"causal_claim": "X and Y are correlated", "strength": "strong", "caveats": [], "actionable_insight": "Investigate further"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("Is X correlated with Y?", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.causal_question.ladder_rung, LadderRung::Association);
        assert_eq!(response.conclusions.strength, CausalStrength::Strong);
    }

    #[tokio::test]
    async fn test_analyze_with_intervention_ladder_rung() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "causal_question": {"statement": "What if we do X?", "ladder_rung": "intervention", "variables": {"cause": "X", "effect": "Y", "intervention": "do X"}},
                    "causal_model": {"nodes": ["X", "Y"], "edges": [{"from": "X", "to": "Y", "type": "mediated"}], "confounders": []},
                    "analysis": {"association_level": {"observed_correlation": 0.5, "interpretation": "Moderate"}, "intervention_level": {"causal_effect": 0.6, "mechanism": "Direct effect"}, "counterfactual_level": {"scenario": "If X done", "outcome": "Y increases", "confidence": 0.7}},
                    "conclusions": {"causal_claim": "Doing X causes Y", "strength": "weak", "caveats": ["Limited data"], "actionable_insight": "Consider doing X"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("What if we do X?", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(
            response.causal_question.ladder_rung,
            LadderRung::Intervention
        );
        assert_eq!(response.conclusions.strength, CausalStrength::Weak);
        // Test mediated edge type was parsed
        assert_eq!(response.causal_model.edges[0].edge_type, EdgeType::Mediated);
    }

    #[tokio::test]
    async fn test_analyze_with_confounded_edge_type() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "causal_question": {"statement": "Test", "ladder_rung": "counterfactual", "variables": {"cause": "X", "effect": "Y", "intervention": "I"}},
                    "causal_model": {"nodes": ["X", "Y", "Z"], "edges": [{"from": "Z", "to": "X", "type": "confounded"}, {"from": "Z", "to": "Y", "type": "confounded"}], "confounders": ["Z"]},
                    "analysis": {"association_level": {"observed_correlation": 0.5, "interpretation": "I"}, "intervention_level": {"causal_effect": 0.5, "mechanism": "M"}, "counterfactual_level": {"scenario": "S", "outcome": "O", "confidence": 0.5}},
                    "conclusions": {"causal_claim": "C", "strength": "moderate", "caveats": [], "actionable_insight": "A"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("Test", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(
            response.causal_model.edges[0].edge_type,
            EdgeType::Confounded
        );
    }

    #[tokio::test]
    async fn test_analyze_invalid_strength() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "causal_question": {"statement": "S", "ladder_rung": "association", "variables": {"cause": "C", "effect": "E", "intervention": "I"}},
                    "causal_model": {"nodes": [], "edges": [], "confounders": []},
                    "analysis": {"association_level": {"observed_correlation": 0.5, "interpretation": "I"}, "intervention_level": {"causal_effect": 0.5, "mechanism": "M"}, "counterfactual_level": {"scenario": "S", "outcome": "O", "confidence": 0.5}},
                    "conclusions": {"causal_claim": "C", "strength": "unknown_strength", "caveats": [], "actionable_insight": "A"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("Test", None).await;

        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "strength")
        );
    }

    #[tokio::test]
    async fn test_analyze_save_thought_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| {
            Err(StorageError::QueryFailed {
                query: "INSERT INTO thoughts".to_string(),
                message: "Save failed".to_string(),
            })
        });

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "causal_question": {"statement": "Test", "ladder_rung": "counterfactual", "variables": {"cause": "C", "effect": "E", "intervention": "I"}},
                    "causal_model": {"nodes": [], "edges": [], "confounders": []},
                    "analysis": {"association_level": {"observed_correlation": 0.5, "interpretation": "I"}, "intervention_level": {"causal_effect": 0.5, "mechanism": "M"}, "counterfactual_level": {"scenario": "S", "outcome": "O", "confidence": 0.5}},
                    "conclusions": {"causal_claim": "C", "strength": "moderate", "caveats": [], "actionable_insight": "A"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = CounterfactualMode::new(mock_storage, mock_client);
        let result = mode.analyze("Test", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[test]
    fn test_edge_type_confounded_serialize() {
        assert_eq!(
            serde_json::to_string(&EdgeType::Confounded).unwrap(),
            "\"confounded\""
        );
    }

    #[test]
    fn test_causal_strength_weak_serialize() {
        assert_eq!(
            serde_json::to_string(&CausalStrength::Weak).unwrap(),
            "\"weak\""
        );
    }
}
