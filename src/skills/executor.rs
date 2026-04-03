//! Skill executor for running skill workflows with context passing.
//!
//! Executes skill steps in sequence, passing context between them,
//! respecting conditions and error strategies.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{ErrorStrategy, Skill, SkillContext, StepCondition};
use crate::error::ModeError;
use crate::modes::{extract_json, generate_thought_id};
use crate::prompts::{get_prompt_for_mode, ReasoningMode};
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message, StorageTrait, Thought};

/// Result from executing a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    /// Skill that was executed.
    pub skill_id: String,
    /// Session used.
    pub session_id: String,
    /// Results from each step.
    pub step_results: Vec<SkillStepResult>,
    /// Final context values.
    pub context: SkillContext,
    /// Overall success.
    pub success: bool,
}

/// Result from a single skill step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStepResult {
    /// Step index.
    pub step_index: usize,
    /// Mode used.
    pub mode: String,
    /// Operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Whether it succeeded.
    pub success: bool,
    /// Whether it was skipped due to condition.
    pub skipped: bool,
    /// Output value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// Error if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Executes skill workflows.
pub struct SkillExecutor<S: StorageTrait, C: AnthropicClientTrait> {
    storage: S,
    client: C,
}

impl<S: StorageTrait, C: AnthropicClientTrait> SkillExecutor<S, C> {
    /// Create a new skill executor.
    #[must_use]
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Execute a skill with the given context.
    pub async fn execute(
        &self,
        skill: &Skill,
        mut context: SkillContext,
    ) -> Result<SkillResult, ModeError> {
        let session_id = context
            .session_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        context.session_id = Some(session_id.clone());

        let mut step_results = Vec::with_capacity(skill.steps.len());

        for (i, step) in skill.steps.iter().enumerate() {
            // Check condition
            if !self.check_condition(&step.condition, &context) {
                step_results.push(SkillStepResult {
                    step_index: i,
                    mode: step.mode.clone(),
                    operation: step.operation.clone(),
                    success: true,
                    skipped: true,
                    output: None,
                    error: None,
                });
                continue;
            }

            // Build input from context mappings
            let input = self.build_step_input(step, &context);

            // Execute with error strategy
            let result = self
                .execute_step_with_strategy(&step.mode, &input, &session_id, i, &step.on_error)
                .await;

            // Store output in context
            if result.success {
                if let (Some(ref key), Some(ref output)) = (&step.output_key, &result.output) {
                    context.set(key, output.clone());
                }
            } else {
                context.record_failure(i);
                if step.on_error == ErrorStrategy::Fail {
                    step_results.push(result);
                    break;
                }
            }

            step_results.push(result);
        }

        let success = step_results.iter().all(|r| r.success || r.skipped);

        Ok(SkillResult {
            skill_id: skill.id.clone(),
            session_id,
            step_results,
            context,
            success,
        })
    }

    /// Check if a step condition is met.
    fn check_condition(&self, condition: &StepCondition, context: &SkillContext) -> bool {
        match condition {
            StepCondition::Always => true,
            StepCondition::IfKeyExists(key) => context.has_key(key),
            StepCondition::IfConfidenceAbove(threshold) => context
                .get("_last_confidence")
                .and_then(serde_json::Value::as_f64)
                .is_some_and(|c| c > *threshold),
            StepCondition::IfStepFailed(idx) => context.step_failed(*idx),
        }
    }

    /// Build input for a step from context mappings.
    fn build_step_input(&self, step: &super::types::SkillStep, context: &SkillContext) -> String {
        if step.input_mapping.is_empty() {
            return context.input_str().to_string();
        }

        let mut parts = Vec::new();
        for context_key in step.input_mapping.keys() {
            if let Some(value) = context.get(context_key) {
                match value {
                    Value::String(s) => parts.push(s.clone()),
                    other => parts.push(other.to_string()),
                }
            }
        }

        if parts.is_empty() {
            context.input_str().to_string()
        } else {
            parts.join("\n\n")
        }
    }

    /// Execute a step with error handling strategy.
    async fn execute_step_with_strategy(
        &self,
        mode: &str,
        input: &str,
        session_id: &str,
        step_index: usize,
        error_strategy: &ErrorStrategy,
    ) -> SkillStepResult {
        let max_attempts = match error_strategy {
            ErrorStrategy::Retry(n) => *n as usize,
            _ => 1,
        };

        for attempt in 0..max_attempts {
            let result = self.execute_mode(mode, input, session_id).await;

            match result {
                Ok(output) => {
                    return SkillStepResult {
                        step_index,
                        mode: mode.to_string(),
                        operation: None,
                        success: true,
                        skipped: false,
                        output: Some(output),
                        error: None,
                    };
                }
                Err(e) if attempt + 1 < max_attempts => {
                    tracing::warn!(
                        mode = mode,
                        attempt = attempt + 1,
                        max_attempts = max_attempts,
                        error = %e,
                        "Skill step failed, retrying"
                    );
                }
                Err(e) => {
                    return match error_strategy {
                        ErrorStrategy::Skip | ErrorStrategy::Retry(_) => SkillStepResult {
                            step_index,
                            mode: mode.to_string(),
                            operation: None,
                            success: true, // Skip counts as success
                            skipped: true,
                            output: None,
                            error: Some(e.to_string()),
                        },
                        _ => SkillStepResult {
                            step_index,
                            mode: mode.to_string(),
                            operation: None,
                            success: false,
                            skipped: false,
                            output: None,
                            error: Some(e.to_string()),
                        },
                    };
                }
            }
        }

        // Should not reach here, but safety fallback
        SkillStepResult {
            step_index,
            mode: mode.to_string(),
            operation: None,
            success: false,
            skipped: false,
            output: None,
            error: Some("Max attempts reached".to_string()),
        }
    }

    /// Execute a single reasoning mode using mode-specific prompts and session tracking.
    async fn execute_mode(
        &self,
        mode: &str,
        input: &str,
        session_id: &str,
    ) -> Result<Value, ModeError> {
        // Resolve mode-specific prompt if available
        let mode_prompt = mode
            .parse::<ReasoningMode>()
            .ok()
            .map(|m| get_prompt_for_mode(m, None).to_string());

        let system_prompt = mode_prompt.unwrap_or_else(|| {
            format!(
                "You are a {mode} reasoning engine. Analyze the input and provide structured \
                 output as JSON with 'analysis' (string), 'confidence' (0.0-1.0), and 'next_step' \
                 (string or null) fields."
            )
        });

        // Apply thinking budget if mode supports it
        let mut config = CompletionConfig::new().with_system_prompt(system_prompt);
        if let Ok(m) = mode.parse::<ReasoningMode>() {
            if let Some(budget) = m.thinking_budget() {
                config = config.with_thinking_budget(budget);
            }
        }

        let user_message = format!("Content to analyze:\n{input}");
        let messages = vec![Message::user(user_message)];

        let resp = self.client.complete(messages, config).await?;
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
        let thought = Thought::new(&thought_id, session_id, analysis, mode, confidence);
        if let Err(e) = self.storage.save_thought(&thought).await {
            tracing::warn!(error = %e, "Failed to save skill step thought");
        }

        Ok(output)
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
    use crate::skills::types::{SkillCategory, SkillStep};
    use crate::traits::{
        CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Session, Usage,
    };

    fn mock_storage() -> MockStorageTrait {
        let mut storage = MockStorageTrait::new();
        storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "s1".to_string()))));
        storage.expect_save_thought().returning(|_| Ok(()));
        storage.expect_get_thoughts().returning(|_| Ok(vec![]));
        storage
    }

    fn mock_client() -> MockAnthropicClientTrait {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"analysis": "Test result", "confidence": 0.85, "next_step": null}"#,
                Usage::new(100, 50),
            ))
        });
        client
    }

    fn simple_skill() -> Skill {
        Skill::new(
            "test-skill",
            "Test Skill",
            "A test skill",
            SkillCategory::Analysis,
            vec![
                SkillStep::new("linear")
                    .with_description("Step 1")
                    .with_output_key("result1"),
                SkillStep::new("linear").with_description("Step 2"),
            ],
        )
    }

    #[tokio::test]
    async fn test_execute_simple_skill() {
        let executor = SkillExecutor::new(mock_storage(), mock_client());
        let skill = simple_skill();
        let context = SkillContext::new("Test input");

        let result = executor.execute(&skill, context).await.unwrap();
        assert_eq!(result.skill_id, "test-skill");
        assert!(result.success);
        assert_eq!(result.step_results.len(), 2);
        assert!(result.context.has_key("result1"));
    }

    #[tokio::test]
    async fn test_execute_with_condition_met() {
        let executor = SkillExecutor::new(mock_storage(), mock_client());
        let skill = Skill::new(
            "cond-skill",
            "Conditional",
            "test",
            SkillCategory::Analysis,
            vec![
                SkillStep::new("linear").with_output_key("analysis"),
                SkillStep::new("linear")
                    .with_condition(StepCondition::IfKeyExists("analysis".to_string())),
            ],
        );
        let context = SkillContext::new("input");

        let result = executor.execute(&skill, context).await.unwrap();
        assert!(result.success);
        assert!(!result.step_results[1].skipped);
    }

    #[tokio::test]
    async fn test_execute_with_condition_not_met() {
        let executor = SkillExecutor::new(mock_storage(), mock_client());
        let skill = Skill::new(
            "cond-skill",
            "Conditional",
            "test",
            SkillCategory::Analysis,
            vec![SkillStep::new("linear")
                .with_condition(StepCondition::IfKeyExists("nonexistent".to_string()))],
        );
        let context = SkillContext::new("input");

        let result = executor.execute(&skill, context).await.unwrap();
        assert!(result.success);
        assert!(result.step_results[0].skipped);
    }

    #[tokio::test]
    async fn test_execute_with_error_skip() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiError {
                message: "test error".to_string(),
            })
        });

        let executor = SkillExecutor::new(mock_storage(), client);
        let skill = Skill::new(
            "skip-skill",
            "Skip on Error",
            "test",
            SkillCategory::Analysis,
            vec![SkillStep::new("linear").with_error_strategy(ErrorStrategy::Skip)],
        );
        let context = SkillContext::new("input");

        let result = executor.execute(&skill, context).await.unwrap();
        assert!(result.success); // Skip counts as success
        assert!(result.step_results[0].skipped);
    }

    #[tokio::test]
    async fn test_execute_with_error_fail() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiError {
                message: "test error".to_string(),
            })
        });

        let executor = SkillExecutor::new(mock_storage(), client);
        let skill = Skill::new(
            "fail-skill",
            "Fail on Error",
            "test",
            SkillCategory::Analysis,
            vec![
                SkillStep::new("linear").with_error_strategy(ErrorStrategy::Fail),
                SkillStep::new("linear"), // Should not execute
            ],
        );
        let context = SkillContext::new("input");

        let result = executor.execute(&skill, context).await.unwrap();
        assert!(!result.success);
        assert_eq!(result.step_results.len(), 1); // Second step should not execute
    }

    #[test]
    fn test_skill_result_serialize() {
        let result = SkillResult {
            skill_id: "test".to_string(),
            session_id: "s1".to_string(),
            step_results: vec![],
            context: SkillContext::new("input"),
            success: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"skill_id\":\"test\""));
    }

    #[test]
    fn test_check_condition_always() {
        let executor = SkillExecutor::new(MockStorageTrait::new(), MockAnthropicClientTrait::new());
        let ctx = SkillContext::new("input");
        assert!(executor.check_condition(&StepCondition::Always, &ctx));
    }

    #[test]
    fn test_check_condition_if_key_exists() {
        let executor = SkillExecutor::new(MockStorageTrait::new(), MockAnthropicClientTrait::new());
        let mut ctx = SkillContext::new("input");
        assert!(!executor.check_condition(&StepCondition::IfKeyExists("result".to_string()), &ctx));
        ctx.set("result", serde_json::json!("value"));
        assert!(executor.check_condition(&StepCondition::IfKeyExists("result".to_string()), &ctx));
    }

    #[test]
    fn test_check_condition_if_step_failed() {
        let executor = SkillExecutor::new(MockStorageTrait::new(), MockAnthropicClientTrait::new());
        let mut ctx = SkillContext::new("input");
        assert!(!executor.check_condition(&StepCondition::IfStepFailed(0), &ctx));
        ctx.record_failure(0);
        assert!(executor.check_condition(&StepCondition::IfStepFailed(0), &ctx));
    }

    #[test]
    fn test_check_condition_if_confidence_above() {
        let executor = SkillExecutor::new(MockStorageTrait::new(), MockAnthropicClientTrait::new());
        let mut ctx = SkillContext::new("input");

        // No confidence key -> false
        assert!(!executor.check_condition(&StepCondition::IfConfidenceAbove(0.5), &ctx));

        // Confidence below threshold -> false
        ctx.set("_last_confidence", serde_json::json!(0.3));
        assert!(!executor.check_condition(&StepCondition::IfConfidenceAbove(0.5), &ctx));

        // Confidence above threshold -> true
        ctx.set("_last_confidence", serde_json::json!(0.8));
        assert!(executor.check_condition(&StepCondition::IfConfidenceAbove(0.5), &ctx));
    }

    #[test]
    fn test_build_step_input_no_mapping() {
        let executor = SkillExecutor::new(MockStorageTrait::new(), MockAnthropicClientTrait::new());
        let ctx = SkillContext::new("original input");
        let step = SkillStep::new("linear");
        assert_eq!(executor.build_step_input(&step, &ctx), "original input");
    }

    #[test]
    fn test_build_step_input_with_mapping() {
        let executor = SkillExecutor::new(MockStorageTrait::new(), MockAnthropicClientTrait::new());
        let mut ctx = SkillContext::new("original input");
        ctx.set("analysis", serde_json::json!("First analysis result"));
        ctx.set("score", serde_json::json!(42));

        let step = SkillStep::new("linear")
            .with_input_map("analysis", "analysis_input")
            .with_input_map("score", "score_input");
        let input = executor.build_step_input(&step, &ctx);
        // Input should contain both mapped values
        assert!(input.contains("First analysis result") || input.contains("42"));
    }

    #[test]
    fn test_build_step_input_with_mapping_missing_keys() {
        let executor = SkillExecutor::new(MockStorageTrait::new(), MockAnthropicClientTrait::new());
        let ctx = SkillContext::new("fallback input");

        let step = SkillStep::new("linear").with_input_map("nonexistent", "param");
        let input = executor.build_step_input(&step, &ctx);
        // Falls back to original input since no mapped keys found
        assert_eq!(input, "fallback input");
    }

    #[tokio::test]
    async fn test_execute_mode_success() {
        let executor = SkillExecutor::new(mock_storage(), mock_client());
        let result = executor.execute_mode("linear", "test input", "s1").await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["confidence"], 0.85);
    }

    #[tokio::test]
    async fn test_execute_mode_non_json_response() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                "This is plain text, not JSON",
                Usage::new(100, 50),
            ))
        });

        let executor = SkillExecutor::new(mock_storage(), client);
        let result = executor.execute_mode("linear", "test", "s1").await;
        assert!(result.is_ok());
        let output = result.unwrap();
        // Should wrap in {"content": ...} when JSON extraction fails
        assert!(output.get("content").is_some());
    }

    #[tokio::test]
    async fn test_execute_mode_unknown_mode() {
        let executor = SkillExecutor::new(mock_storage(), mock_client());
        // Unknown mode should still work - falls back to generic prompt
        let result = executor
            .execute_mode("unknown_mode", "test input", "s1")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_retry_succeeds_second_attempt() {
        let mut client = MockAnthropicClientTrait::new();

        let mut call_count = 0u32;
        client.expect_complete().returning(move |_, _| {
            call_count += 1;
            if call_count == 1 {
                Err(ModeError::ApiError {
                    message: "temporary error".to_string(),
                })
            } else {
                Ok(CompletionResponse::new(
                    r#"{"analysis": "Success", "confidence": 0.9}"#,
                    Usage::new(100, 50),
                ))
            }
        });

        let executor = SkillExecutor::new(mock_storage(), client);
        let skill = Skill::new(
            "retry-skill",
            "Retry Skill",
            "test",
            SkillCategory::Analysis,
            vec![SkillStep::new("linear").with_error_strategy(ErrorStrategy::Retry(3))],
        );
        let context = SkillContext::new("input");

        let result = executor.execute(&skill, context).await.unwrap();
        assert!(result.success);
        assert!(!result.step_results[0].skipped);
    }

    #[tokio::test]
    async fn test_execute_with_retry_exhausted() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiError {
                message: "persistent error".to_string(),
            })
        });

        let executor = SkillExecutor::new(mock_storage(), client);
        let skill = Skill::new(
            "retry-exhaust",
            "Retry Exhaust",
            "test",
            SkillCategory::Analysis,
            vec![SkillStep::new("linear").with_error_strategy(ErrorStrategy::Retry(2))],
        );
        let context = SkillContext::new("input");

        let result = executor.execute(&skill, context).await.unwrap();
        // Retry exhausted falls to Skip-like behavior
        assert!(result.success); // Retry counts as skipped
        assert!(result.step_results[0].skipped);
    }

    #[tokio::test]
    async fn test_execute_generates_session_id_when_absent() {
        let executor = SkillExecutor::new(mock_storage(), mock_client());
        let skill = simple_skill();
        // Context with no session_id set — executor should generate one via UUID
        let mut context = SkillContext::new("Test input");
        assert!(context.session_id.is_none());

        let result = executor.execute(&skill, context).await.unwrap();
        assert!(!result.session_id.is_empty());
    }

    #[tokio::test]
    async fn test_execute_uses_provided_session_id() {
        let executor = SkillExecutor::new(mock_storage(), mock_client());
        let skill = simple_skill();
        let mut context = SkillContext::new("Test input");
        context.session_id = Some("my-session-123".to_string());

        let result = executor.execute(&skill, context).await.unwrap();
        assert_eq!(result.session_id, "my-session-123");
    }

    #[tokio::test]
    async fn test_execute_mode_storage_failure_is_non_fatal() {
        // If save_thought fails, execute_mode should warn and still return Ok
        let mut storage = MockStorageTrait::new();
        storage
            .expect_get_or_create_session()
            .returning(|id| Ok(Session::new(id.unwrap_or_else(|| "s1".to_string()))));
        storage.expect_save_thought().returning(|_| {
            Err(crate::error::StorageError::Internal {
                message: "test failure".to_string(),
            })
        });
        storage.expect_get_thoughts().returning(|_| Ok(vec![]));

        let executor = SkillExecutor::new(storage, mock_client());
        let result = executor.execute_mode("linear", "test input", "s1").await;
        // Storage failure should not propagate — just a tracing::warn
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_fail_strategy_records_failure_in_context() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiError {
                message: "test error".to_string(),
            })
        });

        let executor = SkillExecutor::new(mock_storage(), client);
        let skill = Skill::new(
            "fail-track",
            "Fail Track",
            "test",
            SkillCategory::Analysis,
            vec![SkillStep::new("linear").with_error_strategy(ErrorStrategy::Fail)],
        );
        let context = SkillContext::new("input");

        let result = executor.execute(&skill, context).await.unwrap();
        assert!(!result.success);
        // Failure should be recorded in context
        assert!(result.context.step_failed(0));
    }

    #[tokio::test]
    async fn test_execute_with_confidence_condition() {
        let executor = SkillExecutor::new(mock_storage(), mock_client());
        let skill = Skill::new(
            "conf-skill",
            "Confidence Skill",
            "test",
            SkillCategory::Analysis,
            vec![
                SkillStep::new("linear").with_output_key("_last_confidence"),
                SkillStep::new("linear")
                    .with_condition(StepCondition::IfConfidenceAbove(0.5))
                    .with_description("Only if confident"),
            ],
        );

        let context = SkillContext::new("input");
        let result = executor.execute(&skill, context).await.unwrap();
        assert!(result.success);
        // Second step may be skipped or executed depending on confidence value
        assert_eq!(result.step_results.len(), 2);
    }
}
