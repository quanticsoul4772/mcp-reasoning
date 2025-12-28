//! MCP server implementation.
//!
//! This module provides:
//! - MCP JSON-RPC protocol handling
//! - Tool definitions with rmcp macros
//! - Transport layer (stdio, HTTP)
//! - Handler registry for tool dispatch
//!
//! # Architecture
//!
//! The server is built on the rmcp SDK and provides 15 reasoning tools:
//!
//! - **Core**: linear, tree, divergent, reflection, checkpoint, auto
//! - **Graph**: graph (8 operations)
//! - **Analysis**: detect, decision, evidence
//! - **Advanced**: timeline, mcts, counterfactual
//! - **Infrastructure**: preset, metrics
//!
//! # Example
//!
//! ```ignore
//! use mcp_reasoning::server::{AppState, ReasoningServer};
//!
//! let state = AppState::new(storage, client, config);
//! let server = ReasoningServer::new(state);
//! server.serve_stdio().await?;
//! ```

mod mcp;
mod params;
mod tools;
mod transport;
mod types;

pub use params::{
    AnalysisDepth, AutoParams, CausalModelDef, CausalRelationship, CheckpointOperation,
    CheckpointParams, CounterfactualParams, DecisionCriterion, DecisionParams, DecisionType,
    DetectParams, DetectType, DivergentParams, EvidenceParams, EvidencePieceDef,
    EvidenceSourceType, EvidenceType, GraphConfig, GraphOperation, GraphParams, LinearParams,
    MctsOperation, MctsParams, MergeStrategy, MetricsParams, MetricsQuery, PresetOperation,
    PresetParams, ReflectionOperation, ReflectionParams, StakeholderDef, TimelineOperation,
    TimelineParams, TreeOperation, TreeParams,
};
pub use tools::{
    AutoRequest, AutoResponse, BacktrackSuggestion, Branch, BranchComparison, CausalStep,
    Checkpoint, CheckpointRequest, CheckpointResponse, ConfidenceInterval, CounterfactualRequest,
    CounterfactualResponse, DecisionRequest, DecisionResponse, DetectRequest, DetectResponse,
    Detection, DivergentRequest, DivergentResponse, EvidenceAssessment, EvidenceRequest,
    EvidenceResponse, GraphNode, GraphRequest, GraphResponse, GraphState, Invocation,
    LinearRequest, LinearResponse, MctsNode, MctsRequest, MctsResponse, MetricsRequest,
    MetricsResponse, MetricsSummary, ModeStats, Perspective, PresetExecution, PresetInfo,
    PresetRequest, PresetResponse, RankedOption, ReasoningServer, ReflectionRequest,
    ReflectionResponse, StakeholderMap, TimelineBranch, TimelineRequest, TimelineResponse,
    TreeRequest, TreeResponse,
};
pub use mcp::McpServer;
pub use transport::{StdioTransport, TransportConfig};
pub use types::AppState;
