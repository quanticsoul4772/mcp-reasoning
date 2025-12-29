//! Timeline analysis response types.
//!
//! This module contains all the structured response types for the timeline mode:
//! - Create operation types (timeline events, decision points, temporal structure)
//! - Branch operation types (branch points, branches, comparisons)
//! - Compare operation types (differences, risk/opportunity assessments)
//! - Merge operation types (patterns, strategies, synthesis)

use serde::{Deserialize, Serialize};

// ============================================================================
// Response Types - Create
// ============================================================================

/// Type of timeline element.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// A discrete event.
    Event,
    /// A persistent state.
    State,
    /// A point where a decision can be made.
    DecisionPoint,
}

/// An event on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineEvent {
    /// Event identifier.
    pub id: String,
    /// Description of the event.
    pub description: String,
    /// Time marker (relative or absolute).
    pub time: String,
    /// Type of event.
    #[serde(rename = "type")]
    pub event_type: EventType,
    /// Events that cause this one.
    pub causes: Vec<String>,
    /// Events caused by this one.
    pub effects: Vec<String>,
}

/// A decision point on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionPoint {
    /// Decision identifier.
    pub id: String,
    /// Description of the decision.
    pub description: String,
    /// Possible choices.
    pub options: Vec<String>,
    /// When the decision must be made.
    pub deadline: String,
}

/// Temporal structure of the timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemporalStructure {
    /// Beginning event ID.
    pub start: String,
    /// Current event ID.
    pub current: String,
    /// How far into future we're considering.
    pub horizon: String,
}

/// Response from create operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateTimelineResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Timeline identifier.
    pub timeline_id: String,
    /// Events on the timeline.
    pub events: Vec<TimelineEvent>,
    /// Decision points.
    pub decision_points: Vec<DecisionPoint>,
    /// Temporal structure.
    pub temporal_structure: TemporalStructure,
}

impl CreateTimelineResponse {
    /// Create a new create timeline response.
    #[must_use]
    // Timeline creation requires all temporal components; builder pattern would be verbose
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        timeline_id: impl Into<String>,
        events: Vec<TimelineEvent>,
        decision_points: Vec<DecisionPoint>,
        temporal_structure: TemporalStructure,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            timeline_id: timeline_id.into(),
            events,
            decision_points,
            temporal_structure,
        }
    }
}

// ============================================================================
// Response Types - Branch
// ============================================================================

/// A branch point on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchPoint {
    /// Event ID of the branch point.
    pub event_id: String,
    /// Description of the decision being made.
    pub description: String,
}

/// An event in a branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BranchEvent {
    /// Event identifier.
    pub id: String,
    /// Description of the event.
    pub description: String,
    /// Probability of this event occurring.
    pub probability: f64,
    /// Time offset from branch point.
    pub time_offset: String,
}

/// A timeline branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineBranch {
    /// Branch identifier.
    pub id: String,
    /// The choice made at the branch point.
    pub choice: String,
    /// Events along this branch.
    pub events: Vec<BranchEvent>,
    /// How plausible this branch is.
    pub plausibility: f64,
    /// Quality of the outcome.
    pub outcome_quality: f64,
}

/// Branch comparison summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchComparison {
    /// Branch most likely to have a good outcome.
    pub most_likely_good_outcome: String,
    /// Branch with highest risk.
    pub highest_risk: String,
    /// Key differences between branches.
    pub key_differences: Vec<String>,
}

/// Response from branch operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BranchResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// The branch point.
    pub branch_point: BranchPoint,
    /// Alternative branches.
    pub branches: Vec<TimelineBranch>,
    /// Comparison summary.
    pub comparison: BranchComparison,
}

impl BranchResponse {
    /// Create a new branch response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        branch_point: BranchPoint,
        branches: Vec<TimelineBranch>,
        comparison: BranchComparison,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            branch_point,
            branches,
            comparison,
        }
    }
}

// ============================================================================
// Response Types - Compare
// ============================================================================

/// A difference between branches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchDifference {
    /// What dimension is being compared.
    pub dimension: String,
    /// Outcome in branch 1.
    pub branch_1_value: String,
    /// Outcome in branch 2.
    pub branch_2_value: String,
    /// Why this difference matters.
    pub significance: String,
}

/// Risk assessment per branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskAssessment {
    /// Risks in branch 1.
    pub branch_1_risks: Vec<String>,
    /// Risks in branch 2.
    pub branch_2_risks: Vec<String>,
}

/// Opportunity assessment per branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpportunityAssessment {
    /// Opportunities in branch 1.
    pub branch_1_opportunities: Vec<String>,
    /// Opportunities in branch 2.
    pub branch_2_opportunities: Vec<String>,
}

/// Recommendation based on comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompareRecommendation {
    /// Preferred branch or "depends".
    pub preferred_branch: String,
    /// Conditions under which this is preferred.
    pub conditions: String,
    /// Key factors in the decision.
    pub key_factors: String,
}

/// Response from compare operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompareResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Branches being compared.
    pub branches_compared: Vec<String>,
    /// Where the branches diverged.
    pub divergence_point: String,
    /// Key differences.
    pub key_differences: Vec<BranchDifference>,
    /// Risk assessment.
    pub risk_assessment: RiskAssessment,
    /// Opportunity assessment.
    pub opportunity_assessment: OpportunityAssessment,
    /// Recommendation.
    pub recommendation: CompareRecommendation,
}

impl CompareResponse {
    /// Create a new compare response.
    #[must_use]
    // Comparison analysis requires all assessment components for complete response
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        branches_compared: Vec<String>,
        divergence_point: impl Into<String>,
        key_differences: Vec<BranchDifference>,
        risk_assessment: RiskAssessment,
        opportunity_assessment: OpportunityAssessment,
        recommendation: CompareRecommendation,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            branches_compared,
            divergence_point: divergence_point.into(),
            key_differences,
            risk_assessment,
            opportunity_assessment,
            recommendation,
        }
    }
}

// ============================================================================
// Response Types - Merge
// ============================================================================

/// A pattern observed across branches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommonPattern {
    /// Description of the pattern.
    pub pattern: String,
    /// How often this pattern appears.
    pub frequency: f64,
    /// What this pattern implies.
    pub implications: String,
}

/// A strategy that works across branches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RobustStrategy {
    /// Description of the strategy.
    pub strategy: String,
    /// How effective this strategy is.
    pub effectiveness: f64,
    /// When this strategy is applicable.
    pub conditions: String,
}

/// A strategy that only works in some branches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FragileStrategy {
    /// Description of the strategy.
    pub strategy: String,
    /// When it fails.
    pub failure_modes: String,
}

/// Response from merge operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Branches that were merged.
    pub branches_merged: Vec<String>,
    /// Patterns observed across branches.
    pub common_patterns: Vec<CommonPattern>,
    /// Strategies that work robustly.
    pub robust_strategies: Vec<RobustStrategy>,
    /// Strategies that are fragile.
    pub fragile_strategies: Vec<FragileStrategy>,
    /// Overall synthesis.
    pub synthesis: String,
    /// Actionable recommendations.
    pub recommendations: Vec<String>,
}

impl MergeResponse {
    /// Create a new merge response.
    #[must_use]
    // Merge response requires all synthesis components for coherent analysis
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        branches_merged: Vec<String>,
        common_patterns: Vec<CommonPattern>,
        robust_strategies: Vec<RobustStrategy>,
        fragile_strategies: Vec<FragileStrategy>,
        synthesis: impl Into<String>,
        recommendations: Vec<String>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            branches_merged,
            common_patterns,
            robust_strategies,
            fragile_strategies,
            synthesis: synthesis.into(),
            recommendations,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_serialize() {
        assert_eq!(
            serde_json::to_string(&EventType::Event).unwrap(),
            "\"event\""
        );
        assert_eq!(
            serde_json::to_string(&EventType::DecisionPoint).unwrap(),
            "\"decision_point\""
        );
    }
}
