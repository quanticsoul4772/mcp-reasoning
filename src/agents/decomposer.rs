//! Task decomposer for breaking complex tasks into agent subtasks.
//!
//! Uses the LLM to analyze a task and determine which agents should handle
//! which parts, respecting team topology.

use serde::{Deserialize, Serialize};

use super::team::{AgentTeam, TeamTopology};
use crate::error::ModeError;
use crate::modes::extract_json;
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message};

/// A subtask assigned to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    /// The agent ID to handle this subtask.
    pub agent_id: String,
    /// Description of the subtask.
    pub description: String,
    /// Input context for the subtask.
    pub input: String,
    /// Dependencies on other subtask indices.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<usize>,
    /// Priority (lower = higher priority).
    pub priority: usize,
}

/// Result of task decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResult {
    /// Original task.
    pub original_task: String,
    /// Decomposed subtasks.
    pub subtasks: Vec<Subtask>,
    /// Suggested execution order.
    pub execution_order: Vec<usize>,
}

/// Decomposes tasks into subtasks for agent teams.
pub struct TaskDecomposer<C: AnthropicClientTrait> {
    client: C,
}

impl<C: AnthropicClientTrait> TaskDecomposer<C> {
    /// Create a new task decomposer.
    #[must_use]
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Decompose a task for a team.
    pub async fn decompose(
        &self,
        task: &str,
        team: &AgentTeam,
    ) -> Result<DecompositionResult, ModeError> {
        let members: Vec<String> = team
            .members
            .iter()
            .map(|m| {
                format!(
                    "{} ({:?}){}",
                    m.agent_id,
                    m.team_role,
                    m.focus
                        .as_ref()
                        .map(|f| format!(" - focus: {f}"))
                        .unwrap_or_default()
                )
            })
            .collect();

        let topology_guidance = match &team.topology {
            TeamTopology::Sequential => {
                "Each subtask should depend on the previous one. Order matters."
            }
            TeamTopology::Parallel => "Subtasks should be independent and can run simultaneously.",
            TeamTopology::Hub => {
                "The lead agent coordinates. Create a main task for the lead \
                 and delegate specialized subtasks to members."
            }
            TeamTopology::Adversarial => {
                "Create analysis subtasks for each agent, then add review/challenge \
                 subtasks where agents examine each other's work."
            }
        };

        let prompt = format!(
            "Decompose this task into subtasks for the team.\n\n\
             Task: {task}\n\n\
             Team members: {members}\n\
             Topology: {topology} - {guidance}\n\n\
             Respond with JSON: {{\"subtasks\": [\
             {{\"agent_id\": \"...\", \"description\": \"...\", \"input\": \"...\", \
             \"depends_on\": [], \"priority\": 0}}], \
             \"execution_order\": [0, 1, ...]}}",
            members = members.join(", "),
            topology = team.topology,
            guidance = topology_guidance,
        );

        let messages = vec![Message::user(prompt)];
        let config = CompletionConfig::new()
            .with_system_prompt("You are a task decomposition expert. Output valid JSON only.");

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let subtasks: Vec<Subtask> = json
            .get("subtasks")
            .and_then(|s| serde_json::from_value(s.clone()).ok())
            .unwrap_or_default();

        let execution_order: Vec<usize> = json
            .get("execution_order")
            .and_then(|o| serde_json::from_value(o.clone()).ok())
            .unwrap_or_else(|| (0..subtasks.len()).collect());

        Ok(DecompositionResult {
            original_task: task.to_string(),
            subtasks,
            execution_order,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::agents::team::{TeamMember, TeamRole};
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

    fn test_team() -> AgentTeam {
        AgentTeam::new(
            "test-team",
            "Test Team",
            "A test team",
            TeamTopology::Hub,
            vec![
                TeamMember::new("analyst", TeamRole::Lead),
                TeamMember::new("explorer", TeamRole::Member),
            ],
        )
    }

    #[test]
    fn test_subtask_serialize() {
        let subtask = Subtask {
            agent_id: "analyst".to_string(),
            description: "Analyze code".to_string(),
            input: "Review main.rs".to_string(),
            depends_on: vec![],
            priority: 0,
        };
        let json = serde_json::to_string(&subtask).unwrap();
        assert!(json.contains("\"agent_id\":\"analyst\""));
    }

    #[test]
    fn test_decomposition_result_serialize() {
        let result = DecompositionResult {
            original_task: "Review code".to_string(),
            subtasks: vec![Subtask {
                agent_id: "analyst".to_string(),
                description: "Analyze".to_string(),
                input: "code".to_string(),
                depends_on: vec![],
                priority: 0,
            }],
            execution_order: vec![0],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"original_task\""));
    }

    #[tokio::test]
    async fn test_decompose_task() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"subtasks": [
                    {"agent_id": "analyst", "description": "Analyze code structure", "input": "code", "depends_on": [], "priority": 0},
                    {"agent_id": "explorer", "description": "Explore alternatives", "input": "options", "depends_on": [0], "priority": 1}
                ], "execution_order": [0, 1]}"#,
                Usage::new(200, 100),
            ))
        });

        let decomposer = TaskDecomposer::new(mock_client);
        let team = test_team();

        let result = decomposer
            .decompose("Review this code for quality", &team)
            .await
            .unwrap();

        assert_eq!(result.subtasks.len(), 2);
        assert_eq!(result.subtasks[0].agent_id, "analyst");
        assert_eq!(result.subtasks[1].depends_on, vec![0]);
        assert_eq!(result.execution_order, vec![0, 1]);
    }

    #[tokio::test]
    async fn test_decompose_empty_response() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"subtasks": [], "execution_order": []}"#,
                Usage::new(100, 50),
            ))
        });

        let decomposer = TaskDecomposer::new(mock_client);
        let team = test_team();

        let result = decomposer.decompose("task", &team).await.unwrap();
        assert!(result.subtasks.is_empty());
    }

    #[tokio::test]
    async fn test_decompose_all_topologies() {
        for topology in [
            TeamTopology::Sequential,
            TeamTopology::Parallel,
            TeamTopology::Hub,
            TeamTopology::Adversarial,
        ] {
            let mut mock_client = MockAnthropicClientTrait::new();
            mock_client.expect_complete().returning(|_, _| {
                Ok(CompletionResponse::new(
                    r#"{"subtasks": [{"agent_id": "a", "description": "d", "input": "i", "depends_on": [], "priority": 0}], "execution_order": [0]}"#,
                    Usage::new(100, 50),
                ))
            });

            let decomposer = TaskDecomposer::new(mock_client);
            let team = AgentTeam::new(
                "t",
                "T",
                "d",
                topology,
                vec![TeamMember::new("a", TeamRole::Lead)],
            );

            let result = decomposer.decompose("task", &team).await.unwrap();
            assert!(!result.subtasks.is_empty());
        }
    }
}
