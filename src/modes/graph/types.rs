//! Graph mode types.
//!
//! Shared types for graph-of-thoughts operations.

use serde::{Deserialize, Serialize};

// ============================================================================
// Common Types
// ============================================================================

/// Type of node in the graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// Root node.
    Root,
    /// Reasoning step.
    Reasoning,
    /// Evidence node.
    Evidence,
    /// Hypothesis node.
    Hypothesis,
    /// Conclusion node.
    Conclusion,
    /// Synthesis node.
    Synthesis,
    /// Refined node.
    Refined,
}

/// Relationship between nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeRelationship {
    /// Elaborates on parent.
    Elaborates,
    /// Supports parent claim.
    Supports,
    /// Challenges parent claim.
    Challenges,
    /// Synthesizes multiple inputs.
    Synthesizes,
}

/// Complexity level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComplexityLevel {
    /// Low complexity.
    Low,
    /// Medium complexity.
    Medium,
    /// High complexity.
    High,
}

/// Recommendation for a node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeRecommendation {
    /// Expand this node.
    Expand,
    /// Keep this node as is.
    Keep,
    /// Prune this node.
    Prune,
}

/// Reason for pruning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PruneReason {
    /// Low score.
    LowScore,
    /// Redundant with other nodes.
    Redundant,
    /// Dead end in reasoning.
    DeadEnd,
    /// Off topic.
    OffTopic,
}

/// Impact level of pruning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PruneImpact {
    /// No impact.
    None,
    /// Minor impact.
    Minor,
    /// Moderate impact.
    Moderate,
}

/// Suggested action for a frontier node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SuggestedAction {
    /// Expand the node.
    Expand,
    /// Refine the node.
    Refine,
    /// Aggregate with other nodes.
    Aggregate,
}

// ============================================================================
// Init Types
// ============================================================================

/// Root node of a graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RootNode {
    /// Node identifier.
    pub id: String,
    /// Content of the root.
    pub content: String,
    /// Initial score.
    pub score: f64,
    /// Node type (always root).
    #[serde(rename = "type")]
    pub node_type: NodeType,
}

/// A direction for expansion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExpansionDirection {
    /// Description of the direction.
    pub direction: String,
    /// Potential score.
    pub potential: f64,
}

/// Metadata about the graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphMetadata {
    /// Complexity level.
    pub complexity: ComplexityLevel,
    /// Estimated depth.
    pub estimated_depth: u32,
}

/// Response from init operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InitResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Root node.
    pub root: RootNode,
    /// Expansion directions.
    pub expansion_directions: Vec<ExpansionDirection>,
    /// Graph metadata.
    pub graph_metadata: GraphMetadata,
}

impl InitResponse {
    /// Create a new init response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        root: RootNode,
        expansion_directions: Vec<ExpansionDirection>,
        graph_metadata: GraphMetadata,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            root,
            expansion_directions,
            graph_metadata,
        }
    }
}

// ============================================================================
// Generate Types
// ============================================================================

/// A generated child node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChildNode {
    /// Node identifier.
    pub id: String,
    /// Content of the node.
    pub content: String,
    /// Score.
    pub score: f64,
    /// Type of node.
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Relationship to parent.
    pub relationship: NodeRelationship,
}

/// Response from generate operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerateResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Parent node ID.
    pub parent_id: String,
    /// Generated children.
    pub children: Vec<ChildNode>,
    /// Notes about generation.
    pub generation_notes: String,
}

impl GenerateResponse {
    /// Create a new generate response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        parent_id: impl Into<String>,
        children: Vec<ChildNode>,
        generation_notes: impl Into<String>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            parent_id: parent_id.into(),
            children,
            generation_notes: generation_notes.into(),
        }
    }
}

// ============================================================================
// Score Types
// ============================================================================

/// Scores for a node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeScores {
    /// Relevance score.
    pub relevance: f64,
    /// Coherence score.
    pub coherence: f64,
    /// Depth score.
    pub depth: f64,
    /// Novelty score.
    pub novelty: f64,
    /// Overall score.
    pub overall: f64,
}

/// Assessment of a node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeAssessment {
    /// Strengths.
    pub strengths: Vec<String>,
    /// Weaknesses.
    pub weaknesses: Vec<String>,
    /// Recommendation.
    pub recommendation: NodeRecommendation,
}

/// Response from score operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Node being scored.
    pub node_id: String,
    /// Scores.
    pub scores: NodeScores,
    /// Assessment.
    pub assessment: NodeAssessment,
}

impl ScoreResponse {
    /// Create a new score response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        node_id: impl Into<String>,
        scores: NodeScores,
        assessment: NodeAssessment,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            node_id: node_id.into(),
            scores,
            assessment,
        }
    }
}

// ============================================================================
// Aggregate Types
// ============================================================================

/// A synthesis node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SynthesisNode {
    /// Node identifier.
    pub id: String,
    /// Synthesized content.
    pub content: String,
    /// Score.
    pub score: f64,
    /// Type (always synthesis).
    #[serde(rename = "type")]
    pub node_type: NodeType,
}

/// Notes about integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrationNotes {
    /// Common themes.
    pub common_themes: Vec<String>,
    /// Complementary aspects.
    pub complementary_aspects: Vec<String>,
    /// Resolved contradictions.
    pub resolved_contradictions: Vec<String>,
}

/// Response from aggregate operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Input node IDs.
    pub input_node_ids: Vec<String>,
    /// Synthesis result.
    pub synthesis: SynthesisNode,
    /// Integration notes.
    pub integration_notes: IntegrationNotes,
}

impl AggregateResponse {
    /// Create a new aggregate response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        input_node_ids: Vec<String>,
        synthesis: SynthesisNode,
        integration_notes: IntegrationNotes,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            input_node_ids,
            synthesis,
            integration_notes,
        }
    }
}

// ============================================================================
// Refine Types
// ============================================================================

/// Critique of a node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeCritique {
    /// Issues found.
    pub issues: Vec<String>,
    /// Missing elements.
    pub missing_elements: Vec<String>,
    /// Unclear aspects.
    pub unclear_aspects: Vec<String>,
}

/// A refined node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefinedNode {
    /// Node identifier.
    pub id: String,
    /// Refined content.
    pub content: String,
    /// New score.
    pub score: f64,
    /// Type (always refined).
    #[serde(rename = "type")]
    pub node_type: NodeType,
}

/// Response from refine operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RefineResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Original node ID.
    pub original_node_id: String,
    /// Critique.
    pub critique: NodeCritique,
    /// Refined node.
    pub refined_node: RefinedNode,
    /// Improvement delta.
    pub improvement_delta: f64,
}

impl RefineResponse {
    /// Create a new refine response.
    #[must_use]
    // Refinement requires all critique and improvement components together
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        original_node_id: impl Into<String>,
        critique: NodeCritique,
        refined_node: RefinedNode,
        improvement_delta: f64,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            original_node_id: original_node_id.into(),
            critique,
            refined_node,
            improvement_delta,
        }
    }
}

// ============================================================================
// Prune Types
// ============================================================================

/// A candidate for pruning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PruneCandidate {
    /// Node ID.
    pub node_id: String,
    /// Reason for pruning.
    pub reason: PruneReason,
    /// Confidence in decision.
    pub confidence: f64,
    /// Impact of pruning.
    pub impact: PruneImpact,
}

/// Response from prune operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PruneResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Candidates for pruning.
    pub prune_candidates: Vec<PruneCandidate>,
    /// Nodes to preserve.
    pub preserve_nodes: Vec<String>,
    /// Pruning strategy.
    pub pruning_strategy: String,
}

impl PruneResponse {
    /// Create a new prune response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        prune_candidates: Vec<PruneCandidate>,
        preserve_nodes: Vec<String>,
        pruning_strategy: impl Into<String>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            prune_candidates,
            preserve_nodes,
            pruning_strategy: pruning_strategy.into(),
        }
    }
}

// ============================================================================
// Finalize Types
// ============================================================================

/// A path through the graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphPath {
    /// Node IDs in the path.
    pub path: Vec<String>,
    /// Quality of the path.
    pub path_quality: f64,
    /// Key insight from this path.
    pub key_insight: String,
}

/// A conclusion from the graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphConclusion {
    /// The conclusion.
    pub conclusion: String,
    /// Confidence in the conclusion.
    pub confidence: f64,
    /// Supporting nodes.
    pub supporting_nodes: Vec<String>,
}

/// Quality metrics for the session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionQuality {
    /// Depth achieved.
    pub depth_achieved: f64,
    /// Breadth achieved.
    pub breadth_achieved: f64,
    /// Coherence.
    pub coherence: f64,
    /// Overall quality.
    pub overall: f64,
}

/// Response from finalize operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FinalizeResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Best paths through the graph.
    pub best_paths: Vec<GraphPath>,
    /// Conclusions.
    pub conclusions: Vec<GraphConclusion>,
    /// Final synthesis.
    pub final_synthesis: String,
    /// Session quality.
    pub session_quality: SessionQuality,
}

impl FinalizeResponse {
    /// Create a new finalize response.
    #[must_use]
    // Finalization requires all synthesis components for complete graph summary
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        best_paths: Vec<GraphPath>,
        conclusions: Vec<GraphConclusion>,
        final_synthesis: impl Into<String>,
        session_quality: SessionQuality,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            best_paths,
            conclusions,
            final_synthesis: final_synthesis.into(),
            session_quality,
        }
    }
}

// ============================================================================
// State Types
// ============================================================================

/// Graph structure information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphStructure {
    /// Total nodes.
    pub total_nodes: u32,
    /// Depth of graph.
    pub depth: u32,
    /// Number of branches.
    pub branches: u32,
    /// Number of pruned nodes.
    pub pruned_count: u32,
}

/// A frontier node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrontierNodeInfo {
    /// Node ID.
    pub node_id: String,
    /// Potential for expansion.
    pub potential: f64,
    /// Suggested action.
    pub suggested_action: SuggestedAction,
}

/// Graph metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphMetrics {
    /// Average score.
    pub average_score: f64,
    /// Maximum score.
    pub max_score: f64,
    /// Coverage.
    pub coverage: f64,
}

/// Response from state operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Graph structure.
    pub structure: GraphStructure,
    /// Frontier nodes.
    pub frontiers: Vec<FrontierNodeInfo>,
    /// Graph metrics.
    pub metrics: GraphMetrics,
    /// Suggested next steps.
    pub next_steps: Vec<String>,
}

impl StateResponse {
    /// Create a new state response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        structure: GraphStructure,
        frontiers: Vec<FrontierNodeInfo>,
        metrics: GraphMetrics,
        next_steps: Vec<String>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            structure,
            frontiers,
            metrics,
            next_steps,
        }
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

    #[test]
    fn test_node_type_serialize() {
        assert_eq!(serde_json::to_string(&NodeType::Root).unwrap(), "\"root\"");
        assert_eq!(
            serde_json::to_string(&NodeType::Reasoning).unwrap(),
            "\"reasoning\""
        );
    }

    #[test]
    fn test_node_relationship_serialize() {
        assert_eq!(
            serde_json::to_string(&NodeRelationship::Elaborates).unwrap(),
            "\"elaborates\""
        );
    }

    #[test]
    fn test_complexity_level_serialize() {
        assert_eq!(
            serde_json::to_string(&ComplexityLevel::High).unwrap(),
            "\"high\""
        );
    }

    #[test]
    fn test_prune_reason_serialize() {
        assert_eq!(
            serde_json::to_string(&PruneReason::LowScore).unwrap(),
            "\"low_score\""
        );
    }

    #[test]
    fn test_suggested_action_serialize() {
        assert_eq!(
            serde_json::to_string(&SuggestedAction::Expand).unwrap(),
            "\"expand\""
        );
    }
}
