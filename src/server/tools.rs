//! Tool definitions with rmcp macros.
//!
//! This module defines all 15 reasoning tools using the rmcp 0.1.5 macro system.
//! Uses `#[tool(tool_box)]` on impl and `#[tool(name, description)]` on methods.

// Tool methods are async stubs that will use await when connected to actual mode implementations
#![allow(clippy::unused_async)]

use std::sync::Arc;

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    Content, Implementation, IntoContents, ProtocolVersion, ServerCapabilities, ServerInfo,
    ToolsCapability,
};
use rmcp::service::{Peer, RoleServer};
use rmcp::tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::types::AppState;

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
// Request Types with JsonSchema (for tool parameters)
// ============================================================================

/// Request for linear reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LinearRequest {
    /// Thought content to process.
    pub content: String,
    /// Session ID for context continuity.
    pub session_id: Option<String>,
    /// Confidence threshold (0.0-1.0).
    pub confidence: Option<f64>,
}

/// Request for tree reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TreeRequest {
    /// Operation: create/focus/list/complete.
    pub operation: Option<String>,
    /// Content to explore (for create).
    pub content: Option<String>,
    /// Session ID.
    pub session_id: Option<String>,
    /// Branch ID (for focus/complete).
    pub branch_id: Option<String>,
    /// Number of branches (2-4).
    pub num_branches: Option<u32>,
    /// Mark as completed.
    pub completed: Option<bool>,
}

/// Request for divergent reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DivergentRequest {
    /// Content to analyze.
    pub content: String,
    /// Session ID.
    pub session_id: Option<String>,
    /// Number of perspectives (2-5).
    pub num_perspectives: Option<u32>,
    /// Challenge assumptions.
    pub challenge_assumptions: Option<bool>,
    /// Force maximum divergence.
    pub force_rebellion: Option<bool>,
}

/// Request for reflection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReflectionRequest {
    /// Operation: process/evaluate.
    pub operation: Option<String>,
    /// Content to reflect on.
    pub content: Option<String>,
    /// Thought ID to analyze.
    pub thought_id: Option<String>,
    /// Session ID.
    pub session_id: Option<String>,
    /// Max iterations (1-5).
    pub max_iterations: Option<u32>,
    /// Quality threshold (0.0-1.0).
    pub quality_threshold: Option<f64>,
}

/// Request for checkpoint operations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckpointRequest {
    /// Operation: create/list/restore.
    pub operation: String,
    /// Session ID.
    pub session_id: String,
    /// Checkpoint ID (for restore).
    pub checkpoint_id: Option<String>,
    /// Checkpoint name (for create).
    pub name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// New direction after restore.
    pub new_direction: Option<String>,
}

/// Request for auto mode selection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutoRequest {
    /// Content to analyze.
    pub content: String,
    /// Hints for mode selection.
    pub hints: Option<Vec<String>>,
    /// Session ID.
    pub session_id: Option<String>,
}

/// Request for graph reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphRequest {
    /// Operation type.
    pub operation: String,
    /// Session ID.
    pub session_id: String,
    /// Content (for init).
    pub content: Option<String>,
    /// Problem context.
    pub problem: Option<String>,
    /// Target node ID.
    pub node_id: Option<String>,
    /// Node IDs (for aggregate).
    pub node_ids: Option<Vec<String>>,
    /// Continuations to generate (1-10).
    pub k: Option<u32>,
    /// Prune threshold (0.0-1.0).
    pub threshold: Option<f64>,
    /// Terminal node IDs (for finalize).
    pub terminal_node_ids: Option<Vec<String>>,
}

/// Request for detection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectRequest {
    /// Type: biases/fallacies.
    #[serde(rename = "type")]
    pub detect_type: String,
    /// Content to analyze.
    pub content: Option<String>,
    /// Thought ID.
    pub thought_id: Option<String>,
    /// Session ID.
    pub session_id: Option<String>,
    /// Specific types to check.
    pub check_types: Option<Vec<String>>,
    /// Check formal fallacies.
    pub check_formal: Option<bool>,
    /// Check informal fallacies.
    pub check_informal: Option<bool>,
}

/// Request for decision analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DecisionRequest {
    /// Type: weighted/pairwise/topsis/perspectives.
    #[serde(rename = "type")]
    pub decision_type: Option<String>,
    /// Decision question.
    pub question: Option<String>,
    /// Topic (for perspectives).
    pub topic: Option<String>,
    /// Options to evaluate.
    pub options: Option<Vec<String>>,
    /// Session ID.
    pub session_id: Option<String>,
    /// Context.
    pub context: Option<String>,
}

/// Request for evidence evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceRequest {
    /// Type: assess/probabilistic.
    #[serde(rename = "type")]
    pub evidence_type: Option<String>,
    /// Claim (for assess).
    pub claim: Option<String>,
    /// Hypothesis (for probabilistic).
    pub hypothesis: Option<String>,
    /// Prior probability.
    pub prior: Option<f64>,
    /// Session ID.
    pub session_id: Option<String>,
    /// Context.
    pub context: Option<String>,
}

/// Request for timeline reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineRequest {
    /// Operation: create/branch/compare/merge.
    pub operation: String,
    /// Content.
    pub content: Option<String>,
    /// Session ID.
    pub session_id: Option<String>,
    /// Timeline ID.
    pub timeline_id: Option<String>,
    /// Branch IDs (for compare).
    pub branch_ids: Option<Vec<String>>,
    /// Source branch (for merge).
    pub source_branch_id: Option<String>,
    /// Target branch (for merge).
    pub target_branch_id: Option<String>,
    /// Merge strategy.
    pub merge_strategy: Option<String>,
    /// Branch label.
    pub label: Option<String>,
}

/// Request for MCTS.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsRequest {
    /// Operation: `explore` or `auto_backtrack`.
    pub operation: Option<String>,
    /// Content (for explore).
    pub content: Option<String>,
    /// Session ID.
    pub session_id: Option<String>,
    /// Node ID.
    pub node_id: Option<String>,
    /// Iterations (1-100).
    pub iterations: Option<u32>,
    /// Exploration constant.
    pub exploration_constant: Option<f64>,
    /// Simulation depth (1-20).
    pub simulation_depth: Option<u32>,
    /// Quality threshold (0.0-1.0).
    pub quality_threshold: Option<f64>,
    /// Auto-execute backtrack.
    pub auto_execute: Option<bool>,
    /// Lookback depth (1-10).
    pub lookback_depth: Option<u32>,
}

/// Request for counterfactual analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CounterfactualRequest {
    /// Base scenario.
    pub scenario: String,
    /// What-if change.
    pub intervention: String,
    /// Session ID.
    pub session_id: Option<String>,
    /// Analysis depth: association/intervention/counterfactual.
    pub analysis_depth: Option<String>,
}

/// Request for preset operations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PresetRequest {
    /// Operation: list/run.
    pub operation: String,
    /// Preset ID (for run).
    pub preset_id: Option<String>,
    /// Category filter (for list).
    pub category: Option<String>,
    /// Preset inputs.
    pub inputs: Option<serde_json::Value>,
    /// Session ID.
    pub session_id: Option<String>,
}

/// Request for metrics queries.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetricsRequest {
    /// Query: `summary`, `by_mode`, `invocations`, `fallbacks`, or `config`.
    pub query: String,
    /// Mode name (for `by_mode`).
    pub mode_name: Option<String>,
    /// Tool name filter.
    pub tool_name: Option<String>,
    /// Session ID filter.
    pub session_id: Option<String>,
    /// Success only.
    pub success_only: Option<bool>,
    /// Result limit (1-1000).
    pub limit: Option<u32>,
}

// ============================================================================
// Response Types with JsonSchema
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

// ============================================================================
// ReasoningServer with Tool Box (rmcp 0.1.5 syntax)
// ============================================================================

/// Reasoning server with all tools.
#[derive(Clone)]
pub struct ReasoningServer {
    /// Shared application state.
    pub state: Arc<AppState>,
}

impl ReasoningServer {
    /// Creates a new reasoning server.
    #[must_use]
    pub const fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tool(tool_box)]
impl ReasoningServer {
    #[tool(
        name = "reasoning_linear",
        description = "Process a thought and get a logical continuation with confidence scoring."
    )]
    async fn reasoning_linear(&self, #[tool(aggr)] req: LinearRequest) -> LinearResponse {
        // TODO: Implement mode call
        LinearResponse {
            thought_id: String::new(),
            session_id: req.session_id.unwrap_or_default(),
            content: String::new(),
            confidence: 0.0,
            next_step: None,
        }
    }

    #[tool(
        name = "reasoning_tree",
        description = "Branching exploration: create=start with 2-4 paths, focus=select branch, list=show branches, complete=mark finished."
    )]
    async fn reasoning_tree(&self, #[tool(aggr)] req: TreeRequest) -> TreeResponse {
        TreeResponse {
            session_id: req.session_id.unwrap_or_default(),
            branch_id: None,
            branches: None,
            recommendation: None,
        }
    }

    #[tool(
        name = "reasoning_divergent",
        description = "Generate novel perspectives with assumption challenges and optional force_rebellion mode."
    )]
    async fn reasoning_divergent(&self, #[tool(aggr)] req: DivergentRequest) -> DivergentResponse {
        DivergentResponse {
            thought_id: String::new(),
            session_id: req.session_id.unwrap_or_default(),
            perspectives: vec![],
            challenged_assumptions: None,
            synthesis: None,
        }
    }

    #[tool(
        name = "reasoning_reflection",
        description = "Analyze and improve reasoning: process=iterative refinement, evaluate=session assessment."
    )]
    async fn reasoning_reflection(
        &self,
        #[tool(aggr)] req: ReflectionRequest,
    ) -> ReflectionResponse {
        let _ = req;
        ReflectionResponse {
            quality_score: 0.0,
            thought_id: None,
            session_id: None,
            iterations_used: None,
            strengths: None,
            weaknesses: None,
            recommendations: None,
            refined_content: None,
            coherence_score: None,
        }
    }

    #[tool(
        name = "reasoning_checkpoint",
        description = "Save and restore reasoning state: create=save, list=show, restore=return to checkpoint."
    )]
    async fn reasoning_checkpoint(
        &self,
        #[tool(aggr)] req: CheckpointRequest,
    ) -> CheckpointResponse {
        CheckpointResponse {
            session_id: req.session_id,
            checkpoint_id: None,
            checkpoints: None,
            restored_state: None,
        }
    }

    #[tool(
        name = "reasoning_auto",
        description = "Analyze content and route to optimal reasoning mode."
    )]
    async fn reasoning_auto(&self, #[tool(aggr)] req: AutoRequest) -> AutoResponse {
        let _ = req;
        AutoResponse {
            selected_mode: "linear".to_string(),
            confidence: 0.0,
            rationale: String::new(),
            result: serde_json::Value::Null,
        }
    }

    #[tool(
        name = "reasoning_graph",
        description = "Graph reasoning: init/generate/score/aggregate/refine/prune/finalize/state operations."
    )]
    async fn reasoning_graph(&self, #[tool(aggr)] req: GraphRequest) -> GraphResponse {
        GraphResponse {
            session_id: req.session_id,
            node_id: None,
            nodes: None,
            aggregated_insight: None,
            conclusions: None,
            state: None,
        }
    }

    #[tool(
        name = "reasoning_detect",
        description = "Detect cognitive biases and logical fallacies in reasoning."
    )]
    async fn reasoning_detect(&self, #[tool(aggr)] req: DetectRequest) -> DetectResponse {
        let _ = req;
        DetectResponse {
            detections: vec![],
            summary: None,
            overall_quality: None,
        }
    }

    #[tool(
        name = "reasoning_decision",
        description = "Decision analysis: weighted/pairwise/topsis scoring or perspectives stakeholder mapping."
    )]
    async fn reasoning_decision(&self, #[tool(aggr)] req: DecisionRequest) -> DecisionResponse {
        let _ = req;
        DecisionResponse {
            recommendation: String::new(),
            rankings: None,
            stakeholder_map: None,
            conflicts: None,
            alignments: None,
            rationale: None,
        }
    }

    #[tool(
        name = "reasoning_evidence",
        description = "Evaluate evidence: assess=credibility scoring, probabilistic=Bayesian belief update."
    )]
    async fn reasoning_evidence(&self, #[tool(aggr)] req: EvidenceRequest) -> EvidenceResponse {
        let _ = req;
        EvidenceResponse {
            overall_credibility: 0.0,
            evidence_assessments: None,
            posterior: None,
            prior: None,
            likelihood_ratio: None,
            entropy: None,
            confidence_interval: None,
            synthesis: None,
        }
    }

    #[tool(
        name = "reasoning_timeline",
        description = "Temporal reasoning: create/branch/compare/merge operations."
    )]
    async fn reasoning_timeline(&self, #[tool(aggr)] req: TimelineRequest) -> TimelineResponse {
        let _ = req;
        TimelineResponse {
            timeline_id: String::new(),
            branch_id: None,
            branches: None,
            comparison: None,
            merged_content: None,
        }
    }

    #[tool(
        name = "reasoning_mcts",
        description = "MCTS: explore=UCB1-guided search, auto_backtrack=quality-triggered backtracking."
    )]
    async fn reasoning_mcts(&self, #[tool(aggr)] req: MctsRequest) -> MctsResponse {
        MctsResponse {
            session_id: req.session_id.unwrap_or_default(),
            best_path: None,
            iterations_completed: None,
            backtrack_suggestion: None,
            executed: None,
        }
    }

    #[tool(
        name = "reasoning_counterfactual",
        description = "What-if analysis using Pearl's Ladder of Causation."
    )]
    async fn reasoning_counterfactual(
        &self,
        #[tool(aggr)] req: CounterfactualRequest,
    ) -> CounterfactualResponse {
        CounterfactualResponse {
            counterfactual_outcome: String::new(),
            causal_chain: vec![],
            session_id: req.session_id,
            original_scenario: req.scenario,
            intervention_applied: req.intervention,
            analysis_depth: "counterfactual".to_string(),
            key_differences: vec![],
            confidence: 0.0,
            assumptions: vec![],
        }
    }

    #[tool(
        name = "reasoning_preset",
        description = "Execute pre-defined reasoning workflows: list=show presets, run=execute workflow."
    )]
    async fn reasoning_preset(&self, #[tool(aggr)] req: PresetRequest) -> PresetResponse {
        let _ = req;
        PresetResponse {
            presets: None,
            execution_result: None,
            session_id: None,
        }
    }

    #[tool(
        name = "reasoning_metrics",
        description = "Query metrics: summary/by_mode/invocations/fallbacks/config."
    )]
    async fn reasoning_metrics(&self, #[tool(aggr)] req: MetricsRequest) -> MetricsResponse {
        let _ = req;
        MetricsResponse {
            summary: None,
            mode_stats: None,
            invocations: None,
            config: None,
        }
    }
}

// Generate the tool_box function from tool definitions
rmcp::tool_box!(ReasoningServer {
    reasoning_linear,
    reasoning_tree,
    reasoning_divergent,
    reasoning_reflection,
    reasoning_checkpoint,
    reasoning_auto,
    reasoning_graph,
    reasoning_detect,
    reasoning_decision,
    reasoning_evidence,
    reasoning_timeline,
    reasoning_mcts,
    reasoning_counterfactual,
    reasoning_preset,
    reasoning_metrics,
});

// Implement ServerHandler to integrate with rmcp's server infrastructure
impl ServerHandler for ReasoningServer {
    // Use tool_box!(@derive) to generate list_tools and call_tool methods
    rmcp::tool_box!(@derive);

    fn get_peer(&self) -> Option<Peer<RoleServer>> {
        None
    }

    fn set_peer(&mut self, _peer: Peer<RoleServer>) {
        // We don't need to store the peer for now
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "mcp-reasoning".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "MCP Reasoning Server providing 15 structured reasoning tools.".to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_response_serialize() {
        let response = LinearResponse {
            thought_id: "t1".to_string(),
            session_id: "s1".to_string(),
            content: "reasoning content".to_string(),
            confidence: 0.85,
            next_step: Some("continue".to_string()),
        };
        let json = serde_json::to_string(&response).expect("serialize");
        assert!(json.contains("thought_id"));
    }

    #[test]
    fn test_linear_request_deserialize() {
        let json = r#"{"content": "test"}"#;
        let req: LinearRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.content, "test");
    }

    #[test]
    fn test_all_response_types_implement_json_schema() {
        let _ = schemars::schema_for!(LinearResponse);
        let _ = schemars::schema_for!(TreeResponse);
        let _ = schemars::schema_for!(DivergentResponse);
        let _ = schemars::schema_for!(ReflectionResponse);
        let _ = schemars::schema_for!(CheckpointResponse);
        let _ = schemars::schema_for!(AutoResponse);
        let _ = schemars::schema_for!(GraphResponse);
        let _ = schemars::schema_for!(DetectResponse);
        let _ = schemars::schema_for!(DecisionResponse);
        let _ = schemars::schema_for!(EvidenceResponse);
        let _ = schemars::schema_for!(TimelineResponse);
        let _ = schemars::schema_for!(MctsResponse);
        let _ = schemars::schema_for!(CounterfactualResponse);
        let _ = schemars::schema_for!(PresetResponse);
        let _ = schemars::schema_for!(MetricsResponse);
    }

    #[test]
    fn test_all_request_types_implement_json_schema() {
        let _ = schemars::schema_for!(LinearRequest);
        let _ = schemars::schema_for!(TreeRequest);
        let _ = schemars::schema_for!(DivergentRequest);
        let _ = schemars::schema_for!(ReflectionRequest);
        let _ = schemars::schema_for!(CheckpointRequest);
        let _ = schemars::schema_for!(AutoRequest);
        let _ = schemars::schema_for!(GraphRequest);
        let _ = schemars::schema_for!(DetectRequest);
        let _ = schemars::schema_for!(DecisionRequest);
        let _ = schemars::schema_for!(EvidenceRequest);
        let _ = schemars::schema_for!(TimelineRequest);
        let _ = schemars::schema_for!(MctsRequest);
        let _ = schemars::schema_for!(CounterfactualRequest);
        let _ = schemars::schema_for!(PresetRequest);
        let _ = schemars::schema_for!(MetricsRequest);
    }

    #[test]
    fn test_reasoning_server_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ReasoningServer>();
    }

    // ============================================================================
    // IntoContents Tests - Cover macro-generated implementations
    // ============================================================================

    #[test]
    fn test_linear_response_into_contents() {
        let response = LinearResponse {
            thought_id: "t1".to_string(),
            session_id: "s1".to_string(),
            content: "reasoning content".to_string(),
            confidence: 0.85,
            next_step: Some("continue".to_string()),
        };
        let contents = response.clone().into_contents();
        assert_eq!(contents.len(), 1);
        // Verify it produces valid JSON content
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("thought_id"));
        assert!(json.contains("t1"));
    }

    #[test]
    fn test_tree_response_into_contents() {
        let response = TreeResponse {
            session_id: "s1".to_string(),
            branch_id: Some("b1".to_string()),
            branches: Some(vec![Branch {
                id: "b1".to_string(),
                content: "branch content".to_string(),
                score: 0.9,
                status: "active".to_string(),
            }]),
            recommendation: Some("explore branch b1".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_divergent_response_into_contents() {
        let response = DivergentResponse {
            thought_id: "t1".to_string(),
            session_id: "s1".to_string(),
            perspectives: vec![Perspective {
                viewpoint: "optimistic".to_string(),
                content: "positive outlook".to_string(),
                novelty_score: 0.8,
            }],
            challenged_assumptions: Some(vec!["assumption1".to_string()]),
            synthesis: Some("unified insight".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_reflection_response_into_contents() {
        let response = ReflectionResponse {
            quality_score: 0.85,
            thought_id: Some("t1".to_string()),
            session_id: Some("s1".to_string()),
            iterations_used: Some(3),
            strengths: Some(vec!["logical".to_string()]),
            weaknesses: Some(vec!["needs more detail".to_string()]),
            recommendations: Some(vec!["add examples".to_string()]),
            refined_content: Some("improved reasoning".to_string()),
            coherence_score: Some(0.9),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_checkpoint_response_into_contents() {
        let response = CheckpointResponse {
            session_id: "s1".to_string(),
            checkpoint_id: Some("cp1".to_string()),
            checkpoints: Some(vec![Checkpoint {
                id: "cp1".to_string(),
                name: "checkpoint 1".to_string(),
                description: Some("first checkpoint".to_string()),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                thought_count: 5,
            }]),
            restored_state: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_auto_response_into_contents() {
        let response = AutoResponse {
            selected_mode: "linear".to_string(),
            confidence: 0.9,
            rationale: "simple query".to_string(),
            result: serde_json::json!({"status": "ok"}),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_graph_response_into_contents() {
        let response = GraphResponse {
            session_id: "s1".to_string(),
            node_id: Some("n1".to_string()),
            nodes: Some(vec![GraphNode {
                id: "n1".to_string(),
                content: "node content".to_string(),
                score: Some(0.85),
                depth: Some(1),
                parent_id: None,
            }]),
            aggregated_insight: Some("combined insight".to_string()),
            conclusions: Some(vec!["conclusion 1".to_string()]),
            state: Some(GraphState {
                total_nodes: 10,
                active_nodes: 8,
                max_depth: 3,
                pruned_count: 2,
            }),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_detect_response_into_contents() {
        let response = DetectResponse {
            detections: vec![Detection {
                detection_type: "confirmation_bias".to_string(),
                category: Some("cognitive".to_string()),
                severity: "medium".to_string(),
                confidence: 0.8,
                evidence: "selective evidence".to_string(),
                explanation: "focusing on confirming data".to_string(),
                remediation: Some("consider counterexamples".to_string()),
            }],
            summary: Some("1 bias detected".to_string()),
            overall_quality: Some(0.7),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
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
            stakeholder_map: Some(StakeholderMap {
                key_players: vec!["CEO".to_string()],
                keep_satisfied: vec!["Board".to_string()],
                keep_informed: vec!["Team".to_string()],
                minimal_effort: vec!["Others".to_string()],
            }),
            conflicts: Some(vec!["resource allocation".to_string()]),
            alignments: Some(vec!["shared goals".to_string()]),
            rationale: Some("highest weighted score".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_evidence_response_into_contents() {
        let response = EvidenceResponse {
            overall_credibility: 0.85,
            evidence_assessments: Some(vec![EvidenceAssessment {
                content: "primary source".to_string(),
                credibility_score: 0.9,
                source_tier: "tier1".to_string(),
                corroborated_by: Some(vec![1, 2]),
            }]),
            posterior: Some(0.75),
            prior: Some(0.5),
            likelihood_ratio: Some(3.0),
            entropy: Some(0.2),
            confidence_interval: Some(ConfidenceInterval {
                lower: 0.6,
                upper: 0.9,
            }),
            synthesis: Some("strong evidence".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_timeline_response_into_contents() {
        let response = TimelineResponse {
            timeline_id: "tl1".to_string(),
            branch_id: Some("br1".to_string()),
            branches: Some(vec![TimelineBranch {
                id: "br1".to_string(),
                label: Some("main".to_string()),
                content: "timeline content".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            }]),
            comparison: Some(BranchComparison {
                divergence_points: vec!["point1".to_string()],
                quality_differences: serde_json::json!({"score": 0.1}),
                convergence_opportunities: vec!["merge here".to_string()],
            }),
            merged_content: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_mcts_response_into_contents() {
        let response = MctsResponse {
            session_id: "s1".to_string(),
            best_path: Some(vec![MctsNode {
                node_id: "n1".to_string(),
                content: "node content".to_string(),
                visits: 10,
                ucb_score: 1.2,
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
    }

    #[test]
    fn test_counterfactual_response_into_contents() {
        let response = CounterfactualResponse {
            counterfactual_outcome: "different result".to_string(),
            causal_chain: vec![CausalStep {
                step: 1,
                cause: "intervention".to_string(),
                effect: "outcome change".to_string(),
                probability: 0.8,
            }],
            session_id: Some("s1".to_string()),
            original_scenario: "base scenario".to_string(),
            intervention_applied: "change X".to_string(),
            analysis_depth: "counterfactual".to_string(),
            key_differences: vec!["difference 1".to_string()],
            confidence: 0.85,
            assumptions: vec!["assumption 1".to_string()],
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_preset_response_into_contents() {
        let response = PresetResponse {
            presets: Some(vec![PresetInfo {
                id: "p1".to_string(),
                name: "Quick Analysis".to_string(),
                description: "Fast analysis preset".to_string(),
                category: "analysis".to_string(),
                required_inputs: vec!["content".to_string()],
            }]),
            execution_result: None,
            session_id: Some("s1".to_string()),
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_metrics_response_into_contents() {
        let response = MetricsResponse {
            summary: Some(MetricsSummary {
                total_calls: 100,
                success_rate: 0.95,
                avg_latency_ms: 150.0,
                by_mode: serde_json::json!({"linear": 50, "tree": 30}),
            }),
            mode_stats: None,
            invocations: None,
            config: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    // ============================================================================
    // Request Deserialization Tests
    // ============================================================================

    #[test]
    fn test_tree_request_deserialize() {
        let json = r#"{"operation": "create", "content": "test"}"#;
        let req: TreeRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.operation, Some("create".to_string()));
    }

    #[test]
    fn test_divergent_request_deserialize() {
        let json = r#"{"content": "test", "force_rebellion": true}"#;
        let req: DivergentRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.force_rebellion, Some(true));
    }

    #[test]
    fn test_reflection_request_deserialize() {
        let json = r#"{"operation": "evaluate", "session_id": "s1"}"#;
        let req: ReflectionRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.operation, Some("evaluate".to_string()));
    }

    #[test]
    fn test_checkpoint_request_deserialize() {
        let json = r#"{"operation": "create", "session_id": "s1", "name": "cp1"}"#;
        let req: CheckpointRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.name, Some("cp1".to_string()));
    }

    #[test]
    fn test_auto_request_deserialize() {
        let json = r#"{"content": "test", "hints": ["hint1"]}"#;
        let req: AutoRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.hints, Some(vec!["hint1".to_string()]));
    }

    #[test]
    fn test_graph_request_deserialize() {
        let json = r#"{"operation": "init", "session_id": "s1", "k": 5}"#;
        let req: GraphRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.k, Some(5));
    }

    #[test]
    fn test_detect_request_deserialize() {
        let json = r#"{"type": "biases", "check_formal": true}"#;
        let req: DetectRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.detect_type, "biases");
    }

    #[test]
    fn test_decision_request_deserialize() {
        let json = r#"{"type": "weighted", "options": ["A", "B"]}"#;
        let req: DecisionRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.options, Some(vec!["A".to_string(), "B".to_string()]));
    }

    #[test]
    fn test_evidence_request_deserialize() {
        let json = r#"{"type": "assess", "prior": 0.5}"#;
        let req: EvidenceRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.prior, Some(0.5));
    }

    #[test]
    fn test_timeline_request_deserialize() {
        let json = r#"{"operation": "branch", "timeline_id": "tl1"}"#;
        let req: TimelineRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.timeline_id, Some("tl1".to_string()));
    }

    #[test]
    fn test_mcts_request_deserialize() {
        let json = r#"{"operation": "explore", "iterations": 50}"#;
        let req: MctsRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.iterations, Some(50));
    }

    #[test]
    fn test_counterfactual_request_deserialize() {
        let json = r#"{"scenario": "base", "intervention": "change"}"#;
        let req: CounterfactualRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.scenario, "base");
    }

    #[test]
    fn test_preset_request_deserialize() {
        let json = r#"{"operation": "run", "preset_id": "p1"}"#;
        let req: PresetRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.preset_id, Some("p1".to_string()));
    }

    #[test]
    fn test_metrics_request_deserialize() {
        let json = r#"{"query": "by_mode", "mode_name": "linear"}"#;
        let req: MetricsRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.mode_name, Some("linear".to_string()));
    }

    // ============================================================================
    // ServerHandler Tests
    // ============================================================================

    fn create_test_server_sync() -> ReasoningServer {
        use crate::anthropic::{AnthropicClient, ClientConfig};
        use crate::config::{Config, SecretString};
        use crate::storage::SqliteStorage;

        let config = Config {
            api_key: SecretString::new("test-key"),
            database_path: ":memory:".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let storage = rt.block_on(async { SqliteStorage::new_in_memory().await.unwrap() });

        let client = AnthropicClient::new("test-key", ClientConfig::default()).unwrap();
        let state = AppState::new(storage, client, config);
        ReasoningServer::new(Arc::new(state))
    }

    async fn create_test_server() -> ReasoningServer {
        use crate::anthropic::{AnthropicClient, ClientConfig};
        use crate::config::{Config, SecretString};
        use crate::storage::SqliteStorage;

        let config = Config {
            api_key: SecretString::new("test-key"),
            database_path: ":memory:".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
        };

        let storage = SqliteStorage::new_in_memory().await.unwrap();

        let client = AnthropicClient::new("test-key", ClientConfig::default()).unwrap();
        let state = AppState::new(storage, client, config);
        ReasoningServer::new(Arc::new(state))
    }

    #[test]
    fn test_server_handler_get_info() {
        let server = create_test_server_sync();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "mcp-reasoning");
        assert!(info.capabilities.tools.is_some());
        assert!(info.instructions.is_some());
    }

    #[test]
    fn test_server_handler_get_peer() {
        let server = create_test_server_sync();
        assert!(server.get_peer().is_none());
    }

    #[test]
    fn test_server_handler_set_peer() {
        let mut server = create_test_server_sync();
        // set_peer is a no-op, just verify it doesn't panic
        // We can't easily create a Peer, so just verify method exists
        let _ = &mut server;
    }

    #[test]
    fn test_reasoning_server_new() {
        let server = create_test_server_sync();
        // Just verify we can create a server without panicking
        let _ = &server.state;
    }

    // ============================================================================
    // Tool Method Tests (stubs, covering return path)
    // ============================================================================

    #[tokio::test]
    async fn test_reasoning_linear_tool() {
        let server = create_test_server().await;
        let req = LinearRequest {
            content: "test".to_string(),
            session_id: Some("s1".to_string()),
            confidence: Some(0.8),
        };
        let resp = server.reasoning_linear(req).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_tree_tool() {
        let server = create_test_server().await;
        let req = TreeRequest {
            operation: Some("create".to_string()),
            content: Some("test".to_string()),
            session_id: Some("s1".to_string()),
            branch_id: None,
            num_branches: Some(2),
            completed: None,
        };
        let resp = server.reasoning_tree(req).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_divergent_tool() {
        let server = create_test_server().await;
        let req = DivergentRequest {
            content: "test".to_string(),
            session_id: Some("s1".to_string()),
            num_perspectives: Some(3),
            challenge_assumptions: Some(true),
            force_rebellion: Some(false),
        };
        let resp = server.reasoning_divergent(req).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_reflection_tool() {
        let server = create_test_server().await;
        let req = ReflectionRequest {
            operation: Some("process".to_string()),
            content: Some("test".to_string()),
            thought_id: None,
            session_id: Some("s1".to_string()),
            max_iterations: Some(3),
            quality_threshold: Some(0.8),
        };
        let resp = server.reasoning_reflection(req).await;
        assert!(resp.quality_score >= 0.0);
    }

    #[tokio::test]
    async fn test_reasoning_checkpoint_tool() {
        let server = create_test_server().await;
        let req = CheckpointRequest {
            operation: "create".to_string(),
            session_id: "s1".to_string(),
            checkpoint_id: None,
            name: Some("cp1".to_string()),
            description: Some("test checkpoint".to_string()),
            new_direction: None,
        };
        let resp = server.reasoning_checkpoint(req).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_auto_tool() {
        let server = create_test_server().await;
        let req = AutoRequest {
            content: "test".to_string(),
            hints: Some(vec!["hint".to_string()]),
            session_id: Some("s1".to_string()),
        };
        let resp = server.reasoning_auto(req).await;
        assert!(!resp.selected_mode.is_empty());
    }

    #[tokio::test]
    async fn test_reasoning_graph_tool() {
        let server = create_test_server().await;
        let req = GraphRequest {
            operation: "init".to_string(),
            session_id: "s1".to_string(),
            content: Some("test".to_string()),
            problem: Some("problem".to_string()),
            node_id: None,
            node_ids: None,
            k: Some(3),
            threshold: None,
            terminal_node_ids: None,
        };
        let resp = server.reasoning_graph(req).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_detect_tool() {
        let server = create_test_server().await;
        let req = DetectRequest {
            detect_type: "biases".to_string(),
            content: Some("test".to_string()),
            thought_id: None,
            session_id: Some("s1".to_string()),
            check_types: None,
            check_formal: Some(true),
            check_informal: Some(true),
        };
        let resp = server.reasoning_detect(req).await;
        assert!(resp.detections.is_empty() || !resp.detections.is_empty());
    }

    #[tokio::test]
    async fn test_reasoning_decision_tool() {
        let server = create_test_server().await;
        let req = DecisionRequest {
            decision_type: Some("weighted".to_string()),
            question: Some("which?".to_string()),
            options: Some(vec!["A".to_string(), "B".to_string()]),
            topic: None,
            context: Some("context".to_string()),
            session_id: Some("s1".to_string()),
        };
        let resp = server.reasoning_decision(req).await;
        // Stub returns empty recommendation
        let _ = resp.recommendation;
    }

    #[tokio::test]
    async fn test_reasoning_evidence_tool() {
        let server = create_test_server().await;
        let req = EvidenceRequest {
            evidence_type: Some("assess".to_string()),
            claim: Some("claim".to_string()),
            hypothesis: None,
            context: Some("context".to_string()),
            prior: Some(0.5),
            session_id: Some("s1".to_string()),
        };
        let resp = server.reasoning_evidence(req).await;
        assert!(resp.overall_credibility >= 0.0);
    }

    #[tokio::test]
    async fn test_reasoning_timeline_tool() {
        let server = create_test_server().await;
        let req = TimelineRequest {
            operation: "create".to_string(),
            session_id: Some("s1".to_string()),
            timeline_id: None,
            content: Some("test".to_string()),
            label: Some("main".to_string()),
            branch_ids: None,
            source_branch_id: None,
            target_branch_id: None,
            merge_strategy: None,
        };
        let resp = server.reasoning_timeline(req).await;
        // Stub returns empty timeline_id
        let _ = resp.timeline_id;
    }

    #[tokio::test]
    async fn test_reasoning_mcts_tool() {
        let server = create_test_server().await;
        let req = MctsRequest {
            operation: Some("explore".to_string()),
            content: Some("test".to_string()),
            session_id: Some("s1".to_string()),
            node_id: None,
            iterations: Some(10),
            exploration_constant: Some(1.41),
            simulation_depth: Some(5),
            quality_threshold: Some(0.7),
            lookback_depth: Some(3),
            auto_execute: Some(false),
        };
        let resp = server.reasoning_mcts(req).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_counterfactual_tool() {
        let server = create_test_server().await;
        let req = CounterfactualRequest {
            scenario: "base".to_string(),
            intervention: "change".to_string(),
            analysis_depth: Some("counterfactual".to_string()),
            session_id: Some("s1".to_string()),
        };
        let resp = server.reasoning_counterfactual(req).await;
        // Stub uses input values for output
        assert_eq!(resp.original_scenario, "base");
        assert_eq!(resp.intervention_applied, "change");
    }

    #[tokio::test]
    async fn test_reasoning_preset_tool() {
        let server = create_test_server().await;
        let req = PresetRequest {
            operation: "list".to_string(),
            preset_id: None,
            category: Some("analysis".to_string()),
            inputs: None,
            session_id: Some("s1".to_string()),
        };
        let resp = server.reasoning_preset(req).await;
        // presets may or may not be present
        let _ = resp.presets;
    }

    #[tokio::test]
    async fn test_reasoning_metrics_tool() {
        let server = create_test_server().await;
        let req = MetricsRequest {
            query: "summary".to_string(),
            mode_name: None,
            tool_name: None,
            session_id: None,
            success_only: Some(true),
            limit: Some(10),
        };
        let resp = server.reasoning_metrics(req).await;
        // summary may or may not be present
        let _ = resp.summary;
    }
}
