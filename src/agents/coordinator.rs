//! Team coordinator for executing decomposed tasks across agents.
//!
//! Manages execution order, dependency resolution, and result synthesis
//! across multiple agents in a team.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::decomposer::{DecompositionResult, Subtask};
use super::executor::AgentExecutor;
use super::registry::AgentRegistry;
use super::team::AgentTeam;
use crate::error::ModeError;
use crate::traits::{AnthropicClientTrait, StorageTrait};

/// Result from a team execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResult {
    /// Team that executed.
    pub team_id: String,
    /// Session used.
    pub session_id: String,
    /// Original task.
    pub task: String,
    /// Results from each subtask.
    pub subtask_results: Vec<SubtaskResult>,
    /// Final synthesis across all agents.
    pub synthesis: String,
    /// Overall success.
    pub success: bool,
}

/// Result from executing a single subtask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskResult {
    /// Subtask index.
    pub index: usize,
    /// Agent that executed it.
    pub agent_id: String,
    /// Description of what was done.
    pub description: String,
    /// Whether it succeeded.
    pub success: bool,
    /// Output content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Error if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Coordinates team execution of decomposed tasks.
pub struct TeamCoordinator<S: StorageTrait, C: AnthropicClientTrait> {
    executor: AgentExecutor<S, C>,
    registry: Arc<AgentRegistry>,
}

impl<S: StorageTrait, C: AnthropicClientTrait> TeamCoordinator<S, C> {
    /// Create a new team coordinator.
    #[must_use]
    pub fn new(storage: S, client: C, registry: Arc<AgentRegistry>) -> Self {
        Self {
            executor: AgentExecutor::new(storage, client),
            registry,
        }
    }

    /// Execute a decomposed plan.
    pub async fn execute(
        &self,
        team: &AgentTeam,
        decomposition: &DecompositionResult,
        session_id: &str,
    ) -> Result<TeamResult, ModeError> {
        let mut subtask_results = Vec::new();
        let mut completed: HashMap<usize, String> = HashMap::new();

        for &idx in &decomposition.execution_order {
            let subtask = match decomposition.subtasks.get(idx) {
                Some(s) => s,
                None => continue,
            };

            // Check dependencies
            let deps_met = subtask.depends_on.iter().all(|d| completed.contains_key(d));
            if !deps_met {
                subtask_results.push(SubtaskResult {
                    index: idx,
                    agent_id: subtask.agent_id.clone(),
                    description: subtask.description.clone(),
                    success: false,
                    output: None,
                    error: Some("Dependencies not met".to_string()),
                });
                continue;
            }

            let result = self
                .execute_subtask(subtask, idx, session_id, &completed)
                .await;

            if result.success {
                if let Some(ref output) = result.output {
                    completed.insert(idx, output.clone());
                }
            }

            subtask_results.push(result);
        }

        let all_success = subtask_results.iter().all(|r| r.success);
        let synthesis = self
            .synthesize_team_results(team, &decomposition.original_task, &subtask_results)
            .await
            .unwrap_or_else(|e| format!("Team synthesis failed: {e}"));

        Ok(TeamResult {
            team_id: team.id.clone(),
            session_id: session_id.to_string(),
            task: decomposition.original_task.clone(),
            subtask_results,
            synthesis,
            success: all_success,
        })
    }

    /// Execute a single subtask using the assigned agent.
    async fn execute_subtask(
        &self,
        subtask: &Subtask,
        index: usize,
        session_id: &str,
        _completed: &HashMap<usize, String>,
    ) -> SubtaskResult {
        let agent = match self.registry.get(&subtask.agent_id) {
            Some(a) => a,
            None => {
                return SubtaskResult {
                    index,
                    agent_id: subtask.agent_id.clone(),
                    description: subtask.description.clone(),
                    success: false,
                    output: None,
                    error: Some(format!("Agent '{}' not found", subtask.agent_id)),
                };
            }
        };

        match self
            .executor
            .invoke(agent, &subtask.input, session_id)
            .await
        {
            Ok(result) => SubtaskResult {
                index,
                agent_id: subtask.agent_id.clone(),
                description: subtask.description.clone(),
                success: result.success,
                output: Some(result.synthesis),
                error: None,
            },
            Err(e) => SubtaskResult {
                index,
                agent_id: subtask.agent_id.clone(),
                description: subtask.description.clone(),
                success: false,
                output: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// Synthesize results from all subtasks.
    async fn synthesize_team_results(
        &self,
        team: &AgentTeam,
        task: &str,
        results: &[SubtaskResult],
    ) -> Result<String, ModeError> {
        let summaries: Vec<String> = results
            .iter()
            .map(|r| {
                if r.success {
                    format!(
                        "[{}] {}: {}",
                        r.agent_id,
                        r.description,
                        r.output.as_deref().unwrap_or("(no output)")
                    )
                } else {
                    format!(
                        "[{}] {}: FAILED - {}",
                        r.agent_id,
                        r.description,
                        r.error.as_deref().unwrap_or("unknown")
                    )
                }
            })
            .collect();

        let prompt = format!(
            "Synthesize the results from team '{team_name}' ({topology}).\n\n\
             Original task: {task}\n\n\
             Agent results:\n{results}\n\n\
             Provide a unified synthesis.",
            team_name = team.name,
            topology = team.topology,
            results = summaries.join("\n"),
        );

        self.executor.complete_prompt(&prompt).await
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;
    use crate::agents::team::{TeamMember, TeamRole, TeamTopology};
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    #[test]
    fn test_team_result_serialize() {
        let result = TeamResult {
            team_id: "t1".to_string(),
            session_id: "s1".to_string(),
            task: "Review code".to_string(),
            subtask_results: vec![],
            synthesis: "Done".to_string(),
            success: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"team_id\":\"t1\""));
    }

    #[test]
    fn test_subtask_result_serialize() {
        let result = SubtaskResult {
            index: 0,
            agent_id: "analyst".to_string(),
            description: "Analyze code".to_string(),
            success: true,
            output: Some("Looks good".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"analyst\""));
    }

    #[tokio::test]
    async fn test_execute_team() {
        let mut mock_client = MockAnthropicClientTrait::new();
        // Calls: plan(analyst) + execute(analyst) + synthesize(analyst) + team_synthesize
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"steps": [{"mode": "linear", "input": "analyze"}], "analysis": "good", "confidence": 0.9, "next_step": null}"#,
                Usage::new(100, 50),
            ))
        });

        let mut mock_storage = MockStorageTrait::new();
        mock_storage.expect_get_or_create_session().returning(|id| {
            use crate::traits::Session;
            Ok(Session::new(id.unwrap_or_else(|| "s1".to_string())))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let registry = Arc::new(AgentRegistry::new());
        let coordinator = TeamCoordinator::new(mock_storage, mock_client, registry);

        let team = AgentTeam::new(
            "test",
            "Test",
            "test team",
            TeamTopology::Sequential,
            vec![TeamMember::new("analyst", TeamRole::Lead)],
        );

        let decomposition = DecompositionResult {
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

        let result = coordinator
            .execute(&team, &decomposition, "s1")
            .await
            .unwrap();

        assert_eq!(result.team_id, "test");
        assert!(!result.subtask_results.is_empty());
    }

    #[tokio::test]
    async fn test_execute_unknown_agent() {
        let mut mock_client = MockAnthropicClientTrait::new();
        // Expect synthesis call after subtask failures
        mock_client
            .expect_complete()
            .returning(|_, _| Ok(CompletionResponse::new("Synthesis", Usage::new(50, 25))));
        let mock_storage = MockStorageTrait::new();
        let registry = Arc::new(AgentRegistry::new());

        let coordinator = TeamCoordinator::new(mock_storage, mock_client, registry);

        let team = AgentTeam::new(
            "test",
            "Test",
            "test",
            TeamTopology::Sequential,
            vec![TeamMember::new("nonexistent", TeamRole::Lead)],
        );

        let decomposition = DecompositionResult {
            original_task: "task".to_string(),
            subtasks: vec![Subtask {
                agent_id: "nonexistent".to_string(),
                description: "d".to_string(),
                input: "i".to_string(),
                depends_on: vec![],
                priority: 0,
            }],
            execution_order: vec![0],
        };

        let result = coordinator
            .execute(&team, &decomposition, "s1")
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.subtask_results[0]
            .error
            .as_ref()
            .unwrap()
            .contains("not found"));
    }

    #[tokio::test]
    async fn test_execute_unmet_dependencies() {
        let mut mock_client = MockAnthropicClientTrait::new();
        // Expect synthesis call after dependency failures
        mock_client
            .expect_complete()
            .returning(|_, _| Ok(CompletionResponse::new("Synthesis", Usage::new(50, 25))));
        let mock_storage = MockStorageTrait::new();
        let registry = Arc::new(AgentRegistry::new());

        let coordinator = TeamCoordinator::new(mock_storage, mock_client, registry);

        let team = AgentTeam::new(
            "test",
            "Test",
            "test",
            TeamTopology::Sequential,
            vec![TeamMember::new("analyst", TeamRole::Lead)],
        );

        let decomposition = DecompositionResult {
            original_task: "task".to_string(),
            subtasks: vec![Subtask {
                agent_id: "analyst".to_string(),
                description: "d".to_string(),
                input: "i".to_string(),
                depends_on: vec![99], // Non-existent dependency
                priority: 0,
            }],
            execution_order: vec![0],
        };

        let result = coordinator
            .execute(&team, &decomposition, "s1")
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.subtask_results[0]
            .error
            .as_ref()
            .unwrap()
            .contains("Dependencies not met"));
    }
}
