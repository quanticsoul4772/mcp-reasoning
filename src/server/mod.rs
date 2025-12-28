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
//! ```no_run
//! use mcp_reasoning::server::{AppState, TransportConfig};
//! use mcp_reasoning::storage::SqliteStorage;
//! use mcp_reasoning::anthropic::{AnthropicClient, ClientConfig};
//! use mcp_reasoning::config::{Config, SecretString, DEFAULT_MODEL};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = SqliteStorage::new("./data/reasoning.db").await?;
//! let client = AnthropicClient::new("sk-ant-xxx", ClientConfig::default())?;
//! let config = Config {
//!     api_key: SecretString::new("sk-ant-xxx"),
//!     database_path: "./data/reasoning.db".to_string(),
//!     log_level: "info".to_string(),
//!     request_timeout_ms: 30000,
//!     max_retries: 3,
//!     model: DEFAULT_MODEL.to_string(),
//! };
//! let state = AppState::new(storage, client, config);
//! # Ok(())
//! # }
//! ```

mod mcp;
mod params;
mod tools;
mod transport;
mod types;

pub use mcp::McpServer;
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
pub use transport::{StdioTransport, TransportConfig};
pub use types::AppState;
