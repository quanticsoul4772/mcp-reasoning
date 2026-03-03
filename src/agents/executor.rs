//! Agent executor for LLM-planned tool dispatch.
//!
//! The executor asks the LLM to plan steps from an agent's capabilities,
//! then dispatches each step to the appropriate reasoning mode.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{Agent, AgentStatus};
use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id};
use crate::prompts::{get_prompt_for_mode, ReasoningMode};
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message, StorageTrait, Thought};

/// A single planned step from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
    /// The reasoning mode to invoke.
    pub mode: String,
    /// Operation within the mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Input for this step.
    pub input: String,
    /// Why this step was chosen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// Result of executing a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step index (0-based).
    pub step_index: usize,
    /// Mode used.
    pub mode: String,
    /// Operation used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Whether step succeeded.
    pub success: bool,
    /// Step output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of a complete agent invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationResult {
    /// Agent that was invoked.
    pub agent_id: String,
    /// Session used.
    pub session_id: String,
    /// Planned steps.
    pub planned_steps: Vec<PlannedStep>,
    /// Results from executed steps.
    pub step_results: Vec<StepResult>,
    /// Final synthesis.
    pub synthesis: String,
    /// Overall success.
    pub success: bool,
    /// Agent status.
    pub status: AgentStatus,
}

/// Executes agent tasks by planning steps with the LLM and dispatching to modes.
pub struct AgentExecutor<S: StorageTrait, C: AnthropicClientTrait> {
    storage: S,
    client: C,
}

impl<S: StorageTrait, C: AnthropicClientTrait> AgentExecutor<S, C> {
    /// Create a new agent executor.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Plan steps for a task given an agent's capabilities.
    pub async fn plan_steps(
        &self,
        agent: &Agent,
        task: &str,
        session_id: &str,
    ) -> Result<Vec<PlannedStep>, ModeError> {
        let capabilities: Vec<String> = agent
            .capabilities
            .iter()
            .map(|c| {
                if c.operations.is_empty() {
                    c.mode.clone()
                } else {
                    format!("{} (ops: {})", c.mode, c.operations.join(", "))
                }
            })
            .collect();

        let prompt = format!(
            "You are an agent planner. Given a task and available reasoning tools, \
             plan 1-{max_steps} steps to accomplish the task.\n\n\
             Available tools: {tools}\n\n\
             Task: {task}\n\n\
             Respond with JSON: {{\"steps\": [{{\"mode\": \"...\", \"operation\": \"...\", \
             \"input\": \"...\", \"rationale\": \"...\"}}]}}",
            max_steps = agent.config.max_steps,
            tools = capabilities.join(", "),
            task = task,
        );

        let messages = vec![Message::user(prompt)];
        let config = CompletionConfig::new()
            .with_system_prompt("You are a reasoning agent planner. Output valid JSON only.");

        let response = self.client.complete(messages, config).await?;
        let json = extract_json(&response.content)?;

        let steps: Vec<PlannedStep> = if let Some(steps_arr) = json.get("steps") {
            serde_json::from_value(steps_arr.clone()).map_err(|e| ModeError::JsonParseFailed {
                message: format!("Failed to parse planned steps: {e}"),
            })?
        } else {
            return Err(ModeError::JsonParseFailed {
                message: "Missing 'steps' field in plan".to_string(),
            });
        };

        // Filter to only allowed capabilities
        let filtered: Vec<PlannedStep> = steps
            .into_iter()
            .filter(|s| agent.has_capability(&s.mode))
            .take(agent.config.max_steps)
            .collect();

        tracing::info!(
            agent_id = %agent.id,
            session_id = %session_id,
            step_count = filtered.len(),
            "Agent planned steps"
        );

        Ok(filtered)
    }

    /// Execute a single planned step.
    pub async fn execute_step(
        &self,
        step: &PlannedStep,
        step_index: usize,
        session_id: &str,
    ) -> StepResult {
        tracing::info!(
            mode = %step.mode,
            operation = ?step.operation,
            step_index = step_index,
            "Executing agent step"
        );

        let input = if step.mode == "linear" {
            step.input.clone()
        } else {
            format!(
                "Using {} reasoning{}: {}",
                step.mode,
                step.operation
                    .as_ref()
                    .map(|op| format!(" ({op})"))
                    .unwrap_or_default(),
                step.input
            )
        };

        self.execute_reasoning(&input, session_id, step_index, &step.mode)
            .await
    }

    /// Execute a reasoning step using mode-specific prompts and session tracking.
    async fn execute_reasoning(
        &self,
        input: &str,
        session_id: &str,
        step_index: usize,
        mode_name: &str,
    ) -> StepResult {
        // Resolve mode-specific prompt if available
        let mode_prompt = mode_name
            .parse::<ReasoningMode>()
            .ok()
            .map(|mode| get_prompt_for_mode(mode, None).to_string());

        let system_prompt = mode_prompt.unwrap_or_else(|| {
            format!(
                "You are a {mode_name} reasoning engine. Analyze the input and provide structured \
                 output as JSON with 'analysis' (string), 'confidence' (0.0-1.0), and 'next_step' \
                 (string or null) fields."
            )
        });

        // Apply thinking budget if mode supports it
        let mut config = CompletionConfig::new().with_system_prompt(system_prompt);
        if let Ok(mode) = mode_name.parse::<ReasoningMode>() {
            if let Some(budget) = mode.thinking_budget() {
                config = config.with_thinking_budget(budget);
            }
        }

        let user_message = format!("Content to analyze:\n{input}");
        let messages = vec![Message::user(user_message)];

        match self.client.complete(messages, config).await {
            Ok(resp) => {
                let output = extract_json(&resp.content)
                    .unwrap_or_else(|_| serde_json::json!({"content": resp.content}));

                // Save thought to storage for session tracking
                let confidence = output
                    .get("confidence")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.5);
                let analysis = output
                    .get("analysis")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&resp.content);

                let thought_id = generate_thought_id();
                let thought =
                    Thought::new(&thought_id, session_id, analysis, mode_name, confidence);
                if let Err(e) = self.storage.save_thought(&thought).await {
                    tracing::warn!(error = %e, "Failed to save agent step thought");
                }

                StepResult {
                    step_index,
                    mode: mode_name.to_string(),
                    operation: None,
                    success: true,
                    output: Some(output),
                    error: None,
                }
            }
            Err(e) => StepResult {
                step_index,
                mode: mode_name.to_string(),
                operation: None,
                success: false,
                output: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// Invoke an agent on a task: plan, execute, synthesize.
    pub async fn invoke(
        &self,
        agent: &Agent,
        task: &str,
        session_id: &str,
    ) -> Result<InvocationResult, ModeError> {
        // 1. Plan steps
        let steps = self.plan_steps(agent, task, session_id).await?;

        if steps.is_empty() {
            return Ok(InvocationResult {
                agent_id: agent.id.clone(),
                session_id: session_id.to_string(),
                planned_steps: vec![],
                step_results: vec![],
                synthesis: "No steps planned for this task.".to_string(),
                success: false,
                status: AgentStatus::Failed,
            });
        }

        // 2. Execute steps
        let mut step_results = Vec::with_capacity(steps.len());
        for (i, step) in steps.iter().enumerate() {
            let result = self.execute_step(step, i, session_id).await;
            let failed = !result.success;
            step_results.push(result);

            if failed {
                tracing::warn!(
                    agent_id = %agent.id,
                    step_index = i,
                    "Agent step failed, stopping execution"
                );
                break;
            }
        }

        // 3. Synthesize results
        let all_success = step_results.iter().all(|r| r.success);
        let synthesis = self
            .synthesize(agent, task, &step_results)
            .await
            .unwrap_or_else(|e| format!("Synthesis failed: {e}"));

        Ok(InvocationResult {
            agent_id: agent.id.clone(),
            session_id: session_id.to_string(),
            planned_steps: steps,
            step_results,
            synthesis,
            success: all_success,
            status: if all_success {
                AgentStatus::Completed
            } else {
                AgentStatus::Failed
            },
        })
    }

    /// Complete a prompt using the underlying LLM client.
    ///
    /// This is exposed for use by the team coordinator for synthesis.
    pub async fn complete_prompt(&self, prompt: &str) -> Result<String, ModeError> {
        let messages = vec![Message::user(prompt.to_string())];
        let config = CompletionConfig::new();
        let response = self.client.complete(messages, config).await?;
        Ok(response.content)
    }

    /// Synthesize step results into a final answer.
    async fn synthesize(
        &self,
        agent: &Agent,
        task: &str,
        results: &[StepResult],
    ) -> Result<String, ModeError> {
        let results_summary: Vec<String> = results
            .iter()
            .map(|r| {
                if r.success {
                    format!(
                        "Step {}: {} - {}",
                        r.step_index,
                        r.mode,
                        r.output
                            .as_ref()
                            .and_then(|o| o.get("content").and_then(|c| c.as_str()))
                            .unwrap_or("(no content)")
                    )
                } else {
                    format!(
                        "Step {}: {} - FAILED: {}",
                        r.step_index,
                        r.mode,
                        r.error.as_deref().unwrap_or("unknown error")
                    )
                }
            })
            .collect();

        let prompt = format!(
            "You are the {agent_name} agent. Synthesize the results of your analysis.\n\n\
             Original task: {task}\n\n\
             Step results:\n{results}\n\n\
             Provide a concise synthesis of findings.",
            agent_name = agent.name,
            task = task,
            results = results_summary.join("\n"),
        );

        let messages = vec![Message::user(prompt)];
        let config = CompletionConfig::new();
        let response = self.client.complete(messages, config).await?;

        Ok(response.content)
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
    use crate::agents::types::{AgentCapability, AgentRole};
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    fn mock_agent() -> Agent {
        Agent::new(
            "test-analyst",
            "Test Analyst",
            AgentRole::Analyst,
            "A test analyst agent",
            vec![
                AgentCapability::mode("linear"),
                AgentCapability::mode("tree"),
            ],
        )
    }

    fn plan_response() -> CompletionResponse {
        CompletionResponse::new(
            r#"{"steps": [{"mode": "linear", "input": "Analyze the code", "rationale": "Start with analysis"}]}"#,
            Usage::new(100, 50),
        )
    }

    fn linear_response() -> CompletionResponse {
        CompletionResponse::new(
            r#"{"analysis": "The code looks good", "confidence": 0.85, "next_step": "Review tests"}"#,
            Usage::new(100, 50),
        )
    }

    fn synthesis_response() -> CompletionResponse {
        CompletionResponse::new(
            "Overall the analysis shows good code quality.",
            Usage::new(100, 50),
        )
    }

    #[test]
    fn test_planned_step_serialize() {
        let step = PlannedStep {
            mode: "linear".to_string(),
            operation: None,
            input: "test".to_string(),
            rationale: Some("reason".to_string()),
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"mode\":\"linear\""));
    }

    #[test]
    fn test_step_result_success() {
        let result = StepResult {
            step_index: 0,
            mode: "linear".to_string(),
            operation: None,
            success: true,
            output: Some(serde_json::json!({"content": "test"})),
            error: None,
        };
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_step_result_failure() {
        let result = StepResult {
            step_index: 1,
            mode: "tree".to_string(),
            operation: Some("create".to_string()),
            success: false,
            output: None,
            error: Some("API error".to_string()),
        };
        assert!(!result.success);
        assert_eq!(result.error, Some("API error".to_string()));
    }

    #[test]
    fn test_invocation_result_serialize() {
        let result = InvocationResult {
            agent_id: "test".to_string(),
            session_id: "s1".to_string(),
            planned_steps: vec![],
            step_results: vec![],
            synthesis: "Done".to_string(),
            success: true,
            status: AgentStatus::Completed,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"agent_id\":\"test\""));
        assert!(json.contains("\"completed\""));
    }

    #[tokio::test]
    async fn test_plan_steps() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client
            .expect_complete()
            .returning(|_, _| Ok(plan_response()));

        let mock_storage = MockStorageTrait::new();
        let executor = AgentExecutor::new(mock_storage, mock_client);
        let agent = mock_agent();

        let steps = executor
            .plan_steps(&agent, "Review this code", "session-1")
            .await
            .unwrap();

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].mode, "linear");
    }

    #[tokio::test]
    async fn test_plan_steps_filters_capabilities() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"steps": [
                    {"mode": "linear", "input": "step1"},
                    {"mode": "graph", "input": "step2"},
                    {"mode": "tree", "input": "step3"}
                ]}"#,
                Usage::new(100, 50),
            ))
        });

        let mock_storage = MockStorageTrait::new();
        let executor = AgentExecutor::new(mock_storage, mock_client);
        let agent = mock_agent(); // only has linear + tree

        let steps = executor.plan_steps(&agent, "task", "s1").await.unwrap();

        // graph should be filtered out
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].mode, "linear");
        assert_eq!(steps[1].mode, "tree");
    }

    #[tokio::test]
    async fn test_plan_steps_missing_steps_field() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"plan": "bad format"}"#,
                Usage::new(100, 50),
            ))
        });

        let mock_storage = MockStorageTrait::new();
        let executor = AgentExecutor::new(mock_storage, mock_client);
        let agent = mock_agent();

        let result = executor.plan_steps(&agent, "task", "s1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invoke_empty_plan() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"steps": []}"#,
                Usage::new(100, 50),
            ))
        });

        let mock_storage = MockStorageTrait::new();
        let executor = AgentExecutor::new(mock_storage, mock_client);
        let agent = mock_agent();

        let result = executor.invoke(&agent, "task", "s1").await.unwrap();
        assert!(!result.success);
        assert_eq!(result.status, AgentStatus::Failed);
    }

    #[tokio::test]
    async fn test_invoke_success() {
        let mut mock_client = MockAnthropicClientTrait::new();

        // Call 1: plan_steps
        // Call 2: linear mode process
        // Call 3: synthesize
        let mut call_count = 0u32;
        mock_client.expect_complete().returning(move |_, _| {
            call_count += 1;
            match call_count {
                1 => Ok(plan_response()),
                2 => Ok(linear_response()),
                _ => Ok(synthesis_response()),
            }
        });

        let mut mock_storage = MockStorageTrait::new();
        mock_storage.expect_get_or_create_session().returning(|id| {
            use crate::traits::Session;
            Ok(Session::new(id.unwrap_or_else(|| "s1".to_string())))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let executor = AgentExecutor::new(mock_storage, mock_client);
        let agent = mock_agent();

        let result = executor.invoke(&agent, "Review code", "s1").await.unwrap();
        assert_eq!(result.agent_id, "test-analyst");
        assert!(!result.planned_steps.is_empty());
        assert!(!result.synthesis.is_empty());
    }

    #[tokio::test]
    async fn test_execute_step_non_linear_mode() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client
            .expect_complete()
            .returning(|_, _| Ok(linear_response()));

        let mut mock_storage = MockStorageTrait::new();
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let executor = AgentExecutor::new(mock_storage, mock_client);

        let step = PlannedStep {
            mode: "tree".to_string(),
            operation: Some("create".to_string()),
            input: "Explore branches".to_string(),
            rationale: Some("Need branching".to_string()),
        };

        let result = executor.execute_step(&step, 0, "s1").await;
        assert!(result.success);
        assert_eq!(result.mode, "tree");
        assert!(result.output.is_some());
    }

    #[tokio::test]
    async fn test_execute_step_failure() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiError {
                message: "API failure".to_string(),
            })
        });

        let mock_storage = MockStorageTrait::new();
        let executor = AgentExecutor::new(mock_storage, mock_client);

        let step = PlannedStep {
            mode: "linear".to_string(),
            operation: None,
            input: "test".to_string(),
            rationale: None,
        };

        let result = executor.execute_step(&step, 0, "s1").await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_invoke_with_step_failure() {
        let mut mock_client = MockAnthropicClientTrait::new();

        // Call 1: plan_steps succeeds
        // Call 2: execute_step fails
        // Call 3: synthesize succeeds
        let mut call_count = 0u32;
        mock_client.expect_complete().returning(move |_, _| {
            call_count += 1;
            match call_count {
                1 => Ok(plan_response()),
                2 => Err(ModeError::ApiError {
                    message: "step failed".to_string(),
                }),
                _ => Ok(synthesis_response()),
            }
        });

        let mut mock_storage = MockStorageTrait::new();
        mock_storage.expect_get_or_create_session().returning(|id| {
            use crate::traits::Session;
            Ok(Session::new(id.unwrap_or_else(|| "s1".to_string())))
        });
        mock_storage.expect_save_thought().returning(|_| Ok(()));
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let executor = AgentExecutor::new(mock_storage, mock_client);
        let agent = mock_agent();

        let result = executor.invoke(&agent, "Failing task", "s1").await.unwrap();
        assert!(!result.success);
        assert_eq!(result.status, AgentStatus::Failed);
        // Should stop after first failing step
        assert_eq!(result.step_results.len(), 1);
    }

    #[tokio::test]
    async fn test_complete_prompt() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                "Prompt completed successfully.",
                Usage::new(100, 50),
            ))
        });

        let mock_storage = MockStorageTrait::new();
        let executor = AgentExecutor::new(mock_storage, mock_client);

        let result = executor.complete_prompt("Test prompt").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("successfully"));
    }

    #[tokio::test]
    async fn test_synthesize_with_mixed_results() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                "Synthesis of mixed results: partial success noted.",
                Usage::new(100, 50),
            ))
        });

        let mock_storage = MockStorageTrait::new();
        let executor = AgentExecutor::new(mock_storage, mock_client);
        let agent = mock_agent();

        let results = vec![
            StepResult {
                step_index: 0,
                mode: "linear".to_string(),
                operation: None,
                success: true,
                output: Some(serde_json::json!({"content": "Analysis done"})),
                error: None,
            },
            StepResult {
                step_index: 1,
                mode: "tree".to_string(),
                operation: Some("create".to_string()),
                success: false,
                output: None,
                error: Some("timeout".to_string()),
            },
        ];

        let synthesis = executor.synthesize(&agent, "Complex task", &results).await;
        assert!(synthesis.is_ok());
        assert!(synthesis.unwrap().contains("partial success"));
    }

    #[tokio::test]
    async fn test_execute_reasoning_saves_thought() {
        let mut mock_client = MockAnthropicClientTrait::new();
        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"analysis": "Deep analysis", "confidence": 0.92}"#,
                Usage::new(100, 50),
            ))
        });

        let mut mock_storage = MockStorageTrait::new();
        mock_storage
            .expect_save_thought()
            .times(1)
            .returning(|thought| {
                assert_eq!(thought.mode, "linear");
                assert!((thought.confidence - 0.92).abs() < 0.01);
                Ok(())
            });
        mock_storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let executor = AgentExecutor::new(mock_storage, mock_client);

        let step = PlannedStep {
            mode: "linear".to_string(),
            operation: None,
            input: "test".to_string(),
            rationale: None,
        };

        let result = executor.execute_step(&step, 0, "s1").await;
        assert!(result.success);
    }
}
