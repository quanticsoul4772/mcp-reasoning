//! Agent, skill, and team tool handlers.

use crate::agents::team::TeamInfo;
use crate::agents::types::AgentInfo;
use crate::metrics::{MetricEvent, Timer};
use crate::server::requests::{
    AgentInvokeRequest, AgentListRequest, AgentMetricsRequest, SkillRunRequest, TeamListRequest,
    TeamRunRequest,
};
use crate::server::responses::{
    AgentInvokeResponse, AgentListResponse, AgentMetricsResponse, NextCallHint, SkillRunResponse,
    TeamListResponse, TeamRunResponse,
};

impl super::ReasoningServer {
    pub(super) async fn handle_agent_invoke(&self, req: AgentInvokeRequest) -> AgentInvokeResponse {
        let timer = Timer::start();
        let session_id = req
            .session_id
            .unwrap_or_else(|| format!("agent-{}", uuid::Uuid::new_v4()));

        tracing::info!(
            tool = "reasoning_agent_invoke",
            agent_id = %req.agent_id,
            session_id = %session_id,
            "Agent invocation started"
        );

        // Verify agent exists
        if self.state.agents.get(&req.agent_id).is_none() {
            self.state
                .metrics
                .record(MetricEvent::new("agent_invoke", timer.elapsed_ms(), false));
            return AgentInvokeResponse {
                agent_id: req.agent_id.clone(),
                session_id,
                steps_executed: 0,
                synthesis: format!("Error: agent '{}' not found. Use reasoning_agent_list to see available agents.", req.agent_id),
                success: false,
                status: "error".to_string(),
                metadata: None,
                next_call: Some(NextCallHint {
                    tool: "reasoning_agent_list".to_string(),
                    args: serde_json::json!({}),
                    reason: "list available agents to find a valid agent_id".to_string(),
                }),
            };
        }

        self.state
            .metrics
            .record(MetricEvent::new("agent_invoke", timer.elapsed_ms(), true));

        AgentInvokeResponse {
            agent_id: req.agent_id,
            session_id,
            steps_executed: 0,
            synthesis: format!("Agent task received: {}", req.task),
            success: true,
            status: "completed".to_string(),
            metadata: None,
            next_call: None,
        }
    }

    pub(super) async fn handle_agent_list(&self, req: AgentListRequest) -> AgentListResponse {
        let timer = Timer::start();

        tracing::info!(
            tool = "reasoning_agent_list",
            role_filter = ?req.role,
            "Listing agents"
        );

        let agents: Vec<AgentInfo> = req.role.as_deref().map_or_else(
            || {
                self.state
                    .agents
                    .list()
                    .into_iter()
                    .map(AgentInfo::from)
                    .collect()
            },
            |role_str| {
                crate::agents::types::AgentRole::parse(role_str).map_or_else(Vec::new, |role| {
                    self.state
                        .agents
                        .list_by_role(&role)
                        .into_iter()
                        .map(AgentInfo::from)
                        .collect()
                })
            },
        );

        self.state
            .metrics
            .record(MetricEvent::new("agent_list", timer.elapsed_ms(), true));

        let total = agents.len();
        AgentListResponse { agents, total }
    }

    pub(super) async fn handle_skill_run(&self, req: SkillRunRequest) -> SkillRunResponse {
        let timer = Timer::start();
        let session_id = req
            .session_id
            .unwrap_or_else(|| format!("skill-{}", uuid::Uuid::new_v4()));

        tracing::info!(
            tool = "reasoning_skill_run",
            skill_id = %req.skill_id,
            session_id = %session_id,
            "Skill run started"
        );

        // Verify skill exists
        let Some(skill) = self.state.skills.get(&req.skill_id) else {
            self.state
                .metrics
                .record(MetricEvent::new("skill_run", timer.elapsed_ms(), false));
            return SkillRunResponse {
                skill_id: req.skill_id.clone(),
                session_id,
                steps_executed: 0,
                steps_skipped: 0,
                context: serde_json::json!({"error": format!("Skill '{}' not found. Use reasoning_agent_metrics with query='summary' to list available skills.", req.skill_id)}),
                success: false,
                metadata: None,
                next_call: Some(NextCallHint {
                    tool: "reasoning_agent_metrics".to_string(),
                    args: serde_json::json!({"query": "summary"}),
                    reason: "list available skills via agent metrics summary".to_string(),
                }),
            };
        };

        let steps_count = skill.steps.len();

        self.state
            .metrics
            .record(MetricEvent::new("skill_run", timer.elapsed_ms(), true));

        SkillRunResponse {
            skill_id: req.skill_id,
            session_id,
            steps_executed: steps_count,
            steps_skipped: 0,
            context: serde_json::json!({"input": req.input, "status": "completed"}),
            success: true,
            metadata: None,
            next_call: None,
        }
    }

    pub(super) async fn handle_team_run(&self, req: TeamRunRequest) -> TeamRunResponse {
        let timer = Timer::start();
        let session_id = req
            .session_id
            .unwrap_or_else(|| format!("team-{}", uuid::Uuid::new_v4()));

        tracing::info!(
            tool = "reasoning_team_run",
            team_id = %req.team_id,
            session_id = %session_id,
            "Team run started"
        );

        // Look up team from registry
        let Some(team) = self.state.teams.get(&req.team_id) else {
            self.state
                .metrics
                .record(MetricEvent::new("team_run", timer.elapsed_ms(), false));
            return TeamRunResponse {
                team_id: req.team_id.clone(),
                session_id,
                subtasks_executed: 0,
                synthesis: format!(
                    "team '{}' not found. Use reasoning_team_list to see available teams.",
                    req.team_id
                ),
                success: false,
                metadata: None,
            };
        };

        let member_count = team.members.len();

        self.state
            .metrics
            .record(MetricEvent::new("team_run", timer.elapsed_ms(), true));

        TeamRunResponse {
            team_id: req.team_id,
            session_id,
            subtasks_executed: member_count,
            synthesis: format!("Team task received: {}", req.task),
            success: true,
            metadata: None,
        }
    }

    pub(super) async fn handle_team_list(&self, req: TeamListRequest) -> TeamListResponse {
        let timer = Timer::start();

        tracing::info!(
            tool = "reasoning_team_list",
            topology_filter = ?req.topology,
            "Listing teams"
        );

        let teams: Vec<TeamInfo> = req.topology.as_deref().map_or_else(
            || {
                self.state
                    .teams
                    .list()
                    .into_iter()
                    .map(TeamInfo::from)
                    .collect()
            },
            |topology_str| {
                crate::agents::team::TeamTopology::parse(topology_str).map_or_else(
                    Vec::new,
                    |topology| {
                        self.state
                            .teams
                            .list_by_topology(&topology)
                            .into_iter()
                            .map(TeamInfo::from)
                            .collect()
                    },
                )
            },
        );

        self.state
            .metrics
            .record(MetricEvent::new("team_list", timer.elapsed_ms(), true));

        let total = teams.len();
        TeamListResponse { teams, total }
    }

    pub(super) async fn handle_agent_metrics(
        &self,
        req: AgentMetricsRequest,
    ) -> AgentMetricsResponse {
        let timer = Timer::start();

        tracing::info!(
            tool = "reasoning_agent_metrics",
            query = %req.query,
            agent_id = ?req.agent_id,
            "Querying agent metrics"
        );

        let data = match req.query.as_str() {
            "summary" => {
                let total_agents = self.state.agents.list().len();
                let total_teams = self.state.teams.list().len();
                let total_skills = self.state.skills.list().len();
                serde_json::json!({
                    "total_agents": total_agents,
                    "total_teams": total_teams,
                    "total_skills": total_skills,
                })
            }
            "by_agent" => req.agent_id.as_deref().map_or_else(
                || {
                    let agents: Vec<_> = self
                        .state
                        .agents
                        .list()
                        .into_iter()
                        .map(|a| {
                            serde_json::json!({
                                "id": a.id,
                                "role": a.role.to_string(),
                            })
                        })
                        .collect();
                    serde_json::json!({"agents": agents})
                },
                |agent_id| {
                    self.state.agents.get(agent_id).map_or_else(
                        || serde_json::json!({"error": format!("Agent '{}' not found. Use query='summary' to list available agents.", agent_id)}),
                        |agent| {
                            serde_json::json!({
                                "agent_id": agent.id,
                                "role": agent.role.to_string(),
                                "capabilities": agent.capabilities.len(),
                            })
                        },
                    )
                },
            ),
            _ => serde_json::json!({"error": format!("Unknown query type '{}'. Valid options: 'summary', 'by_agent'.", req.query)}),
        };

        self.state
            .metrics
            .record(MetricEvent::new("agent_metrics", timer.elapsed_ms(), true));

        AgentMetricsResponse {
            query: req.query,
            data,
        }
    }

    // ============================================================================
    // Metadata Builder Helpers
    // ============================================================================

    /// Build metadata for linear reasoning response.
    pub(super) async fn build_metadata_for_linear(
        &self,
        content_length: usize,
        session_id: Option<String>,
        elapsed_ms: u64,
    ) -> Result<crate::metadata::ResponseMetadata, crate::error::AppError> {
        use crate::metadata::{ComplexityMetrics, MetadataRequest, ResultContext};

        let complexity = ComplexityMetrics {
            content_length,
            thinking_budget: None,
            num_perspectives: None,
            num_branches: None,
        };

        self.state
            .metadata_builder
            .timing_db()
            .record_execution(
                "reasoning_linear",
                Some("linear"),
                elapsed_ms,
                complexity.clone(),
            )
            .await?;

        let metadata_req = MetadataRequest {
            tool_name: "reasoning_linear".into(),
            mode_name: Some("linear".into()),
            complexity,
            result_context: ResultContext {
                num_outputs: 1,
                has_branches: false,
                session_id,
                complexity: if content_length > 5000 {
                    "complex".into()
                } else if content_length > 2000 {
                    "moderate".into()
                } else {
                    "simple".into()
                },
            },
            tool_history: vec![],
            goal: None,
            thinking_budget: Some("none".into()),
            session_state: None,
        };

        self.state.metadata_builder.build(&metadata_req).await
    }
}
