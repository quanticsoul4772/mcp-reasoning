//! Reasoning modes.
//!
//! This module implements the 13 reasoning modes:
//! - Core: linear, tree, divergent, reflection, checkpoint, auto
//! - Graph: graph (8 operations)
//! - Analysis: detect, decision, evidence
//! - Advanced: timeline, mcts, counterfactual
//!
//! # Architecture
//!
//! All modes use trait-based dependency injection for testability.
//! Each mode accepts implementations of [`StorageTrait`] and [`AnthropicClientTrait`].
//!
//! # Example
//!
//! ```ignore
//! use mcp_reasoning::modes::{LinearMode, extract_json};
//! use mcp_reasoning::prompts::{ReasoningMode, get_prompt_for_mode};
//!
//! let mode = LinearMode::new(storage, client);
//! let response = mode.process("Analyze this", None, None).await?;
//! ```
//!
//! [`StorageTrait`]: crate::traits::StorageTrait
//! [`AnthropicClientTrait`]: crate::traits::AnthropicClientTrait

mod auto;
mod checkpoint;
mod core;
mod counterfactual;
mod decision;
mod detect;
mod divergent;
mod evidence;
mod graph;
mod linear;
mod mcts;
mod reflection;
mod timeline;
mod tree;

pub use auto::{AlternativeMode, AutoMode, AutoResponse};
pub use checkpoint::{
    CheckpointContext, CheckpointMode, CheckpointSummary, CreateResponse, ListResponse,
    RestoreResponse, RestoredState,
};
pub use core::{
    extract_json, generate_branch_id, generate_checkpoint_id, generate_node_id,
    generate_session_id, generate_thought_id, serialize_for_log, validate_confidence,
    validate_content, ModeCore,
};
pub use counterfactual::{
    AssociationLevel, CausalAnalysis, CausalConclusions, CausalEdge, CausalModel, CausalQuestion,
    CausalStrength, CausalVariables, CounterfactualLevel, CounterfactualMode,
    CounterfactualResponse, EdgeType, InterventionLevel, LadderRung,
};
pub use decision::{
    Alignment, BalancedRecommendation, Conflict, ConflictSeverity, Criterion, CriterionType,
    DecisionMode, InfluenceLevel, PairwiseComparison, PairwiseRank, PairwiseResponse,
    PerspectivesResponse, PreferenceResult, PreferenceStrength, RankedOption, Stakeholder,
    TopsisCreterion, TopsisDistances, TopsisRank, TopsisResponse, WeightedResponse,
};
pub use detect::{
    ArgumentStructure, ArgumentValidity, BiasAssessment, BiasSeverity, BiasesResponse,
    DetectMode, DetectedBias, DetectedFallacy, FallaciesResponse, FallacyAssessment,
    FallacyCategory,
};
pub use divergent::{DivergentMode, DivergentResponse, Perspective};
pub use evidence::{
    AssessResponse, BeliefDirection, BeliefMagnitude, BeliefUpdate, Credibility, EvidenceAnalysis,
    EvidenceMode, EvidencePiece, EvidenceQuality, OverallEvidenceAssessment, Posterior, Prior,
    ProbabilisticResponse, SourceType,
};
pub use graph::{
    AggregateResponse, ChildNode, ComplexityLevel, ExpansionDirection, FinalizeResponse,
    FrontierNodeInfo, GenerateResponse, GraphConclusion, GraphMetadata, GraphMetrics, GraphMode,
    GraphPath, GraphStructure, InitResponse, IntegrationNotes, NodeAssessment, NodeCritique,
    NodeRecommendation, NodeRelationship, NodeScores, NodeType, PruneCandidate, PruneImpact,
    PruneReason, PruneResponse, RefineResponse, RefinedNode, RootNode, ScoreResponse,
    SessionQuality, StateResponse, SuggestedAction, SynthesisNode,
};
pub use linear::{LinearMode, LinearResponse};
pub use mcts::{
    AlternativeAction, AlternativeOption, BacktrackDecision, BacktrackResponse, Backpropagation,
    Expansion, ExploreResponse, FrontierNode, MctsMode, NewNode, QualityAssessment, QualityTrend,
    Recommendation, RecommendedAction, SearchStatus, SelectedNode,
};
pub use reflection::{
    EvaluateResponse, Improvement, Priority, ProcessResponse, ReasoningAnalysis, ReflectionMode,
    SessionAssessment,
};
pub use timeline::{
    BranchComparison, BranchDifference, BranchEvent, BranchPoint, BranchResponse, CommonPattern,
    CompareRecommendation, CompareResponse, CreateTimelineResponse, DecisionPoint, EventType,
    FragileStrategy, MergeResponse, OpportunityAssessment, RiskAssessment, RobustStrategy,
    TemporalStructure, TimelineBranch, TimelineEvent, TimelineMode,
};
pub use tree::{Branch, BranchStatus, TreeMode, TreeResponse};

// Re-export from prompts for convenience
pub use crate::prompts::{get_prompt_for_mode, Operation, ParseModeError, ReasoningMode};
