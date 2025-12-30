//! Monte Carlo Tree Search mode.
//!
//! This mode provides MCTS-based exploration:
//! - `explore`: UCB1-based search with expansion and backpropagation
//! - `auto_backtrack`: Quality monitoring with automatic backtracking
//!
//! # Output Schema
//!
//! ## Explore Operation
//! - `frontier_evaluation`: Current nodes with UCB1 scores
//! - `selected_node`: Node chosen for expansion
//! - `expansion`: New nodes generated
//! - `backpropagation`: Updated node statistics
//!
//! ## Auto Backtrack Operation
//! - `quality_assessment`: Recent value trends
//! - `backtrack_decision`: Whether to backtrack
//! - `alternative_actions`: Other options considered
//! - `recommendation`: Final recommended action

#![allow(clippy::missing_const_for_fn)]

mod parsing;
mod types;

pub use types::{
    AlternativeAction, AlternativeOption, Backpropagation, BacktrackDecision, BacktrackResponse,
    Expansion, ExploreResponse, FrontierNode, NewNode, QualityAssessment, QualityTrend,
    Recommendation, RecommendedAction, SearchStatus, SelectedNode,
};

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{mcts_backtrack_prompt, mcts_explore_prompt};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

use parsing::{
    parse_alternatives, parse_backpropagation, parse_backtrack_decision, parse_expansion,
    parse_frontier, parse_quality_assessment, parse_recommendation, parse_search_status,
    parse_selected,
};

/// Monte Carlo Tree Search mode.
///
/// Provides UCB1-based exploration and automatic backtracking.
pub struct MctsMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> MctsMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new MCTS mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Perform MCTS exploration step.
    ///
    /// # Arguments
    ///
    /// * `content` - Current search state to explore
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn explore(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<ExploreResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = mcts_explore_prompt();
        let user_message = format!("{prompt}\n\nSearch state:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(32768)
            .with_temperature(0.5)
            .with_maximum_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let frontier_evaluation = parse_frontier(&json)?;
        let selected_node = parse_selected(&json)?;
        let expansion = parse_expansion(&json)?;
        let backpropagation = parse_backpropagation(&json)?;
        let search_status = parse_search_status(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "MCTS explore: {} new nodes, best value {:.2}",
                expansion.new_nodes.len(),
                search_status.best_path_value
            ),
            "mcts_explore",
            search_status.best_path_value,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(ExploreResponse::new(
            thought_id,
            session.id,
            frontier_evaluation,
            selected_node,
            expansion,
            backpropagation,
            search_status,
        ))
    }

    /// Evaluate search quality and decide on backtracking.
    ///
    /// # Arguments
    ///
    /// * `content` - Search history to evaluate
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn auto_backtrack(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<BacktrackResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = mcts_backtrack_prompt();
        let user_message = format!("{prompt}\n\nSearch history:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(32768)
            .with_temperature(0.3)
            .with_maximum_thinking();

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let quality_assessment = parse_quality_assessment(&json)?;
        let backtrack_decision = parse_backtrack_decision(&json)?;
        let alternative_actions = parse_alternatives(&json)?;
        let recommendation = parse_recommendation(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "MCTS backtrack: {} (confidence {:.2})",
                match recommendation.action {
                    RecommendedAction::Backtrack => "backtrack",
                    RecommendedAction::Continue => "continue",
                    RecommendedAction::Terminate => "terminate",
                },
                recommendation.confidence
            ),
            "mcts_backtrack",
            recommendation.confidence,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(BacktrackResponse::new(
            thought_id,
            session.id,
            quality_assessment,
            backtrack_decision,
            alternative_actions,
            recommendation,
        ))
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

impl<S, C> std::fmt::Debug for MctsMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MctsMode")
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

    fn mock_explore_response() -> String {
        r#"{
            "frontier_evaluation": [
                {
                    "node_id": "node_1",
                    "visits": 5,
                    "average_value": 0.6,
                    "ucb1_score": 0.85,
                    "exploration_bonus": 0.25
                }
            ],
            "selected_node": {
                "node_id": "node_1",
                "selection_reason": "Highest UCB1 score"
            },
            "expansion": {
                "new_nodes": [
                    {
                        "id": "node_2",
                        "content": "New exploration path",
                        "simulated_value": 0.7
                    }
                ]
            },
            "backpropagation": {
                "updated_nodes": ["node_1"],
                "value_changes": {"node_1": 0.05}
            },
            "search_status": {
                "total_nodes": 10,
                "total_simulations": 50,
                "best_path_value": 0.8
            }
        }"#
        .to_string()
    }

    fn mock_backtrack_response() -> String {
        r#"{
            "quality_assessment": {
                "recent_values": [0.7, 0.65, 0.5, 0.4],
                "trend": "declining",
                "decline_magnitude": 0.3
            },
            "backtrack_decision": {
                "should_backtrack": true,
                "reason": "Sustained quality decline",
                "backtrack_to": "node_3",
                "depth_reduction": 2
            },
            "alternative_actions": [
                {
                    "action": "prune",
                    "rationale": "Remove low-value branches"
                }
            ],
            "recommendation": {
                "action": "backtrack",
                "confidence": 0.8,
                "expected_benefit": "Recover from local minimum"
            }
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn test_explore_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_explore_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = MctsMode::new(mock_storage, mock_client);
        let result = mode
            .explore("Current search state", Some("test-session".to_string()))
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.session_id, "test-session");
        assert_eq!(response.frontier_evaluation.len(), 1);
        assert_eq!(response.selected_node.node_id, "node_1");
        assert_eq!(response.expansion.new_nodes.len(), 1);
    }

    #[tokio::test]
    async fn test_explore_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = MctsMode::new(mock_storage, mock_client);
        let result = mode.explore("", None).await;

        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "content"));
    }

    #[tokio::test]
    async fn test_auto_backtrack_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage.expect_get_or_create_session().returning(|id| {
            Ok(Session::new(
                id.unwrap_or_else(|| "test-session".to_string()),
            ))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let response_json = mock_backtrack_response();
        mock_client.expect_complete().returning(move |_, _| {
            Ok(CompletionResponse::new(
                response_json.clone(),
                Usage::new(100, 200),
            ))
        });

        let mode = MctsMode::new(mock_storage, mock_client);
        let result = mode
            .auto_backtrack("Search history", Some("test-session".to_string()))
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.backtrack_decision.should_backtrack);
        assert_eq!(response.quality_assessment.trend, QualityTrend::Declining);
        assert_eq!(response.recommendation.action, RecommendedAction::Backtrack);
    }

    #[tokio::test]
    async fn test_auto_backtrack_invalid_trend() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "quality_assessment": {"recent_values": [0.5], "trend": "unknown", "decline_magnitude": 0},
                    "backtrack_decision": {"should_backtrack": false, "reason": "R"},
                    "alternative_actions": [],
                    "recommendation": {"action": "continue", "confidence": 0.5, "expected_benefit": "B"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = MctsMode::new(mock_storage, mock_client);
        let result = mode.auto_backtrack("Test", None).await;

        assert!(matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "trend"));
    }

    #[tokio::test]
    async fn test_auto_backtrack_invalid_action() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test-session")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "quality_assessment": {"recent_values": [0.5], "trend": "stable", "decline_magnitude": 0},
                    "backtrack_decision": {"should_backtrack": false, "reason": "R"},
                    "alternative_actions": [],
                    "recommendation": {"action": "unknown", "confidence": 0.5, "expected_benefit": "B"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = MctsMode::new(mock_storage, mock_client);
        let result = mode.auto_backtrack("Test", None).await;

        assert!(matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "action"));
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

        let mode = MctsMode::new(mock_storage, mock_client);
        let result = mode.explore("Test", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[test]
    fn test_mcts_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = MctsMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("MctsMode"));
    }
}
