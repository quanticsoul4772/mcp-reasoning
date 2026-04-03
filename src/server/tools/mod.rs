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
    ConfidenceRouteRequest, CounterfactualRequest, DecisionRequest, DetectRequest,
    DivergentRequest, EvidenceRequest, GraphRequest, LinearRequest, ListSessionsRequest,
    MctsRequest, MetaRequest, MetricsRequest, PresetRequest, ReflectionRequest,
    RelateSessionsRequest, ResumeSessionRequest, SearchSessionsRequest, SiApproveRequest,
    SiDiagnosesRequest, SiRejectRequest, SiRollbackRequest, SiStatusRequest, SiTriggerRequest,
    SkillRunRequest, TeamListRequest, TeamRunRequest, TimelineRequest, TreeRequest,
};
use super::responses::{
    AgentInvokeResponse, AgentListResponse, AgentMetricsResponse, AutoResponse, CheckpointResponse,
    ConfidenceRouteResponse, CounterfactualResponse, DecisionResponse, DetectResponse,
    DivergentResponse, EvidenceResponse, GraphResponse, LinearResponse, ListSessionsResponse,
    MctsResponse, MetaResponse, MetricsResponse, PresetResponse, ReflectionResponse,
    RelateSessionsResponse, ResumeSessionResponse, SearchSessionsResponse, SiApproveResponse,
    SiDiagnosesResponse, SiRejectResponse, SiRollbackResponse, SiStatusResponse, SiTriggerResponse,
    SkillRunResponse, TeamListResponse, TeamRunResponse, TimelineResponse, TreeResponse,
};
use super::types::AppState;

// Handler modules (each contains `impl ReasoningServer` with handler methods)
mod handlers_agents;
mod handlers_basic;
mod handlers_cognitive;
mod handlers_confidence;
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
        description = "Step-by-step reasoning for single-path analysis, explanations, and direct problem solving. Returns analysis with confidence score and suggested next step."
    )]
    async fn reasoning_linear(&self, req: Parameters<LinearRequest>) -> LinearResponse {
        self.handle_linear(req.0).await
    }

    #[tool(
        name = "reasoning_tree",
        description = "Branching exploration: create=start with 2-4 paths, focus=select branch, list=show branches, complete=mark finished, summarize=synthesize all branches into final answer."
    )]
    async fn reasoning_tree(&self, req: Parameters<TreeRequest>) -> TreeResponse {
        self.handle_tree(req.0).await
    }

    #[tool(
        name = "reasoning_auto",
        description = "Start here when unsure which tool to use. Analyzes content and routes automatically to the best reasoning mode. Does NOT apply empirical usage history — use reasoning_meta instead when 10+ prior sessions exist."
    )]
    async fn reasoning_auto(&self, req: Parameters<AutoRequest>) -> AutoResponse {
        self.handle_auto(req.0).await
    }

    #[tool(
        name = "reasoning_meta",
        description = "Select the best reasoning tool based on historical effectiveness data. Use instead of reasoning_auto when 10+ prior sessions exist — classifies the problem type and picks the tool with the highest empirical success rate for that class. Falls back to reasoning_auto when no data exists. Does NOT execute reasoning itself — returns a tool recommendation."
    )]
    async fn reasoning_meta(&self, req: Parameters<MetaRequest>) -> MetaResponse {
        self.handle_meta(req.0).await
    }

    #[tool(
        name = "reasoning_confidence_route",
        description = "Confidence-aware routing: detects the best reasoning strategy, checks confidence, then executes it or escalates. \
                       High confidence (≥0.75) → executes the auto-selected mode directly. \
                       Low confidence → escalates to tree reasoning for thorough exploration. \
                       Budget overrides: 'low' forces linear (fast), 'high' forces tree (thorough). \
                       Returns the result plus a routing_trace showing confidence, decision, and why."
    )]
    async fn reasoning_confidence_route(
        &self,
        req: Parameters<ConfidenceRouteRequest>,
    ) -> ConfidenceRouteResponse {
        self.handle_confidence_route(req.0).await
    }

    // -- Cognitive tools --

    #[tool(
        name = "reasoning_divergent",
        description = "Break out of conventional thinking: generates multiple novel perspectives, challenges assumptions, optional force_rebellion for radical alternatives."
    )]
    async fn reasoning_divergent(&self, req: Parameters<DivergentRequest>) -> DivergentResponse {
        self.handle_divergent(req.0).await
    }

    #[tool(
        name = "reasoning_reflection",
        description = "Meta-cognitive reasoning that operates on reasoning itself, not on the original problem. \
                       process=iterative self-critique and improvement of a prior reasoning output (pass previous result as input). \
                       evaluate=assess an entire reasoning session for quality, consistency, and blind spots. \
                       Does NOT re-solve the original problem — use after reasoning_linear/tree/etc. to improve their output."
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
        description = "Graph-of-Thoughts for complex multi-faceted problems requiring decomposition and aggregation: init/generate/score/aggregate/refine/prune/finalize/state."
    )]
    async fn reasoning_graph(&self, req: Parameters<GraphRequest>) -> GraphResponse {
        self.handle_graph(req.0).await
    }

    #[tool(
        name = "reasoning_detect",
        description = "Detect flaws and gaps in reasoning: biases=cognitive distortions (anchoring, confirmation bias), fallacies=logical errors (ad hominem, strawman), knowledge_gaps=absent information that could change the conclusion (unknown unknowns, unchecked assumptions)."
    )]
    async fn reasoning_detect(&self, req: Parameters<DetectRequest>) -> DetectResponse {
        self.handle_detect(req.0).await
    }

    // -- Decision tools --

    #[tool(
        name = "reasoning_decision",
        description = "Decision analysis for choosing between options: weighted=score options against weighted criteria (most common), pairwise=compare options head-to-head, topsis=rank by ideal/worst distance, perspectives=map stakeholder viewpoints. Does NOT produce a single answer — returns scored rankings and rationale."
    )]
    async fn reasoning_decision(&self, req: Parameters<DecisionRequest>) -> DecisionResponse {
        self.handle_decision(req.0).await
    }

    #[tool(
        name = "reasoning_evidence",
        description = "Evaluate evidence quality and update beliefs from it. \
                       assess=credibility scoring for sources (authoritativeness, bias, recency, corroboration). \
                       probabilistic=Bayesian belief update — given prior probability and new evidence, returns posterior. \
                       Use when deciding how much to trust a claim or source, not for reasoning about solutions."
    )]
    async fn reasoning_evidence(&self, req: Parameters<EvidenceRequest>) -> EvidenceResponse {
        self.handle_evidence(req.0).await
    }

    // -- Temporal tools --

    #[tool(
        name = "reasoning_timeline",
        description = "Temporal reasoning for time-ordered scenarios and alternative decision paths: create=establish sequence, branch=split paths, compare=evaluate options, merge=synthesize."
    )]
    async fn reasoning_timeline(&self, req: Parameters<TimelineRequest>) -> TimelineResponse {
        self.handle_timeline(req.0).await
    }

    #[tool(
        name = "reasoning_mcts",
        description = "Monte Carlo Tree Search for exploring large solution spaces: explore=expand promising paths with UCB1 balance exploitation/exploration, auto_backtrack=retrace when quality degrades."
    )]
    async fn reasoning_mcts(&self, req: Parameters<MctsRequest>) -> MctsResponse {
        self.handle_mcts(req.0).await
    }

    #[tool(
        name = "reasoning_counterfactual",
        description = "Causal what-if analysis using Pearl's Ladder: Level 1=association (what correlates?), Level 2=intervention (what happens if I do X?), Level 3=counterfactual (what would have happened?)."
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
        description = "Execute pre-built multi-step reasoning workflows that chain multiple tools automatically. \
                       list=show available presets (code-review, debug-analysis, architecture-decision, strategic-decision, evidence-conclusion, brainstorming). \
                       run=execute a named preset on your input — handles tool chaining internally. \
                       Use instead of manually chaining tools for these common patterns."
    )]
    async fn reasoning_preset(&self, req: Parameters<PresetRequest>) -> PresetResponse {
        self.handle_preset(req.0).await
    }

    #[tool(
        name = "reasoning_metrics",
        description = "Query usage and performance metrics for the reasoning server. \
                       summary=aggregate stats across all tools. by_mode=breakdown per reasoning mode. \
                       invocations=raw call log with timestamps. fallbacks=cases where auto-routing changed mode. \
                       config=current server configuration. Use to understand which tools are effective before choosing reasoning_meta."
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
        description = "Get pending self-improvement diagnoses awaiting approval. Returns diagnosis IDs and proposed actions — does NOT execute any changes. Use reasoning_si_approve or reasoning_si_reject with the returned diagnosis_id to act on them."
    )]
    async fn reasoning_si_diagnoses(
        &self,
        req: Parameters<SiDiagnosesRequest>,
    ) -> SiDiagnosesResponse {
        self.handle_si_diagnoses(req.0).await
    }

    #[tool(
        name = "reasoning_si_approve",
        description = "Approve a pending self-improvement diagnosis to execute its proposed actions. Requires a diagnosis_id from reasoning_si_diagnoses — does NOT accept free-form instructions. [DESTRUCTIVE: modifies system configuration]"
    )]
    async fn reasoning_si_approve(&self, req: Parameters<SiApproveRequest>) -> SiApproveResponse {
        self.handle_si_approve(req.0).await
    }

    #[tool(
        name = "reasoning_si_reject",
        description = "Reject a pending self-improvement diagnosis to prevent its proposed actions from executing. \
                       Requires a diagnosis_id from reasoning_si_diagnoses. Does NOT accept free-form instructions. \
                       Safe action — no system changes occur. Use when a proposed action is unsafe, incorrect, or premature."
    )]
    async fn reasoning_si_reject(&self, req: Parameters<SiRejectRequest>) -> SiRejectResponse {
        self.handle_si_reject(req.0).await
    }

    #[tool(
        name = "reasoning_si_trigger",
        description = "Trigger an immediate self-improvement cycle without waiting for the scheduled interval. \
                       Runs the full 4-phase cycle: metrics collection → LLM diagnosis → action proposal → approval gate. \
                       Does NOT execute changes — proposed actions still require reasoning_si_approve. \
                       Use when you want a fresh diagnosis now rather than waiting for the next automatic cycle."
    )]
    async fn reasoning_si_trigger(&self, req: Parameters<SiTriggerRequest>) -> SiTriggerResponse {
        self.handle_si_trigger(req.0).await
    }

    #[tool(
        name = "reasoning_si_rollback",
        description = "Rollback a previously executed self-improvement action. Requires an action_id from reasoning_si_status — does NOT accept free-form descriptions. [DESTRUCTIVE: reverts system configuration changes]"
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
        description = "List reasoning sessions with pagination and optional mode/date filtering. Returns session IDs, modes, and timestamps — does NOT return session content or reasoning results. Use reasoning_resume with a session_id to load full content."
    )]
    async fn reasoning_list_sessions(
        &self,
        req: Parameters<ListSessionsRequest>,
    ) -> ListSessionsResponse {
        self.handle_list_sessions(req.0).await
    }

    #[tool(
        name = "reasoning_resume",
        description = "Load a past reasoning session by ID, returning all thoughts, branches, and intermediate results. \
                       Use after reasoning_list_sessions or reasoning_search to retrieve session content. \
                       Returns the full reasoning trace including confidence scores, branch decisions, and final conclusions. \
                       Does NOT continue reasoning — pass the returned context to a reasoning tool to extend the session."
    )]
    async fn reasoning_resume(
        &self,
        req: Parameters<ResumeSessionRequest>,
    ) -> ResumeSessionResponse {
        self.handle_resume(req.0).await
    }

    #[tool(
        name = "reasoning_search",
        description = "Search reasoning sessions by semantic similarity to a query string. Returns matching session IDs and similarity scores — does NOT return session content. Follow up with reasoning_resume to load the full content of a matched session."
    )]
    async fn reasoning_search(
        &self,
        req: Parameters<SearchSessionsRequest>,
    ) -> SearchSessionsResponse {
        self.handle_search(req.0).await
    }

    #[tool(
        name = "reasoning_relate",
        description = "Find conceptual connections between past reasoning sessions — shared themes, contradictions, or evolution of thinking on a topic. \
                       Returns session pairs with relationship type and strength scores. \
                       Use to discover if you have prior reasoning on a topic before starting fresh, or to spot conflicting conclusions across sessions."
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
        description = "Invoke a single specialized agent by name to work on a focused task. \
                       Use reasoning_agent_list first to see available agents and their roles. \
                       For tasks requiring multiple agents with decomposition, use reasoning_team_run instead. \
                       Returns the agent's output and any discovered sub-skills or performance data."
    )]
    async fn reasoning_agent_invoke(
        &self,
        req: Parameters<AgentInvokeRequest>,
    ) -> AgentInvokeResponse {
        self.handle_agent_invoke(req.0).await
    }

    #[tool(
        name = "reasoning_agent_list",
        description = "List available specialized agents with their roles, capabilities, and default skills. \
                       Filter by role to find agents suited for a specific task type (e.g., 'analyst', 'coder', 'reviewer'). \
                       Use before reasoning_agent_invoke or reasoning_team_run to identify the right agent."
    )]
    async fn reasoning_agent_list(&self, req: Parameters<AgentListRequest>) -> AgentListResponse {
        self.handle_agent_list(req.0).await
    }

    #[tool(
        name = "reasoning_skill_run",
        description = "Run a pre-built skill — a named sequence of reasoning steps chained together with context passing. \
                       Skills compose multiple reasoning tools (linear → reflection → decision, etc.) into a single call. \
                       Use reasoning_preset for the 6 common built-in workflows, or this tool for custom-registered skills. \
                       Returns step-by-step results and the final context produced by the chain."
    )]
    async fn reasoning_skill_run(&self, req: Parameters<SkillRunRequest>) -> SkillRunResponse {
        self.handle_skill_run(req.0).await
    }

    #[tool(
        name = "reasoning_team_run",
        description = "Run a coordinated team of agents on a complex task that benefits from parallel specialization. \
                       The team decomposes the task, assigns sub-tasks to specialist agents, and synthesizes results. \
                       Use instead of reasoning_agent_invoke when the task has multiple independent components. \
                       Use reasoning_team_list to see available team configurations before invoking."
    )]
    async fn reasoning_team_run(&self, req: Parameters<TeamRunRequest>) -> TeamRunResponse {
        self.handle_team_run(req.0).await
    }

    #[tool(
        name = "reasoning_team_list",
        description = "List available agent team configurations with their member agents and coordination strategy. \
                       Use before reasoning_team_run to identify the right team for a task type. \
                       Returns team names, member agent roles, and the decomposition strategy each team uses."
    )]
    async fn reasoning_team_list(&self, req: Parameters<TeamListRequest>) -> TeamListResponse {
        self.handle_team_list(req.0).await
    }

    #[tool(
        name = "reasoning_agent_metrics",
        description = "Query agent system performance: task success rates, latency, discovered skill patterns, and optimization suggestions. \
                       Returns per-agent metrics plus system-wide recommendations for improving agent configuration. \
                       Use after multiple reasoning_agent_invoke or reasoning_team_run calls to understand what's working."
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
            .with_instructions(
                "MCP Reasoning Server with 32 tools: 15 core reasoning modes \
                 (linear/tree/divergent/reflection/graph/mcts/counterfactual/timeline/decision/evidence/detect/checkpoint/auto/meta/preset), \
                 6 self-improvement tools (si_*), \
                 4 session management tools, \
                 6 agent and team tools, \
                 plus metrics. \
                 Use reasoning_auto when unsure which tool fits.",
            )
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
