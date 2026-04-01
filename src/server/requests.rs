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
    /// Progress token for streaming notifications (auto-generated if not provided).
    pub progress_token: Option<String>,
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
    /// Progress token for streaming notifications (auto-generated if not provided).
    pub progress_token: Option<String>,
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
    /// Operation: explore or auto_backtrack.
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
    /// Base scenario.
    pub scenario: String,
    /// What-if change.
    pub intervention: String,
    /// Session ID.
    pub session_id: Option<String>,
    /// Analysis depth: association/intervention/counterfactual.
    pub analysis_depth: Option<String>,
    /// Progress token for streaming notifications (auto-generated if not provided).
    pub progress_token: Option<String>,
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
    /// Query: summary, by_mode, invocations, fallbacks, or config.
    pub query: String,
    /// Mode name (for by_mode query).
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
