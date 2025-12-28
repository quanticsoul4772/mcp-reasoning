//! MCTS mode response types.
//!
//! This module contains all the structured response types for the MCTS mode:
//! - Explore operation types (FrontierNode, SelectedNode, Expansion, etc.)
//! - Auto backtrack operation types (QualityAssessment, BacktrackDecision, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Response Types - Explore
// ============================================================================

/// A node in the MCTS frontier with UCB1 scores.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrontierNode {
    /// Node identifier.
    pub node_id: String,
    /// Number of visits.
    pub visits: u32,
    /// Average value from simulations.
    pub average_value: f64,
    /// UCB1 score for selection.
    pub ucb1_score: f64,
    /// Exploration bonus component.
    pub exploration_bonus: f64,
}

/// The selected node for expansion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectedNode {
    /// Node identifier.
    pub node_id: String,
    /// Why UCB1 selected this node.
    pub selection_reason: String,
}

/// A newly generated child node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewNode {
    /// Node identifier.
    pub id: String,
    /// The thought content.
    pub content: String,
    /// Simulated value.
    pub simulated_value: f64,
}

/// Expansion results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Expansion {
    /// Newly created nodes.
    pub new_nodes: Vec<NewNode>,
}

/// Backpropagation results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Backpropagation {
    /// Nodes whose stats were updated.
    pub updated_nodes: Vec<String>,
    /// Value changes per node.
    pub value_changes: HashMap<String, f64>,
}

/// Search status summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchStatus {
    /// Total nodes in tree.
    pub total_nodes: u32,
    /// Total simulations run.
    pub total_simulations: u32,
    /// Best path value found.
    pub best_path_value: f64,
}

/// Response from explore operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExploreResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Frontier evaluation.
    pub frontier_evaluation: Vec<FrontierNode>,
    /// Selected node for expansion.
    pub selected_node: SelectedNode,
    /// Expansion results.
    pub expansion: Expansion,
    /// Backpropagation results.
    pub backpropagation: Backpropagation,
    /// Current search status.
    pub search_status: SearchStatus,
}

impl ExploreResponse {
    /// Create a new explore response.
    #[must_use]
    // MCTS exploration requires all components together; splitting would break cohesion
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        frontier_evaluation: Vec<FrontierNode>,
        selected_node: SelectedNode,
        expansion: Expansion,
        backpropagation: Backpropagation,
        search_status: SearchStatus,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            frontier_evaluation,
            selected_node,
            expansion,
            backpropagation,
            search_status,
        }
    }
}

// ============================================================================
// Response Types - Auto Backtrack
// ============================================================================

/// Quality trend direction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QualityTrend {
    /// Quality is declining.
    Declining,
    /// Quality is stable.
    Stable,
    /// Quality is improving.
    Improving,
}

/// Quality assessment of recent search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualityAssessment {
    /// Recent value samples.
    pub recent_values: Vec<f64>,
    /// Overall trend.
    pub trend: QualityTrend,
    /// Magnitude of decline if any.
    pub decline_magnitude: f64,
}

/// Backtrack decision details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BacktrackDecision {
    /// Whether to backtrack.
    pub should_backtrack: bool,
    /// Reason for decision.
    pub reason: String,
    /// Node to return to if backtracking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backtrack_to: Option<String>,
    /// Depth reduction if backtracking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_reduction: Option<u32>,
}

/// Alternative action type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlternativeAction {
    /// Prune unpromising branches.
    Prune,
    /// Refine current approach.
    Refine,
    /// Widen search.
    Widen,
    /// Continue current path.
    Continue,
}

/// An alternative action option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlternativeOption {
    /// Action type.
    pub action: AlternativeAction,
    /// Why this might be appropriate.
    pub rationale: String,
}

/// Recommended action type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecommendedAction {
    /// Backtrack to earlier node.
    Backtrack,
    /// Continue current search.
    Continue,
    /// Terminate search.
    Terminate,
}

/// Final recommendation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recommendation {
    /// Recommended action.
    pub action: RecommendedAction,
    /// Confidence in recommendation.
    pub confidence: f64,
    /// Expected benefit of action.
    pub expected_benefit: String,
}

/// Response from auto backtrack operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BacktrackResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Quality assessment.
    pub quality_assessment: QualityAssessment,
    /// Backtrack decision.
    pub backtrack_decision: BacktrackDecision,
    /// Alternative actions considered.
    pub alternative_actions: Vec<AlternativeOption>,
    /// Final recommendation.
    pub recommendation: Recommendation,
}

impl BacktrackResponse {
    /// Create a new backtrack response.
    #[must_use]
    // Backtrack decision requires all components for coherent recommendation
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        quality_assessment: QualityAssessment,
        backtrack_decision: BacktrackDecision,
        alternative_actions: Vec<AlternativeOption>,
        recommendation: Recommendation,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            quality_assessment,
            backtrack_decision,
            alternative_actions,
            recommendation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_trend_serialize() {
        assert_eq!(
            serde_json::to_string(&QualityTrend::Declining).unwrap(),
            "\"declining\""
        );
    }

    #[test]
    fn test_alternative_action_serialize() {
        assert_eq!(
            serde_json::to_string(&AlternativeAction::Prune).unwrap(),
            "\"prune\""
        );
    }

    #[test]
    fn test_recommended_action_serialize() {
        assert_eq!(
            serde_json::to_string(&RecommendedAction::Backtrack).unwrap(),
            "\"backtrack\""
        );
    }
}
