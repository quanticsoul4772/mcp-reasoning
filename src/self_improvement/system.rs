//! Self-improvement system orchestration.
//!
//! Coordinates the 4-phase optimization loop:
//! Monitor → Analyze → Execute → Learn

use crate::error::ModeError;
use crate::metrics::MetricsCollector;
use crate::traits::AnthropicClientTrait;

use super::allowlist::{Allowlist, AllowlistConfig, ValidationError};
use super::analyzer::{AnalysisResult, Analyzer};
use super::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use super::executor::{ConfigState, ExecutionResult, Executor};
use super::learner::{Learner, LearnerConfig, LearningResult, LearningSummary};
use super::monitor::{Monitor, MonitorConfig, MonitorResult};
use super::types::SelfImprovementAction;

/// Configuration for the self-improvement system.
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// Monitor configuration.
    pub monitor: MonitorConfig,
    /// Circuit breaker configuration.
    pub circuit_breaker: CircuitBreakerConfig,
    /// Allowlist configuration.
    pub allowlist: AllowlistConfig,
    /// Learner configuration.
    pub learner: LearnerConfig,
    /// Whether to require approval before execution.
    pub require_approval: bool,
    /// Maximum actions per improvement cycle.
    pub max_actions_per_cycle: usize,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            monitor: MonitorConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
            allowlist: AllowlistConfig::default(),
            learner: LearnerConfig::default(),
            require_approval: true,
            max_actions_per_cycle: 3,
        }
    }
}

/// Result of an improvement cycle.
#[derive(Debug)]
pub struct CycleResult {
    /// Monitoring result.
    pub monitor_result: MonitorResult,
    /// Analysis result (if performed).
    pub analysis_result: Option<AnalysisResult>,
    /// Execution results.
    pub execution_results: Vec<ExecutionResult>,
    /// Learning results.
    pub learning_results: Vec<LearningResult>,
    /// Whether the cycle was blocked by circuit breaker.
    pub blocked: bool,
    /// Error message if any.
    pub error: Option<String>,
}

/// The self-improvement system.
pub struct SelfImprovementSystem<C: AnthropicClientTrait> {
    config: SystemConfig,
    monitor: Monitor,
    analyzer: Analyzer<C>,
    executor: Executor,
    learner: Learner,
    circuit_breaker: CircuitBreaker,
    allowlist: Allowlist,
    pending_actions: Vec<SelfImprovementAction>,
}

impl<C: AnthropicClientTrait> SelfImprovementSystem<C> {
    /// Create a new self-improvement system.
    pub fn new(config: SystemConfig, client: C) -> Self {
        Self {
            monitor: Monitor::new(config.monitor.clone()),
            analyzer: Analyzer::new(client).with_max_actions(config.max_actions_per_cycle),
            executor: Executor::new(),
            learner: Learner::new(config.learner.clone()),
            circuit_breaker: CircuitBreaker::new(config.circuit_breaker.clone()),
            allowlist: Allowlist::new(config.allowlist.clone()),
            pending_actions: Vec::new(),
            config,
        }
    }

    /// Create a system with default configuration.
    pub fn with_defaults(client: C) -> Self {
        Self::new(SystemConfig::default(), client)
    }

    /// Run a complete improvement cycle.
    pub async fn run_cycle(
        &mut self,
        metrics: &MetricsCollector,
    ) -> Result<CycleResult, ModeError> {
        // Check circuit breaker
        if !self.circuit_breaker.is_allowed() {
            return Ok(CycleResult {
                monitor_result: self.monitor.check(metrics),
                analysis_result: None,
                execution_results: Vec::new(),
                learning_results: Vec::new(),
                blocked: true,
                error: Some("Circuit breaker is open".to_string()),
            });
        }

        // Phase 1: Monitor
        let monitor_result = self.monitor.check(metrics);

        if !monitor_result.action_recommended {
            return Ok(CycleResult {
                monitor_result,
                analysis_result: None,
                execution_results: Vec::new(),
                learning_results: Vec::new(),
                blocked: false,
                error: None,
            });
        }

        // Phase 2: Analyze
        let analysis_result = match self.analyzer.analyze(&monitor_result).await {
            Ok(result) => result,
            Err(e) => {
                self.circuit_breaker.record_failure();
                return Ok(CycleResult {
                    monitor_result,
                    analysis_result: None,
                    execution_results: Vec::new(),
                    learning_results: Vec::new(),
                    blocked: false,
                    error: Some(format!("Analysis failed: {e}")),
                });
            }
        };

        // Store pending actions if approval required
        if self.config.require_approval {
            self.pending_actions = analysis_result.actions.clone();
            return Ok(CycleResult {
                monitor_result,
                analysis_result: Some(analysis_result),
                execution_results: Vec::new(),
                learning_results: Vec::new(),
                blocked: false,
                error: None,
            });
        }

        // Phase 3 & 4: Execute and Learn
        let (execution_results, learning_results) =
            self.execute_and_learn(analysis_result.actions.clone());

        Ok(CycleResult {
            monitor_result,
            analysis_result: Some(analysis_result),
            execution_results,
            learning_results,
            blocked: false,
            error: None,
        })
    }

    /// Approve pending actions and execute them.
    pub fn approve_and_execute(&mut self) -> (Vec<ExecutionResult>, Vec<LearningResult>) {
        let actions = std::mem::take(&mut self.pending_actions);
        self.execute_and_learn(actions)
    }

    /// Approve specific actions by ID.
    pub fn approve_actions(
        &mut self,
        action_ids: &[String],
    ) -> (Vec<ExecutionResult>, Vec<LearningResult>) {
        let actions: Vec<_> = self
            .pending_actions
            .iter()
            .filter(|a| action_ids.contains(&a.id))
            .cloned()
            .collect();

        self.pending_actions.retain(|a| !action_ids.contains(&a.id));

        self.execute_and_learn(actions)
    }

    /// Reject all pending actions.
    pub fn reject_pending(&mut self) {
        self.pending_actions.clear();
    }

    /// Get pending actions.
    pub fn pending_actions(&self) -> &[SelfImprovementAction] {
        &self.pending_actions
    }

    /// Get circuit breaker state.
    pub fn circuit_state(&self) -> CircuitState {
        self.circuit_breaker.state()
    }

    /// Reset circuit breaker.
    pub fn reset_circuit_breaker(&mut self) {
        self.circuit_breaker.reset();
    }

    /// Get learning summary.
    pub fn learning_summary(&self) -> LearningSummary {
        self.learner.summary()
    }

    /// Get executor config state.
    pub fn config_state(&self) -> &ConfigState {
        self.executor.config()
    }

    /// Get mutable executor config state.
    pub fn config_state_mut(&mut self) -> &mut ConfigState {
        self.executor.config_mut()
    }

    /// Rollback an action by ID.
    pub fn rollback(&mut self, action_id: &str) -> Result<(), String> {
        self.executor.rollback(action_id)
    }

    /// Validate an action.
    pub fn validate_action(
        &mut self,
        action: &SelfImprovementAction,
    ) -> Result<(), ValidationError> {
        self.allowlist.validate(action)
    }

    /// Set baseline from metrics.
    pub fn set_baseline(&mut self, metrics: &MetricsCollector) {
        self.monitor.calculate_baseline(metrics);
    }

    fn execute_and_learn(
        &mut self,
        actions: Vec<SelfImprovementAction>,
    ) -> (Vec<ExecutionResult>, Vec<LearningResult>) {
        let mut execution_results = Vec::new();
        let mut learning_results = Vec::new();

        for mut action in actions {
            // Validate action
            if let Err(e) = self.allowlist.validate_and_record(&action) {
                action.fail();
                execution_results.push(ExecutionResult {
                    action,
                    success: false,
                    message: format!("Validation failed: {e}"),
                    measured_improvement: None,
                });
                self.circuit_breaker.record_failure();
                continue;
            }

            // Approve action
            action.approve();

            // Execute action
            let exec_result = self.executor.execute(action);

            // Update circuit breaker
            if exec_result.success {
                self.circuit_breaker.record_success();
            } else {
                self.circuit_breaker.record_failure();
            }

            // Learn from result
            if let Some(learning) = self.learner.learn(&exec_result) {
                learning_results.push(learning);
            }

            execution_results.push(exec_result);

            // Check circuit breaker after each action
            if self.circuit_breaker.state() == CircuitState::Open {
                break;
            }
        }

        (execution_results, learning_results)
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
mod tests {
    use super::*;
    use crate::metrics::MetricEvent;
    use crate::self_improvement::types::ActionType;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

    fn mock_response(content: &str) -> CompletionResponse {
        CompletionResponse::new(content, Usage::new(100, 50))
    }

    fn create_test_system() -> SelfImprovementSystem<MockAnthropicClientTrait> {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(
                r#"{
                "summary": "Test analysis",
                "confidence": 0.8,
                "actions": [
                    {
                        "action_type": "config_adjust",
                        "description": "Test action",
                        "rationale": "Testing",
                        "expected_improvement": 0.1,
                        "parameters": {"timeout_ms": 30000}
                    }
                ]
            }"#,
            ))
        });

        let config = SystemConfig {
            require_approval: false,
            monitor: MonitorConfig {
                min_invocations: 5,
                min_success_rate: 0.8,
                ..Default::default()
            },
            ..Default::default()
        };

        SelfImprovementSystem::new(config, client)
    }

    fn create_metrics_with_issues() -> MetricsCollector {
        let collector = MetricsCollector::new();
        // 50% success rate - below threshold
        for _ in 0..5 {
            collector.record(MetricEvent::new("linear", 100, true));
        }
        for _ in 0..5 {
            collector.record(MetricEvent::new("linear", 100, false));
        }
        collector
    }

    #[tokio::test]
    async fn test_system_no_issues() {
        let mut system = create_test_system();
        let metrics = MetricsCollector::new();

        // Add good metrics
        for _ in 0..10 {
            metrics.record(MetricEvent::new("linear", 100, true));
        }

        let result = system.run_cycle(&metrics).await.unwrap();

        assert!(!result.blocked);
        assert!(result.analysis_result.is_none()); // No analysis needed
    }

    #[tokio::test]
    async fn test_system_with_issues() {
        let mut system = create_test_system();
        let metrics = create_metrics_with_issues();

        let result = system.run_cycle(&metrics).await.unwrap();

        assert!(!result.blocked);
        assert!(result.analysis_result.is_some());
        assert!(!result.execution_results.is_empty());
    }

    #[tokio::test]
    async fn test_system_require_approval() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(r#"{
                "summary": "Test",
                "confidence": 0.8,
                "actions": [{"action_type": "log_observation", "description": "Test", "rationale": "Test", "expected_improvement": 0.1}]
            }"#))
        });

        let config = SystemConfig {
            require_approval: true,
            monitor: MonitorConfig {
                min_invocations: 5,
                min_success_rate: 0.8,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut system = SelfImprovementSystem::new(config, client);
        let metrics = create_metrics_with_issues();

        let result = system.run_cycle(&metrics).await.unwrap();

        assert!(result.execution_results.is_empty()); // Not executed yet
        assert!(!system.pending_actions().is_empty());
    }

    #[tokio::test]
    async fn test_system_approve_and_execute() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(r#"{
                "summary": "Test",
                "confidence": 0.8,
                "actions": [{"action_type": "log_observation", "description": "Test", "rationale": "Test", "expected_improvement": 0.1}]
            }"#))
        });

        let config = SystemConfig {
            require_approval: true,
            monitor: MonitorConfig {
                min_invocations: 5,
                min_success_rate: 0.8,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut system = SelfImprovementSystem::new(config, client);
        let metrics = create_metrics_with_issues();

        system.run_cycle(&metrics).await.unwrap();
        assert!(!system.pending_actions().is_empty());

        let (exec_results, _) = system.approve_and_execute();
        assert!(!exec_results.is_empty());
        assert!(system.pending_actions().is_empty());
    }

    #[tokio::test]
    async fn test_system_reject_pending() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(r#"{
                "summary": "Test",
                "confidence": 0.8,
                "actions": [{"action_type": "log_observation", "description": "Test", "rationale": "Test", "expected_improvement": 0.1}]
            }"#))
        });

        let config = SystemConfig {
            require_approval: true,
            monitor: MonitorConfig {
                min_invocations: 5,
                min_success_rate: 0.8,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut system = SelfImprovementSystem::new(config, client);
        let metrics = create_metrics_with_issues();

        system.run_cycle(&metrics).await.unwrap();
        assert!(!system.pending_actions().is_empty());

        system.reject_pending();
        assert!(system.pending_actions().is_empty());
    }

    #[test]
    fn test_system_circuit_breaker() {
        let client = MockAnthropicClientTrait::new();
        let mut system = SelfImprovementSystem::with_defaults(client);

        assert_eq!(system.circuit_state(), CircuitState::Closed);

        system.circuit_breaker.trip();
        assert_eq!(system.circuit_state(), CircuitState::Open);

        system.reset_circuit_breaker();
        assert_eq!(system.circuit_state(), CircuitState::Closed);
    }

    #[test]
    fn test_system_config_state() {
        let client = MockAnthropicClientTrait::new();
        let mut system = SelfImprovementSystem::with_defaults(client);

        system
            .config_state_mut()
            .set("key", serde_json::json!("value"));

        assert!(system.config_state().get("key").is_some());
    }

    #[test]
    fn test_system_learning_summary() {
        let client = MockAnthropicClientTrait::new();
        let system = SelfImprovementSystem::with_defaults(client);

        let summary = system.learning_summary();
        assert_eq!(summary.total_lessons, 0);
    }

    #[test]
    fn test_system_validate_action() {
        let client = MockAnthropicClientTrait::new();
        let mut system = SelfImprovementSystem::with_defaults(client);

        let action =
            SelfImprovementAction::new("test", ActionType::ConfigAdjust, "Test", "Test", 0.1);

        let result = system.validate_action(&action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_system_set_baseline() {
        let client = MockAnthropicClientTrait::new();
        let mut system = SelfImprovementSystem::with_defaults(client);
        let metrics = MetricsCollector::new();

        for _ in 0..10 {
            metrics.record(MetricEvent::new("linear", 100, true));
        }

        system.set_baseline(&metrics);
        // No assertion needed - just verify it doesn't panic
    }

    #[tokio::test]
    async fn test_system_blocked_by_circuit_breaker() {
        let client = MockAnthropicClientTrait::new();
        let mut system = SelfImprovementSystem::with_defaults(client);
        let metrics = create_metrics_with_issues();

        system.circuit_breaker.trip();

        let result = system.run_cycle(&metrics).await.unwrap();

        assert!(result.blocked);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_system_approve_specific_actions() {
        let client = MockAnthropicClientTrait::new();
        let config = SystemConfig {
            require_approval: true,
            ..Default::default()
        };
        let mut system = SelfImprovementSystem::new(config, client);

        // Add some pending actions manually
        system.pending_actions.push(SelfImprovementAction::new(
            "action-1",
            ActionType::LogObservation,
            "Test 1",
            "Test",
            0.1,
        ));
        system.pending_actions.push(SelfImprovementAction::new(
            "action-2",
            ActionType::LogObservation,
            "Test 2",
            "Test",
            0.1,
        ));

        let (results, _) = system.approve_actions(&["action-1".to_string()]);

        assert_eq!(results.len(), 1);
        assert_eq!(system.pending_actions().len(), 1);
        assert_eq!(system.pending_actions()[0].id, "action-2");
    }
}
