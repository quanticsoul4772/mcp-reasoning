//! Request types for reasoning tools.
//!
//! This module contains all request types with JsonSchema support for tool parameters.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
