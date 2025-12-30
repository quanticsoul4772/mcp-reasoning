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
);
