//! Tool definitions with rmcp macros.
//!
//! This module defines all reasoning tools using the rmcp 0.12 macro system.
//! Uses `#[tool_router]` on impl with tools and `#[tool_handler]` on ServerHandler.
//!
//! Request and response types are defined in separate modules for maintainability.
//! Handler implementations are split across category-specific files.

// Tool methods are async stubs that will use await when connected to actual mode implementations
#![allow(clippy::unused_async)]

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router};

use super::requests::{
    AgentInvokeRequest, AgentListRequest, AgentMetricsRequest, AutoRequest, CheckpointRequest,
    CounterfactualRequest, DecisionRequest, DetectRequest, DivergentRequest, EvidenceRequest,
    GraphRequest, LinearRequest, ListSessionsRequest, MctsRequest, MetaRequest, MetricsRequest,
    PresetRequest, ReflectionRequest, RelateSessionsRequest, ResumeSessionRequest,
    SearchSessionsRequest, SiApproveRequest, SiDiagnosesRequest, SiRejectRequest,
    SiRollbackRequest, SiStatusRequest, SiTriggerRequest, SkillRunRequest, TeamListRequest,
    TeamRunRequest, TimelineRequest, TreeRequest,
};
use super::responses::{
    AgentInvokeResponse, AgentListResponse, AgentMetricsResponse, AutoResponse, CheckpointResponse,
    CounterfactualResponse, DecisionResponse, DetectResponse, DivergentResponse, EvidenceResponse,
    GraphResponse, LinearResponse, ListSessionsResponse, MctsResponse, MetaResponse,
    MetricsResponse, PresetResponse, ReflectionResponse, RelateSessionsResponse,
    ResumeSessionResponse, SearchSessionsResponse, SiApproveResponse, SiDiagnosesResponse,
    SiRejectResponse, SiRollbackResponse, SiStatusResponse, SiTriggerResponse, SkillRunResponse,
    TeamListResponse, TeamRunResponse, TimelineResponse, TreeResponse,
};
use super::types::AppState;

// Handler modules (each contains `impl ReasoningServer` with handler methods)
mod handlers_agents;
mod handlers_basic;
mod handlers_cognitive;
mod handlers_decision;
mod handlers_graph;
mod handlers_infra;
mod handlers_sessions;
mod handlers_si;
mod handlers_temporal;

// ============================================================================
// Thinking Budget Constants for Timeout Selection
// ============================================================================

/// No extended thinking (fast modes) - 30s timeout
const NO_THINKING: Option<u32> = None;
/// Standard thinking budget (4096 tokens) - 30s timeout
const STANDARD_THINKING: Option<u32> = Some(4096);
/// Deep thinking budget (8192 tokens) - 60s timeout
const DEEP_THINKING: Option<u32> = Some(8192);
/// Maximum thinking budget (16384 tokens) - 120s timeout
const MAXIMUM_THINKING: Option<u32> = Some(16384);

// ============================================================================
// ReasoningServer with Tool Router (rmcp 0.12 syntax)
// ============================================================================

/// Reasoning server with all tools.
#[derive(Clone)]
pub struct ReasoningServer {
    /// Shared application state.
    pub state: Arc<AppState>,
    /// Tool router for handling tool calls.
    tool_router: ToolRouter<Self>,
}

impl ReasoningServer {
    /// Creates a new reasoning server.
    #[must_use]
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }
}

// ============================================================================
// Tool Router - Thin delegations to handler methods
// ============================================================================

#[tool_router]
impl ReasoningServer {
    // -- Core reasoning tools --

    #[tool(
        name = "reasoning_linear",
        description = "Process a thought and get a logical continuation with confidence scoring."
    )]
    async fn reasoning_linear(&self, req: Parameters<LinearRequest>) -> LinearResponse {
        self.handle_linear(req.0).await
    }

    #[tool(
        name = "reasoning_tree",
        description = "Branching exploration: create=start with 2-4 paths, focus=select branch, list=show branches, complete=mark finished."
    )]
    async fn reasoning_tree(&self, req: Parameters<TreeRequest>) -> TreeResponse {
        self.handle_tree(req.0).await
    }

    #[tool(
        name = "reasoning_auto",
        description = "Analyze content and route to optimal reasoning mode."
    )]
    async fn reasoning_auto(&self, req: Parameters<AutoRequest>) -> AutoResponse {
        self.handle_auto(req.0).await
    }

    #[tool(
        name = "reasoning_meta",
        description = "Select the best reasoning tool based on historical effectiveness data. Classifies the problem type and recommends the most effective tool from empirical success rates. Falls back to reasoning_auto when no data exists."
    )]
    async fn reasoning_meta(&self, req: Parameters<MetaRequest>) -> MetaResponse {
        self.handle_meta(req.0).await
    }

    // -- Cognitive tools --

    #[tool(
        name = "reasoning_divergent",
        description = "Generate novel perspectives with assumption challenges and optional force_rebellion mode."
    )]
    async fn reasoning_divergent(&self, req: Parameters<DivergentRequest>) -> DivergentResponse {
        self.handle_divergent(req.0).await
    }

    #[tool(
        name = "reasoning_reflection",
        description = "Analyze and improve reasoning: process=iterative refinement, evaluate=session assessment."
    )]
    async fn reasoning_reflection(&self, req: Parameters<ReflectionRequest>) -> ReflectionResponse {
        self.handle_reflection(req.0).await
    }

    #[tool(
        name = "reasoning_checkpoint",
        description = "Save and restore reasoning state: create=save, list=show, restore=return to checkpoint."
    )]
    async fn reasoning_checkpoint(&self, req: Parameters<CheckpointRequest>) -> CheckpointResponse {
        self.handle_checkpoint(req.0).await
    }

    // -- Graph tools --

    #[tool(
        name = "reasoning_graph",
        description = "Graph reasoning: init/generate/score/aggregate/refine/prune/finalize/state operations."
    )]
    async fn reasoning_graph(&self, req: Parameters<GraphRequest>) -> GraphResponse {
        self.handle_graph(req.0).await
    }

    #[tool(
        name = "reasoning_detect",
        description = "Detect cognitive biases and logical fallacies in reasoning."
    )]
    async fn reasoning_detect(&self, req: Parameters<DetectRequest>) -> DetectResponse {
        self.handle_detect(req.0).await
    }

    // -- Decision tools --

    #[tool(
        name = "reasoning_decision",
        description = "Decision analysis: weighted/pairwise/topsis scoring or perspectives stakeholder mapping."
    )]
    async fn reasoning_decision(&self, req: Parameters<DecisionRequest>) -> DecisionResponse {
        self.handle_decision(req.0).await
    }

    #[tool(
        name = "reasoning_evidence",
        description = "Evaluate evidence: assess=credibility scoring, probabilistic=Bayesian belief update."
    )]
    async fn reasoning_evidence(&self, req: Parameters<EvidenceRequest>) -> EvidenceResponse {
        self.handle_evidence(req.0).await
    }

    // -- Temporal tools --

    #[tool(
        name = "reasoning_timeline",
        description = "Temporal reasoning: create/branch/compare/merge operations."
    )]
    async fn reasoning_timeline(&self, req: Parameters<TimelineRequest>) -> TimelineResponse {
        self.handle_timeline(req.0).await
    }

    #[tool(
        name = "reasoning_mcts",
        description = "MCTS: explore=UCB1-guided search, auto_backtrack=quality-triggered backtracking."
    )]
    async fn reasoning_mcts(&self, req: Parameters<MctsRequest>) -> MctsResponse {
        self.handle_mcts(req.0).await
    }

    #[tool(
        name = "reasoning_counterfactual",
        description = "What-if analysis using Pearl's Ladder of Causation."
    )]
    async fn reasoning_counterfactual(
        &self,
        req: Parameters<CounterfactualRequest>,
    ) -> CounterfactualResponse {
        self.handle_counterfactual(req.0).await
    }

    // -- Infrastructure tools --

    #[tool(
        name = "reasoning_preset",
        description = "Execute pre-defined reasoning workflows: list=show presets, run=execute workflow."
    )]
    async fn reasoning_preset(&self, req: Parameters<PresetRequest>) -> PresetResponse {
        self.handle_preset(req.0).await
    }

    #[tool(
        name = "reasoning_metrics",
        description = "Query metrics: summary/by_mode/invocations/fallbacks/config."
    )]
    async fn reasoning_metrics(&self, req: Parameters<MetricsRequest>) -> MetricsResponse {
        self.handle_metrics(req.0).await
    }

    // -- Self-improvement tools --

    #[tool(
        name = "reasoning_si_status",
        description = "Get self-improvement system status including cycle stats and circuit breaker state."
    )]
    async fn reasoning_si_status(&self, req: Parameters<SiStatusRequest>) -> SiStatusResponse {
        self.handle_si_status(req.0).await
    }

    #[tool(
        name = "reasoning_si_diagnoses",
        description = "Get pending diagnoses awaiting approval."
    )]
    async fn reasoning_si_diagnoses(
        &self,
        req: Parameters<SiDiagnosesRequest>,
    ) -> SiDiagnosesResponse {
        self.handle_si_diagnoses(req.0).await
    }

    #[tool(
        name = "reasoning_si_approve",
        description = "Approve a pending diagnosis to execute its proposed actions."
    )]
    async fn reasoning_si_approve(&self, req: Parameters<SiApproveRequest>) -> SiApproveResponse {
        self.handle_si_approve(req.0).await
    }

    #[tool(
        name = "reasoning_si_reject",
        description = "Reject a pending diagnosis."
    )]
    async fn reasoning_si_reject(&self, req: Parameters<SiRejectRequest>) -> SiRejectResponse {
        self.handle_si_reject(req.0).await
    }

    #[tool(
        name = "reasoning_si_trigger",
        description = "Trigger an immediate improvement cycle."
    )]
    async fn reasoning_si_trigger(&self, req: Parameters<SiTriggerRequest>) -> SiTriggerResponse {
        self.handle_si_trigger(req.0).await
    }

    #[tool(
        name = "reasoning_si_rollback",
        description = "Rollback a previously executed action."
    )]
    async fn reasoning_si_rollback(
        &self,
        req: Parameters<SiRollbackRequest>,
    ) -> SiRollbackResponse {
        self.handle_si_rollback(req.0).await
    }

    // -- Session tools --

    #[tool(
        name = "reasoning_list_sessions",
        description = "List reasoning sessions with pagination and filtering."
    )]
    async fn reasoning_list_sessions(
        &self,
        req: Parameters<ListSessionsRequest>,
    ) -> ListSessionsResponse {
        self.handle_list_sessions(req.0).await
    }

    #[tool(
        name = "reasoning_resume",
        description = "Resume a reasoning session with full context."
    )]
    async fn reasoning_resume(
        &self,
        req: Parameters<ResumeSessionRequest>,
    ) -> ResumeSessionResponse {
        self.handle_resume(req.0).await
    }

    #[tool(
        name = "reasoning_search",
        description = "Search reasoning sessions by semantic similarity."
    )]
    async fn reasoning_search(
        &self,
        req: Parameters<SearchSessionsRequest>,
    ) -> SearchSessionsResponse {
        self.handle_search(req.0).await
    }

    #[tool(
        name = "reasoning_relate",
        description = "Analyze relationships between reasoning sessions."
    )]
    async fn reasoning_relate(
        &self,
        req: Parameters<RelateSessionsRequest>,
    ) -> RelateSessionsResponse {
        self.handle_relate(req.0).await
    }

    // -- Agent & Skill tools --

    #[tool(
        name = "reasoning_agent_invoke",
        description = "Invoke a specialized agent to work on a task using its capabilities."
    )]
    async fn reasoning_agent_invoke(
        &self,
        req: Parameters<AgentInvokeRequest>,
    ) -> AgentInvokeResponse {
        self.handle_agent_invoke(req.0).await
    }

    #[tool(
        name = "reasoning_agent_list",
        description = "List available agents with optional role filter."
    )]
    async fn reasoning_agent_list(&self, req: Parameters<AgentListRequest>) -> AgentListResponse {
        self.handle_agent_list(req.0).await
    }

    #[tool(
        name = "reasoning_skill_run",
        description = "Run a composable skill (tool chain) on input."
    )]
    async fn reasoning_skill_run(&self, req: Parameters<SkillRunRequest>) -> SkillRunResponse {
        self.handle_skill_run(req.0).await
    }

    #[tool(
        name = "reasoning_team_run",
        description = "Run an agent team on a complex task with decomposition."
    )]
    async fn reasoning_team_run(&self, req: Parameters<TeamRunRequest>) -> TeamRunResponse {
        self.handle_team_run(req.0).await
    }

    #[tool(
        name = "reasoning_team_list",
        description = "List available team configurations."
    )]
    async fn reasoning_team_list(&self, req: Parameters<TeamListRequest>) -> TeamListResponse {
        self.handle_team_list(req.0).await
    }

    #[tool(
        name = "reasoning_agent_metrics",
        description = "Query agent performance, discovered skills, optimization suggestions."
    )]
    async fn reasoning_agent_metrics(
        &self,
        req: Parameters<AgentMetricsRequest>,
    ) -> AgentMetricsResponse {
        self.handle_agent_metrics(req.0).await
    }
}

// Implement ServerHandler to integrate with rmcp's server infrastructure
#[tool_handler]
impl ServerHandler for ReasoningServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("MCP Reasoning Server providing 15 structured reasoning tools.")
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests;
