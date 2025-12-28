//! Decision analysis mode.
//!
//! This mode provides multi-criteria decision analysis:
//! - `weighted`: Weighted multi-criteria analysis
//! - `pairwise`: Direct pairwise comparison
//! - `topsis`: TOPSIS ideal solution method
//! - `perspectives`: Multi-stakeholder perspective analysis
//!
//! # Output Schema
//!
//! ## Weighted Operation
//! - `options`: List of options being compared
//! - `criteria`: Weighted evaluation criteria
//! - `ranking`: Final ranking with weighted scores
//!
//! ## Pairwise Operation
//! - `comparisons`: Each pairwise comparison result
//! - `ranking`: Derived ranking from wins
//!
//! ## TOPSIS Operation
//! - `ideal_solution`: Best values per criterion
//! - `relative_closeness`: Score 0-1 for each option
//! - `ranking`: Options by closeness to ideal
//!
//! ## Perspectives Operation
//! - `stakeholders`: Each stakeholder's view
//! - `conflicts`: Areas of disagreement
//! - `balanced_recommendation`: Synthesized recommendation

mod parsing;
mod types;

pub use types::{
    Alignment, BalancedRecommendation, Conflict, ConflictSeverity, Criterion, CriterionType,
    InfluenceLevel, PairwiseComparison, PairwiseRank, PairwiseResponse, PerspectivesResponse,
    PreferenceResult, PreferenceStrength, RankedOption, Stakeholder, TopsisCreterion,
    TopsisDistances, TopsisRank, TopsisResponse, WeightedResponse,
};

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{
    decision_pairwise_prompt, decision_perspectives_prompt, decision_topsis_prompt,
    decision_weighted_prompt,
};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

// ============================================================================
// DecisionMode
// ============================================================================

/// Decision analysis mode.
///
/// Provides multi-criteria decision analysis with weighted scoring,
/// pairwise comparisons, TOPSIS method, and stakeholder perspectives.
pub struct DecisionMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> DecisionMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new decision mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Perform weighted multi-criteria analysis.
    ///
    /// # Arguments
    ///
    /// * `content` - Decision scenario to analyze
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn weighted(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<WeightedResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = decision_weighted_prompt();
        let user_message = format!("{prompt}\n\nDecision scenario:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.3);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let options = parsing::get_string_array(&json, "options")?;
        let criteria = parsing::parse_criteria(&json)?;
        let scores = parsing::parse_scores(&json)?;
        let weighted_totals = parsing::parse_weighted_totals(&json)?;
        let ranking = parsing::parse_weighted_ranking(&json)?;
        let sensitivity_notes = parsing::get_str(&json, "sensitivity_notes")?;

        let thought_id = generate_thought_id();
        let best_option = ranking.first().map_or("none", |r| r.option.as_str());
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Decision weighted: {} options, best is {}",
                options.len(),
                best_option
            ),
            "decision_weighted",
            ranking.first().map_or(0.0, |r| r.score),
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(WeightedResponse::new(
            thought_id,
            session.id,
            options,
            criteria,
            scores,
            weighted_totals,
            ranking,
            sensitivity_notes,
        ))
    }

    /// Perform pairwise comparison analysis.
    ///
    /// # Arguments
    ///
    /// * `content` - Options to compare pairwise
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn pairwise(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<PairwiseResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = decision_pairwise_prompt();
        let user_message = format!("{prompt}\n\nOptions to compare:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.3);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let comparisons = parsing::parse_comparisons(&json)?;
        let pairwise_matrix = parsing::parse_pairwise_matrix(&json)?;
        let ranking = parsing::parse_pairwise_ranking(&json)?;
        let consistency_check = parsing::get_str(&json, "consistency_check")?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Decision pairwise: {} comparisons", comparisons.len()),
            "decision_pairwise",
            0.8,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(PairwiseResponse::new(
            thought_id,
            session.id,
            comparisons,
            pairwise_matrix,
            ranking,
            consistency_check,
        ))
    }

    /// Apply TOPSIS decision method.
    ///
    /// # Arguments
    ///
    /// * `content` - Decision scenario for TOPSIS
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn topsis(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<TopsisResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = decision_topsis_prompt();
        let user_message = format!("{prompt}\n\nDecision scenario:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.3);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let criteria = parsing::parse_topsis_criteria(&json)?;
        let decision_matrix = parsing::parse_decision_matrix(&json)?;
        let ideal_solution = parsing::parse_f64_array(&json, "ideal_solution")?;
        let anti_ideal_solution = parsing::parse_f64_array(&json, "anti_ideal_solution")?;
        let distances = parsing::parse_distances(&json)?;
        let relative_closeness = parsing::parse_relative_closeness(&json)?;
        let ranking = parsing::parse_topsis_ranking(&json)?;

        let thought_id = generate_thought_id();
        let best_closeness = ranking.first().map_or(0.0, |r| r.closeness);
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Decision TOPSIS: best closeness {best_closeness:.2}"),
            "decision_topsis",
            best_closeness,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(TopsisResponse::new(
            thought_id,
            session.id,
            criteria,
            decision_matrix,
            ideal_solution,
            anti_ideal_solution,
            distances,
            relative_closeness,
            ranking,
        ))
    }

    /// Analyze from multiple stakeholder perspectives.
    ///
    /// # Arguments
    ///
    /// * `content` - Decision scenario to analyze
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn perspectives(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<PerspectivesResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = decision_perspectives_prompt();
        let user_message = format!("{prompt}\n\nDecision scenario:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.4);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let stakeholders = parsing::parse_stakeholders(&json)?;
        let conflicts = parsing::parse_conflicts(&json)?;
        let alignments = parsing::parse_alignments(&json)?;
        let balanced_recommendation = parsing::parse_balanced_recommendation(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Decision perspectives: {} stakeholders, {} conflicts",
                stakeholders.len(),
                conflicts.len()
            ),
            "decision_perspectives",
            0.75,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(PerspectivesResponse::new(
            thought_id,
            session.id,
            stakeholders,
            conflicts,
            alignments,
            balanced_recommendation,
        ))
    }

    // ========================================================================
    // Private Helpers
    // ========================================================================

    async fn get_or_create_session(&self, session_id: Option<String>) -> Result<Session, ModeError> {
        self.storage
            .get_or_create_session(session_id)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to get or create session: {e}"),
            })
    }
}

impl<S, C> std::fmt::Debug for DecisionMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecisionMode")
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

    fn mock_weighted_response() -> String {
        r#"{
            "options": ["Option A", "Option B"],
            "criteria": [
                {"name": "Cost", "weight": 0.4, "description": "Total cost"}
            ],
            "scores": {
                "Option A": {"Cost": 0.8},
                "Option B": {"Cost": 0.6}
            },
            "weighted_totals": {"Option A": 0.8, "Option B": 0.6},
            "ranking": [
                {"option": "Option A", "score": 0.8, "rank": 1}
            ],
            "sensitivity_notes": "Robust to small weight changes"
        }"#
        .to_string()
    }

    fn mock_pairwise_response() -> String {
        r#"{
            "comparisons": [
                {
                    "option_a": "A",
                    "option_b": "B",
                    "preferred": "option_a",
                    "strength": "strong",
                    "reasoning": "A is better"
                }
            ],
            "pairwise_matrix": {"A vs B": 1},
            "ranking": [{"option": "A", "wins": 1, "rank": 1}],
            "consistency_check": "Preferences are transitive"
        }"#
        .to_string()
    }

    fn mock_topsis_response() -> String {
        r#"{
            "criteria": [
                {"name": "Quality", "type": "benefit", "weight": 0.5}
            ],
            "decision_matrix": {"A": [0.8], "B": [0.6]},
            "ideal_solution": [0.8],
            "anti_ideal_solution": [0.6],
            "distances": {
                "A": {"to_ideal": 0.0, "to_anti_ideal": 0.2},
                "B": {"to_ideal": 0.2, "to_anti_ideal": 0.0}
            },
            "relative_closeness": {"A": 1.0, "B": 0.0},
            "ranking": [{"option": "A", "closeness": 1.0, "rank": 1}]
        }"#
        .to_string()
    }

    fn mock_perspectives_response() -> String {
        r#"{
            "stakeholders": [
                {
                    "name": "Customer",
                    "interests": ["Low price"],
                    "preferred_option": "B",
                    "concerns": ["Quality"],
                    "influence_level": "high"
                }
            ],
            "conflicts": [
                {"between": ["Customer", "Vendor"], "issue": "Price", "severity": "medium"}
            ],
            "alignments": [
                {"stakeholders": ["Customer", "Support"], "common_ground": "Quality"}
            ],
            "balanced_recommendation": {
                "option": "A",
                "rationale": "Balances cost and quality",
                "mitigation": "Offer price discount"
            }
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn test_weighted_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let resp = mock_weighted_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200)))
        });

        let mode = DecisionMode::new(mock_storage, mock_client);
        let result = mode.weighted("Compare options", Some("test".to_string())).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.options.len(), 2);
        assert_eq!(response.ranking.len(), 1);
    }

    #[tokio::test]
    async fn test_pairwise_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let resp = mock_pairwise_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200)))
        });

        let mode = DecisionMode::new(mock_storage, mock_client);
        let result = mode.pairwise("A vs B", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.comparisons.len(), 1);
        assert_eq!(response.comparisons[0].preferred, PreferenceResult::OptionA);
    }

    #[tokio::test]
    async fn test_topsis_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let resp = mock_topsis_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200)))
        });

        let mode = DecisionMode::new(mock_storage, mock_client);
        let result = mode.topsis("Decide", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.criteria.len(), 1);
        assert_eq!(response.criteria[0].criterion_type, CriterionType::Benefit);
    }

    #[tokio::test]
    async fn test_perspectives_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let resp = mock_perspectives_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200)))
        });

        let mode = DecisionMode::new(mock_storage, mock_client);
        let result = mode.perspectives("Stakeholder analysis", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.stakeholders.len(), 1);
        assert_eq!(response.conflicts.len(), 1);
    }

    #[tokio::test]
    async fn test_weighted_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = DecisionMode::new(mock_storage, mock_client);
        let result = mode.weighted("", None).await;

        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_storage_error() {
        let mut mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|_| {
            Err(StorageError::ConnectionFailed {
                message: "DB error".to_string(),
            })
        });

        let mode = DecisionMode::new(mock_storage, mock_client);
        let result = mode.weighted("Test", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_invalid_preference() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "comparisons": [{"option_a": "A", "option_b": "B", "preferred": "invalid", "strength": "strong", "reasoning": "R"}],
                    "pairwise_matrix": {},
                    "ranking": [],
                    "consistency_check": "C"
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DecisionMode::new(mock_storage, mock_client);
        let result = mode.pairwise("Test", None).await;

        assert!(matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "preferred"));
    }

    #[tokio::test]
    async fn test_invalid_criterion_type() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "criteria": [{"name": "X", "type": "invalid", "weight": 0.5}],
                    "decision_matrix": {},
                    "ideal_solution": [],
                    "anti_ideal_solution": [],
                    "distances": {},
                    "relative_closeness": {},
                    "ranking": []
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = DecisionMode::new(mock_storage, mock_client);
        let result = mode.topsis("Test", None).await;

        assert!(matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "type"));
    }

    #[test]
    fn test_decision_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = DecisionMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("DecisionMode"));
    }
}
