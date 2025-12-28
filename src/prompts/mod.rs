//! Prompt templates.
//!
//! This module provides prompt templates for each reasoning mode,
//! organized by category.
//!
//! # Architecture
//!
//! Prompts are organized into submodules by mode category:
//! - `core`: Linear, tree, divergent, reflection, checkpoint, auto
//! - `graph`: Graph-of-Thoughts (8 operations)
//! - `detect`: Bias and fallacy detection
//! - `decision`: Multi-criteria decision analysis
//! - `evidence`: Evidence evaluation and Bayesian updates
//! - `timeline`: Temporal reasoning
//! - `mcts`: Monte Carlo Tree Search
//! - `counterfactual`: Causal analysis (Pearl's Ladder)
//!
//! The [`get_prompt_for_mode`] function routes to the appropriate prompt
//! based on the reasoning mode and operation.
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::prompts::{get_prompt_for_mode, ReasoningMode, Operation};
//!
//! let prompt = get_prompt_for_mode(ReasoningMode::Linear, None);
//! assert!(prompt.contains("step-by-step"));
//! ```

#![allow(clippy::match_same_arms)]

mod core;
mod counterfactual;
mod decision;
mod detect;
mod evidence;
mod graph;
mod mcts;
mod timeline;

pub use core::{
    auto_select_prompt, checkpoint_create_prompt, divergent_prompt, divergent_rebellion_prompt,
    linear_prompt, reflection_evaluate_prompt, reflection_process_prompt, tree_complete_prompt,
    tree_create_prompt, tree_focus_prompt, tree_list_prompt,
};
pub use counterfactual::counterfactual_prompt;
pub use decision::{
    decision_pairwise_prompt, decision_perspectives_prompt, decision_topsis_prompt,
    decision_weighted_prompt,
};
pub use detect::{detect_biases_prompt, detect_fallacies_prompt};
pub use evidence::{evidence_assess_prompt, evidence_probabilistic_prompt};
pub use graph::{
    graph_aggregate_prompt, graph_finalize_prompt, graph_generate_prompt, graph_init_prompt,
    graph_prune_prompt, graph_refine_prompt, graph_score_prompt, graph_state_prompt,
};
pub use mcts::{mcts_backtrack_prompt, mcts_explore_prompt};
pub use timeline::{
    timeline_branch_prompt, timeline_compare_prompt, timeline_create_prompt, timeline_merge_prompt,
};

/// Reasoning mode enumeration.
///
/// This defines all available reasoning modes that can be used
/// with the prompt system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningMode {
    /// Sequential step-by-step reasoning.
    Linear,
    /// Branching exploration.
    Tree,
    /// Multi-perspective analysis.
    Divergent,
    /// Meta-cognitive evaluation.
    Reflection,
    /// State management.
    Checkpoint,
    /// Mode selection router.
    Auto,
    /// Graph-of-Thoughts reasoning.
    Graph,
    /// Bias and fallacy detection.
    Detect,
    /// Multi-criteria decision analysis.
    Decision,
    /// Evidence evaluation.
    Evidence,
    /// Temporal reasoning.
    Timeline,
    /// Monte Carlo Tree Search.
    Mcts,
    /// Causal counterfactual analysis.
    Counterfactual,
}

impl ReasoningMode {
    /// Returns true if this mode requires extended thinking.
    #[must_use]
    pub const fn requires_thinking(&self) -> bool {
        matches!(
            self,
            Self::Divergent
                | Self::Graph
                | Self::Reflection
                | Self::Decision
                | Self::Evidence
                | Self::Counterfactual
                | Self::Mcts
        )
    }

    /// Returns the recommended thinking budget for this mode.
    #[must_use]
    pub const fn thinking_budget(&self) -> Option<u32> {
        match self {
            Self::Linear | Self::Tree | Self::Auto | Self::Checkpoint => None,
            Self::Divergent | Self::Graph | Self::Detect => Some(4096),
            Self::Reflection | Self::Decision | Self::Evidence => Some(8192),
            Self::Counterfactual | Self::Mcts | Self::Timeline => Some(16384),
        }
    }

    /// Returns the mode name as a string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Tree => "tree",
            Self::Divergent => "divergent",
            Self::Reflection => "reflection",
            Self::Checkpoint => "checkpoint",
            Self::Auto => "auto",
            Self::Graph => "graph",
            Self::Detect => "detect",
            Self::Decision => "decision",
            Self::Evidence => "evidence",
            Self::Timeline => "timeline",
            Self::Mcts => "mcts",
            Self::Counterfactual => "counterfactual",
        }
    }

    /// Returns all available modes.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Linear,
            Self::Tree,
            Self::Divergent,
            Self::Reflection,
            Self::Checkpoint,
            Self::Auto,
            Self::Graph,
            Self::Detect,
            Self::Decision,
            Self::Evidence,
            Self::Timeline,
            Self::Mcts,
            Self::Counterfactual,
        ]
    }
}

impl std::fmt::Display for ReasoningMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ReasoningMode {
    type Err = ParseModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "linear" => Ok(Self::Linear),
            "tree" => Ok(Self::Tree),
            "divergent" => Ok(Self::Divergent),
            "reflection" => Ok(Self::Reflection),
            "checkpoint" => Ok(Self::Checkpoint),
            "auto" => Ok(Self::Auto),
            "graph" => Ok(Self::Graph),
            "detect" => Ok(Self::Detect),
            "decision" => Ok(Self::Decision),
            "evidence" => Ok(Self::Evidence),
            "timeline" => Ok(Self::Timeline),
            "mcts" => Ok(Self::Mcts),
            "counterfactual" => Ok(Self::Counterfactual),
            _ => Err(ParseModeError {
                input: s.to_string(),
            }),
        }
    }
}

/// Error when parsing a reasoning mode from string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseModeError {
    /// The input that failed to parse.
    pub input: String,
}

impl std::fmt::Display for ParseModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Unknown reasoning mode: '{}'. Valid modes: {}",
            self.input,
            ReasoningMode::all()
                .iter()
                .map(ReasoningMode::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl std::error::Error for ParseModeError {}

/// Operation within a mode.
///
/// Some modes support multiple operations (e.g., tree has create, focus, list, complete).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Tree: Create new exploration branches.
    Create,
    /// Tree: Focus on a specific branch.
    Focus,
    /// Tree: List all branches.
    List,
    /// Tree: Complete the exploration.
    Complete,
    /// Reflection: Process for improvement.
    Process,
    /// Reflection: Evaluate the session.
    Evaluate,
    /// Divergent: Force rebellion mode.
    ForceRebellion,
    /// Graph: Initialize the graph.
    Init,
    /// Graph: Generate child nodes.
    Generate,
    /// Graph: Score a node.
    Score,
    /// Graph: Aggregate nodes.
    Aggregate,
    /// Graph: Refine a node.
    Refine,
    /// Graph: Prune low-quality nodes.
    Prune,
    /// Graph: Finalize and extract conclusions.
    Finalize,
    /// Graph: Get current state.
    State,
    /// Detect: Find biases.
    Biases,
    /// Detect: Find fallacies.
    Fallacies,
    /// Decision: Weighted scoring.
    Weighted,
    /// Decision: Pairwise comparison.
    Pairwise,
    /// Decision: TOPSIS method.
    Topsis,
    /// Decision: Stakeholder perspectives.
    Perspectives,
    /// Evidence: Assess credibility.
    Assess,
    /// Evidence: Probabilistic (Bayesian).
    Probabilistic,
    /// Timeline: Create timeline.
    TimelineCreate,
    /// Timeline: Branch the timeline.
    Branch,
    /// Timeline: Compare branches.
    Compare,
    /// Timeline: Merge branches.
    Merge,
    /// MCTS: Explore with UCB1.
    Explore,
    /// MCTS: Auto backtrack.
    AutoBacktrack,
}

impl Operation {
    /// Returns the operation name as a string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Focus => "focus",
            Self::List => "list",
            Self::Complete => "complete",
            Self::Process => "process",
            Self::Evaluate => "evaluate",
            Self::ForceRebellion => "force_rebellion",
            Self::Init => "init",
            Self::Generate => "generate",
            Self::Score => "score",
            Self::Aggregate => "aggregate",
            Self::Refine => "refine",
            Self::Prune => "prune",
            Self::Finalize => "finalize",
            Self::State => "state",
            Self::Biases => "biases",
            Self::Fallacies => "fallacies",
            Self::Weighted => "weighted",
            Self::Pairwise => "pairwise",
            Self::Topsis => "topsis",
            Self::Perspectives => "perspectives",
            Self::Assess => "assess",
            Self::Probabilistic => "probabilistic",
            Self::TimelineCreate => "create",
            Self::Branch => "branch",
            Self::Compare => "compare",
            Self::Merge => "merge",
            Self::Explore => "explore",
            Self::AutoBacktrack => "auto_backtrack",
        }
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Get the appropriate prompt for a reasoning mode and optional operation.
///
/// This function routes to the correct prompt template based on the
/// mode and operation combination.
///
/// # Arguments
///
/// * `mode` - The reasoning mode to get a prompt for
/// * `operation` - Optional operation within the mode
///
/// # Returns
///
/// The appropriate prompt string for the mode/operation combination.
///
/// # Examples
///
/// ```
/// use mcp_reasoning::prompts::{get_prompt_for_mode, ReasoningMode, Operation};
///
/// // Simple mode without operation
/// let prompt = get_prompt_for_mode(ReasoningMode::Linear, None);
/// assert!(!prompt.is_empty());
///
/// // Mode with specific operation
/// let prompt = get_prompt_for_mode(ReasoningMode::Tree, Some(&Operation::Create));
/// assert!(prompt.contains("branches"));
/// ```
#[must_use]
pub fn get_prompt_for_mode(mode: ReasoningMode, operation: Option<&Operation>) -> &'static str {
    match (mode, operation) {
        // Core modes
        (ReasoningMode::Linear, _) => linear_prompt(),

        (ReasoningMode::Tree, None | Some(Operation::Create)) => tree_create_prompt(),
        (ReasoningMode::Tree, Some(Operation::Focus)) => tree_focus_prompt(),
        (ReasoningMode::Tree, Some(Operation::List)) => tree_list_prompt(),
        (ReasoningMode::Tree, Some(Operation::Complete)) => tree_complete_prompt(),

        (ReasoningMode::Divergent, Some(Operation::ForceRebellion)) => divergent_rebellion_prompt(),
        (ReasoningMode::Divergent, _) => divergent_prompt(),

        (ReasoningMode::Reflection, Some(Operation::Evaluate)) => reflection_evaluate_prompt(),
        (ReasoningMode::Reflection, _) => reflection_process_prompt(),

        (ReasoningMode::Checkpoint, _) => checkpoint_create_prompt(),
        (ReasoningMode::Auto, _) => auto_select_prompt(),

        // Graph mode (8 operations)
        (ReasoningMode::Graph, None | Some(Operation::Init)) => graph_init_prompt(),
        (ReasoningMode::Graph, Some(Operation::Generate)) => graph_generate_prompt(),
        (ReasoningMode::Graph, Some(Operation::Score)) => graph_score_prompt(),
        (ReasoningMode::Graph, Some(Operation::Aggregate)) => graph_aggregate_prompt(),
        (ReasoningMode::Graph, Some(Operation::Refine)) => graph_refine_prompt(),
        (ReasoningMode::Graph, Some(Operation::Prune)) => graph_prune_prompt(),
        (ReasoningMode::Graph, Some(Operation::Finalize)) => graph_finalize_prompt(),
        (ReasoningMode::Graph, Some(Operation::State)) => graph_state_prompt(),

        // Detect mode
        (ReasoningMode::Detect, Some(Operation::Fallacies)) => detect_fallacies_prompt(),
        (ReasoningMode::Detect, _) => detect_biases_prompt(),

        // Decision mode
        (ReasoningMode::Decision, None | Some(Operation::Weighted)) => decision_weighted_prompt(),
        (ReasoningMode::Decision, Some(Operation::Pairwise)) => decision_pairwise_prompt(),
        (ReasoningMode::Decision, Some(Operation::Topsis)) => decision_topsis_prompt(),
        (ReasoningMode::Decision, Some(Operation::Perspectives)) => decision_perspectives_prompt(),

        // Evidence mode
        (ReasoningMode::Evidence, Some(Operation::Probabilistic)) => {
            evidence_probabilistic_prompt()
        }
        (ReasoningMode::Evidence, _) => evidence_assess_prompt(),

        // Timeline mode
        (ReasoningMode::Timeline, None | Some(Operation::TimelineCreate | Operation::Create)) => {
            timeline_create_prompt()
        }
        (ReasoningMode::Timeline, Some(Operation::Branch)) => timeline_branch_prompt(),
        (ReasoningMode::Timeline, Some(Operation::Compare)) => timeline_compare_prompt(),
        (ReasoningMode::Timeline, Some(Operation::Merge)) => timeline_merge_prompt(),

        // MCTS mode
        (ReasoningMode::Mcts, Some(Operation::AutoBacktrack)) => mcts_backtrack_prompt(),
        (ReasoningMode::Mcts, _) => mcts_explore_prompt(),

        // Counterfactual mode
        (ReasoningMode::Counterfactual, _) => counterfactual_prompt(),

        // Fallback for any unmatched combinations - use default for the mode
        (ReasoningMode::Tree, _) => tree_create_prompt(),
        (ReasoningMode::Graph, _) => graph_init_prompt(),
        (ReasoningMode::Decision, _) => decision_weighted_prompt(),
        (ReasoningMode::Timeline, _) => timeline_create_prompt(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ReasoningMode tests
    #[test]
    fn test_reasoning_mode_as_str() {
        assert_eq!(ReasoningMode::Linear.as_str(), "linear");
        assert_eq!(ReasoningMode::Tree.as_str(), "tree");
        assert_eq!(ReasoningMode::Divergent.as_str(), "divergent");
        assert_eq!(ReasoningMode::Reflection.as_str(), "reflection");
        assert_eq!(ReasoningMode::Checkpoint.as_str(), "checkpoint");
        assert_eq!(ReasoningMode::Auto.as_str(), "auto");
        assert_eq!(ReasoningMode::Graph.as_str(), "graph");
        assert_eq!(ReasoningMode::Detect.as_str(), "detect");
        assert_eq!(ReasoningMode::Decision.as_str(), "decision");
        assert_eq!(ReasoningMode::Evidence.as_str(), "evidence");
        assert_eq!(ReasoningMode::Timeline.as_str(), "timeline");
        assert_eq!(ReasoningMode::Mcts.as_str(), "mcts");
        assert_eq!(ReasoningMode::Counterfactual.as_str(), "counterfactual");
    }

    #[test]
    fn test_reasoning_mode_display() {
        assert_eq!(format!("{}", ReasoningMode::Linear), "linear");
        assert_eq!(
            format!("{}", ReasoningMode::Counterfactual),
            "counterfactual"
        );
    }

    #[test]
    fn test_reasoning_mode_from_str_valid() {
        assert_eq!(
            "linear".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Linear)
        );
        assert_eq!(
            "tree".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Tree)
        );
        assert_eq!(
            "DIVERGENT".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Divergent)
        );
        assert_eq!(
            "Reflection".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Reflection)
        );
        assert_eq!(
            "checkpoint".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Checkpoint)
        );
        assert_eq!(
            "auto".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Auto)
        );
        assert_eq!(
            "graph".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Graph)
        );
        assert_eq!(
            "detect".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Detect)
        );
        assert_eq!(
            "decision".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Decision)
        );
        assert_eq!(
            "evidence".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Evidence)
        );
        assert_eq!(
            "timeline".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Timeline)
        );
        assert_eq!(
            "mcts".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Mcts)
        );
        assert_eq!(
            "counterfactual".parse::<ReasoningMode>().ok(),
            Some(ReasoningMode::Counterfactual)
        );
    }

    #[test]
    fn test_reasoning_mode_from_str_invalid() {
        let result = "invalid".parse::<ReasoningMode>();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.input, "invalid");
        assert!(err.to_string().contains("Unknown reasoning mode"));
    }

    #[test]
    fn test_reasoning_mode_all() {
        let all = ReasoningMode::all();
        assert_eq!(all.len(), 13);
        assert!(all.contains(&ReasoningMode::Linear));
        assert!(all.contains(&ReasoningMode::Counterfactual));
    }

    #[test]
    fn test_reasoning_mode_requires_thinking() {
        // Fast modes - no thinking
        assert!(!ReasoningMode::Linear.requires_thinking());
        assert!(!ReasoningMode::Tree.requires_thinking());
        assert!(!ReasoningMode::Auto.requires_thinking());
        assert!(!ReasoningMode::Checkpoint.requires_thinking());

        // Modes with thinking
        assert!(ReasoningMode::Divergent.requires_thinking());
        assert!(ReasoningMode::Graph.requires_thinking());
        assert!(ReasoningMode::Reflection.requires_thinking());
        assert!(ReasoningMode::Decision.requires_thinking());
        assert!(ReasoningMode::Evidence.requires_thinking());
        assert!(ReasoningMode::Counterfactual.requires_thinking());
        assert!(ReasoningMode::Mcts.requires_thinking());
    }

    #[test]
    fn test_reasoning_mode_thinking_budget() {
        // No thinking
        assert_eq!(ReasoningMode::Linear.thinking_budget(), None);
        assert_eq!(ReasoningMode::Tree.thinking_budget(), None);

        // Standard (4096)
        assert_eq!(ReasoningMode::Divergent.thinking_budget(), Some(4096));
        assert_eq!(ReasoningMode::Graph.thinking_budget(), Some(4096));

        // Deep (8192)
        assert_eq!(ReasoningMode::Reflection.thinking_budget(), Some(8192));
        assert_eq!(ReasoningMode::Decision.thinking_budget(), Some(8192));
        assert_eq!(ReasoningMode::Evidence.thinking_budget(), Some(8192));

        // Maximum (16384)
        assert_eq!(ReasoningMode::Counterfactual.thinking_budget(), Some(16384));
        assert_eq!(ReasoningMode::Mcts.thinking_budget(), Some(16384));
        assert_eq!(ReasoningMode::Timeline.thinking_budget(), Some(16384));
    }

    // ParseModeError tests
    #[test]
    fn test_parse_mode_error_display() {
        let err = ParseModeError {
            input: "foo".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Unknown reasoning mode: 'foo'"));
        assert!(msg.contains("linear"));
        assert!(msg.contains("tree"));
    }

    #[test]
    fn test_parse_mode_error_is_error() {
        let err = ParseModeError {
            input: "test".to_string(),
        };
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &err;
    }

    // Operation tests
    #[test]
    fn test_operation_as_str() {
        assert_eq!(Operation::Create.as_str(), "create");
        assert_eq!(Operation::Focus.as_str(), "focus");
        assert_eq!(Operation::List.as_str(), "list");
        assert_eq!(Operation::Complete.as_str(), "complete");
        assert_eq!(Operation::Process.as_str(), "process");
        assert_eq!(Operation::Evaluate.as_str(), "evaluate");
        assert_eq!(Operation::ForceRebellion.as_str(), "force_rebellion");
        assert_eq!(Operation::Init.as_str(), "init");
        assert_eq!(Operation::Generate.as_str(), "generate");
        assert_eq!(Operation::Score.as_str(), "score");
        assert_eq!(Operation::Aggregate.as_str(), "aggregate");
        assert_eq!(Operation::Refine.as_str(), "refine");
        assert_eq!(Operation::Prune.as_str(), "prune");
        assert_eq!(Operation::Finalize.as_str(), "finalize");
        assert_eq!(Operation::State.as_str(), "state");
        assert_eq!(Operation::Biases.as_str(), "biases");
        assert_eq!(Operation::Fallacies.as_str(), "fallacies");
        assert_eq!(Operation::Weighted.as_str(), "weighted");
        assert_eq!(Operation::Pairwise.as_str(), "pairwise");
        assert_eq!(Operation::Topsis.as_str(), "topsis");
        assert_eq!(Operation::Perspectives.as_str(), "perspectives");
        assert_eq!(Operation::Assess.as_str(), "assess");
        assert_eq!(Operation::Probabilistic.as_str(), "probabilistic");
        assert_eq!(Operation::TimelineCreate.as_str(), "create");
        assert_eq!(Operation::Branch.as_str(), "branch");
        assert_eq!(Operation::Compare.as_str(), "compare");
        assert_eq!(Operation::Merge.as_str(), "merge");
        assert_eq!(Operation::Explore.as_str(), "explore");
        assert_eq!(Operation::AutoBacktrack.as_str(), "auto_backtrack");
    }

    #[test]
    fn test_operation_display() {
        assert_eq!(format!("{}", Operation::Create), "create");
        assert_eq!(format!("{}", Operation::AutoBacktrack), "auto_backtrack");
    }

    // get_prompt_for_mode tests
    #[test]
    fn test_get_prompt_linear() {
        let prompt = get_prompt_for_mode(ReasoningMode::Linear, None);
        assert!(prompt.contains("step-by-step"));
    }

    #[test]
    fn test_get_prompt_tree_operations() {
        let prompt = get_prompt_for_mode(ReasoningMode::Tree, None);
        assert!(prompt.contains("branches"));

        let prompt = get_prompt_for_mode(ReasoningMode::Tree, Some(&Operation::Create));
        assert!(prompt.contains("branches"));

        let prompt = get_prompt_for_mode(ReasoningMode::Tree, Some(&Operation::Focus));
        assert!(prompt.contains("exploration"));

        let prompt = get_prompt_for_mode(ReasoningMode::Tree, Some(&Operation::List));
        assert!(prompt.contains("status"));

        let prompt = get_prompt_for_mode(ReasoningMode::Tree, Some(&Operation::Complete));
        assert!(prompt.contains("synthesis"));
    }

    #[test]
    fn test_get_prompt_divergent() {
        let prompt = get_prompt_for_mode(ReasoningMode::Divergent, None);
        assert!(prompt.contains("perspectives"));

        let prompt =
            get_prompt_for_mode(ReasoningMode::Divergent, Some(&Operation::ForceRebellion));
        assert!(prompt.contains("contrarian"));
        assert!(prompt.contains("challenge"));
    }

    #[test]
    fn test_get_prompt_reflection() {
        let prompt = get_prompt_for_mode(ReasoningMode::Reflection, None);
        assert!(prompt.contains("strengths"));

        let prompt = get_prompt_for_mode(ReasoningMode::Reflection, Some(&Operation::Evaluate));
        assert!(prompt.contains("session_assessment"));
    }

    #[test]
    fn test_get_prompt_checkpoint() {
        let prompt = get_prompt_for_mode(ReasoningMode::Checkpoint, None);
        assert!(prompt.contains("checkpoint"));
    }

    #[test]
    fn test_get_prompt_auto() {
        let prompt = get_prompt_for_mode(ReasoningMode::Auto, None);
        assert!(prompt.contains("selected_mode"));
    }

    #[test]
    fn test_get_prompt_graph_operations() {
        let prompt = get_prompt_for_mode(ReasoningMode::Graph, None);
        assert!(prompt.contains("root"));

        let prompt = get_prompt_for_mode(ReasoningMode::Graph, Some(&Operation::Init));
        assert!(prompt.contains("root"));

        let prompt = get_prompt_for_mode(ReasoningMode::Graph, Some(&Operation::Generate));
        assert!(prompt.contains("children"));

        let prompt = get_prompt_for_mode(ReasoningMode::Graph, Some(&Operation::Score));
        assert!(prompt.contains("scores"));

        let prompt = get_prompt_for_mode(ReasoningMode::Graph, Some(&Operation::Aggregate));
        assert!(prompt.contains("synthesis"));

        let prompt = get_prompt_for_mode(ReasoningMode::Graph, Some(&Operation::Refine));
        assert!(prompt.contains("critique"));

        let prompt = get_prompt_for_mode(ReasoningMode::Graph, Some(&Operation::Prune));
        assert!(prompt.contains("prune"));

        let prompt = get_prompt_for_mode(ReasoningMode::Graph, Some(&Operation::Finalize));
        assert!(prompt.contains("conclusions"));

        let prompt = get_prompt_for_mode(ReasoningMode::Graph, Some(&Operation::State));
        assert!(prompt.contains("structure"));
    }

    #[test]
    fn test_get_prompt_detect_operations() {
        let prompt = get_prompt_for_mode(ReasoningMode::Detect, None);
        assert!(prompt.contains("biases"));

        let prompt = get_prompt_for_mode(ReasoningMode::Detect, Some(&Operation::Biases));
        assert!(prompt.contains("biases"));

        let prompt = get_prompt_for_mode(ReasoningMode::Detect, Some(&Operation::Fallacies));
        assert!(prompt.contains("fallacies"));
    }

    #[test]
    fn test_get_prompt_decision_operations() {
        let prompt = get_prompt_for_mode(ReasoningMode::Decision, None);
        assert!(prompt.contains("weighted"));

        let prompt = get_prompt_for_mode(ReasoningMode::Decision, Some(&Operation::Weighted));
        assert!(prompt.contains("weighted"));

        let prompt = get_prompt_for_mode(ReasoningMode::Decision, Some(&Operation::Pairwise));
        assert!(prompt.contains("pairwise"));

        let prompt = get_prompt_for_mode(ReasoningMode::Decision, Some(&Operation::Topsis));
        assert!(prompt.contains("TOPSIS"));

        let prompt = get_prompt_for_mode(ReasoningMode::Decision, Some(&Operation::Perspectives));
        assert!(prompt.contains("stakeholder"));
    }

    #[test]
    fn test_get_prompt_evidence_operations() {
        let prompt = get_prompt_for_mode(ReasoningMode::Evidence, None);
        assert!(prompt.contains("credibility"));

        let prompt = get_prompt_for_mode(ReasoningMode::Evidence, Some(&Operation::Assess));
        assert!(prompt.contains("credibility"));

        let prompt = get_prompt_for_mode(ReasoningMode::Evidence, Some(&Operation::Probabilistic));
        assert!(prompt.contains("Bayesian"));
    }

    #[test]
    fn test_get_prompt_timeline_operations() {
        let prompt = get_prompt_for_mode(ReasoningMode::Timeline, None);
        assert!(prompt.contains("timeline"));

        let prompt = get_prompt_for_mode(ReasoningMode::Timeline, Some(&Operation::TimelineCreate));
        assert!(prompt.contains("events"));

        let prompt = get_prompt_for_mode(ReasoningMode::Timeline, Some(&Operation::Branch));
        assert!(prompt.contains("branch"));

        let prompt = get_prompt_for_mode(ReasoningMode::Timeline, Some(&Operation::Compare));
        assert!(prompt.contains("compare"));

        let prompt = get_prompt_for_mode(ReasoningMode::Timeline, Some(&Operation::Merge));
        assert!(prompt.contains("robust"));
    }

    #[test]
    fn test_get_prompt_mcts_operations() {
        let prompt = get_prompt_for_mode(ReasoningMode::Mcts, None);
        assert!(prompt.contains("UCB1"));

        let prompt = get_prompt_for_mode(ReasoningMode::Mcts, Some(&Operation::Explore));
        assert!(prompt.contains("UCB1"));

        let prompt = get_prompt_for_mode(ReasoningMode::Mcts, Some(&Operation::AutoBacktrack));
        assert!(prompt.contains("backtrack"));
    }

    #[test]
    fn test_get_prompt_counterfactual() {
        let prompt = get_prompt_for_mode(ReasoningMode::Counterfactual, None);
        assert!(prompt.contains("Pearl"));
        assert!(prompt.contains("causal"));
    }

    #[test]
    fn test_all_modes_return_valid_prompts() {
        for mode in ReasoningMode::all() {
            let prompt = get_prompt_for_mode(*mode, None);
            assert!(!prompt.is_empty(), "Mode {:?} returned empty prompt", mode);
            assert!(prompt.len() > 100, "Mode {:?} prompt too short", mode);
        }
    }
}
