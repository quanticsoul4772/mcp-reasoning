//! Response types for reasoning tools.
//!
//! This module contains all response types with JsonSchema support.

use rmcp::model::{Content, IntoContents};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Macro to implement `IntoContents` for response types by serializing to JSON.
macro_rules! impl_into_contents {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoContents for $ty {
                fn into_contents(self) -> Vec<Content> {
                    match serde_json::to_string(&self) {
                        Ok(json) => vec![Content::text(json)],
                        Err(e) => vec![Content::text(format!("{{\"error\": \"{}\"}}", e))],
                    }
                }
            }
        )*
    };
}

// ============================================================================
// Core Reasoning Responses
// ============================================================================

/// Response from linear reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LinearResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// The reasoning continuation.
    pub content: String,
    /// Model's confidence in the reasoning.
    pub confidence: f64,
    /// Suggested next reasoning step.
    pub next_step: Option<String>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

/// A branch in tree reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Branch {
    /// Branch identifier.
    pub id: String,
    /// Branch content.
    pub content: String,
    /// Branch quality score.
    pub score: f64,
    /// Branch status.
    pub status: String,
}

/// Response from tree reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TreeResponse {
    /// Session identifier.
    pub session_id: String,
    /// Current or created branch ID.
    pub branch_id: Option<String>,
    /// List of branches.
    pub branches: Option<Vec<Branch>>,
    /// Suggested next branch to explore.
    pub recommendation: Option<String>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

/// A perspective in divergent reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Perspective {
    /// Name or description of this perspective.
    pub viewpoint: String,
    /// Reasoning from this perspective.
    pub content: String,
    /// Novelty score.
    pub novelty_score: f64,
}

/// Response from divergent reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DivergentResponse {
    /// Thought identifier.
    pub thought_id: String,
    /// Session identifier.
    pub session_id: String,
    /// Generated perspectives.
    pub perspectives: Vec<Perspective>,
    /// Challenged assumptions.
    pub challenged_assumptions: Option<Vec<String>>,
    /// Unified insight from all perspectives.
    pub synthesis: Option<String>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

/// Response from reflection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReflectionResponse {
    /// Quality score.
    pub quality_score: f64,
    /// Thought identifier.
    pub thought_id: Option<String>,
    /// Session identifier.
    pub session_id: Option<String>,
    /// Iterations used.
    pub iterations_used: Option<u32>,
    /// Identified strengths.
    pub strengths: Option<Vec<String>>,
    /// Identified weaknesses.
    pub weaknesses: Option<Vec<String>>,
    /// Recommendations for improvement.
    pub recommendations: Option<Vec<String>>,
    /// Improved reasoning content.
    pub refined_content: Option<String>,
    /// Session coherence score.
    pub coherence_score: Option<f64>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

/// A checkpoint entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Checkpoint {
    /// Checkpoint identifier.
    pub id: String,
    /// Checkpoint name.
    pub name: String,
    /// Checkpoint description.
    pub description: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Number of thoughts at checkpoint.
    pub thought_count: u32,
}

/// Response from checkpoint operations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckpointResponse {
    /// Session identifier.
    pub session_id: String,
    /// Checkpoint identifier.
    pub checkpoint_id: Option<String>,
    /// List of checkpoints.
    pub checkpoints: Option<Vec<Checkpoint>>,
    /// Restored session state.
    pub restored_state: Option<serde_json::Value>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

/// Response from auto mode selection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutoResponse {
    /// Selected reasoning mode.
    pub selected_mode: String,
    /// Confidence in selection.
    pub confidence: f64,
    /// Rationale for selection.
    pub rationale: String,
    /// Result from executing selected mode.
    pub result: serde_json::Value,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

// ============================================================================
// Graph Reasoning Responses
// ============================================================================

/// A node in graph reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphNode {
    /// Node identifier.
    pub id: String,
    /// Node content.
    pub content: String,
    /// Node quality score.
    pub score: Option<f64>,
    /// Node depth in graph.
    pub depth: Option<u32>,
    /// Parent node identifier.
    pub parent_id: Option<String>,
}

/// Graph state information.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphState {
    /// Total nodes in graph.
    pub total_nodes: u32,
    /// Active nodes count.
    pub active_nodes: u32,
    /// Maximum depth reached.
    pub max_depth: u32,
    /// Pruned nodes count.
    pub pruned_count: u32,
}

/// Response from graph reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphResponse {
    /// Session identifier.
    pub session_id: String,
    /// Created or modified node ID.
    pub node_id: Option<String>,
    /// Generated or affected nodes.
    pub nodes: Option<Vec<GraphNode>>,
    /// Aggregated insight.
    pub aggregated_insight: Option<String>,
    /// Extracted conclusions.
    pub conclusions: Option<Vec<String>>,
    /// Graph state.
    pub state: Option<GraphState>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

// ============================================================================
// Analysis Tool Responses
// ============================================================================

/// A detected bias or fallacy.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Detection {
    /// Type of bias or fallacy.
    #[serde(rename = "type")]
    pub detection_type: String,
    /// Category (formal/informal for fallacies).
    pub category: Option<String>,
    /// Severity level.
    pub severity: String,
    /// Detection confidence.
    pub confidence: f64,
    /// Evidence text.
    pub evidence: String,
    /// Explanation.
    pub explanation: String,
    /// Remediation suggestion.
    pub remediation: Option<String>,
}

/// Response from detection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectResponse {
    /// Detected biases or fallacies.
    pub detections: Vec<Detection>,
    /// Summary of findings.
    pub summary: Option<String>,
    /// Overall reasoning quality.
    pub overall_quality: Option<f64>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

/// A ranked option in decision analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RankedOption {
    /// Option name.
    pub option: String,
    /// Option score.
    pub score: f64,
    /// Option rank.
    pub rank: u32,
}

/// Stakeholder mapping result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StakeholderMap {
    /// Key players (high power, high interest).
    pub key_players: Vec<String>,
    /// Keep satisfied (high power, low interest).
    pub keep_satisfied: Vec<String>,
    /// Keep informed (low power, high interest).
    pub keep_informed: Vec<String>,
    /// Minimal effort (low power, low interest).
    pub minimal_effort: Vec<String>,
}

/// Response from decision analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DecisionResponse {
    /// Best option or action.
    pub recommendation: String,
    /// Ranked options.
    pub rankings: Option<Vec<RankedOption>>,
    /// Stakeholder mapping.
    pub stakeholder_map: Option<StakeholderMap>,
    /// Identified conflicts.
    pub conflicts: Option<Vec<String>>,
    /// Identified alignments.
    pub alignments: Option<Vec<String>>,
    /// Decision rationale.
    pub rationale: Option<String>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

/// Evidence assessment result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceAssessment {
    /// Evidence content.
    pub content: String,
    /// Credibility score.
    pub credibility_score: f64,
    /// Source tier.
    pub source_tier: String,
    /// Corroborating evidence indices.
    pub corroborated_by: Option<Vec<u32>>,
}

/// Confidence interval.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConfidenceInterval {
    /// Lower bound.
    pub lower: f64,
    /// Upper bound.
    pub upper: f64,
}

/// Response from evidence evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceResponse {
    /// Overall credibility.
    pub overall_credibility: f64,
    /// Individual evidence assessments.
    pub evidence_assessments: Option<Vec<EvidenceAssessment>>,
    /// Posterior probability.
    pub posterior: Option<f64>,
    /// Prior probability.
    pub prior: Option<f64>,
    /// Likelihood ratio.
    pub likelihood_ratio: Option<f64>,
    /// Uncertainty measure.
    pub entropy: Option<f64>,
    /// Confidence interval.
    pub confidence_interval: Option<ConfidenceInterval>,
    /// Synthesis of evidence.
    pub synthesis: Option<String>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

// ============================================================================
// Advanced Reasoning Responses
// ============================================================================

/// A timeline branch.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineBranch {
    /// Branch identifier.
    pub id: String,
    /// Branch label.
    pub label: Option<String>,
    /// Branch content.
    pub content: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Branch comparison result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BranchComparison {
    /// Points where branches diverge.
    pub divergence_points: Vec<String>,
    /// Quality differences.
    pub quality_differences: serde_json::Value,
    /// Opportunities to merge.
    pub convergence_opportunities: Vec<String>,
}

/// Response from timeline reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineResponse {
    /// Timeline identifier.
    pub timeline_id: String,
    /// Branch identifier.
    pub branch_id: Option<String>,
    /// Timeline branches.
    pub branches: Option<Vec<TimelineBranch>>,
    /// Branch comparison.
    pub comparison: Option<BranchComparison>,
    /// Merged content.
    pub merged_content: Option<String>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

/// A node in MCTS path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsNode {
    /// Node identifier.
    pub node_id: String,
    /// Node content.
    pub content: String,
    /// UCB score.
    pub ucb_score: f64,
    /// Visit count.
    pub visits: u32,
}

/// Backtrack suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BacktrackSuggestion {
    /// Whether to backtrack.
    pub should_backtrack: bool,
    /// Target step to return to.
    pub target_step: Option<u32>,
    /// Reason for backtracking.
    pub reason: Option<String>,
    /// Quality drop amount.
    pub quality_drop: Option<f64>,
}

/// Response from MCTS.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsResponse {
    /// Session identifier.
    pub session_id: String,
    /// Best path found.
    pub best_path: Option<Vec<MctsNode>>,
    /// Iterations completed.
    pub iterations_completed: Option<u32>,
    /// Backtrack suggestion.
    pub backtrack_suggestion: Option<BacktrackSuggestion>,
    /// Whether backtrack was executed.
    pub executed: Option<bool>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

/// A step in causal chain.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CausalStep {
    /// Step number.
    pub step: u32,
    /// Cause.
    pub cause: String,
    /// Effect.
    pub effect: String,
    /// Probability.
    pub probability: f64,
}

/// Response from counterfactual analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CounterfactualResponse {
    /// Counterfactual outcome.
    pub counterfactual_outcome: String,
    /// Causal chain.
    pub causal_chain: Vec<CausalStep>,
    /// Session identifier.
    pub session_id: Option<String>,
    /// Original scenario.
    pub original_scenario: String,
    /// Applied intervention.
    pub intervention_applied: String,
    /// Analysis depth used.
    pub analysis_depth: String,
    /// Key differences.
    pub key_differences: Vec<String>,
    /// Confidence in analysis.
    pub confidence: f64,
    /// Assumptions made.
    pub assumptions: Vec<String>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
}

// ============================================================================
// Infrastructure Responses
// ============================================================================

/// A preset definition.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PresetInfo {
    /// Preset identifier.
    pub id: String,
    /// Preset name.
    pub name: String,
    /// Preset description.
    pub description: String,
    /// Preset category.
    pub category: String,
    /// Required inputs.
    pub required_inputs: Vec<String>,
}

/// Preset execution result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PresetExecution {
    /// Preset identifier.
    pub preset_id: String,
    /// Steps completed.
    pub steps_completed: u32,
    /// Total steps.
    pub total_steps: u32,
    /// Individual step results.
    pub step_results: Vec<serde_json::Value>,
    /// Final output.
    pub final_output: serde_json::Value,
}

/// Response from preset operations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PresetResponse {
    /// Available presets.
    pub presets: Option<Vec<PresetInfo>>,
    /// Execution result.
    pub execution_result: Option<PresetExecution>,
    /// Session identifier.
    pub session_id: Option<String>,
}

/// Summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetricsSummary {
    /// Total calls.
    pub total_calls: u64,
    /// Success rate.
    pub success_rate: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Calls by mode.
    pub by_mode: serde_json::Value,
}

/// Mode-specific statistics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModeStats {
    /// Mode name.
    pub mode_name: String,
    /// Call count.
    pub call_count: u64,
    /// Success count.
    pub success_count: u64,
    /// Failure count.
    pub failure_count: u64,
    /// Success rate.
    pub success_rate: f64,
    /// P50 latency.
    pub latency_p50_ms: f64,
    /// P95 latency.
    pub latency_p95_ms: f64,
    /// P99 latency.
    pub latency_p99_ms: f64,
}

/// An invocation record.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Invocation {
    /// Invocation identifier.
    pub id: String,
    /// Tool name.
    pub tool_name: String,
    /// Session identifier.
    pub session_id: Option<String>,
    /// Whether successful.
    pub success: bool,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Timestamp.
    pub created_at: String,
}

/// Response from metrics queries.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetricsResponse {
    /// Summary statistics.
    pub summary: Option<MetricsSummary>,
    /// Mode-specific statistics.
    pub mode_stats: Option<ModeStats>,
    /// Invocation history.
    pub invocations: Option<Vec<Invocation>>,
    /// Configuration info.
    pub config: Option<serde_json::Value>,
}

// ============================================================================
// Self-Improvement Responses
// ============================================================================

/// Response for self-improvement status.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiStatusResponse {
    /// Whether the manager is running.
    pub running: bool,
    /// Circuit breaker state.
    pub circuit_state: String,
    /// Total cycles run.
    pub total_cycles: u64,
    /// Successful cycles.
    pub successful_cycles: u64,
    /// Failed cycles.
    pub failed_cycles: u64,
    /// Pending diagnoses count.
    pub pending_diagnoses: usize,
    /// Total actions executed.
    pub total_actions_executed: u64,
    /// Total actions rolled back.
    pub total_actions_rolled_back: u64,
    /// Last cycle time (Unix epoch milliseconds).
    pub last_cycle_at: Option<u64>,
    /// Average reward from learning.
    pub average_reward: f64,
}

/// A pending diagnosis in the response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiPendingDiagnosis {
    /// Action/diagnosis ID.
    pub id: String,
    /// Action type.
    pub action_type: String,
    /// Description.
    pub description: String,
    /// Rationale.
    pub rationale: String,
    /// Expected improvement.
    pub expected_improvement: f64,
    /// Created timestamp (Unix epoch milliseconds).
    pub created_at: u64,
}

/// Response for pending diagnoses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiDiagnosesResponse {
    /// List of pending diagnoses.
    pub diagnoses: Vec<SiPendingDiagnosis>,
    /// Total count.
    pub total: usize,
}

/// Execution result summary in approve response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiExecutionSummary {
    /// Action ID.
    pub action_id: String,
    /// Whether execution succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Learning result summary in approve response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiLearningSummary {
    /// Action ID.
    pub action_id: String,
    /// Insight learned.
    pub insight: String,
    /// Reward value.
    pub reward: f64,
}

/// Response for approve operation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiApproveResponse {
    /// Whether the approval succeeded.
    pub success: bool,
    /// Number of actions executed.
    pub actions_executed: usize,
    /// Number of lessons learned.
    pub lessons_learned: usize,
    /// Execution results.
    pub execution_results: Vec<SiExecutionSummary>,
    /// Learning results.
    pub learning_results: Vec<SiLearningSummary>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Response for reject operation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiRejectResponse {
    /// Whether the rejection succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Response for trigger operation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiTriggerResponse {
    /// Whether the cycle succeeded.
    pub success: bool,
    /// Number of actions proposed.
    pub actions_proposed: usize,
    /// Number of actions executed.
    pub actions_executed: usize,
    /// Whether analysis was skipped due to insufficient data.
    pub analysis_skipped: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Response for rollback operation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiRollbackResponse {
    /// Whether the rollback succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

// Apply IntoContents to all response types
impl_into_contents!(
    LinearResponse,
    TreeResponse,
    DivergentResponse,
    ReflectionResponse,
    CheckpointResponse,
    AutoResponse,
    GraphResponse,
    DetectResponse,
    DecisionResponse,
    EvidenceResponse,
    TimelineResponse,
    MctsResponse,
    CounterfactualResponse,
    PresetResponse,
    MetricsResponse,
    SiStatusResponse,
    SiDiagnosesResponse,
    SiApproveResponse,
    SiRejectResponse,
    SiTriggerResponse,
    SiRollbackResponse,
);

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
    use rmcp::model::IntoContents;

    #[test]
    fn test_linear_response_into_contents() {
        let response = LinearResponse {
            thought_id: "t-1".to_string(),
            session_id: "s-1".to_string(),
            content: "Analysis result".to_string(),
            confidence: 0.85,
            next_step: Some("Continue".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("thought_id"));
        assert!(text.text.contains("t-1"));
    }

    #[test]
    fn test_tree_response_into_contents() {
        let response = TreeResponse {
            session_id: "s-1".to_string(),
            branch_id: Some("b-1".to_string()),
            branches: Some(vec![Branch {
                id: "b-1".to_string(),
                content: "Branch content".to_string(),
                score: 0.8,
                status: "active".to_string(),
            }]),
            recommendation: Some("Explore branch 1".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("branch_id"));
    }

    #[test]
    fn test_divergent_response_into_contents() {
        let response = DivergentResponse {
            thought_id: "t-1".to_string(),
            session_id: "s-1".to_string(),
            perspectives: vec![Perspective {
                viewpoint: "Technical".to_string(),
                content: "Technical view".to_string(),
                novelty_score: 0.7,
            }],
            challenged_assumptions: Some(vec!["Assumption 1".to_string()]),
            synthesis: Some("Unified view".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("perspectives"));
    }

    #[test]
    fn test_reflection_response_into_contents() {
        let response = ReflectionResponse {
            quality_score: 0.75,
            thought_id: Some("t-1".to_string()),
            session_id: Some("s-1".to_string()),
            iterations_used: Some(2),
            strengths: Some(vec!["Clear".to_string()]),
            weaknesses: Some(vec!["Verbose".to_string()]),
            recommendations: Some(vec!["Be concise".to_string()]),
            refined_content: Some("Better content".to_string()),
            coherence_score: Some(0.8),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("quality_score"));
    }

    #[test]
    fn test_checkpoint_response_into_contents() {
        let response = CheckpointResponse {
            session_id: "s-1".to_string(),
            checkpoint_id: Some("cp-1".to_string()),
            checkpoints: Some(vec![Checkpoint {
                id: "cp-1".to_string(),
                name: "Checkpoint 1".to_string(),
                description: Some("Test checkpoint".to_string()),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                thought_count: 5,
            }]),
            restored_state: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("checkpoint_id"));
    }

    #[test]
    fn test_auto_response_into_contents() {
        let response = AutoResponse {
            selected_mode: "linear".to_string(),
            confidence: 0.9,
            rationale: "Simple analysis".to_string(),
            result: serde_json::json!({"key": "value"}),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("selected_mode"));
    }

    #[test]
    fn test_graph_response_into_contents() {
        let response = GraphResponse {
            session_id: "s-1".to_string(),
            node_id: Some("n-1".to_string()),
            nodes: Some(vec![GraphNode {
                id: "n-1".to_string(),
                content: "Node content".to_string(),
                score: Some(0.8),
                depth: Some(1),
                parent_id: None,
            }]),
            aggregated_insight: Some("Insight".to_string()),
            conclusions: Some(vec!["Conclusion 1".to_string()]),
            state: Some(GraphState {
                total_nodes: 10,
                active_nodes: 8,
                max_depth: 3,
                pruned_count: 2,
            }),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("node_id"));
    }

    #[test]
    fn test_detect_response_into_contents() {
        let response = DetectResponse {
            detections: vec![Detection {
                detection_type: "confirmation_bias".to_string(),
                category: Some("cognitive".to_string()),
                severity: "high".to_string(),
                confidence: 0.8,
                evidence: "Only cited supporting".to_string(),
                explanation: "Ignored contrary".to_string(),
                remediation: Some("Seek disconfirming".to_string()),
            }],
            summary: Some("1 bias found".to_string()),
            overall_quality: Some(0.6),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("detections"));
    }

    #[test]
    fn test_decision_response_into_contents() {
        let response = DecisionResponse {
            recommendation: "Option A".to_string(),
            rankings: Some(vec![RankedOption {
                option: "Option A".to_string(),
                score: 0.9,
                rank: 1,
            }]),
            stakeholder_map: None,
            conflicts: None,
            alignments: None,
            rationale: Some("Best fit".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("recommendation"));
    }

    #[test]
    fn test_evidence_response_into_contents() {
        let response = EvidenceResponse {
            overall_credibility: 0.75,
            evidence_assessments: Some(vec![EvidenceAssessment {
                content: "Study shows...".to_string(),
                credibility_score: 0.8,
                source_tier: "primary".to_string(),
                corroborated_by: Some(vec![1, 2]),
            }]),
            posterior: Some(0.8),
            prior: Some(0.5),
            likelihood_ratio: Some(2.0),
            entropy: Some(0.3),
            confidence_interval: Some(ConfidenceInterval {
                lower: 0.7,
                upper: 0.9,
            }),
            synthesis: Some("Strong evidence".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("overall_credibility"));
    }

    #[test]
    fn test_timeline_response_into_contents() {
        let response = TimelineResponse {
            timeline_id: "tl-1".to_string(),
            branch_id: Some("br-1".to_string()),
            branches: Some(vec![TimelineBranch {
                id: "br-1".to_string(),
                label: Some("Main".to_string()),
                content: "Branch content".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            }]),
            comparison: None,
            merged_content: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("timeline_id"));
    }

    #[test]
    fn test_mcts_response_into_contents() {
        let response = MctsResponse {
            session_id: "s-1".to_string(),
            best_path: Some(vec![MctsNode {
                node_id: "n-1".to_string(),
                content: "Node content".to_string(),
                ucb_score: 1.5,
                visits: 10,
            }]),
            iterations_completed: Some(50),
            backtrack_suggestion: Some(BacktrackSuggestion {
                should_backtrack: false,
                target_step: None,
                reason: None,
                quality_drop: None,
            }),
            executed: Some(false),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("session_id"));
    }

    #[test]
    fn test_counterfactual_response_into_contents() {
        let response = CounterfactualResponse {
            counterfactual_outcome: "Different result".to_string(),
            causal_chain: vec![CausalStep {
                step: 1,
                cause: "X".to_string(),
                effect: "Y".to_string(),
                probability: 0.8,
            }],
            session_id: Some("s-1".to_string()),
            original_scenario: "Original".to_string(),
            intervention_applied: "Change X".to_string(),
            analysis_depth: "counterfactual".to_string(),
            key_differences: vec!["Diff 1".to_string()],
            confidence: 0.75,
            assumptions: vec!["Assumption 1".to_string()],
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("counterfactual_outcome"));
    }

    #[test]
    fn test_preset_response_into_contents() {
        let response = PresetResponse {
            presets: Some(vec![PresetInfo {
                id: "p-1".to_string(),
                name: "Preset 1".to_string(),
                description: "Test preset".to_string(),
                category: "analysis".to_string(),
                required_inputs: vec!["topic".to_string()],
            }]),
            execution_result: None,
            session_id: Some("s-1".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("presets"));
    }

    #[test]
    fn test_metrics_response_into_contents() {
        let response = MetricsResponse {
            summary: Some(MetricsSummary {
                total_calls: 100,
                success_rate: 0.95,
                avg_latency_ms: 150.0,
                by_mode: serde_json::json!({"linear": 50}),
            }),
            mode_stats: None,
            invocations: None,
            config: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("total_calls"));
    }

    #[test]
    fn test_si_status_response_into_contents() {
        let response = SiStatusResponse {
            running: true,
            circuit_state: "closed".to_string(),
            total_cycles: 10,
            successful_cycles: 8,
            failed_cycles: 2,
            pending_diagnoses: 0,
            total_actions_executed: 5,
            total_actions_rolled_back: 1,
            last_cycle_at: Some(1234567890),
            average_reward: 0.75,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("running"));
    }

    #[test]
    fn test_si_diagnoses_response_into_contents() {
        let response = SiDiagnosesResponse {
            diagnoses: vec![SiPendingDiagnosis {
                id: "d-1".to_string(),
                action_type: "adjust_param".to_string(),
                description: "Adjust threshold".to_string(),
                rationale: "Performance issue".to_string(),
                expected_improvement: 0.2,
                created_at: 1234567890,
            }],
            total: 1,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("diagnoses"));
    }

    #[test]
    fn test_si_approve_response_into_contents() {
        let response = SiApproveResponse {
            success: true,
            actions_executed: 2,
            lessons_learned: 1,
            execution_results: vec![SiExecutionSummary {
                action_id: "a-1".to_string(),
                success: true,
                error: None,
            }],
            learning_results: vec![SiLearningSummary {
                action_id: "a-1".to_string(),
                insight: "Improved latency".to_string(),
                reward: 0.5,
            }],
            error: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("actions_executed"));
    }

    #[test]
    fn test_si_reject_response_into_contents() {
        let response = SiRejectResponse {
            success: true,
            error: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("success"));
    }

    #[test]
    fn test_si_trigger_response_into_contents() {
        let response = SiTriggerResponse {
            success: true,
            actions_proposed: 3,
            actions_executed: 2,
            analysis_skipped: false,
            error: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("actions_proposed"));
    }

    #[test]
    fn test_si_rollback_response_into_contents() {
        let response = SiRollbackResponse {
            success: true,
            error: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
        let text = contents[0].as_text().unwrap();
        assert!(text.text.contains("success"));
    }

    #[test]
    fn test_branch_serialize() {
        let branch = Branch {
            id: "b-1".to_string(),
            content: "Content".to_string(),
            score: 0.8,
            status: "active".to_string(),
        };
        let json = serde_json::to_string(&branch).unwrap();
        assert!(json.contains("b-1"));
    }

    #[test]
    fn test_perspective_serialize() {
        let p = Perspective {
            viewpoint: "Tech".to_string(),
            content: "View".to_string(),
            novelty_score: 0.5,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("Tech"));
    }

    #[test]
    fn test_graph_node_serialize() {
        let node = GraphNode {
            id: "n-1".to_string(),
            content: "Content".to_string(),
            score: Some(0.9),
            depth: Some(2),
            parent_id: Some("n-0".to_string()),
        };
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("n-1"));
    }

    #[test]
    fn test_detection_serialize() {
        let d = Detection {
            detection_type: "bias".to_string(),
            category: None,
            severity: "high".to_string(),
            confidence: 0.9,
            evidence: "Proof".to_string(),
            explanation: "Reason".to_string(),
            remediation: None,
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("bias"));
    }

    #[test]
    fn test_stakeholder_map_serialize() {
        let sm = StakeholderMap {
            key_players: vec!["CEO".to_string()],
            keep_satisfied: vec!["Board".to_string()],
            keep_informed: vec!["Users".to_string()],
            minimal_effort: vec!["Others".to_string()],
        };
        let json = serde_json::to_string(&sm).unwrap();
        assert!(json.contains("key_players"));
    }
}
