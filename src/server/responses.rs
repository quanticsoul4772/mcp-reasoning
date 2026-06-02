//! Response types for reasoning tools.
//!
//! This module contains all response types with JsonSchema support.

use std::collections::HashMap;

use crate::metadata::ResponseMetadata;
use rmcp::model::{Content, IntoContents};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Macro to implement `IntoContents` for response types by serializing to JSON.
macro_rules! impl_into_contents {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoContents for $ty {
                fn into_contents(self) -> Vec<Content> {
                    match serde_json::to_string_pretty(&self) {
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

/// Machine-readable hint for the next tool call on error or workflow continuation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NextCallHint {
    /// Tool name to call next (e.g. "reasoning_checkpoint").
    pub tool: String,
    /// Arguments to pass verbatim to the next tool.
    pub args: serde_json::Value,
    /// Human-readable reason for this suggestion.
    pub reason: String,
}

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
    /// Whether `confidence` met the requested threshold (`None` if none requested,
    /// `Some(false)` if the returned analysis fell short).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meets_threshold: Option<bool>,
    /// Whether the model reported it lacked the information to reach a conclusion.
    #[serde(default)]
    pub insufficient_context: bool,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
    /// Machine-readable hint for the next step in the workflow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_call: Option<NextCallHint>,
}

/// A branch in tree reasoning.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Branch {
    /// Branch identifier.
    pub id: String,
    /// Branch title/label.
    #[serde(default)]
    pub title: String,
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
    /// Synthesized conclusion across all branches (summarize operation).
    pub synthesis: Option<String>,
    /// Key findings across all branches (summarize operation).
    pub key_findings: Option<Vec<String>>,
    /// Best insights from the exploration (summarize operation).
    pub best_insights: Option<Vec<String>>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
    /// Machine-readable hint for the next step in the workflow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_call: Option<NextCallHint>,
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
    /// The single most important insight from this viewpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_insight: Option<String>,
    /// What this perspective might miss.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blind_spots: Option<Vec<String>>,
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
    /// Session completeness score (evaluate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completeness_score: Option<f64>,
    /// Session depth score (evaluate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_score: Option<f64>,
    /// Expected confidence improvement from the refinement (process).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_improvement: Option<f64>,
    /// Key insights drawn from the session (evaluate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_insights: Option<Vec<String>>,
    /// Higher-level meta observations (evaluate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_observations: Option<String>,
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
    /// Machine-readable suggestion for the next tool call on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_call: Option<NextCallHint>,
}

/// A suggestion to use a named skill workflow instead of a single reasoning mode.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillSuggestion {
    /// Skill ID to run (e.g. "claim-verification").
    pub skill_id: String,
    /// Why this skill is recommended for this content.
    pub reason: String,
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
    /// Machine-readable hint for the next step in the workflow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_call: Option<NextCallHint>,
    /// Whether the selected mode was immediately executed (execute=true was requested and the mode is supported).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed: Option<bool>,
    /// Suggested skill workflow to run instead of (or after) the selected mode.
    /// Present when content is better served by a multi-step skill pipeline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_suggestion: Option<SkillSuggestion>,
}

/// Response from confidence-based routing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConfidenceRouteResponse {
    /// Mode actually executed (may differ from auto_suggested_mode when confidence was low).
    pub executed_mode: String,
    /// Mode that reasoning_auto would have suggested.
    pub auto_suggested_mode: String,
    /// Confidence score from auto-detection (0.0-1.0).
    pub routing_confidence: f64,
    /// Routing decision made: "direct" (high confidence), "escalated_to_tree" (low confidence), "budget_override".
    pub routing_decision: String,
    /// Human-readable explanation of why this mode was chosen.
    pub routing_reason: String,
    /// Reasoning result from the executed mode.
    pub result: serde_json::Value,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::metadata::ResponseMetadata>,
    /// Machine-readable hint for the next step in the workflow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_call: Option<NextCallHint>,
}

/// Response from meta-reasoning tool selection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetaResponse {
    /// Recommended reasoning tool to use next.
    pub selected_tool: String,
    /// Classified problem type.
    pub problem_type: String,
    /// Confidence in recommendation (0.0-1.0).
    pub confidence: f64,
    /// Reasoning for the selection.
    pub reasoning: String,
    /// Whether the selection fell back to auto mode (no effectiveness data).
    pub fallback_to_auto: bool,
    /// Number of candidate tools considered.
    pub candidates_evaluated: usize,
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

/// Consistency check over a graph operation's numeric/structural output.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphValidationInfo {
    /// True when every checked value is in range and the state counts reconcile.
    pub consistent: bool,
    /// Descriptions of every discrepancy (out-of-range score, count mismatch).
    pub warnings: Vec<String>,
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
    /// Consistency check over this operation's output. Set for operations that
    /// return verifiable numeric/structural data (generate scores, state counts).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<GraphValidationInfo>,
    /// Set when one or more nodes/edges failed to persist to storage, so the
    /// caller knows the graph wasn't fully saved (reasoning result is unaffected).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence_warning: Option<String>,
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
    /// Whether removing this would change the conclusion ("yes"/"no"/"maybe").
    /// The most actionable signal: which findings actually matter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes_conclusion: Option<String>,
    /// Whether the cited evidence/passage was found verbatim in the analyzed
    /// content. `None` when grounding does not apply (e.g. knowledge gaps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounded: Option<bool>,
}

/// Structure of an analyzed argument (premises, conclusion, validity).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArgumentStructureInfo {
    /// Identified premises.
    pub premises: Vec<String>,
    /// The main conclusion.
    pub conclusion: String,
    /// Validity: "valid", "invalid", or "partially_valid".
    pub validity: String,
}

/// Result of verifying a detection result against the analyzed content.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectValidationInfo {
    /// True when reported counts match the detections and all quotes are grounded.
    pub consistent: bool,
    /// Descriptions of every discrepancy (count mismatch, ungrounded quote).
    pub warnings: Vec<String>,
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
    /// Debiased restatement of the argument (biases operation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debiased_version: Option<String>,
    /// Decomposed argument structure (fallacies operation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument_structure: Option<ArgumentStructureInfo>,
    /// Assumptions taken as given without verification (knowledge_gaps operation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unchallenged_assumptions: Option<Vec<String>>,
    /// The subset of biases that, if removed, would change the conclusion
    /// (biases operation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conclusion_altering_biases: Option<String>,
    /// Result of verifying counts and quote grounding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<DetectValidationInfo>,
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

/// A criterion with its weight (weighted analysis breakdown).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CriterionInfo {
    /// Criterion name.
    pub name: String,
    /// Weight (0.0-1.0).
    pub weight: f64,
    /// What this criterion measures.
    pub description: String,
}

/// A criterion with its benefit/cost type (TOPSIS breakdown).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TopsisCriterionInfo {
    /// Criterion name.
    pub name: String,
    /// "benefit" (higher is better) or "cost" (lower is better).
    pub criterion_type: String,
    /// Weight (0.0-1.0).
    pub weight: f64,
}

/// Distances to the ideal and anti-ideal solutions (TOPSIS breakdown).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DistanceInfo {
    /// Distance to the ideal solution.
    pub to_ideal: f64,
    /// Distance to the anti-ideal solution.
    pub to_anti_ideal: f64,
}

/// A single head-to-head comparison (pairwise breakdown).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComparisonInfo {
    /// First option.
    pub option_a: String,
    /// Second option.
    pub option_b: String,
    /// Which was preferred: "option_a", "option_b", or "tie".
    pub preferred: String,
    /// Preference strength: "strong", "moderate", or "slight".
    pub strength: String,
    /// Why this option was preferred.
    pub reasoning: String,
}

/// Full scoring breakdown for a weighted analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeightedBreakdown {
    /// Evaluation criteria with weights.
    pub criteria: Vec<CriterionInfo>,
    /// Per-option, per-criterion scores.
    pub scores: HashMap<String, HashMap<String, f64>>,
    /// Verified weighted total per option.
    pub weighted_totals: HashMap<String, f64>,
}

/// Full scoring breakdown for a TOPSIS analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TopsisBreakdown {
    /// Criteria with benefit/cost types and weights.
    pub criteria: Vec<TopsisCriterionInfo>,
    /// Verified relative closeness (0-1) per option.
    pub closeness: HashMap<String, f64>,
    /// Distances to ideal/anti-ideal per option.
    pub distances: HashMap<String, DistanceInfo>,
}

/// Full breakdown for a pairwise analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PairwiseBreakdown {
    /// Each head-to-head comparison.
    pub comparisons: Vec<ComparisonInfo>,
    /// The model's transitivity/consistency note.
    pub consistency_check: String,
}

/// Operation-specific scoring breakdown, so the recommendation is auditable.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DecisionBreakdown {
    /// Present for weighted analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weighted: Option<WeightedBreakdown>,
    /// Present for TOPSIS analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topsis: Option<TopsisBreakdown>,
    /// Present for pairwise analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairwise: Option<PairwiseBreakdown>,
}

/// Result of verifying the arithmetic behind the decision.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DecisionValidationInfo {
    /// True when the model's numbers reconcile with the recomputed values.
    pub consistent: bool,
    /// Descriptions of every discrepancy found.
    pub warnings: Vec<String>,
    /// True when the ranking was re-ordered to match the verified arithmetic.
    pub ranking_corrected: bool,
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
    /// Operation-specific scoring breakdown (criteria, weights, scores).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakdown: Option<DecisionBreakdown>,
    /// Arithmetic-verification result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<DecisionValidationInfo>,
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
    /// Per-dimension credibility breakdown (assess).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credibility: Option<CredibilityBreakdown>,
    /// Quality breakdown — relevance/strength/representativeness (assess).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<QualityBreakdown>,
}

/// Per-dimension credibility breakdown for a piece of evidence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CredibilityBreakdown {
    /// Expertise of the source (0.0-1.0).
    pub expertise: f64,
    /// Objectivity of the source (0.0-1.0).
    pub objectivity: f64,
    /// Level of corroboration (0.0-1.0).
    pub corroboration: f64,
    /// Recency of the evidence (0.0-1.0).
    pub recency: f64,
    /// Overall credibility (0.0-1.0).
    pub overall: f64,
}

/// Quality breakdown for a piece of evidence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QualityBreakdown {
    /// Relevance to the claim (0.0-1.0).
    pub relevance: f64,
    /// Strength of support (0.0-1.0).
    pub strength: f64,
    /// Representativeness (0.0-1.0).
    pub representativeness: f64,
    /// Overall quality (0.0-1.0).
    pub overall: f64,
}

/// One piece of evidence in a Bayesian update.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BayesianEvidence {
    /// Description of the evidence.
    pub evidence: String,
    /// P(E|H) — likelihood if the hypothesis is true.
    pub likelihood_if_true: f64,
    /// P(E|¬H) — likelihood if the hypothesis is false.
    pub likelihood_if_false: f64,
    /// Bayes factor = P(E|H) / P(E|¬H).
    pub bayes_factor: f64,
}

/// Full Bayesian breakdown for a probabilistic update.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BayesianBreakdown {
    /// Prior probability stated by the model.
    pub prior: f64,
    /// Why the prior was chosen.
    pub prior_basis: String,
    /// Per-evidence likelihoods and Bayes factors.
    pub evidence: Vec<BayesianEvidence>,
    /// Product of the per-evidence Bayes factors.
    pub combined_bayes_factor: f64,
    /// Posterior the model stated.
    pub stated_posterior: f64,
    /// Posterior recomputed from prior × combined Bayes factor (Bayes' rule).
    pub computed_posterior: f64,
    /// The model's explanation of the posterior calculation.
    pub posterior_calculation: String,
    /// Belief change direction: "increase"/"decrease"/"unchanged".
    pub belief_direction: String,
    /// Belief change magnitude: "strong"/"moderate"/"slight".
    pub belief_magnitude: String,
    /// Plain-language interpretation.
    pub interpretation: String,
    /// Sensitivity of the posterior to prior assumptions.
    pub sensitivity: String,
}

/// Result of verifying the Bayesian arithmetic.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceValidationInfo {
    /// True when the stated posterior, Bayes factors, and direction all reconcile.
    pub consistent: bool,
    /// Descriptions of every discrepancy found.
    pub warnings: Vec<String>,
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
    /// Overall evidential support (assess).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidential_support: Option<f64>,
    /// The single piece of evidence that, if false, would most change the
    /// conclusion (assess).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pivot_evidence: Option<String>,
    /// Full Bayesian breakdown (probabilistic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bayesian: Option<BayesianBreakdown>,
    /// Result of verifying the Bayesian arithmetic (probabilistic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<EvidenceValidationInfo>,
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

/// A timeline event with its causal links (create).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineEventInfo {
    /// Event identifier.
    pub id: String,
    /// Description of the event.
    pub description: String,
    /// Time marker (relative or absolute).
    pub time: String,
    /// Type: "event"/"state"/"decision_point".
    pub event_type: String,
    /// Event IDs that cause this one.
    pub causes: Vec<String>,
    /// Event IDs caused by this one.
    pub effects: Vec<String>,
}

/// A decision point on the timeline (create).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DecisionPointInfo {
    /// Decision identifier.
    pub id: String,
    /// Description of the decision.
    pub description: String,
    /// Possible choices.
    pub options: Vec<String>,
    /// When the decision must be made.
    pub deadline: String,
}

/// The timeline's temporal structure (create).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TemporalStructureInfo {
    /// Beginning event ID.
    pub start: String,
    /// Current event ID.
    pub current: String,
    /// How far into the future is considered.
    pub horizon: String,
}

/// An event within a branch (branch).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BranchEventInfo {
    /// Event identifier.
    pub id: String,
    /// Description of the event.
    pub description: String,
    /// Probability of this event occurring (0.0-1.0).
    pub probability: f64,
    /// Time offset from the branch point.
    pub time_offset: String,
}

/// A branch with its events and quality scores (branch).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BranchInfo {
    /// Branch identifier.
    pub id: String,
    /// The choice made at the branch point.
    pub choice: String,
    /// How plausible this branch is (0.0-1.0).
    pub plausibility: f64,
    /// Quality of the outcome (0.0-1.0).
    pub outcome_quality: f64,
    /// Events along this branch.
    pub events: Vec<BranchEventInfo>,
}

/// A dimension-by-dimension difference between branches (compare).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BranchDifferenceInfo {
    /// What dimension is being compared.
    pub dimension: String,
    /// Outcome in branch 1.
    pub branch_1_value: String,
    /// Outcome in branch 2.
    pub branch_2_value: String,
    /// Why this difference matters.
    pub significance: String,
}

/// Per-branch risk assessment (compare).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RiskAssessmentInfo {
    /// Risks in branch 1.
    pub branch_1_risks: Vec<String>,
    /// Risks in branch 2.
    pub branch_2_risks: Vec<String>,
}

/// Per-branch opportunity assessment (compare).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OpportunityAssessmentInfo {
    /// Opportunities in branch 1.
    pub branch_1_opportunities: Vec<String>,
    /// Opportunities in branch 2.
    pub branch_2_opportunities: Vec<String>,
}

/// Recommendation from a comparison (compare).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompareRecommendationInfo {
    /// Preferred branch, or "depends".
    pub preferred_branch: String,
    /// Conditions under which this is preferred.
    pub conditions: String,
    /// Key factors in the decision.
    pub key_factors: String,
}

/// A pattern observed across branches (merge).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommonPatternInfo {
    /// Description of the pattern.
    pub pattern: String,
    /// How often it appears (0.0-1.0).
    pub frequency: f64,
    /// What the pattern implies.
    pub implications: String,
}

/// A strategy that works robustly across branches (merge).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RobustStrategyInfo {
    /// Description of the strategy.
    pub strategy: String,
    /// How effective it is (0.0-1.0).
    pub effectiveness: f64,
    /// When it is applicable.
    pub conditions: String,
}

/// A strategy that only works in some branches (merge).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FragileStrategyInfo {
    /// Description of the strategy.
    pub strategy: String,
    /// When it fails.
    pub failure_modes: String,
}

/// Result of validating a timeline's references and value ranges.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineValidationInfo {
    /// True when references resolve and values are in range.
    pub consistent: bool,
    /// Descriptions of every issue found.
    pub warnings: Vec<String>,
}

/// Response from timeline reasoning.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
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
    /// Events on the timeline (create).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<TimelineEventInfo>>,
    /// Decision points (create).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_points: Option<Vec<DecisionPointInfo>>,
    /// Temporal structure (create).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporal_structure: Option<TemporalStructureInfo>,
    /// Branches with their events and quality scores (branch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_details: Option<Vec<BranchInfo>>,
    /// Where the branches diverged (compare).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergence_point: Option<String>,
    /// Dimension-by-dimension differences (compare).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub differences: Option<Vec<BranchDifferenceInfo>>,
    /// Per-branch risk assessment (compare).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_assessment: Option<RiskAssessmentInfo>,
    /// Per-branch opportunity assessment (compare).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opportunity_assessment: Option<OpportunityAssessmentInfo>,
    /// Recommendation from the comparison (compare).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<CompareRecommendationInfo>,
    /// Patterns observed across branches (merge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub common_patterns: Option<Vec<CommonPatternInfo>>,
    /// Strategies that work robustly (merge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robust_strategies: Option<Vec<RobustStrategyInfo>>,
    /// Strategies that are fragile (merge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fragile_strategies: Option<Vec<FragileStrategyInfo>>,
    /// Overall synthesis (merge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthesis: Option<String>,
    /// Actionable recommendations (merge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendations: Option<Vec<String>>,
    /// Branch IDs involved (branch/compare/merge).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_ids: Option<Vec<String>>,
    /// Result of validating references and value ranges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<TimelineValidationInfo>,
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

/// A frontier node with its full UCB1 decomposition (explore).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsFrontierNode {
    /// Node identifier.
    pub node_id: String,
    /// Number of visits.
    pub visits: u32,
    /// Exploitation term — average value from simulations.
    pub average_value: f64,
    /// Exploration term — the UCB1 bonus.
    pub exploration_bonus: f64,
    /// UCB1 score = `average_value + exploration_bonus`.
    pub ucb1_score: f64,
}

/// The node UCB1 selected for expansion (explore).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsSelectedNode {
    /// Node identifier.
    pub node_id: String,
    /// Why UCB1 selected this node.
    pub selection_reason: String,
}

/// A newly expanded child node with its generated content (explore).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsExpandedNode {
    /// Node identifier.
    pub id: String,
    /// The generated thought.
    pub content: String,
    /// Simulated value.
    pub simulated_value: f64,
}

/// Backpropagation results (explore).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsBackpropagation {
    /// Nodes whose statistics were updated.
    pub updated_nodes: Vec<String>,
    /// Value change per node.
    pub value_changes: HashMap<String, f64>,
}

/// An alternative action considered during backtracking.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsAlternative {
    /// Action: "prune"/"refine"/"widen"/"continue".
    pub action: String,
    /// Why this might be appropriate.
    pub rationale: String,
}

/// The final recommended action (auto_backtrack).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsRecommendation {
    /// Action: "backtrack"/"continue"/"terminate".
    pub action: String,
    /// Confidence in the recommendation (0.0-1.0).
    pub confidence: f64,
    /// Expected benefit of the action.
    pub expected_benefit: String,
}

/// Result of verifying the UCB1 arithmetic / selection / quality trend.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsValidationInfo {
    /// True when the UCB1 decomposition, selection, and trend all reconcile.
    pub consistent: bool,
    /// Descriptions of every discrepancy found.
    pub warnings: Vec<String>,
}

/// Advisory stop signal for an explore step: whether the search has converged
/// enough to commit, so the caller knows when to stop iterating. Derived from
/// the frontier UCB1 scores and best-path value; never blocks, only advises.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsConvergence {
    /// True when one candidate clearly dominates or the best value is near-optimal.
    pub converged: bool,
    /// Human-readable explanation of the convergence verdict.
    pub reason: String,
    /// UCB1 gap between the top frontier node and the runner-up (0.0 if <2 nodes).
    pub top_gap: f64,
    /// Best-path value reported by this step, echoed for the stop decision.
    pub best_value: f64,
}

/// Response from MCTS.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsResponse {
    /// Session identifier.
    pub session_id: String,
    /// Deprecated: legacy alias that actually holds the frontier nodes, kept for
    /// backward compatibility. Prefer `frontier`, which carries the full UCB1
    /// decomposition (visits, average value, exploration bonus).
    pub best_path: Option<Vec<MctsNode>>,
    /// Iterations completed.
    pub iterations_completed: Option<u32>,
    /// Backtrack suggestion.
    pub backtrack_suggestion: Option<BacktrackSuggestion>,
    /// Whether backtrack was executed.
    pub executed: Option<bool>,
    /// Frontier nodes with their full UCB1 decomposition (explore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontier: Option<Vec<MctsFrontierNode>>,
    /// The node UCB1 selected for expansion (explore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_node: Option<MctsSelectedNode>,
    /// Newly expanded child nodes with their generated content (explore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded_nodes: Option<Vec<MctsExpandedNode>>,
    /// Backpropagation results (explore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backpropagation: Option<MctsBackpropagation>,
    /// Best path value found so far (explore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_path_value: Option<f64>,
    /// The node to return to if backtracking (auto_backtrack).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backtrack_to: Option<String>,
    /// Recent value samples used to assess quality (auto_backtrack).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_values: Option<Vec<f64>>,
    /// Quality trend: "declining"/"stable"/"improving" (auto_backtrack).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_trend: Option<String>,
    /// Alternative actions considered (auto_backtrack).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<MctsAlternative>>,
    /// Final recommended action (auto_backtrack).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<MctsRecommendation>,
    /// Result of verifying the UCB1 math / selection / quality trend.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<MctsValidationInfo>,
    /// Advisory stop signal: whether the search has converged enough to commit
    /// (explore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convergence: Option<MctsConvergence>,
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

/// A causal edge with its relationship type.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CausalEdgeInfo {
    /// Source variable.
    pub from: String,
    /// Target variable.
    pub to: String,
    /// Relationship: "direct"/"mediated"/"confounded".
    pub edge_type: String,
}

/// The causal model (DAG): variables, typed edges, and confounders.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CausalModelInfo {
    /// Variable names.
    pub nodes: Vec<String>,
    /// Causal edges with their types.
    pub edges: Vec<CausalEdgeInfo>,
    /// Variables that affect both cause and effect.
    pub confounders: Vec<String>,
}

/// Rung 1 — association: what correlates with what.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssociationInfo {
    /// Observed correlation (-1.0 to 1.0).
    pub observed_correlation: f64,
    /// Interpretation of the association.
    pub interpretation: String,
}

/// Rung 2 — intervention: the effect of `do(X)`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InterventionInfo {
    /// Estimated causal effect.
    pub causal_effect: f64,
    /// How the intervention would work.
    pub mechanism: String,
}

/// Result of validating the causal model and value ranges.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CounterfactualValidationInfo {
    /// True when the DAG is structurally consistent and values are in range.
    pub consistent: bool,
    /// Descriptions of every issue found.
    pub warnings: Vec<String>,
}

/// Response from counterfactual analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CounterfactualResponse {
    /// Counterfactual outcome.
    pub counterfactual_outcome: String,
    /// Deprecated: legacy flat edge list, kept for backward compatibility. Each
    /// step's `probability` is the overall analysis confidence repeated, NOT a
    /// real per-edge probability. Prefer `causal_model`, which carries the typed
    /// edges (direct/mediated/confounded).
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
    /// Which rung of Pearl's Ladder the question sits on:
    /// "association"/"intervention"/"counterfactual".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ladder_rung: Option<String>,
    /// Rung 1 — the observed association.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub association: Option<AssociationInfo>,
    /// Rung 2 — the interventional effect, `P(Y|do(X))`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intervention: Option<InterventionInfo>,
    /// Rung 3 — the counterfactual scenario (the outcome is in
    /// `counterfactual_outcome`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterfactual_scenario: Option<String>,
    /// The causal model (variables, typed edges, confounders).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_model: Option<CausalModelInfo>,
    /// The conclusion's causal claim.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_claim: Option<String>,
    /// Strength of the causal evidence: "strong"/"moderate"/"weak".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causal_strength: Option<String>,
    /// What the analysis means for decisions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actionable_insight: Option<String>,
    /// Result of validating the causal model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<CounterfactualValidationInfo>,
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
    /// Metadata about execution timing and suggestions
    pub metadata: Option<ResponseMetadata>,
    /// Machine-readable suggestion for the next tool call on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_call: Option<NextCallHint>,
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
// ============================================================================
// Memory Tools Responses
// ============================================================================

/// Response from listing reasoning sessions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSessionsResponse {
    /// List of session summaries.
    pub sessions: Vec<SessionSummary>,
    /// Total number of sessions.
    pub total: u32,
    /// Whether there are more results.
    pub has_more: bool,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

/// Summary of a reasoning session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionSummary {
    /// Session ID.
    pub session_id: String,
    /// Session creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
    /// Number of thoughts in session.
    pub thought_count: u32,
    /// Preview of first thought.
    pub preview: String,
    /// Primary reasoning mode used.
    pub primary_mode: Option<String>,
}

/// Response from resuming a reasoning session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResumeSessionResponse {
    /// Session ID.
    pub session_id: String,
    /// Session creation timestamp.
    pub created_at: String,
    /// Session summary.
    pub summary: String,
    /// Full thought chain.
    pub thought_chain: Vec<ThoughtSummary>,
    /// Key conclusions from the session.
    pub key_conclusions: Vec<String>,
    /// Last reasoning mode used.
    pub last_mode: Option<String>,
    /// Latest checkpoint if any.
    pub checkpoint: Option<CheckpointInfo>,
    /// Continuation suggestions.
    pub continuation_suggestions: Vec<String>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
    /// Machine-readable suggestion for the next tool call on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_call: Option<NextCallHint>,
}

/// Summary of a thought in a session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThoughtSummary {
    /// Thought ID.
    pub id: String,
    /// Reasoning mode used.
    pub mode: String,
    /// Thought content.
    pub content: String,
    /// Confidence score.
    pub confidence: f64,
}

/// Checkpoint information.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckpointInfo {
    /// Checkpoint ID.
    pub id: String,
    /// Checkpoint name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
}

/// Response from semantic search.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchSessionsResponse {
    /// Search results.
    pub results: Vec<SearchResult>,
    /// Number of results returned.
    pub count: u32,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    /// Session ID.
    pub session_id: String,
    /// Similarity score (0.0-1.0).
    pub similarity_score: f64,
    /// Content preview.
    pub preview: String,
    /// Session creation timestamp.
    pub created_at: String,
    /// Primary reasoning mode.
    pub primary_mode: Option<String>,
}

/// Response from relationship analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelateSessionsResponse {
    /// Session nodes in the graph.
    pub nodes: Vec<SessionNode>,
    /// Relationship edges.
    pub edges: Vec<RelationshipEdge>,
    /// Response metadata for discoverability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

/// A session node in the relationship graph.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionNode {
    /// Session ID.
    pub session_id: String,
    /// Content preview.
    pub preview: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// A relationship edge between sessions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelationshipEdge {
    /// Source session ID.
    pub from_session: String,
    /// Target session ID.
    pub to_session: String,
    /// Relationship type.
    pub relationship_type: String,
    /// Relationship strength (0.0-1.0).
    pub strength: f64,
}

// ============================================================================
// Agent & Skill Responses
// ============================================================================

/// Response from agent invocation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentInvokeResponse {
    /// Agent that was invoked.
    pub agent_id: String,
    /// Session used.
    pub session_id: String,
    /// Number of steps executed.
    pub steps_executed: usize,
    /// Final synthesis.
    pub synthesis: String,
    /// Overall success.
    pub success: bool,
    /// Agent status.
    pub status: String,
    /// Response metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
    /// Machine-readable suggestion for the next tool call on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_call: Option<NextCallHint>,
}

/// Response from listing agents.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentListResponse {
    /// Available agents.
    pub agents: Vec<crate::agents::types::AgentInfo>,
    /// Total count.
    pub total: usize,
}

/// Response from invoking a CrewAI crew.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrewInvokeResponse {
    /// Crew type that was invoked.
    pub crew_type: String,
    /// Unique run ID for this crew invocation.
    pub run_id: String,
    /// Status: "started" (background) or "error".
    pub status: String,
    /// Path where the crew will write its output (poll this file for results).
    pub output_path: String,
    /// Human-readable description of what was launched.
    pub message: String,
    /// Error detail if status == "error".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response from running a skill.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillRunResponse {
    /// Skill that was run.
    pub skill_id: String,
    /// Session used.
    pub session_id: String,
    /// Number of steps executed.
    pub steps_executed: usize,
    /// Number of steps skipped.
    pub steps_skipped: usize,
    /// Final context values.
    pub context: serde_json::Value,
    /// Overall success.
    pub success: bool,
    /// Response metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
    /// Machine-readable suggestion for the next tool call on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_call: Option<NextCallHint>,
}

/// Response from running an agent team.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeamRunResponse {
    /// Team that executed.
    pub team_id: String,
    /// Session used.
    pub session_id: String,
    /// Number of subtasks executed.
    pub subtasks_executed: usize,
    /// Final synthesis.
    pub synthesis: String,
    /// Overall success.
    pub success: bool,
    /// Response metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

/// Response from listing teams.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeamListResponse {
    /// Available teams.
    pub teams: Vec<crate::agents::team::TeamInfo>,
    /// Total count.
    pub total: usize,
}

/// Response from querying agent metrics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentMetricsResponse {
    /// Query type.
    pub query: String,
    /// Metrics data.
    pub data: serde_json::Value,
}

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
    ListSessionsResponse,
    ResumeSessionResponse,
    SearchSessionsResponse,
    RelateSessionsResponse,
    AgentInvokeResponse,
    AgentListResponse,
    SkillRunResponse,
    TeamRunResponse,
    TeamListResponse,
    AgentMetricsResponse,
    MetaResponse,
    ConfidenceRouteResponse,
    CrewInvokeResponse,
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
            meets_threshold: None,
            insufficient_context: false,
            metadata: None,
            next_call: None,
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
                title: "Branch 1".to_string(),
                content: "Branch content".to_string(),
                score: 0.8,
                status: "active".to_string(),
            }]),
            recommendation: Some("Explore branch 1".to_string()),
            synthesis: None,
            key_findings: None,
            best_insights: None,
            metadata: None,
            next_call: None,
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
                key_insight: None,
                blind_spots: None,
            }],
            challenged_assumptions: Some(vec!["Assumption 1".to_string()]),
            synthesis: Some("Unified view".to_string()),
            metadata: None,
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
            completeness_score: None,
            depth_score: None,
            confidence_improvement: None,
            key_insights: None,
            meta_observations: None,
            metadata: None,
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
            metadata: None,
            next_call: None,
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
            metadata: None,
            next_call: None,
            executed: None,
            skill_suggestion: None,
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
            validation: None,
            persistence_warning: None,
            metadata: None,
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
                changes_conclusion: Some("yes".to_string()),
                grounded: Some(true),
            }],
            summary: Some("1 bias found".to_string()),
            overall_quality: Some(0.6),
            debiased_version: Some("Balanced restatement".to_string()),
            argument_structure: None,
            unchallenged_assumptions: None,
            conclusion_altering_biases: Some("confirmation_bias".to_string()),
            validation: None,
            metadata: None,
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
            breakdown: None,
            validation: None,
            metadata: None,
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
                credibility: None,
                quality: None,
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
            evidential_support: None,
            pivot_evidence: None,
            bayesian: None,
            validation: None,
            metadata: None,
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
            ..Default::default()
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
            frontier: None,
            selected_node: None,
            expanded_nodes: None,
            backpropagation: None,
            best_path_value: None,
            backtrack_to: None,
            recent_values: None,
            quality_trend: None,
            alternatives: None,
            recommendation: None,
            validation: None,
            convergence: None,
            metadata: None,
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
            ladder_rung: None,
            association: None,
            intervention: None,
            counterfactual_scenario: None,
            causal_model: None,
            causal_claim: None,
            causal_strength: None,
            actionable_insight: None,
            validation: None,
            metadata: None,
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
            metadata: None,
            next_call: None,
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
            title: "Branch 1".to_string(),
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
            key_insight: None,
            blind_spots: None,
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
            changes_conclusion: None,
            grounded: None,
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
