//! Self-improvement manager for server integration.
//!
//! This module provides the `SelfImprovementManager` which runs as a background task
//! and `ManagerHandle` which allows MCP tools to interact with the manager.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  SelfImprovementManager                      │
//! │  (Background Tokio Task)                                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │  SelfImprovementSystem                                │   │
//! │  │  ├─Monitor (metrics → triggers)                       │   │
//! │  │  ├─Analyzer (LLM diagnosis)                           │   │
//! │  │  ├─Executor (action execution)                        │   │
//! │  │  └─Learner (reward calculation)                       │   │
//! │  └──────────────────────────────────────────────────────┘   │
//! │                          ▲                                   │
//! │                          │                                   │
//! │  ┌─────────────┐    ┌────────────┐    ┌─────────────────┐   │
//! │  │ Interval    │    │ Command RX │    │ Shutdown Signal │   │
//! │  │ Ticker      │    │ (mpsc)     │    │ (watch)         │   │
//! │  └─────────────┘    └────────────┘    └─────────────────┘   │
//! │                          ▲                                   │
//! └──────────────────────────┼───────────────────────────────────┘
//!                            │
//! ┌──────────────────────────┼───────────────────────────────────┐
//! │                  ManagerHandle                               │
//! │  (Clone-able, Send+Sync)                                     │
//! ├──────────────────────────────────────────────────────────────┤
//! │  command_tx: mpsc::Sender<ManagerCommand>                    │
//! │  status_rx: watch::Receiver<ManagerStatus>                   │
//! └──────────────────────────────────────────────────────────────┘
//! ```

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use crate::config::SelfImprovementConfig;
use crate::error::ModeError;
use crate::metrics::MetricsCollector;
use crate::traits::AnthropicClientTrait;

use super::circuit_breaker::CircuitState;
use super::executor::ExecutionResult;
use super::learner::{LearningResult, LearningSummary};
use super::storage::SelfImprovementStorage;
use super::system::{CycleResult, SelfImprovementSystem, SystemConfig};
use super::types::SelfImprovementAction;

/// Commands that can be sent to the manager.
#[derive(Debug)]
pub enum ManagerCommand {
    /// Trigger an immediate improvement cycle.
    TriggerCycle {
        /// Response channel.
        response_tx: oneshot::Sender<Result<CycleResult, ModeError>>,
    },
    /// Approve a pending diagnosis.
    Approve {
        /// Diagnosis ID to approve.
        diagnosis_id: String,
        /// Response channel.
        response_tx: oneshot::Sender<Result<ApproveResult, String>>,
    },
    /// Reject a pending diagnosis.
    Reject {
        /// Diagnosis ID to reject.
        diagnosis_id: String,
        /// Optional reason.
        reason: Option<String>,
        /// Response channel.
        response_tx: oneshot::Sender<Result<(), String>>,
    },
    /// Rollback a previously executed action.
    Rollback {
        /// Action ID to rollback.
        action_id: String,
        /// Response channel.
        response_tx: oneshot::Sender<Result<(), String>>,
    },
    /// Get current status.
    GetStatus {
        /// Response channel.
        response_tx: oneshot::Sender<ManagerStatus>,
    },
    /// Get pending diagnoses.
    GetPending {
        /// Maximum number to return.
        limit: Option<u32>,
        /// Response channel.
        response_tx: oneshot::Sender<Vec<PendingDiagnosis>>,
    },
}

/// Result of approving a diagnosis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveResult {
    /// The diagnosis ID that was approved.
    pub diagnosis_id: String,
    /// Execution results.
    pub execution_results: Vec<ExecutionResultSummary>,
    /// Learning results.
    pub learning_results: Vec<LearningResultSummary>,
}

/// Summary of an execution result (serializable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResultSummary {
    /// Action ID.
    pub action_id: String,
    /// Whether execution succeeded.
    pub success: bool,
    /// Result message.
    pub message: String,
}

impl From<&ExecutionResult> for ExecutionResultSummary {
    fn from(result: &ExecutionResult) -> Self {
        Self {
            action_id: result.action.id.clone(),
            success: result.success,
            message: result.message.clone(),
        }
    }
}

/// Summary of a learning result (serializable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningResultSummary {
    /// Action ID.
    pub action_id: String,
    /// Lesson learned.
    pub lesson: String,
    /// Reward value.
    pub reward: f64,
}

impl From<&LearningResult> for LearningResultSummary {
    fn from(result: &LearningResult) -> Self {
        Self {
            action_id: result.lesson.action_id.clone(),
            lesson: result.lesson.insight.clone(),
            reward: result.lesson.reward,
        }
    }
}

/// Current manager status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatus {
    /// Whether the manager is running.
    pub running: bool,
    /// Circuit breaker state.
    pub circuit_state: String,
    /// Total cycles run.
    pub total_cycles: u64,
    /// Successful cycles.
    pub successful_cycles: u64,
    /// Failed cycles.
    pub failed_cycles: u64,
    /// Pending diagnoses count.
    pub pending_diagnoses: usize,
    /// Total actions executed.
    pub total_actions_executed: u64,
    /// Total actions rolled back.
    pub total_actions_rolled_back: u64,
    /// Last cycle time (Unix epoch milliseconds).
    pub last_cycle_at: Option<u64>,
    /// Learning summary.
    pub learning_summary: LearningSummaryData,
}

impl Default for ManagerStatus {
    fn default() -> Self {
        Self {
            running: false,
            circuit_state: "Closed".to_string(),
            total_cycles: 0,
            successful_cycles: 0,
            failed_cycles: 0,
            pending_diagnoses: 0,
            total_actions_executed: 0,
            total_actions_rolled_back: 0,
            last_cycle_at: None,
            learning_summary: LearningSummaryData::default(),
        }
    }
}

/// Serializable learning summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningSummaryData {
    /// Total lessons learned.
    pub total_lessons: u64,
    /// Average reward.
    pub average_reward: f64,
}

impl From<LearningSummary> for LearningSummaryData {
    fn from(summary: LearningSummary) -> Self {
        Self {
            total_lessons: summary.total_lessons as u64,
            average_reward: summary.avg_reward,
        }
    }
}

/// A pending diagnosis awaiting approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDiagnosis {
    /// Action ID (used as diagnosis ID).
    pub id: String,
    /// Action type.
    pub action_type: String,
    /// Description.
    pub description: String,
    /// Rationale.
    pub rationale: String,
    /// Expected improvement.
    pub expected_improvement: f64,
    /// Created timestamp (Unix epoch milliseconds).
    pub created_at: u64,
}

impl From<&SelfImprovementAction> for PendingDiagnosis {
    fn from(action: &SelfImprovementAction) -> Self {
        Self {
            id: action.id.clone(),
            action_type: format!("{:?}", action.action_type),
            description: action.description.clone(),
            rationale: action.rationale.clone(),
            expected_improvement: action.expected_improvement,
            created_at: action.created_at,
        }
    }
}

/// Handle for interacting with the manager from MCP tools.
///
/// This handle is cheap to clone and can be shared across tasks.
#[derive(Clone)]
pub struct ManagerHandle {
    command_tx: mpsc::Sender<ManagerCommand>,
    status_rx: watch::Receiver<ManagerStatus>,
}

impl ManagerHandle {
    /// Trigger an immediate improvement cycle.
    pub async fn trigger_cycle(&self) -> Result<CycleResult, ModeError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.command_tx
            .send(ManagerCommand::TriggerCycle { response_tx })
            .await
            .map_err(|_| ModeError::ApiUnavailable {
                message: "Manager not running".into(),
            })?;

        response_rx.await.map_err(|_| ModeError::ApiUnavailable {
            message: "Manager disconnected".into(),
        })?
    }

    /// Approve a pending diagnosis.
    pub async fn approve(&self, diagnosis_id: String) -> Result<ApproveResult, String> {
        let (response_tx, response_rx) = oneshot::channel();
        self.command_tx
            .send(ManagerCommand::Approve {
                diagnosis_id,
                response_tx,
            })
            .await
            .map_err(|_| "Manager not running".to_string())?;

        response_rx
            .await
            .map_err(|_| "Manager disconnected".to_string())?
    }

    /// Reject a pending diagnosis.
    pub async fn reject(&self, diagnosis_id: String, reason: Option<String>) -> Result<(), String> {
        let (response_tx, response_rx) = oneshot::channel();
        self.command_tx
            .send(ManagerCommand::Reject {
                diagnosis_id,
                reason,
                response_tx,
            })
            .await
            .map_err(|_| "Manager not running".to_string())?;

        response_rx
            .await
            .map_err(|_| "Manager disconnected".to_string())?
    }

    /// Rollback a previously executed action.
    pub async fn rollback(&self, action_id: String) -> Result<(), String> {
        let (response_tx, response_rx) = oneshot::channel();
        self.command_tx
            .send(ManagerCommand::Rollback {
                action_id,
                response_tx,
            })
            .await
            .map_err(|_| "Manager not running".to_string())?;

        response_rx
            .await
            .map_err(|_| "Manager disconnected".to_string())?
    }

    /// Get current status.
    pub async fn status(&self) -> ManagerStatus {
        let (response_tx, response_rx) = oneshot::channel();
        if self
            .command_tx
            .send(ManagerCommand::GetStatus { response_tx })
            .await
            .is_err()
        {
            return self.status_rx.borrow().clone();
        }

        response_rx
            .await
            .unwrap_or_else(|_| self.status_rx.borrow().clone())
    }

    /// Get pending diagnoses.
    pub async fn pending_diagnoses(&self, limit: Option<u32>) -> Vec<PendingDiagnosis> {
        let (response_tx, response_rx) = oneshot::channel();
        if self
            .command_tx
            .send(ManagerCommand::GetPending { limit, response_tx })
            .await
            .is_err()
        {
            return Vec::new();
        }

        response_rx.await.unwrap_or_default()
    }

    /// Subscribe to status updates.
    pub fn subscribe(&self) -> watch::Receiver<ManagerStatus> {
        self.status_rx.clone()
    }

    /// Create a dummy handle for testing.
    ///
    /// This creates a handle that is not connected to any manager.
    /// All commands will fail with appropriate errors.
    /// Use this in integration tests that don't need self-improvement functionality.
    #[must_use]
    pub fn for_testing() -> Self {
        let (command_tx, _command_rx) = mpsc::channel(1);
        let (status_tx, status_rx) = watch::channel(ManagerStatus::default());
        // Drop the sender so status never changes
        drop(status_tx);
        Self {
            command_tx,
            status_rx,
        }
    }
}

/// Internal manager state.
struct ManagerState {
    running: bool,
    total_cycles: u64,
    successful_cycles: u64,
    failed_cycles: u64,
    total_actions_executed: u64,
    total_actions_rolled_back: u64,
    last_cycle_at: Option<u64>,
}

impl Default for ManagerState {
    fn default() -> Self {
        Self {
            running: true,
            total_cycles: 0,
            successful_cycles: 0,
            failed_cycles: 0,
            total_actions_executed: 0,
            total_actions_rolled_back: 0,
            last_cycle_at: None,
        }
    }
}

/// Get current timestamp as Unix epoch milliseconds.
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The self-improvement manager.
///
/// Runs as a background task and coordinates the self-improvement system.
pub struct SelfImprovementManager<C: AnthropicClientTrait> {
    config: SelfImprovementConfig,
    system: SelfImprovementSystem<C>,
    metrics: Arc<MetricsCollector>,
    #[allow(dead_code)]
    storage: Arc<SelfImprovementStorage>,
    command_rx: mpsc::Receiver<ManagerCommand>,
    status_tx: watch::Sender<ManagerStatus>,
    state: ManagerState,
}

impl<C: AnthropicClientTrait + Send + 'static> SelfImprovementManager<C> {
    /// Create a new manager and its handle.
    ///
    /// Returns the manager (to be run as a background task) and a handle
    /// for interacting with it from MCP tools.
    pub fn new(
        config: SelfImprovementConfig,
        client: C,
        metrics: Arc<MetricsCollector>,
        storage: Arc<SelfImprovementStorage>,
    ) -> (Self, ManagerHandle) {
        let (command_tx, command_rx) = mpsc::channel(32);
        let (status_tx, status_rx) = watch::channel(ManagerStatus {
            running: true,
            circuit_state: "closed".to_string(),
            total_cycles: 0,
            successful_cycles: 0,
            failed_cycles: 0,
            pending_diagnoses: 0,
            total_actions_executed: 0,
            total_actions_rolled_back: 0,
            last_cycle_at: None,
            learning_summary: LearningSummaryData::default(),
        });

        let system_config = SystemConfig {
            require_approval: config.require_approval,
            max_actions_per_cycle: config.max_actions_per_cycle as usize,
            circuit_breaker: super::circuit_breaker::CircuitBreakerConfig {
                failure_threshold: config.circuit_breaker_threshold,
                ..Default::default()
            },
            ..Default::default()
        };

        let system = SelfImprovementSystem::new(system_config, client);

        let manager = Self {
            config,
            system,
            metrics,
            storage,
            command_rx,
            status_tx,
            state: ManagerState::default(),
        };

        let handle = ManagerHandle {
            command_tx,
            status_rx,
        };

        (manager, handle)
    }

    /// Run the manager background loop.
    ///
    /// This method runs until the shutdown signal is received.
    pub async fn run(mut self, mut shutdown_rx: watch::Receiver<bool>) {
        let interval_duration = Duration::from_secs(self.config.cycle_interval_secs);
        let mut interval = tokio::time::interval(interval_duration);

        // Skip the first immediate tick
        interval.tick().await;

        tracing::info!(
            interval_secs = self.config.cycle_interval_secs,
            min_invocations = self.config.min_invocations_for_analysis,
            require_approval = self.config.require_approval,
            "Self-improvement manager started"
        );

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if self.should_run_cycle() {
                        self.run_cycle().await;
                    }
                }
                Some(command) = self.command_rx.recv() => {
                    self.handle_command(command).await;
                }
                result = shutdown_rx.changed() => {
                    if result.is_ok() && *shutdown_rx.borrow() {
                        tracing::info!("Self-improvement manager shutting down");
                        break;
                    }
                }
            }
        }

        self.state.running = false;
        self.update_status();
    }

    fn should_run_cycle(&self) -> bool {
        let summary = self.metrics.summary();
        summary.total_invocations >= self.config.min_invocations_for_analysis
    }

    async fn run_cycle(&mut self) {
        self.state.total_cycles += 1;
        self.state.last_cycle_at = Some(now_millis());

        match self.system.run_cycle(&self.metrics).await {
            Ok(result) => {
                if result.blocked {
                    tracing::warn!("Improvement cycle blocked by circuit breaker");
                } else if result.error.is_some() {
                    self.state.failed_cycles += 1;
                    tracing::error!(error = ?result.error, "Improvement cycle failed");
                } else {
                    self.state.successful_cycles += 1;
                    self.state.total_actions_executed += result
                        .execution_results
                        .iter()
                        .filter(|r| r.success)
                        .count() as u64;
                    tracing::info!(
                        actions_proposed = result
                            .analysis_result
                            .as_ref()
                            .map_or(0, |a| a.actions.len()),
                        actions_executed = result.execution_results.len(),
                        "Improvement cycle completed"
                    );
                }
            }
            Err(e) => {
                self.state.failed_cycles += 1;
                tracing::error!(error = %e, "Improvement cycle error");
            }
        }

        self.update_status();
    }

    async fn handle_command(&mut self, command: ManagerCommand) {
        match command {
            ManagerCommand::TriggerCycle { response_tx } => {
                let result = self.system.run_cycle(&self.metrics).await;
                if result.is_ok() {
                    self.state.total_cycles += 1;
                    self.state.last_cycle_at = Some(now_millis());
                    self.update_status();
                }
                let _ = response_tx.send(result);
            }
            ManagerCommand::Approve {
                diagnosis_id,
                response_tx,
            } => {
                let result = self.handle_approve(&diagnosis_id);
                let _ = response_tx.send(result);
            }
            ManagerCommand::Reject {
                diagnosis_id,
                reason,
                response_tx,
            } => {
                let result = self.handle_reject(&diagnosis_id, reason.as_deref());
                let _ = response_tx.send(result);
            }
            ManagerCommand::Rollback {
                action_id,
                response_tx,
            } => {
                let result = self.handle_rollback(&action_id);
                let _ = response_tx.send(result);
            }
            ManagerCommand::GetStatus { response_tx } => {
                let _ = response_tx.send(self.build_status());
            }
            ManagerCommand::GetPending { limit, response_tx } => {
                let pending = self.get_pending_diagnoses(limit);
                let _ = response_tx.send(pending);
            }
        }
    }

    fn handle_approve(&mut self, diagnosis_id: &str) -> Result<ApproveResult, String> {
        let pending = self.system.pending_actions();
        if !pending.iter().any(|a| a.id == diagnosis_id) {
            return Err(format!("Diagnosis not found: {diagnosis_id}"));
        }

        let (exec_results, learn_results) =
            self.system.approve_actions(&[diagnosis_id.to_string()]);

        self.state.total_actions_executed +=
            exec_results.iter().filter(|r| r.success).count() as u64;

        self.update_status();

        Ok(ApproveResult {
            diagnosis_id: diagnosis_id.to_string(),
            execution_results: exec_results
                .iter()
                .map(ExecutionResultSummary::from)
                .collect(),
            learning_results: learn_results
                .iter()
                .map(LearningResultSummary::from)
                .collect(),
        })
    }

    fn handle_reject(&self, diagnosis_id: &str, reason: Option<&str>) -> Result<(), String> {
        let pending = self.system.pending_actions();
        if !pending.iter().any(|a| a.id == diagnosis_id) {
            return Err(format!("Diagnosis not found: {diagnosis_id}"));
        }

        tracing::info!(
            diagnosis_id = diagnosis_id,
            reason = reason,
            "Diagnosis rejected"
        );

        // Remove the rejected diagnosis from pending
        // Note: The current system doesn't have a method to reject specific actions,
        // so we approve an empty list which effectively leaves it pending.
        // For now, we just log the rejection.
        // TODO: Add proper rejection handling to SelfImprovementSystem

        self.update_status();
        Ok(())
    }

    fn handle_rollback(&mut self, action_id: &str) -> Result<(), String> {
        let result = self.system.rollback(action_id);
        if result.is_ok() {
            self.state.total_actions_rolled_back += 1;
            self.update_status();
        }
        result
    }

    fn get_pending_diagnoses(&self, limit: Option<u32>) -> Vec<PendingDiagnosis> {
        let pending = self.system.pending_actions();
        let limit = limit.unwrap_or(100) as usize;
        pending
            .iter()
            .take(limit)
            .map(PendingDiagnosis::from)
            .collect()
    }

    fn build_status(&self) -> ManagerStatus {
        let circuit_state = match self.system.circuit_state() {
            CircuitState::Closed => "closed",
            CircuitState::Open => "open",
            CircuitState::HalfOpen => "half_open",
        };

        ManagerStatus {
            running: self.state.running,
            circuit_state: circuit_state.to_string(),
            total_cycles: self.state.total_cycles,
            successful_cycles: self.state.successful_cycles,
            failed_cycles: self.state.failed_cycles,
            pending_diagnoses: self.system.pending_actions().len(),
            total_actions_executed: self.state.total_actions_executed,
            total_actions_rolled_back: self.state.total_actions_rolled_back,
            last_cycle_at: self.state.last_cycle_at,
            learning_summary: self.system.learning_summary().into(),
        }
    }

    fn update_status(&self) {
        let _ = self.status_tx.send(self.build_status());
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::significant_drop_tightening
)]
mod tests {
    use super::*;
    use crate::storage::SqliteStorage;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

    fn mock_response(content: &str) -> CompletionResponse {
        CompletionResponse::new(content, Usage::new(100, 50))
    }

    fn create_mock_client() -> MockAnthropicClientTrait {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(
                r#"{
                "summary": "Test analysis",
                "confidence": 0.8,
                "actions": []
            }"#,
            ))
        });
        client
    }

    async fn create_test_storage() -> Arc<SelfImprovementStorage> {
        let sqlite_storage = SqliteStorage::new_in_memory().await.unwrap();
        Arc::new(SelfImprovementStorage::new(sqlite_storage.pool.clone()))
    }

    #[tokio::test]
    async fn test_manager_handle_clone() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (_manager, handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let handle2 = handle.clone();
        assert!(handle2.command_tx.capacity() > 0);
    }

    #[tokio::test]
    async fn test_manager_status_default() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (_manager, handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let status = handle.status_rx.borrow();
        assert!(status.running);
        assert_eq!(status.circuit_state, "closed");
        assert_eq!(status.total_cycles, 0);
    }

    #[test]
    fn test_pending_diagnosis_from_action() {
        use super::super::types::ActionType;

        let action = SelfImprovementAction::new(
            "test-id",
            ActionType::ConfigAdjust,
            "Test desc",
            "Rationale",
            0.15,
        );

        let pending = PendingDiagnosis::from(&action);

        assert_eq!(pending.id, "test-id");
        assert_eq!(pending.description, "Test desc");
        assert_eq!(pending.rationale, "Rationale");
        assert!((pending.expected_improvement - 0.15).abs() < 0.001);
    }

    #[test]
    fn test_learning_summary_data_from() {
        let summary = LearningSummary {
            total_lessons: 5,
            avg_reward: 0.75,
            successful: 4,
            failed: 1,
            by_type: std::collections::HashMap::new(),
        };

        let data = LearningSummaryData::from(summary);

        assert_eq!(data.total_lessons, 5);
        assert!((data.average_reward - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_execution_result_summary_from() {
        use super::super::types::ActionType;

        let action =
            SelfImprovementAction::new("action-1", ActionType::ConfigAdjust, "Test", "Test", 0.1);

        let result = ExecutionResult {
            action,
            success: true,
            message: "Success".to_string(),
            measured_improvement: Some(0.05),
        };

        let summary = ExecutionResultSummary::from(&result);

        assert_eq!(summary.action_id, "action-1");
        assert!(summary.success);
        assert_eq!(summary.message, "Success");
    }

    #[tokio::test]
    async fn test_manager_should_run_cycle() {
        let config = SelfImprovementConfig {
            min_invocations_for_analysis: 10,
            ..Default::default()
        };
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        // No invocations yet
        assert!(!manager.should_run_cycle());
    }

    #[tokio::test]
    async fn test_handle_status() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (_manager, handle) = SelfImprovementManager::new(config, client, metrics, storage);

        // Can get status through the receiver
        let status = handle.status_rx.borrow().clone();
        assert!(status.running);
    }

    #[test]
    fn test_manager_state_default() {
        let state = ManagerState::default();
        assert!(state.running);
        assert_eq!(state.total_cycles, 0);
        assert!(state.last_cycle_at.is_none());
    }

    #[test]
    fn test_manager_status_default_values() {
        let status = ManagerStatus::default();
        assert!(!status.running);
        assert_eq!(status.circuit_state, "Closed");
        assert_eq!(status.total_cycles, 0);
        assert_eq!(status.successful_cycles, 0);
        assert_eq!(status.failed_cycles, 0);
        assert_eq!(status.pending_diagnoses, 0);
        assert_eq!(status.total_actions_executed, 0);
        assert_eq!(status.total_actions_rolled_back, 0);
        assert!(status.last_cycle_at.is_none());
    }

    #[test]
    fn test_learning_summary_data_default() {
        let data = LearningSummaryData::default();
        assert_eq!(data.total_lessons, 0);
        assert!((data.average_reward - 0.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_handle_for_testing() {
        let handle = ManagerHandle::for_testing();

        // Status should return default since sender is dropped
        let status = handle.status_rx.borrow().clone();
        assert!(!status.running);
        assert_eq!(status.circuit_state, "Closed");
    }

    #[tokio::test]
    async fn test_handle_subscribe() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (_manager, handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let subscriber = handle.subscribe();
        let status = subscriber.borrow().clone();
        assert!(status.running);
    }

    #[tokio::test]
    async fn test_handle_trigger_cycle_disconnected() {
        let handle = ManagerHandle::for_testing();

        // Should fail because manager is not running
        let result = handle.trigger_cycle().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_approve_disconnected() {
        let handle = ManagerHandle::for_testing();

        let result = handle.approve("test-id".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not running"));
    }

    #[tokio::test]
    async fn test_handle_reject_disconnected() {
        let handle = ManagerHandle::for_testing();

        let result = handle
            .reject("test-id".to_string(), Some("reason".to_string()))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not running"));
    }

    #[tokio::test]
    async fn test_handle_rollback_disconnected() {
        let handle = ManagerHandle::for_testing();

        let result = handle.rollback("test-id".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not running"));
    }

    #[tokio::test]
    async fn test_handle_status_disconnected() {
        let handle = ManagerHandle::for_testing();

        // Should return default status when disconnected
        let status = handle.status().await;
        assert!(!status.running);
    }

    #[tokio::test]
    async fn test_handle_pending_diagnoses_disconnected() {
        let handle = ManagerHandle::for_testing();

        // Should return empty vector when disconnected
        let pending = handle.pending_diagnoses(Some(10)).await;
        assert!(pending.is_empty());
    }

    #[test]
    fn test_now_millis() {
        let millis1 = now_millis();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let millis2 = now_millis();

        assert!(millis2 >= millis1);
        // Should be reasonable timestamp (after year 2020)
        assert!(millis1 > 1_577_836_800_000);
    }

    #[test]
    fn test_learning_result_summary_from() {
        use super::super::learner::LearningResult;
        use super::super::types::Lesson;

        let lesson = Lesson {
            id: "lesson-1".to_string(),
            action_id: "action-123".to_string(),
            insight: "Test lesson learned".to_string(),
            reward: 0.85,
            applicable_contexts: vec![],
            created_at: 0,
        };

        let result = LearningResult {
            lesson: lesson.clone(),
            context: std::collections::HashMap::new(),
        };

        let summary = LearningResultSummary::from(&result);

        assert_eq!(summary.action_id, "action-123");
        assert_eq!(summary.lesson, "Test lesson learned");
        assert!((summary.reward - 0.85).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_manager_build_status() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let status = manager.build_status();
        assert!(status.running);
        assert_eq!(status.circuit_state, "closed");
        assert_eq!(status.total_cycles, 0);
    }

    #[tokio::test]
    async fn test_manager_get_pending_diagnoses() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let pending = manager.get_pending_diagnoses(Some(5));
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_manager_get_pending_diagnoses_no_limit() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let pending = manager.get_pending_diagnoses(None);
        assert!(pending.is_empty());
    }
}
