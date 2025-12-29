//! Temporal reasoning mode.
//!
//! This mode provides timeline-based reasoning:
//! - `create`: Create a new timeline with events and decision points
//! - `branch`: Create alternative branches from decision points
//! - `compare`: Compare timeline branches
//! - `merge`: Synthesize insights from multiple branches
//!
//! # Output Schema
//!
//! ## Create Operation
//! - `events`: Ordered events with causal relationships
//! - `decision_points`: Points where choices can be made
//! - `temporal_structure`: Start, current, and horizon
//!
//! ## Branch Operation
//! - `branches`: Alternative futures from decision point
//! - `comparison`: Summary of branch differences
//!
//! ## Compare Operation
//! - `key_differences`: Dimension-by-dimension comparison
//! - `risk_assessment`: Risks per branch
//! - `recommendation`: Preferred branch with conditions
//!
//! ## Merge Operation
//! - `common_patterns`: Patterns across branches
//! - `robust_strategies`: Strategies that work across scenarios
//! - `recommendations`: Actionable next steps

mod parsing;
mod types;

pub use types::{
    BranchComparison, BranchDifference, BranchEvent, BranchPoint, BranchResponse, CommonPattern,
    CompareRecommendation, CompareResponse, CreateTimelineResponse, DecisionPoint, EventType,
    FragileStrategy, MergeResponse, OpportunityAssessment, RiskAssessment, RobustStrategy,
    TemporalStructure, TimelineBranch, TimelineEvent,
};

use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id, validate_content};
use crate::prompts::{
    timeline_branch_prompt, timeline_compare_prompt, timeline_create_prompt, timeline_merge_prompt,
};
use crate::traits::{
    AnthropicClientTrait, CompletionConfig, Message, Session, StorageTrait, Thought,
};

use parsing::{
    get_str, get_string_array, parse_branch_comparison, parse_branch_point, parse_branches,
    parse_common_patterns, parse_compare_recommendation, parse_decision_points, parse_events,
    parse_fragile_strategies, parse_key_differences, parse_opportunity_assessment,
    parse_risk_assessment, parse_robust_strategies, parse_temporal_structure,
};

// ============================================================================
// TimelineMode
// ============================================================================

/// Temporal reasoning mode.
///
/// Provides timeline creation, branching, comparison, and merging.
pub struct TimelineMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    storage: S,
    client: C,
}

impl<S, C> TimelineMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new timeline mode instance.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Create a new timeline.
    ///
    /// # Arguments
    ///
    /// * `content` - Scenario to create timeline for
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn create(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<CreateTimelineResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = timeline_create_prompt();
        let user_message = format!("{prompt}\n\nScenario:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.4);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let timeline_id = get_str(&json, "timeline_id")?;
        let events = parse_events(&json)?;
        let decision_points = parse_decision_points(&json)?;
        let temporal_structure = parse_temporal_structure(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Timeline create: {} events, {} decision points",
                events.len(),
                decision_points.len()
            ),
            "timeline_create",
            0.8,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(CreateTimelineResponse::new(
            thought_id,
            session.id,
            timeline_id,
            events,
            decision_points,
            temporal_structure,
        ))
    }

    /// Create timeline branches from a decision point.
    ///
    /// # Arguments
    ///
    /// * `content` - Decision point to branch from
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn branch(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<BranchResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = timeline_branch_prompt();
        let user_message = format!("{prompt}\n\nDecision point:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.5);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let branch_point = parse_branch_point(&json)?;
        let branches = parse_branches(&json)?;
        let comparison = parse_branch_comparison(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Timeline branch: {} branches", branches.len()),
            "timeline_branch",
            0.75,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(BranchResponse::new(
            thought_id,
            session.id,
            branch_point,
            branches,
            comparison,
        ))
    }

    /// Compare timeline branches.
    ///
    /// # Arguments
    ///
    /// * `content` - Branches to compare
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn compare(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<CompareResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = timeline_compare_prompt();
        let user_message = format!("{prompt}\n\nBranches to compare:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.3);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let branches_compared = get_string_array(&json, "branches_compared")?;
        let divergence_point = get_str(&json, "divergence_point")?;
        let key_differences = parse_key_differences(&json)?;
        let risk_assessment = parse_risk_assessment(&json)?;
        let opportunity_assessment = parse_opportunity_assessment(&json)?;
        let recommendation = parse_compare_recommendation(&json)?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!("Timeline compare: {} differences", key_differences.len()),
            "timeline_compare",
            0.8,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(CompareResponse::new(
            thought_id,
            session.id,
            branches_compared,
            divergence_point,
            key_differences,
            risk_assessment,
            opportunity_assessment,
            recommendation,
        ))
    }

    /// Merge timeline branches to synthesize insights.
    ///
    /// # Arguments
    ///
    /// * `content` - Branch exploration to merge
    /// * `session_id` - Optional session ID
    ///
    /// # Errors
    ///
    /// Returns [`ModeError`] if content is empty, API fails, or parsing fails.
    pub async fn merge(
        &self,
        content: &str,
        session_id: Option<String>,
    ) -> Result<MergeResponse, ModeError> {
        validate_content(content)?;

        let session = self.get_or_create_session(session_id).await?;

        let prompt = timeline_merge_prompt();
        let user_message = format!("{prompt}\n\nBranch exploration:\n{content}");

        let messages = vec![Message::user(user_message)];
        let config = CompletionConfig::new()
            .with_max_tokens(4096)
            .with_temperature(0.3);

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let branches_merged = get_string_array(&json, "branches_merged")?;
        let common_patterns = parse_common_patterns(&json)?;
        let robust_strategies = parse_robust_strategies(&json)?;
        let fragile_strategies = parse_fragile_strategies(&json)?;
        let synthesis = get_str(&json, "synthesis")?;
        let recommendations = get_string_array(&json, "recommendations")?;

        let thought_id = generate_thought_id();
        let thought = Thought::new(
            &thought_id,
            &session.id,
            format!(
                "Timeline merge: {} patterns, {} robust strategies",
                common_patterns.len(),
                robust_strategies.len()
            ),
            "timeline_merge",
            0.85,
        );

        self.storage
            .save_thought(&thought)
            .await
            .map_err(|e| ModeError::ApiUnavailable {
                message: format!("Failed to save thought: {e}"),
            })?;

        Ok(MergeResponse::new(
            thought_id,
            session.id,
            branches_merged,
            common_patterns,
            robust_strategies,
            fragile_strategies,
            synthesis,
            recommendations,
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

impl<S, C> std::fmt::Debug for TimelineMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimelineMode")
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

    fn mock_create_response() -> String {
        r#"{
            "timeline_id": "tl_1",
            "events": [
                {
                    "id": "e1",
                    "description": "Start",
                    "time": "T0",
                    "type": "event",
                    "causes": [],
                    "effects": ["e2"]
                }
            ],
            "decision_points": [
                {
                    "id": "d1",
                    "description": "Choose path",
                    "options": ["A", "B"],
                    "deadline": "T1"
                }
            ],
            "temporal_structure": {
                "start": "e1",
                "current": "e1",
                "horizon": "1 year"
            }
        }"#
        .to_string()
    }

    fn mock_branch_response() -> String {
        r#"{
            "branch_point": {
                "event_id": "d1",
                "description": "Choose path"
            },
            "branches": [
                {
                    "id": "b1",
                    "choice": "Option A",
                    "events": [
                        {"id": "be1", "description": "Result A", "probability": 0.8, "time_offset": "1 month"}
                    ],
                    "plausibility": 0.7,
                    "outcome_quality": 0.6
                }
            ],
            "comparison": {
                "most_likely_good_outcome": "b1",
                "highest_risk": "b2",
                "key_differences": ["Cost vs speed"]
            }
        }"#
        .to_string()
    }

    fn mock_compare_response() -> String {
        r#"{
            "branches_compared": ["b1", "b2"],
            "divergence_point": "d1",
            "key_differences": [
                {
                    "dimension": "Cost",
                    "branch_1_value": "High",
                    "branch_2_value": "Low",
                    "significance": "Budget impact"
                }
            ],
            "risk_assessment": {
                "branch_1_risks": ["Over budget"],
                "branch_2_risks": ["Delays"]
            },
            "opportunity_assessment": {
                "branch_1_opportunities": ["Quality"],
                "branch_2_opportunities": ["Speed"]
            },
            "recommendation": {
                "preferred_branch": "b1",
                "conditions": "If budget allows",
                "key_factors": "Quality priority"
            }
        }"#
        .to_string()
    }

    fn mock_merge_response() -> String {
        r#"{
            "branches_merged": ["b1", "b2"],
            "common_patterns": [
                {"pattern": "Quality matters", "frequency": 0.9, "implications": "Invest in QA"}
            ],
            "robust_strategies": [
                {"strategy": "Iterative approach", "effectiveness": 0.8, "conditions": "Complex projects"}
            ],
            "fragile_strategies": [
                {"strategy": "Big bang release", "failure_modes": "Integration failures"}
            ],
            "synthesis": "Prefer iterative over big bang",
            "recommendations": ["Start small", "Iterate often"]
        }"#
        .to_string()
    }

    #[tokio::test]
    async fn test_create_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let resp = mock_create_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = TimelineMode::new(mock_storage, mock_client);
        let result = mode.create("Scenario", Some("test".to_string())).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.timeline_id, "tl_1");
        assert_eq!(response.events.len(), 1);
    }

    #[tokio::test]
    async fn test_branch_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let resp = mock_branch_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = TimelineMode::new(mock_storage, mock_client);
        let result = mode.branch("Decision", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.branches.len(), 1);
    }

    #[tokio::test]
    async fn test_compare_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let resp = mock_compare_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = TimelineMode::new(mock_storage, mock_client);
        let result = mode.compare("Branches", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.key_differences.len(), 1);
    }

    #[tokio::test]
    async fn test_merge_success() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "test".to_string()))));
        mock_storage.expect_save_thought().returning(|_| Ok(()));

        let resp = mock_merge_response();
        mock_client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(resp.clone(), Usage::new(100, 200))));

        let mode = TimelineMode::new(mock_storage, mock_client);
        let result = mode.merge("Exploration", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.robust_strategies.len(), 1);
    }

    #[tokio::test]
    async fn test_create_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();

        let mode = TimelineMode::new(mock_storage, mock_client);
        let result = mode.create("", None).await;

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

        let mode = TimelineMode::new(mock_storage, mock_client);
        let result = mode.create("Test", None).await;

        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    #[tokio::test]
    async fn test_invalid_event_type() {
        let mut mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();

        mock_storage
            .expect_get_or_create_session()
            .returning(|_| Ok(Session::new("test")));

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{
                    "timeline_id": "t1",
                    "events": [{"id": "e1", "description": "D", "time": "T", "type": "invalid", "causes": [], "effects": []}],
                    "decision_points": [],
                    "temporal_structure": {"start": "e1", "current": "e1", "horizon": "1d"}
                }"#,
                Usage::new(50, 100),
            ))
        });

        let mode = TimelineMode::new(mock_storage, mock_client);
        let result = mode.create("Test", None).await;

        assert!(matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "type"));
    }

    #[test]
    fn test_timeline_mode_debug() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let mode = TimelineMode::new(mock_storage, mock_client);
        let debug = format!("{mode:?}");
        assert!(debug.contains("TimelineMode"));
    }
}
