//! Request types for reasoning tools.
//!
//! This module contains all request types with JsonSchema support for tool parameters.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// Validated scalar types
// ============================================================================

/// A confidence threshold value guaranteed to be a finite f64 in [0.0, 1.0].
///
/// Validation happens at construction via [`TryFrom<f64>`] and at JSON
/// deserialization, so handler code never needs to re-validate this field.
///
/// # Examples
///
/// ```
/// use mcp_reasoning::server::ConfidenceThreshold;
///
/// let t = ConfidenceThreshold::try_from(0.8).unwrap();
/// assert_eq!(t.value(), 0.8);
///
/// assert!(ConfidenceThreshold::try_from(f64::NAN).is_err());
/// assert!(ConfidenceThreshold::try_from(-0.1).is_err());
/// assert!(ConfidenceThreshold::try_from(1.1).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct ConfidenceThreshold(f64);

impl ConfidenceThreshold {
    /// Returns the inner f64 value.
    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for ConfidenceThreshold {
    type Error = String;

    fn try_from(v: f64) -> Result<Self, Self::Error> {
        if v.is_finite() && (0.0..=1.0).contains(&v) {
            Ok(Self(v))
        } else {
            Err(format!(
                "confidence threshold must be a finite value between 0.0 and 1.0, got {v}"
            ))
        }
    }
}

impl<'de> Deserialize<'de> for ConfidenceThreshold {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f64::deserialize(deserializer)?;
        Self::try_from(v).map_err(serde::de::Error::custom)
    }
}

/// Request for linear reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LinearRequest {
    /// Thought content to process.
    pub content: String,
    /// Session ID for context continuity.
    pub session_id: Option<String>,
    /// Minimum confidence threshold (0.0-1.0). Responses below this score are rejected.
    /// Invalid values (NaN, infinity, out of range) are rejected at parse time.
    pub confidence: Option<ConfidenceThreshold>,
    /// Per-request timeout override in milliseconds. Overrides server default when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Request for tree reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TreeRequest {
    /// Operation: create=start 2-4 exploration branches; focus=select a branch to develop further;
    /// list=review all branches and their status; complete=mark a branch finished; summarize=synthesize
    /// all branches into a final answer. Typical sequence: create → focus → list → complete → summarize.
    #[schemars(example = &"create", example = &"focus", example = &"list", example = &"complete", example = &"summarize")]
    pub operation: Option<String>,
    /// Content to explore (required for create).
    pub content: Option<String>,
    /// Session ID. Reuse to continue an existing tree; omit to start fresh.
    pub session_id: Option<String>,
    /// Branch ID (required for focus and complete operations).
    pub branch_id: Option<String>,
    /// Number of branches to create (2-4, default 3). Only used for create.
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
    /// Progress token for streaming notifications (auto-generated if not provided).
    pub progress_token: Option<String>,
}

/// Request for reflection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReflectionRequest {
    /// Operation: process=iteratively self-critique and improve a prior reasoning output (pass previous
    /// result as content); evaluate=assess an entire session for quality, consistency, and blind spots.
    /// Omit to default to process.
    #[schemars(example = &"process", example = &"evaluate")]
    pub operation: Option<String>,
    /// Prior reasoning output to improve (for process), or topic to evaluate (for evaluate).
    pub content: Option<String>,
    /// Thought ID of a specific thought to analyze (alternative to content).
    pub thought_id: Option<String>,
    /// Session ID. Required for evaluate; optional for process.
    pub session_id: Option<String>,
    /// Max critique-improve iterations (1-5, default 3). Only for process.
    pub max_iterations: Option<u32>,
    /// Stop iterating when quality reaches this threshold (0.0-1.0, default 0.8). Only for process.
    pub quality_threshold: Option<f64>,
    /// Progress token for streaming notifications (auto-generated if not provided).
    pub progress_token: Option<String>,
}

/// Request for checkpoint operations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckpointRequest {
    /// Operation: create=save current state with a label (use before exploring a risky direction);
    /// list=show available checkpoints with labels and timestamps; restore=return to a saved
    /// snapshot, discarding all reasoning done after that point.
    #[schemars(example = &"create", example = &"list", example = &"restore")]
    pub operation: String,
    /// Session ID of the reasoning session to checkpoint.
    pub session_id: String,
    /// Checkpoint ID to restore (required for restore; from list output).
    pub checkpoint_id: Option<String>,
    /// Label for the checkpoint (for create, e.g. "before risky branch").
    pub name: Option<String>,
    /// Description of what this checkpoint represents.
    pub description: Option<String>,
    /// New reasoning direction to pursue after restoring (optional, for restore).
    pub new_direction: Option<String>,
}

/// Request for confidence-based routing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConfidenceRouteRequest {
    /// Content to analyze and reason about.
    pub content: String,
    /// Session ID for context continuity.
    pub session_id: Option<String>,
    /// Confidence threshold above which the auto-selected mode is used directly (0.0-1.0, default 0.75).
    /// Below this threshold, falls back to tree reasoning for thoroughness.
    pub high_confidence_threshold: Option<f64>,
    /// Compute budget: "low" forces linear (fast), "high" forces tree (thorough), "auto" (default) uses confidence routing.
    pub budget: Option<String>,
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
    /// When true, select the mode and immediately execute it.
    /// Supported modes: linear, divergent. Other modes return selection only with a next_call hint.
    #[serde(default)]
    pub execute: Option<bool>,
}

/// Request for meta-reasoning (empirical tool selection).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetaRequest {
    /// Content/problem to analyze for routing.
    pub content: String,
    /// Manual problem type hint (skips classification if provided).
    /// Categories: math, code_review, planning, brainstorming, summarization, research, evaluation, causal, temporal, other.
    pub problem_type_hint: Option<String>,
    /// Minimum confidence threshold for recommendation (0.0-1.0, default 0.4).
    pub min_confidence: Option<f64>,
}

/// Request for graph reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphRequest {
    /// Operation: init=start graph with a problem; generate=expand node with continuations;
    /// score=evaluate node quality; aggregate=merge multiple nodes; refine=improve a node;
    /// prune=remove low-quality nodes below threshold; finalize=synthesize terminal nodes into answer;
    /// state=show current graph structure. Typical sequence: init → generate → score → prune → finalize.
    #[schemars(example = &"init", example = &"generate", example = &"score", example = &"prune", example = &"finalize", example = &"state")]
    pub operation: String,
    /// Session ID. Required for all operations except init.
    pub session_id: String,
    /// Problem description (required for init).
    pub content: Option<String>,
    /// Additional problem context passed to generation/scoring operations.
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
    /// Type: biases=detect cognitive distortions (anchoring, confirmation bias, availability heuristic);
    /// fallacies=detect logical errors (ad hominem, strawman, false dichotomy, slippery slope).
    #[serde(rename = "type")]
    #[schemars(example = &"biases", example = &"fallacies")]
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
    /// Type: weighted=score options against weighted criteria (most common); pairwise=compare options
    /// head-to-head in pairs; topsis=rank by distance from ideal/worst solution; perspectives=map
    /// stakeholder viewpoints on a topic. Omit to default to weighted.
    #[serde(rename = "type")]
    #[schemars(example = &"weighted", example = &"pairwise", example = &"topsis", example = &"perspectives")]
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
    /// Type: assess=evaluate evidence quality and credibility for a claim; probabilistic=Bayesian
    /// belief update (provide prior + evidence to get posterior probability). Omit to default to assess.
    #[serde(rename = "type")]
    #[schemars(example = &"assess", example = &"probabilistic")]
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
    /// Operation: create=start a timeline with an initial event sequence; branch=create an alternate
    /// timeline from a decision point; compare=diff two timelines to show divergence; merge=synthesize
    /// two timelines into a unified view.
    #[schemars(example = &"create", example = &"branch", example = &"compare", example = &"merge")]
    pub operation: String,
    /// Initial event sequence or branching context (required for create and branch).
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
    /// Operation: explore=run UCB1-guided MCTS iterations from a node; auto_backtrack=automatically
    /// backtrack and re-explore when a path yields low reward. Omit to default to explore.
    #[schemars(example = &"explore", example = &"auto_backtrack")]
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
    /// Progress token for streaming notifications (auto-generated if not provided).
    pub progress_token: Option<String>,
}

/// Request for counterfactual analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CounterfactualRequest {
    /// The base situation to analyze (e.g., "Company X launched product Y in Q1").
    pub scenario: String,
    /// The hypothetical change to evaluate (e.g., "What if they had launched in Q3 instead?").
    pub intervention: String,
    /// Session ID for context continuity.
    pub session_id: Option<String>,
    /// Pearl's Ladder level — how deep the causal analysis goes:
    /// association=correlations and patterns (Level 1, fastest);
    /// intervention=effects of actively changing something (Level 2);
    /// counterfactual=what would have happened under a different history (Level 3, most thorough).
    /// Omit to default to counterfactual (full analysis).
    #[schemars(example = &"association", example = &"intervention", example = &"counterfactual")]
    pub analysis_depth: Option<String>,
    /// Progress token for streaming notifications (auto-generated if not provided).
    pub progress_token: Option<String>,
}

/// Request for preset operations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PresetRequest {
    /// Operation: list=show available presets with descriptions; run=execute a preset by ID.
    #[schemars(example = &"list", example = &"run")]
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
    /// Query type: summary=overall usage stats; by_mode=stats for a specific reasoning mode
    /// (requires mode_name); invocations=recent tool call log; fallbacks=cases where mode
    /// selection fell back; config=current server configuration.
    #[schemars(example = &"summary", example = &"by_mode", example = &"invocations", example = &"fallbacks", example = &"config")]
    pub query: String,
    /// Mode name to query (required for by_mode, e.g. "linear", "tree", "graph").
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
// Self-Improvement Requests
// ============================================================================

/// Request for self-improvement status.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiStatusRequest {
    // Empty - no parameters needed
}

/// Request for pending diagnoses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiDiagnosesRequest {
    /// Maximum number of diagnoses to return.
    pub limit: Option<u32>,
}

/// Request to approve a diagnosis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiApproveRequest {
    /// The diagnosis ID to approve.
    pub diagnosis_id: String,
}

/// Request to reject a diagnosis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiRejectRequest {
    /// The diagnosis ID to reject.
    pub diagnosis_id: String,
    /// Optional reason for rejection.
    pub reason: Option<String>,
}

/// Request to trigger an improvement cycle.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiTriggerRequest {
    // Empty - no parameters needed
}

/// Request to rollback an action.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SiRollbackRequest {
    /// The action ID to rollback.
    pub action_id: String,
}

// ============================================================================
// Memory Tools Requests
// ============================================================================

/// Request for listing reasoning sessions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSessionsRequest {
    /// Maximum number of sessions to return.
    pub limit: Option<u32>,
    /// Number of sessions to skip.
    pub offset: Option<u32>,
    /// Filter by reasoning mode.
    pub mode_filter: Option<String>,
}

/// Request for resuming a reasoning session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResumeSessionRequest {
    /// Session ID to resume.
    pub session_id: String,
    /// Include checkpoints in the response.
    pub include_checkpoints: Option<bool>,
    /// Compress long content using Claude.
    pub compress: Option<bool>,
}

/// Request for semantic search over reasoning sessions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchSessionsRequest {
    /// Search query.
    pub query: String,
    /// Maximum number of results.
    pub limit: Option<u32>,
    /// Minimum similarity score (0.0-1.0).
    pub min_similarity: Option<f32>,
    /// Filter by reasoning mode.
    pub mode_filter: Option<String>,
}

/// Request for analyzing session relationships.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelateSessionsRequest {
    /// Session ID to analyze (if None, analyzes all sessions).
    pub session_id: Option<String>,
    /// Maximum graph depth.
    pub depth: Option<u32>,
    /// Minimum relationship strength (0.0-1.0).
    pub min_strength: Option<f32>,
}

// ============================================================================
// Agent & Skill Requests
// ============================================================================

/// Request to invoke an agent on a task.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentInvokeRequest {
    /// The agent ID to invoke (e.g., "analyst", "strategist", "explorer").
    pub agent_id: String,
    /// The task for the agent to work on.
    pub task: String,
    /// Session ID for context continuity.
    pub session_id: Option<String>,
}

/// Request to list available agents.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentListRequest {
    /// Filter by role (e.g., "analyst", "strategist").
    pub role: Option<String>,
}

/// Request to run a composable skill.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillRunRequest {
    /// The skill ID to run.
    pub skill_id: String,
    /// Input for the skill.
    pub input: String,
    /// Session ID for context continuity.
    pub session_id: Option<String>,
}

/// Request to run an agent team on a task.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeamRunRequest {
    /// The team ID to run.
    pub team_id: String,
    /// The task for the team.
    pub task: String,
    /// Session ID for context continuity.
    pub session_id: Option<String>,
}

/// Request to list available teams.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeamListRequest {
    /// Optional filter.
    pub topology: Option<String>,
}

/// Request to query agent metrics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentMetricsRequest {
    /// Query type: "summary", "by_agent", "discovered_skills".
    pub query: String,
    /// Optional agent ID filter.
    pub agent_id: Option<String>,
}

/// Request to invoke a CrewAI hierarchical crew.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrewInvokeRequest {
    /// The task or research question for the crew.
    pub task: String,
    /// Crew type: "research" (Searcher+Verifier), "code" (Planner+Implementer+Critic),
    /// or "infra" (Monitor+Fixer).
    pub crew_type: String,
    /// For code crew: the repository directory to work in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_dir: Option<String>,
    /// Optional output file path. If provided, the crew writes its report there.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file: Option<String>,
}
