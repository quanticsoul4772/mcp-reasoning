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
//! use std::sync::Arc;
//! use mcp_reasoning::server::{AppState, TransportConfig, create_progress_channel};
//! use mcp_reasoning::storage::SqliteStorage;
//! use mcp_reasoning::anthropic::{AnthropicClient, ClientConfig};
//! use mcp_reasoning::config::{Config, SecretString, DEFAULT_MODEL};
//! use mcp_reasoning::metrics::MetricsCollector;
//! use mcp_reasoning::self_improvement::ManagerHandle;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = SqliteStorage::new("./data/reasoning.db").await?;
//! let client = AnthropicClient::new("sk-ant-xxx", ClientConfig::default())?;
//! let config = Config {
//!     api_key: SecretString::new("sk-ant-xxx"),
//!     database_path: "./data/reasoning.db".to_string(),
//!     log_level: "info".to_string(),
//!     request_timeout_ms: 30000,
//!     request_timeout_deep_ms: 60000,
//!     request_timeout_maximum_ms: 120000,
//!     max_retries: 3,
//!     model: DEFAULT_MODEL.to_string(),
//! };
//! let metrics = Arc::new(MetricsCollector::new());
//! let si_handle = ManagerHandle::for_testing(); // In production, use SelfImprovementManager::new()
//! let metadata_builder = mcp_reasoning::metadata::MetadataBuilder::new(
//!     Arc::new(mcp_reasoning::metadata::TimingDatabase::new(Arc::new(storage.clone()))),
//!     Arc::new(mcp_reasoning::metadata::PresetIndex::build()),
//!     30000,
//! );
//! let (progress_tx, _rx) = create_progress_channel();
//! let state = AppState::new(storage, client, config, metrics, si_handle, metadata_builder, progress_tx);
//! # Ok(())
//! # }
//! ```

mod mcp;
mod metadata_builders;
mod params;
mod progress;
mod requests;
mod responses;
mod tools;
mod transport;
mod types;

pub use mcp::McpServer;
pub use progress::{create_progress_channel, ProgressEvent, ProgressMilestone, ProgressReporter};
pub use params::{
    AnalysisDepth, AutoParams, CausalModelDef, CausalRelationship, CheckpointOperation,
    CheckpointParams, CounterfactualParams, DecisionCriterion, DecisionParams, DecisionType,
    DetectParams, DetectType, DivergentParams, EvidenceParams, EvidencePieceDef,
    EvidenceSourceType, EvidenceType, GraphConfig, GraphOperation, GraphParams, LinearParams,
    MctsOperation, MctsParams, MergeStrategy, MetricsParams, MetricsQuery, PresetOperation,
    PresetParams, ReflectionOperation, ReflectionParams, StakeholderDef, TimelineOperation,
    TimelineParams, TreeOperation, TreeParams,
};
pub use requests::{
    AutoRequest, CheckpointRequest, CounterfactualRequest, DecisionRequest, DetectRequest,
    DivergentRequest, EvidenceRequest, GraphRequest, LinearRequest, MctsRequest, MetricsRequest,
    PresetRequest, ReflectionRequest, SiApproveRequest, SiDiagnosesRequest, SiRejectRequest,
    SiRollbackRequest, SiStatusRequest, SiTriggerRequest, TimelineRequest, TreeRequest,
};
pub use responses::{
    AutoResponse, BacktrackSuggestion, Branch, BranchComparison, CausalStep, Checkpoint,
    CheckpointResponse, ConfidenceInterval, CounterfactualResponse, DecisionResponse,
    DetectResponse, Detection, DivergentResponse, EvidenceAssessment, EvidenceResponse, GraphNode,
    GraphResponse, GraphState, Invocation, LinearResponse, MctsNode, MctsResponse, MetricsResponse,
    MetricsSummary, ModeStats, Perspective, PresetExecution, PresetInfo, PresetResponse,
    RankedOption, ReflectionResponse, SiApproveResponse, SiDiagnosesResponse, SiExecutionSummary,
    SiLearningSummary, SiPendingDiagnosis, SiRejectResponse, SiRollbackResponse, SiStatusResponse,
    SiTriggerResponse, StakeholderMap, TimelineBranch, TimelineResponse, TreeResponse,
};
pub use tools::ReasoningServer;
pub use transport::{StdioTransport, TransportConfig};
pub use types::AppState;
