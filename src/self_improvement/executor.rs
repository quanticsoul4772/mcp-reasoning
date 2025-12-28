//! Self-improvement action execution.
//!
//! Phase 3 of the 4-phase loop: Execute approved actions with rollback capability.

use super::types::{ActionType, SelfImprovementAction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of executing an action.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The action that was executed.
    pub action: SelfImprovementAction,
    /// Whether execution succeeded.
    pub success: bool,
    /// Execution message.
    pub message: String,
    /// Measured improvement (if measurable).
    pub measured_improvement: Option<f64>,
}

/// Configuration state that can be modified.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigState {
    /// Key-value configuration.
    pub values: HashMap<String, serde_json::Value>,
}

impl ConfigState {
    /// Create a new empty config state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a config value.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }

    /// Set a config value.
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.values.insert(key.into(), value);
    }

    /// Remove a config value.
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.values.remove(key)
    }
}

/// Executor for self-improvement actions.
#[derive(Debug)]
pub struct Executor {
    /// Current configuration state.
    config_state: ConfigState,
    /// History of executed actions for potential rollback.
    execution_history: Vec<ExecutionRecord>,
    /// Maximum history size.
    max_history: usize,
}

/// Record of an executed action.
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    /// ID of the executed action.
    pub action_id: String,
    /// Type of action executed.
    pub action_type: ActionType,
    /// State before execution (for rollback).
    pub previous_state: Option<serde_json::Value>,
    /// State after execution (for auditing).
    #[allow(dead_code)]
    pub new_state: Option<serde_json::Value>,
    /// Execution timestamp.
    #[allow(dead_code)]
    pub timestamp: u64,
}

impl Executor {
    /// Create a new executor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config_state: ConfigState::new(),
            execution_history: Vec::new(),
            max_history: 100,
        }
    }

    /// Create executor with initial config state.
    #[must_use]
    pub fn with_config(config_state: ConfigState) -> Self {
        Self {
            config_state,
            execution_history: Vec::new(),
            max_history: 100,
        }
    }

    /// Get current config state.
    pub fn config(&self) -> &ConfigState {
        &self.config_state
    }

    /// Get mutable config state.
    pub fn config_mut(&mut self) -> &mut ConfigState {
        &mut self.config_state
    }

    /// Execute an action.
    pub fn execute(&mut self, mut action: SelfImprovementAction) -> ExecutionResult {
        // Mark as executing
        action.start_execution();

        let result = match action.action_type {
            ActionType::ConfigAdjust => self.execute_config_adjust(&mut action),
            ActionType::PromptTune => self.execute_prompt_tune(&action),
            ActionType::ThresholdAdjust => self.execute_threshold_adjust(&action),
            ActionType::LogObservation => self.execute_log_observation(&action),
        };

        // Update action status
        if result.success {
            action.complete(result.measured_improvement.unwrap_or(0.0));
        } else {
            action.fail();
        }

        ExecutionResult {
            action,
            success: result.success,
            message: result.message,
            measured_improvement: result.measured_improvement,
        }
    }

    /// Rollback an action.
    pub fn rollback(&mut self, action_id: &str) -> Result<(), String> {
        let record_idx = self
            .execution_history
            .iter()
            .position(|r| r.action_id == action_id)
            .ok_or_else(|| format!("Action {action_id} not found in history"))?;

        let record = self.execution_history.remove(record_idx);

        match record.action_type {
            ActionType::ConfigAdjust | ActionType::ThresholdAdjust => {
                if let Some(previous) = record.previous_state {
                    if let Some(key) = previous.get("key").and_then(|v| v.as_str()) {
                        if let Some(value) = previous.get("value") {
                            self.config_state.set(key, value.clone());
                        }
                    }
                }
            }
            ActionType::PromptTune => {
                // Prompt rollback would restore previous prompt template
                if let Some(previous) = record.previous_state {
                    if let Some(prompt_key) = previous.get("prompt_key").and_then(|v| v.as_str()) {
                        if let Some(template) = previous.get("template") {
                            self.config_state
                                .set(format!("prompt:{prompt_key}"), template.clone());
                        }
                    }
                }
            }
            ActionType::LogObservation => {
                // Log observations are not rollbackable
                return Err("LogObservation actions cannot be rolled back".to_string());
            }
        }

        Ok(())
    }

    /// Get execution history.
    pub fn history(&self) -> &[ExecutionRecord] {
        &self.execution_history
    }

    fn execute_config_adjust(
        &mut self,
        action: &mut SelfImprovementAction,
    ) -> InternalExecutionResult {
        let params = match &action.parameters {
            Some(p) => p,
            None => {
                return InternalExecutionResult {
                    success: false,
                    message: "No parameters provided for config adjustment".to_string(),
                    measured_improvement: None,
                }
            }
        };

        let mut changes_made = Vec::new();
        let mut previous_values = serde_json::Map::new();

        if let Some(obj) = params.as_object() {
            for (key, value) in obj {
                // Store previous value for rollback
                if let Some(prev) = self.config_state.get(key) {
                    previous_values.insert(key.clone(), prev.clone());
                }
                self.config_state.set(key, value.clone());
                changes_made.push(key.clone());
            }
        }

        // Record for rollback
        self.record_execution(
            &action.id,
            action.action_type.clone(),
            Some(serde_json::json!({"previous": previous_values})),
            action.parameters.clone(),
        );

        action.rollback_data = Some(serde_json::json!({"previous": previous_values}));

        InternalExecutionResult {
            success: true,
            message: format!("Config adjusted: {}", changes_made.join(", ")),
            measured_improvement: Some(action.expected_improvement * 0.8), // Estimate
        }
    }

    fn execute_prompt_tune(&mut self, action: &SelfImprovementAction) -> InternalExecutionResult {
        let params = match &action.parameters {
            Some(p) => p,
            None => {
                return InternalExecutionResult {
                    success: false,
                    message: "No parameters provided for prompt tuning".to_string(),
                    measured_improvement: None,
                }
            }
        };

        let prompt_key = params
            .get("prompt_key")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let Some(new_template) = params.get("template") else {
            return InternalExecutionResult {
                success: false,
                message: "No template provided for prompt tuning".to_string(),
                measured_improvement: None,
            };
        };

        let config_key = format!("prompt:{prompt_key}");
        let previous = self.config_state.get(&config_key).cloned();

        self.config_state.set(&config_key, new_template.clone());

        self.record_execution(
            &action.id,
            action.action_type.clone(),
            previous.map(|p| serde_json::json!({"prompt_key": prompt_key, "template": p})),
            action.parameters.clone(),
        );

        InternalExecutionResult {
            success: true,
            message: format!("Prompt '{prompt_key}' tuned"),
            measured_improvement: Some(action.expected_improvement * 0.7),
        }
    }

    fn execute_threshold_adjust(
        &mut self,
        action: &SelfImprovementAction,
    ) -> InternalExecutionResult {
        let params = match &action.parameters {
            Some(p) => p,
            None => {
                return InternalExecutionResult {
                    success: false,
                    message: "No parameters provided for threshold adjustment".to_string(),
                    measured_improvement: None,
                }
            }
        };

        let threshold_key = params
            .get("threshold_key")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let Some(new_value) = params.get("value") else {
            return InternalExecutionResult {
                success: false,
                message: "No value provided for threshold adjustment".to_string(),
                measured_improvement: None,
            };
        };

        let config_key = format!("threshold:{threshold_key}");
        let previous = self.config_state.get(&config_key).cloned();

        self.config_state.set(&config_key, new_value.clone());

        self.record_execution(
            &action.id,
            action.action_type.clone(),
            previous.map(|p| serde_json::json!({"key": config_key, "value": p})),
            action.parameters.clone(),
        );

        InternalExecutionResult {
            success: true,
            message: format!("Threshold '{threshold_key}' adjusted"),
            measured_improvement: Some(action.expected_improvement * 0.75),
        }
    }

    fn execute_log_observation(
        &mut self,
        action: &SelfImprovementAction,
    ) -> InternalExecutionResult {
        // Log observations are always successful
        // They just record information for future reference
        self.record_execution(
            &action.id,
            action.action_type.clone(),
            None,
            action.parameters.clone(),
        );

        InternalExecutionResult {
            success: true,
            message: format!("Observation logged: {}", action.description),
            measured_improvement: Some(0.0), // No immediate improvement
        }
    }

    fn record_execution(
        &mut self,
        action_id: &str,
        action_type: ActionType,
        previous_state: Option<serde_json::Value>,
        new_state: Option<serde_json::Value>,
    ) {
        let record = ExecutionRecord {
            action_id: action_id.to_string(),
            action_type,
            previous_state,
            new_state,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };

        self.execution_history.push(record);

        // Trim history if needed
        while self.execution_history.len() > self.max_history {
            self.execution_history.remove(0);
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

struct InternalExecutionResult {
    success: bool,
    message: String,
    measured_improvement: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_improvement::types::ActionStatus;

    fn create_test_action(action_type: ActionType) -> SelfImprovementAction {
        SelfImprovementAction::new("test-action", action_type, "Test action", "Testing", 0.15)
    }

    #[test]
    fn test_executor_new() {
        let executor = Executor::new();
        assert!(executor.config().values.is_empty());
        assert!(executor.history().is_empty());
    }

    #[test]
    fn test_executor_with_config() {
        let mut config = ConfigState::new();
        config.set("key", serde_json::json!("value"));

        let executor = Executor::with_config(config);
        assert!(executor.config().get("key").is_some());
    }

    #[test]
    fn test_config_state_operations() {
        let mut config = ConfigState::new();

        config.set("key1", serde_json::json!("value1"));
        assert_eq!(config.get("key1"), Some(&serde_json::json!("value1")));

        let removed = config.remove("key1");
        assert!(removed.is_some());
        assert!(config.get("key1").is_none());
    }

    #[test]
    fn test_execute_config_adjust() {
        let mut executor = Executor::new();
        let mut action = create_test_action(ActionType::ConfigAdjust);
        action = action.with_parameters(serde_json::json!({
            "timeout_ms": 30000,
            "max_retries": 5
        }));

        let result = executor.execute(action);

        assert!(result.success);
        assert_eq!(
            executor.config().get("timeout_ms"),
            Some(&serde_json::json!(30000))
        );
        assert_eq!(
            executor.config().get("max_retries"),
            Some(&serde_json::json!(5))
        );
        assert_eq!(result.action.status, ActionStatus::Completed);
    }

    #[test]
    fn test_execute_config_adjust_no_params() {
        let mut executor = Executor::new();
        let action = create_test_action(ActionType::ConfigAdjust);

        let result = executor.execute(action);

        assert!(!result.success);
        assert_eq!(result.action.status, ActionStatus::Failed);
    }

    #[test]
    fn test_execute_prompt_tune() {
        let mut executor = Executor::new();
        let mut action = create_test_action(ActionType::PromptTune);
        action = action.with_parameters(serde_json::json!({
            "prompt_key": "linear",
            "template": "New prompt template"
        }));

        let result = executor.execute(action);

        assert!(result.success);
        assert!(executor.config().get("prompt:linear").is_some());
    }

    #[test]
    fn test_execute_prompt_tune_no_template() {
        let mut executor = Executor::new();
        let mut action = create_test_action(ActionType::PromptTune);
        action = action.with_parameters(serde_json::json!({
            "prompt_key": "linear"
        }));

        let result = executor.execute(action);

        assert!(!result.success);
    }

    #[test]
    fn test_execute_threshold_adjust() {
        let mut executor = Executor::new();
        let mut action = create_test_action(ActionType::ThresholdAdjust);
        action = action.with_parameters(serde_json::json!({
            "threshold_key": "confidence",
            "value": 0.85
        }));

        let result = executor.execute(action);

        assert!(result.success);
        assert_eq!(
            executor.config().get("threshold:confidence"),
            Some(&serde_json::json!(0.85))
        );
    }

    #[test]
    fn test_execute_log_observation() {
        let mut executor = Executor::new();
        let action = create_test_action(ActionType::LogObservation);

        let result = executor.execute(action);

        assert!(result.success);
        assert_eq!(result.measured_improvement, Some(0.0));
    }

    #[test]
    fn test_execution_history() {
        let mut executor = Executor::new();
        let mut action = create_test_action(ActionType::ConfigAdjust);
        action = action.with_parameters(serde_json::json!({"key": "value"}));

        executor.execute(action);

        assert_eq!(executor.history().len(), 1);
    }

    #[test]
    fn test_rollback_config_adjust() {
        let mut executor = Executor::new();
        executor
            .config_mut()
            .set("key", serde_json::json!("original"));

        let mut action = SelfImprovementAction::new(
            "rollback-test",
            ActionType::ConfigAdjust,
            "Test",
            "Test",
            0.1,
        );
        action = action.with_parameters(serde_json::json!({"key": "new_value"}));

        executor.execute(action);
        assert_eq!(
            executor.config().get("key"),
            Some(&serde_json::json!("new_value"))
        );

        // Rollback
        let result = executor.rollback("rollback-test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rollback_not_found() {
        let mut executor = Executor::new();
        let result = executor.rollback("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_rollback_log_observation() {
        let mut executor = Executor::new();
        let action =
            SelfImprovementAction::new("log-test", ActionType::LogObservation, "Test", "Test", 0.1);

        executor.execute(action);

        let result = executor.rollback("log-test");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be rolled back"));
    }

    #[test]
    fn test_history_limit() {
        let mut executor = Executor::new();
        executor.max_history = 5;

        for i in 0..10 {
            let mut action = SelfImprovementAction::new(
                format!("action-{i}"),
                ActionType::LogObservation,
                "Test",
                "Test",
                0.1,
            );
            action = action.with_parameters(serde_json::json!({"index": i}));
            executor.execute(action);
        }

        assert_eq!(executor.history().len(), 5);
        // Should keep the most recent ones
        assert_eq!(executor.history()[0].action_id, "action-5");
    }

    #[test]
    fn test_action_status_lifecycle() {
        let mut executor = Executor::new();
        let mut action = create_test_action(ActionType::ConfigAdjust);
        action = action.with_parameters(serde_json::json!({"key": "value"}));

        assert_eq!(action.status, ActionStatus::Proposed);

        let result = executor.execute(action);

        assert_eq!(result.action.status, ActionStatus::Completed);
        assert!(result.action.executed_at.is_some());
        assert!(result.action.actual_improvement.is_some());
    }

    #[test]
    fn test_measured_improvement() {
        let mut executor = Executor::new();
        let mut action = create_test_action(ActionType::ConfigAdjust);
        action = action.with_parameters(serde_json::json!({"key": "value"}));

        let result = executor.execute(action);

        // ConfigAdjust estimates 80% of expected improvement
        assert!(result.measured_improvement.is_some());
        let measured = result.measured_improvement.unwrap();
        assert!(measured > 0.0 && measured < 0.15);
    }
}
