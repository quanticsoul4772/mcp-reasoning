//! Tool parameter types.
//!
//! This module defines the input parameter structures for all 15 reasoning tools.
//! Each struct uses schemars for automatic JSON schema generation.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Parameters for the linear reasoning tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LinearParams {
    /// Thought content to process.
    #[schemars(description = "Thought content to process")]
    pub content: String,

    /// Session ID for context continuity.
    #[schemars(description = "Session ID for context continuity")]
    pub session_id: Option<String>,

    /// Confidence threshold (0.0-1.0).
    #[schemars(range(min = 0.0, max = 1.0))]
    #[schemars(description = "Confidence threshold (0.0-1.0)")]
    pub confidence: Option<f64>,
}

/// Tree operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum TreeOperation {
    /// Start exploration with 2-4 branches.
    #[default]
    Create,
    /// Select a branch for continued reasoning.
    Focus,
    /// Show all branches in the session.
    List,
    /// Mark a branch as finished or abandoned.
    Complete,
}

/// Parameters for the tree reasoning tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TreeParams {
    /// Tree operation to perform.
    #[schemars(
        description = "create=start exploration, focus=select branch, list=show branches, complete=finish branch"
    )]
    #[serde(default)]
    pub operation: TreeOperation,

    /// Content to explore (for create operation).
    #[schemars(description = "Content to explore (for create)")]
    pub content: Option<String>,

    /// Session ID for context continuity.
    pub session_id: Option<String>,

    /// Branch ID (for focus/complete operations).
    #[schemars(description = "Branch ID (for focus/complete)")]
    pub branch_id: Option<String>,

    /// Number of branches to create (2-4).
    #[schemars(range(min = 2, max = 4))]
    #[serde(default = "default_num_branches")]
    pub num_branches: u32,

    /// Whether the branch is completed (for complete operation).
    #[serde(default = "default_completed")]
    pub completed: bool,
}

const fn default_num_branches() -> u32 {
    3
}

const fn default_completed() -> bool {
    true
}

/// Parameters for the divergent reasoning tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DivergentParams {
    /// Content to analyze from multiple perspectives.
    #[schemars(description = "Content to analyze")]
    pub content: String,

    /// Session ID for context continuity.
    pub session_id: Option<String>,

    /// Number of perspectives to generate (2-5).
    #[schemars(range(min = 2, max = 5))]
    #[serde(default = "default_num_perspectives")]
    pub num_perspectives: u32,

    /// Whether to challenge underlying assumptions.
    #[serde(default)]
    pub challenge_assumptions: bool,

    /// Force maximum creative divergence.
    #[serde(default)]
    pub force_rebellion: bool,
}

const fn default_num_perspectives() -> u32 {
    3
}

/// Reflection operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReflectionOperation {
    /// Reflect on content or thought.
    #[default]
    Process,
    /// Assess session quality.
    Evaluate,
}

/// Parameters for the reflection tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReflectionParams {
    /// Reflection operation to perform.
    #[schemars(
        description = "process=reflect on content/thought, evaluate=assess session quality"
    )]
    #[serde(default)]
    pub operation: ReflectionOperation,

    /// Content to reflect on (for process operation).
    #[schemars(description = "Content to reflect on (for process)")]
    pub content: Option<String>,

    /// Existing thought to analyze.
    pub thought_id: Option<String>,

    /// Session ID (required for evaluate operation).
    pub session_id: Option<String>,

    /// Maximum refinement iterations (1-5).
    #[schemars(range(min = 1, max = 5))]
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,

    /// Quality threshold to stop refinement (0.0-1.0).
    #[schemars(range(min = 0.0, max = 1.0))]
    #[serde(default = "default_quality_threshold")]
    pub quality_threshold: f64,
}

const fn default_max_iterations() -> u32 {
    3
}

const fn default_quality_threshold() -> f64 {
    0.8
}

/// Checkpoint operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointOperation {
    /// Save current state.
    Create,
    /// Show all checkpoints.
    List,
    /// Return to a checkpoint.
    Restore,
}

/// Parameters for the checkpoint tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckpointParams {
    /// Checkpoint operation to perform.
    #[schemars(
        description = "create=save state, list=show checkpoints, restore=return to checkpoint"
    )]
    pub operation: CheckpointOperation,

    /// Session ID (required).
    pub session_id: String,

    /// Checkpoint ID (for restore operation).
    #[schemars(description = "Checkpoint ID (for restore)")]
    pub checkpoint_id: Option<String>,

    /// Checkpoint name (for create operation).
    #[schemars(description = "Checkpoint name (for create)")]
    pub name: Option<String>,

    /// Checkpoint description.
    pub description: Option<String>,

    /// New direction after restore.
    #[schemars(description = "New approach after restore")]
    pub new_direction: Option<String>,
}

/// Parameters for the auto mode selection tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutoParams {
    /// Content to analyze for mode selection.
    #[schemars(description = "Content to analyze")]
    pub content: String,

    /// Optional hints for mode selection.
    #[schemars(description = "Hints for mode selection")]
    pub hints: Option<Vec<String>>,

    /// Session ID for context continuity.
    pub session_id: Option<String>,
}

/// Graph operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GraphOperation {
    /// Create graph with root node.
    Init,
    /// Expand nodes with k continuations.
    Generate,
    /// Evaluate node quality.
    Score,
    /// Merge multiple nodes.
    Aggregate,
    /// Improve via self-critique.
    Refine,
    /// Remove low-scoring nodes.
    Prune,
    /// Extract conclusions.
    Finalize,
    /// Show graph structure.
    State,
}

/// Graph configuration options.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphConfig {
    /// Maximum nodes in graph.
    #[serde(default = "default_max_nodes")]
    pub max_nodes: u32,

    /// Maximum depth of graph.
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,

    /// Prune threshold for node quality.
    #[schemars(range(min = 0.0, max = 1.0))]
    #[serde(default = "default_prune_threshold")]
    pub prune_threshold: f64,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            max_nodes: default_max_nodes(),
            max_depth: default_max_depth(),
            prune_threshold: default_prune_threshold(),
        }
    }
}

const fn default_max_nodes() -> u32 {
    100
}

const fn default_max_depth() -> u32 {
    10
}

const fn default_prune_threshold() -> f64 {
    0.3
}

/// Parameters for the graph reasoning tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphParams {
    /// Graph operation to perform.
    #[schemars(
        description = "init=create graph, generate=expand nodes, score=evaluate, aggregate=merge, refine=improve, prune=remove weak, finalize=extract conclusions, state=show structure"
    )]
    pub operation: GraphOperation,

    /// Session ID (required).
    pub session_id: String,

    /// Content for init operation.
    #[schemars(description = "For init operation")]
    pub content: Option<String>,

    /// Problem context.
    pub problem: Option<String>,

    /// Target node ID.
    #[schemars(description = "Target node for operations")]
    pub node_id: Option<String>,

    /// Multiple node IDs (for aggregate).
    #[schemars(description = "For aggregate operation")]
    pub node_ids: Option<Vec<String>>,

    /// Number of continuations to generate (1-10).
    #[schemars(range(min = 1, max = 10))]
    #[serde(default = "default_k")]
    pub k: u32,

    /// Prune threshold (0.0-1.0).
    #[schemars(range(min = 0.0, max = 1.0))]
    #[serde(default = "default_prune_threshold")]
    pub threshold: f64,

    /// Terminal node IDs (for finalize).
    #[schemars(description = "For finalize operation")]
    pub terminal_node_ids: Option<Vec<String>>,

    /// Graph configuration.
    #[serde(default)]
    pub config: GraphConfig,
}

const fn default_k() -> u32 {
    3
}

/// Detection type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DetectType {
    /// Detect cognitive biases.
    Biases,
    /// Detect logical fallacies.
    Fallacies,
}

/// Parameters for the detect tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetectParams {
    /// Detection type.
    #[schemars(
        description = "biases=cognitive bias detection, fallacies=logical fallacy detection"
    )]
    #[serde(rename = "type")]
    pub detect_type: DetectType,

    /// Content to analyze.
    pub content: Option<String>,

    /// Existing thought to analyze.
    pub thought_id: Option<String>,

    /// Session ID for context.
    pub session_id: Option<String>,

    /// Specific types to check for.
    #[schemars(description = "Specific bias/fallacy types to check")]
    pub check_types: Option<Vec<String>>,

    /// Check for formal fallacies.
    #[serde(default = "default_true")]
    pub check_formal: bool,

    /// Check for informal fallacies.
    #[serde(default = "default_true")]
    pub check_informal: bool,
}

const fn default_true() -> bool {
    true
}

/// Decision type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    /// Weighted scoring.
    #[default]
    Weighted,
    /// Direct comparison.
    Pairwise,
    /// Ideal-point distance.
    Topsis,
    /// Stakeholder analysis.
    Perspectives,
}

/// Decision criterion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DecisionCriterion {
    /// Criterion name.
    pub name: String,

    /// Criterion weight (0.0-1.0).
    #[schemars(range(min = 0.0, max = 1.0))]
    pub weight: f64,
}

/// Stakeholder definition.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StakeholderDef {
    /// Stakeholder name.
    pub name: String,

    /// Stakeholder role.
    pub role: Option<String>,

    /// Power level (0.0-1.0).
    #[schemars(range(min = 0.0, max = 1.0))]
    pub power_level: Option<f64>,

    /// Interest level (0.0-1.0).
    #[schemars(range(min = 0.0, max = 1.0))]
    pub interest_level: Option<f64>,
}

/// Parameters for the decision tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DecisionParams {
    /// Decision type.
    #[schemars(
        description = "weighted=scored ranking, pairwise=direct comparison, topsis=ideal point, perspectives=stakeholder analysis"
    )]
    #[serde(rename = "type", default)]
    pub decision_type: DecisionType,

    /// Decision question (for weighted/pairwise/topsis).
    #[schemars(description = "Decision question")]
    pub question: Option<String>,

    /// Analysis topic (for perspectives).
    #[schemars(description = "Analysis topic (for perspectives)")]
    pub topic: Option<String>,

    /// Options to evaluate.
    pub options: Option<Vec<String>>,

    /// Evaluation criteria.
    pub criteria: Option<Vec<DecisionCriterion>>,

    /// Stakeholders (for perspectives).
    pub stakeholders: Option<Vec<StakeholderDef>>,

    /// Session ID for context.
    pub session_id: Option<String>,

    /// Additional context.
    pub context: Option<String>,
}

/// Evidence source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceType {
    /// Primary source.
    Primary,
    /// Secondary source.
    Secondary,
    /// Tertiary source.
    Tertiary,
    /// Expert opinion.
    Expert,
    /// Anecdotal evidence.
    Anecdotal,
}

/// Evidence type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    /// Assess evidence quality.
    #[default]
    Assess,
    /// Bayesian belief update.
    Probabilistic,
}

/// Evidence piece for assessment.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidencePieceDef {
    /// Evidence content.
    pub content: String,

    /// Source of evidence.
    pub source: Option<String>,

    /// Source type.
    pub source_type: Option<EvidenceSourceType>,

    /// Likelihood if hypothesis is true (for probabilistic).
    #[schemars(range(min = 0.0, max = 1.0))]
    pub likelihood_if_true: Option<f64>,

    /// Likelihood if hypothesis is false (for probabilistic).
    #[schemars(range(min = 0.0, max = 1.0))]
    pub likelihood_if_false: Option<f64>,
}

/// Parameters for the evidence tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceParams {
    /// Evidence type.
    #[schemars(
        description = "assess=evidence quality evaluation, probabilistic=Bayesian belief update"
    )]
    #[serde(rename = "type", default)]
    pub evidence_type: EvidenceType,

    /// Claim to evaluate (for assess).
    #[schemars(description = "Claim to evaluate (for assess)")]
    pub claim: Option<String>,

    /// Hypothesis to test (for probabilistic).
    #[schemars(description = "Hypothesis to test (for probabilistic)")]
    pub hypothesis: Option<String>,

    /// Prior probability (for probabilistic).
    #[schemars(range(min = 0.0, max = 1.0))]
    #[schemars(description = "Prior probability")]
    pub prior: Option<f64>,

    /// Evidence pieces.
    pub evidence: Vec<EvidencePieceDef>,

    /// Session ID for context.
    pub session_id: Option<String>,

    /// Additional context.
    pub context: Option<String>,
}

/// Timeline operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TimelineOperation {
    /// Create new timeline.
    Create,
    /// Fork a path.
    Branch,
    /// Analyze branches.
    Compare,
    /// Combine branches.
    Merge,
}

/// Merge strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Synthesize both branches.
    #[default]
    Synthesize,
    /// Prefer source branch.
    PreferSource,
    /// Prefer target branch.
    PreferTarget,
    /// Interleave content.
    Interleave,
}

/// Parameters for the timeline tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineParams {
    /// Timeline operation to perform.
    #[schemars(
        description = "create=new timeline, branch=fork path, compare=analyze branches, merge=combine branches"
    )]
    pub operation: TimelineOperation,

    /// Content for create/branch.
    #[schemars(description = "For create/branch")]
    pub content: Option<String>,

    /// Session ID for context.
    pub session_id: Option<String>,

    /// Timeline ID (for branch/compare/merge).
    #[schemars(description = "For branch/compare/merge")]
    pub timeline_id: Option<String>,

    /// Branch IDs to compare.
    #[schemars(description = "For compare")]
    pub branch_ids: Option<Vec<String>>,

    /// Source branch for merge.
    #[schemars(description = "For merge")]
    pub source_branch_id: Option<String>,

    /// Target branch for merge.
    #[schemars(description = "For merge")]
    pub target_branch_id: Option<String>,

    /// Parent branch for new branch.
    #[schemars(description = "For branch")]
    pub parent_branch_id: Option<String>,

    /// Merge strategy.
    #[serde(default)]
    pub merge_strategy: MergeStrategy,

    /// Branch label.
    pub label: Option<String>,

    /// Additional metadata.
    pub metadata: Option<serde_json::Value>,
}

/// MCTS operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum MctsOperation {
    /// UCB1-guided search.
    #[default]
    Explore,
    /// Quality-triggered backtracking.
    AutoBacktrack,
}

/// Parameters for the MCTS tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MctsParams {
    /// MCTS operation to perform.
    #[schemars(
        description = "explore=MCTS search with UCB1, auto_backtrack=quality-triggered backtracking"
    )]
    #[serde(default)]
    pub operation: MctsOperation,

    /// Content to explore (for explore).
    #[schemars(description = "For explore")]
    pub content: Option<String>,

    /// Session ID for context.
    pub session_id: Option<String>,

    /// Starting node ID.
    pub node_id: Option<String>,

    /// Number of MCTS iterations (1-100).
    #[schemars(range(min = 1, max = 100))]
    #[serde(default = "default_iterations")]
    pub iterations: u32,

    /// UCB1 exploration constant.
    #[schemars(range(min = 0.0, max = 10.0))]
    #[serde(default = "default_exploration_constant")]
    pub exploration_constant: f64,

    /// Simulation depth (1-20).
    #[schemars(range(min = 1, max = 20))]
    #[serde(default = "default_simulation_depth")]
    pub simulation_depth: u32,

    /// Quality threshold for backtracking (0.0-1.0).
    #[schemars(range(min = 0.0, max = 1.0))]
    #[serde(default = "default_mcts_quality_threshold")]
    pub quality_threshold: f64,

    /// Auto-execute backtracking.
    #[serde(default)]
    pub auto_execute: bool,

    /// Lookback depth (1-10).
    #[schemars(range(min = 1, max = 10))]
    #[serde(default = "default_lookback_depth")]
    pub lookback_depth: u32,
}

const fn default_iterations() -> u32 {
    10
}

const fn default_exploration_constant() -> f64 {
    1.414 // sqrt(2)
}

const fn default_simulation_depth() -> u32 {
    5
}

const fn default_mcts_quality_threshold() -> f64 {
    0.5
}

const fn default_lookback_depth() -> u32 {
    5
}

/// Analysis depth for counterfactual reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisDepth {
    /// Correlation only.
    Association,
    /// Do-calculus.
    Intervention,
    /// Full causal reasoning.
    #[default]
    Counterfactual,
}

/// Causal relationship.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CausalRelationship {
    /// Cause variable.
    pub cause: String,

    /// Effect variable.
    pub effect: String,

    /// Relationship strength (0.0-1.0).
    #[schemars(range(min = 0.0, max = 1.0))]
    pub strength: Option<f64>,
}

/// Causal model definition.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct CausalModelDef {
    /// Variables in the model.
    pub variables: Option<Vec<String>>,

    /// Causal relationships.
    pub relationships: Option<Vec<CausalRelationship>>,
}

/// Parameters for the counterfactual tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CounterfactualParams {
    /// Base scenario.
    #[schemars(description = "Base scenario")]
    pub scenario: String,

    /// What-if change.
    #[schemars(description = "What-if change")]
    pub intervention: String,

    /// Session ID for context.
    pub session_id: Option<String>,

    /// Analysis depth.
    #[serde(default)]
    pub analysis_depth: AnalysisDepth,

    /// Optional causal model.
    pub causal_model: Option<CausalModelDef>,
}

/// Preset operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PresetOperation {
    /// List available presets.
    List,
    /// Run a preset.
    Run,
}

/// Parameters for the preset tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PresetParams {
    /// Preset operation to perform.
    #[schemars(description = "list=show available presets, run=execute preset workflow")]
    pub operation: PresetOperation,

    /// Preset ID (for run).
    #[schemars(description = "For run operation")]
    pub preset_id: Option<String>,

    /// Category filter (for list).
    #[schemars(description = "Filter for list")]
    pub category: Option<String>,

    /// Preset inputs (for run).
    #[schemars(description = "Preset inputs for run")]
    pub inputs: Option<serde_json::Value>,

    /// Session ID for context.
    pub session_id: Option<String>,
}

/// Metrics query type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricsQuery {
    /// All stats.
    Summary,
    /// Mode-specific stats.
    ByMode,
    /// Call history.
    Invocations,
    /// Fallback usage.
    Fallbacks,
    /// Debug info.
    Config,
}

/// Parameters for the metrics tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MetricsParams {
    /// Metrics query type.
    #[schemars(
        description = "summary=all stats, by_mode=mode stats, invocations=call history, fallbacks=fallback usage, config=debug info"
    )]
    pub query: MetricsQuery,

    /// Mode name (for by_mode query).
    #[schemars(description = "For by_mode query")]
    pub mode_name: Option<String>,

    /// Tool name filter (for invocations).
    #[schemars(description = "Filter for invocations")]
    pub tool_name: Option<String>,

    /// Session ID filter (for invocations).
    #[schemars(description = "Filter for invocations")]
    pub session_id: Option<String>,

    /// Filter to successful only.
    pub success_only: Option<bool>,

    /// Result limit (1-1000).
    #[schemars(range(min = 1, max = 1000))]
    #[serde(default = "default_limit")]
    pub limit: u32,
}

const fn default_limit() -> u32 {
    100
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_params_deserialize() {
        let json = r#"{"content": "test content"}"#;
        let params: LinearParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.content, "test content");
        assert!(params.session_id.is_none());
        assert!(params.confidence.is_none());
    }

    #[test]
    fn test_linear_params_full_deserialize() {
        let json = r#"{"content": "test", "session_id": "sess1", "confidence": 0.8}"#;
        let params: LinearParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.content, "test");
        assert_eq!(params.session_id, Some("sess1".to_string()));
        assert_eq!(params.confidence, Some(0.8));
    }

    #[test]
    fn test_tree_operation_default() {
        let op = TreeOperation::default();
        assert_eq!(op, TreeOperation::Create);
    }

    #[test]
    fn test_tree_params_defaults() {
        let json = r#"{}"#;
        let params: TreeParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.operation, TreeOperation::Create);
        assert_eq!(params.num_branches, 3);
        assert!(params.completed);
    }

    #[test]
    fn test_tree_operation_deserialize() {
        let json = r#"{"operation": "focus", "branch_id": "br1"}"#;
        let params: TreeParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.operation, TreeOperation::Focus);
        assert_eq!(params.branch_id, Some("br1".to_string()));
    }

    #[test]
    fn test_divergent_params_defaults() {
        let json = r#"{"content": "test"}"#;
        let params: DivergentParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.num_perspectives, 3);
        assert!(!params.challenge_assumptions);
        assert!(!params.force_rebellion);
    }

    #[test]
    fn test_reflection_operation_default() {
        let op = ReflectionOperation::default();
        assert_eq!(op, ReflectionOperation::Process);
    }

    #[test]
    fn test_reflection_params_defaults() {
        let json = r#"{}"#;
        let params: ReflectionParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.max_iterations, 3);
        assert!((params.quality_threshold - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_checkpoint_params_deserialize() {
        let json = r#"{"operation": "create", "session_id": "sess1", "name": "cp1"}"#;
        let params: CheckpointParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.operation, CheckpointOperation::Create);
        assert_eq!(params.session_id, "sess1");
        assert_eq!(params.name, Some("cp1".to_string()));
    }

    #[test]
    fn test_graph_operation_deserialize() {
        let json = r#"{"operation": "generate", "session_id": "s1", "k": 5}"#;
        let params: GraphParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.operation, GraphOperation::Generate);
        assert_eq!(params.k, 5);
    }

    #[test]
    fn test_graph_config_defaults() {
        let config = GraphConfig::default();
        assert_eq!(config.max_nodes, 100);
        assert_eq!(config.max_depth, 10);
        assert!((config.prune_threshold - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_detect_type_deserialize() {
        let json = r#"{"type": "biases"}"#;
        let params: DetectParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.detect_type, DetectType::Biases);
        assert!(params.check_formal);
        assert!(params.check_informal);
    }

    #[test]
    fn test_decision_type_default() {
        let dt = DecisionType::default();
        assert_eq!(dt, DecisionType::Weighted);
    }

    #[test]
    fn test_decision_params_deserialize() {
        let json = r#"{"type": "topsis", "question": "Which option?", "options": ["A", "B"]}"#;
        let params: DecisionParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.decision_type, DecisionType::Topsis);
        assert_eq!(params.question, Some("Which option?".to_string()));
    }

    #[test]
    fn test_evidence_type_default() {
        let et = EvidenceType::default();
        assert_eq!(et, EvidenceType::Assess);
    }

    #[test]
    fn test_evidence_params_deserialize() {
        let json = r#"{"type": "probabilistic", "hypothesis": "H1", "prior": 0.5, "evidence": [{"content": "e1"}]}"#;
        let params: EvidenceParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.evidence_type, EvidenceType::Probabilistic);
        assert_eq!(params.prior, Some(0.5));
        assert_eq!(params.evidence.len(), 1);
    }

    #[test]
    fn test_timeline_operation_deserialize() {
        let json = r#"{"operation": "branch", "timeline_id": "t1"}"#;
        let params: TimelineParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.operation, TimelineOperation::Branch);
        assert_eq!(params.merge_strategy, MergeStrategy::Synthesize);
    }

    #[test]
    fn test_mcts_params_defaults() {
        let json = r#"{}"#;
        let params: MctsParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.operation, MctsOperation::Explore);
        assert_eq!(params.iterations, 10);
        assert!((params.exploration_constant - 1.414).abs() < 0.001);
        assert_eq!(params.simulation_depth, 5);
    }

    #[test]
    fn test_analysis_depth_default() {
        let ad = AnalysisDepth::default();
        assert_eq!(ad, AnalysisDepth::Counterfactual);
    }

    #[test]
    fn test_counterfactual_params_deserialize() {
        let json = r#"{"scenario": "X happened", "intervention": "What if Y?"}"#;
        let params: CounterfactualParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.scenario, "X happened");
        assert_eq!(params.intervention, "What if Y?");
        assert_eq!(params.analysis_depth, AnalysisDepth::Counterfactual);
    }

    #[test]
    fn test_preset_operation_deserialize() {
        let json = r#"{"operation": "run", "preset_id": "code-review"}"#;
        let params: PresetParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.operation, PresetOperation::Run);
        assert_eq!(params.preset_id, Some("code-review".to_string()));
    }

    #[test]
    fn test_metrics_params_defaults() {
        let json = r#"{"query": "summary"}"#;
        let params: MetricsParams = serde_json::from_str(json).expect("deserialize");
        assert_eq!(params.query, MetricsQuery::Summary);
        assert_eq!(params.limit, 100);
    }

    #[test]
    fn test_stakeholder_def_deserialize() {
        let json = r#"{"name": "CEO", "role": "Executive", "power_level": 0.9}"#;
        let stakeholder: StakeholderDef = serde_json::from_str(json).expect("deserialize");
        assert_eq!(stakeholder.name, "CEO");
        assert_eq!(stakeholder.power_level, Some(0.9));
    }

    #[test]
    fn test_evidence_piece_def_deserialize() {
        let json = r#"{"content": "data", "source_type": "primary", "likelihood_if_true": 0.9}"#;
        let piece: EvidencePieceDef = serde_json::from_str(json).expect("deserialize");
        assert_eq!(piece.source_type, Some(EvidenceSourceType::Primary));
        assert_eq!(piece.likelihood_if_true, Some(0.9));
    }

    #[test]
    fn test_causal_relationship_deserialize() {
        let json = r#"{"cause": "X", "effect": "Y", "strength": 0.7}"#;
        let rel: CausalRelationship = serde_json::from_str(json).expect("deserialize");
        assert_eq!(rel.cause, "X");
        assert_eq!(rel.effect, "Y");
        assert_eq!(rel.strength, Some(0.7));
    }
}
