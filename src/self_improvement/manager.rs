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

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use chrono::Utc;

use crate::config::SelfImprovementConfig;
use crate::error::ModeError;
use crate::metrics::MetricsCollector;
use crate::traits::AnthropicClientTrait;

use super::circuit_breaker::CircuitState;
use super::executor::ExecutionResult;
use super::learner::{ActionTypeStats, LearningResult, LearningSummary};
use super::storage::{
    ActionRecord, ActionTypeStatRecord, ConfigOverrideRecord, DiagnosisRecord,
    SelfImprovementStorage,
};
use super::system::{CycleResult, SelfImprovementSystem, SystemConfig};
use super::types::{ActionStatus, ActionType, DiagnosisStatus, SelfImprovementAction, Severity};

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
    /// Get persisted config-override recommendations.
    GetConfigOverrides {
        /// Maximum number to return.
        limit: Option<u32>,
        /// Response channel.
        response_tx: oneshot::Sender<Vec<ConfigRecommendation>>,
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
    /// Total actions rejected.
    pub total_actions_rejected: u64,
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
            total_actions_rejected: 0,
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

/// Map a persisted diagnosis to the user-facing pending-diagnosis view.
///
/// A diagnosis is cycle-level (it can carry several proposed actions); this
/// flattens it to one entry keyed by the diagnosis id, summarizing its actions.
fn diagnosis_to_pending(record: DiagnosisRecord) -> PendingDiagnosis {
    let actions: Vec<SelfImprovementAction> =
        serde_json::from_str(&record.suggested_action_json).unwrap_or_default();

    let action_type = match actions.as_slice() {
        [] => "Unknown".to_string(),
        [single] => format!("{:?}", single.action_type),
        _ => "Multiple".to_string(),
    };
    let rationale = record.action_rationale.clone().unwrap_or_else(|| {
        actions
            .first()
            .map_or_else(String::new, |a| a.rationale.clone())
    });
    let expected_improvement = actions
        .iter()
        .map(|a| a.expected_improvement)
        .fold(0.0_f64, f64::max);

    PendingDiagnosis {
        id: record.id,
        action_type,
        description: record.description,
        rationale,
        expected_improvement,
        created_at: u64::try_from(record.created_at.timestamp_millis()).unwrap_or(0),
    }
}

/// A persisted config-override recommendation produced by a successful
/// self-improvement action.
///
/// These are advisory: SI records what it would change (keyed by the real
/// `Config` field) but does not apply it to the running server. This type is
/// the read side that surfaces them so the owner does not have to query the
/// database directly.
#[derive(Debug, Clone)]
pub struct ConfigRecommendation {
    /// Config key (a real `Config` field, or `threshold:<name>`).
    pub key: String,
    /// Recommended value (JSON).
    pub value: serde_json::Value,
    /// The `si_actions` row id that produced this recommendation, if any.
    pub applied_by_action: Option<String>,
    /// When the recommendation was last updated (RFC 3339).
    pub updated_at: String,
}

impl From<ConfigOverrideRecord> for ConfigRecommendation {
    fn from(record: ConfigOverrideRecord) -> Self {
        // value_json is stored as serialized JSON; surface it parsed, falling
        // back to the raw string if it is not valid JSON.
        let value = serde_json::from_str(&record.value_json)
            .unwrap_or(serde_json::Value::String(record.value_json));
        Self {
            key: record.key,
            value,
            applied_by_action: record.applied_by_action,
            updated_at: record.updated_at.to_rfc3339(),
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

    /// Get persisted config-override recommendations.
    pub async fn config_overrides(&self, limit: Option<u32>) -> Vec<ConfigRecommendation> {
        let (response_tx, response_rx) = oneshot::channel();
        if self
            .command_tx
            .send(ManagerCommand::GetConfigOverrides { limit, response_tx })
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
    total_actions_rejected: u64,
    last_cycle_at: Option<u64>,
    /// Cached count of persisted `Pending` diagnoses (the DB-backed approval
    /// queue), refreshed after each cycle/approve/reject so `build_status` can
    /// stay synchronous.
    pending_diagnoses: usize,
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
            total_actions_rejected: 0,
            last_cycle_at: None,
            pending_diagnoses: 0,
        }
    }
}

/// Get current timestamp as Unix epoch milliseconds.
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// The self-improvement manager.
///
/// Runs as a background task and coordinates the self-improvement system.
pub struct SelfImprovementManager<C: AnthropicClientTrait> {
    config: SelfImprovementConfig,
    system: SelfImprovementSystem<C>,
    metrics: Arc<MetricsCollector>,
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
            total_actions_rejected: 0,
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

        // Restore learning effectiveness persisted by prior runs so guidance is
        // warm from the first cycle instead of resetting on every restart.
        self.load_learner_stats().await;

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
        let result = self.system.run_cycle(&self.metrics).await;
        self.record_cycle_result(&result);
        if let Err(e) = self.persist_cycle_audit(&result).await {
            tracing::error!(error = %e, "Failed to persist self-improvement cycle audit");
        }
        self.persist_learner_stats().await;
        self.refresh_pending_count().await;
        self.update_status();
    }

    /// Restore the Learner's per-action-type effectiveness from storage so SI
    /// guidance survives restarts. Best-effort: a storage error logs and leaves
    /// the Learner empty (it re-accumulates from new cycles). Unknown persisted
    /// action types are skipped.
    async fn load_learner_stats(&mut self) {
        let records = match self.storage.get_all_action_type_stats().await {
            Ok(records) => records,
            Err(e) => {
                tracing::error!(error = %e, "Failed to load persisted SI learning stats");
                return;
            }
        };
        if records.is_empty() {
            return;
        }

        let mut stats: HashMap<ActionType, ActionTypeStats> = HashMap::new();
        for record in records {
            match record.action_type.parse::<ActionType>() {
                Ok(action_type) => {
                    stats.insert(
                        action_type,
                        ActionTypeStats {
                            total_executions: u64::try_from(record.total_executions).unwrap_or(0),
                            successful: u64::try_from(record.successful).unwrap_or(0),
                            avg_reward: record.avg_reward,
                            total_expected: record.total_expected,
                            total_actual: record.total_actual,
                        },
                    );
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Skipping unknown persisted SI action type");
                }
            }
        }

        let restored = stats.len();
        self.system.seed_learner_stats(stats);
        tracing::info!(
            action_types = restored,
            "Restored SI learning stats from storage"
        );
    }

    /// Persist the Learner's current per-action-type effectiveness so it survives
    /// restarts. Called after each cycle; best-effort (errors are logged, not
    /// propagated, so a storage hiccup never aborts the improvement loop).
    async fn persist_learner_stats(&self) {
        // Snapshot to owned records first so the borrow of `self.system` is
        // released before the upsert awaits.
        let records: Vec<ActionTypeStatRecord> = self
            .system
            .learner_stats()
            .iter()
            .map(|(action_type, stats)| ActionTypeStatRecord {
                action_type: action_type.to_string(),
                total_executions: i64::try_from(stats.total_executions).unwrap_or(i64::MAX),
                successful: i64::try_from(stats.successful).unwrap_or(i64::MAX),
                avg_reward: stats.avg_reward,
                total_expected: stats.total_expected,
                total_actual: stats.total_actual,
            })
            .collect();

        for record in records {
            if let Err(e) = self.storage.upsert_action_type_stats(&record).await {
                tracing::error!(
                    error = %e,
                    action_type = %record.action_type,
                    "Failed to persist SI learning stats"
                );
            }
        }
    }

    /// Fold a completed cycle's outcome into the aggregate counters and refresh
    /// status. Shared by the scheduled loop and the manual `TriggerCycle`
    /// command so both account outcomes identically — previously the trigger
    /// path bumped only `total_cycles`, leaving `successful_cycles`,
    /// `failed_cycles`, and `total_actions_executed` permanently at zero.
    fn record_cycle_result(&mut self, result: &Result<CycleResult, ModeError>) {
        self.state.total_cycles += 1;
        self.state.last_cycle_at = Some(now_millis());

        match result {
            Ok(cycle) => {
                if cycle.blocked {
                    tracing::warn!("Improvement cycle blocked by circuit breaker");
                } else if cycle.error.is_some() {
                    self.state.failed_cycles += 1;
                    tracing::error!(error = ?cycle.error, "Improvement cycle failed");
                } else {
                    let succeeded = cycle.execution_results.iter().filter(|r| r.success).count();
                    self.state.total_actions_executed += succeeded as u64;
                    if !cycle.execution_results.is_empty() && succeeded == 0 {
                        // Every proposed action failed to execute. The circuit
                        // breaker records these failures, so the cycle must not
                        // read as a success (which is what let the breaker open
                        // while successful_cycles still climbed).
                        self.state.failed_cycles += 1;
                        tracing::warn!(
                            attempted = cycle.execution_results.len(),
                            "Improvement cycle executed but every action failed"
                        );
                    } else {
                        self.state.successful_cycles += 1;
                        tracing::info!(
                            actions_proposed = cycle
                                .analysis_result
                                .as_ref()
                                .map_or(0, |a| a.actions.len()),
                            actions_executed = succeeded,
                            "Improvement cycle completed"
                        );
                    }
                }
            }
            Err(e) => {
                self.state.failed_cycles += 1;
                tracing::error!(error = %e, "Improvement cycle error");
            }
        }

        self.update_status();
    }

    /// Write a cycle's diagnosis and per-action outcomes to the audit tables so
    /// self-modification attempts leave a record.
    ///
    /// Audit only: the in-memory approve/reject runtime is unchanged; these rows
    /// are not read back (the previously-empty `diagnoses` / `si_actions` tables
    /// are the inspectable trail). A no-op for skipped/blocked cycles that
    /// produced no analysis. Errors are returned so the caller can surface them;
    /// they do not undo the already-completed cycle.
    async fn persist_cycle_audit(
        &self,
        result: &Result<CycleResult, ModeError>,
    ) -> Result<(), ModeError> {
        let Ok(cycle) = result else { return Ok(()) };
        let Some(analysis) = &cycle.analysis_result else {
            return Ok(());
        };

        let severity = cycle
            .monitor_result
            .triggers
            .iter()
            .map(|t| t.severity)
            .max()
            .unwrap_or(Severity::Info);
        let outcomes: Vec<ActionStatus> = cycle
            .execution_results
            .iter()
            .map(|exec| self.action_outcome(exec))
            .collect();
        let status = Self::diagnosis_status_from(&outcomes);

        let diagnosis_id = uuid::Uuid::new_v4().to_string();
        let diagnosis = DiagnosisRecord {
            id: diagnosis_id.clone(),
            trigger_type: "cycle".to_string(),
            trigger_json: serde_json::to_string(&cycle.monitor_result.metrics)
                .unwrap_or_else(|_| "{}".to_string()),
            severity,
            description: analysis.summary.clone(),
            suspected_cause: None,
            suggested_action_json: serde_json::to_string(&analysis.actions)
                .unwrap_or_else(|_| "[]".to_string()),
            action_rationale: None,
            status,
            created_at: Utc::now(),
        };
        self.storage
            .insert_diagnosis(&diagnosis)
            .await
            .map_err(|e| ModeError::StorageError {
                message: e.to_string(),
            })?;

        for exec in &cycle.execution_results {
            self.persist_action_outcome(&diagnosis_id, exec).await?;
        }

        Ok(())
    }

    /// Write one executed action's outcome to `si_actions`, and (on success)
    /// its config recommendation to `config_overrides`.
    ///
    /// Shared by the cycle-audit path and the approval path so both leave an
    /// identical record. Returns the inserted `si_actions` row id.
    async fn persist_action_outcome(
        &self,
        diagnosis_id: &str,
        exec: &ExecutionResult,
    ) -> Result<String, ModeError> {
        let action_record_id = uuid::Uuid::new_v4().to_string();
        let record = ActionRecord {
            id: action_record_id.clone(),
            diagnosis_id: diagnosis_id.to_string(),
            action_type: format!("{:?}", exec.action.action_type),
            action_json: serde_json::to_string(&exec.action).unwrap_or_else(|_| "{}".to_string()),
            outcome: self.action_outcome(exec),
            pre_metrics_json: "{}".to_string(),
            post_metrics_json: None,
            execution_time_ms: 0,
            error_message: (!exec.success).then(|| exec.message.clone()),
            created_at: Utc::now(),
        };
        self.storage
            .insert_action(&record)
            .await
            .map_err(|e| ModeError::StorageError {
                message: e.to_string(),
            })?;

        if exec.success {
            self.persist_config_recommendations(&action_record_id, &exec.action)
                .await?;
        }

        Ok(action_record_id)
    }

    /// The accurate recorded outcome for an executed action.
    ///
    /// A failed action is `Failed`. A successful action is `Completed` only when
    /// its effect actually reaches the running system — a `LogObservation`
    /// (which logs), or a `ConfigAdjust`/`ThresholdAdjust` when override
    /// application is enabled (so the change is applied at the next startup).
    /// Otherwise the action only recorded an advisory recommendation that never
    /// touches the live server, so it is `Recommended` — not `Completed`.
    fn action_outcome(&self, exec: &ExecutionResult) -> ActionStatus {
        if !exec.success {
            return ActionStatus::Failed;
        }
        match exec.action.action_type {
            ActionType::LogObservation => ActionStatus::Completed,
            ActionType::ConfigAdjust | ActionType::ThresholdAdjust
                if self.config.apply_config_overrides =>
            {
                ActionStatus::Completed
            }
            _ => ActionStatus::Recommended,
        }
    }

    /// Derive a diagnosis (cycle) status from its actions' recorded outcomes.
    ///
    /// `Executed` if any action's effect reached the running system; otherwise
    /// `Recommended` if any action recorded a recommendation; `Failed` if every
    /// action failed; `Pending` when there were no executed actions.
    fn diagnosis_status_from(outcomes: &[ActionStatus]) -> DiagnosisStatus {
        if outcomes.is_empty() {
            DiagnosisStatus::Pending
        } else if outcomes.contains(&ActionStatus::Completed) {
            DiagnosisStatus::Executed
        } else if outcomes.contains(&ActionStatus::Recommended) {
            DiagnosisStatus::Recommended
        } else {
            DiagnosisStatus::Failed
        }
    }

    /// Persist a successful config/threshold action's concrete parameters to the
    /// `config_overrides` table as advisory recommendations.
    ///
    /// Advisory only: these rows are a durable, attributable record of what SI
    /// proposes — they are NOT read back into the live `Config`, so the running
    /// server's settings do not change on their own. The owner reviews them (the
    /// table is keyed by the real `Config` field, with `applied_by_action`
    /// pointing at the producing action) and applies any they want by hand.
    /// Other action types (`prompt_tune`, `log_observation`) carry no
    /// `Config`-field change and are skipped.
    ///
    /// `action_record_id` is the id of the just-inserted `si_actions` row;
    /// `config_overrides.applied_by_action` is a foreign key onto it.
    async fn persist_config_recommendations(
        &self,
        action_record_id: &str,
        action: &SelfImprovementAction,
    ) -> Result<(), ModeError> {
        let Some(params) = action
            .parameters
            .as_ref()
            .and_then(serde_json::Value::as_object)
        else {
            return Ok(());
        };

        // (override key, value) pairs to record for this action. ConfigAdjust
        // and ThresholdAdjust both carry real, applyable `Config` field keys, so
        // their parameters are recorded directly.
        let overrides: Vec<(String, serde_json::Value)> = match action.action_type {
            ActionType::ConfigAdjust | ActionType::ThresholdAdjust => {
                params.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            }
            // ProposePR carries no applyable `Config` field — its artifact is a PR,
            // recorded elsewhere — so there is nothing to persist as an override.
            ActionType::PromptTune | ActionType::LogObservation | ActionType::ProposePR => {
                Vec::new()
            }
        };

        for (key, value) in overrides {
            let record = ConfigOverrideRecord {
                key,
                value_json: value.to_string(),
                applied_by_action: Some(action_record_id.to_string()),
                updated_at: Utc::now(),
            };
            self.storage
                .upsert_config_override(&record)
                .await
                .map_err(|e| ModeError::StorageError {
                    message: e.to_string(),
                })?;
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: ManagerCommand) {
        match command {
            ManagerCommand::TriggerCycle { response_tx } => {
                let result = self.system.run_cycle(&self.metrics).await;
                self.record_cycle_result(&result);
                if let Err(e) = self.persist_cycle_audit(&result).await {
                    tracing::error!(error = %e, "Failed to persist self-improvement cycle audit");
                }
                self.persist_learner_stats().await;
                self.refresh_pending_count().await;
                self.update_status();
                let _ = response_tx.send(result);
            }
            ManagerCommand::Approve {
                diagnosis_id,
                response_tx,
            } => {
                let result = self.handle_approve(&diagnosis_id).await;
                let _ = response_tx.send(result);
            }
            ManagerCommand::Reject {
                diagnosis_id,
                reason,
                response_tx,
            } => {
                let result = self.handle_reject(&diagnosis_id, reason.as_deref()).await;
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
                let pending = self.get_pending_diagnoses(limit).await;
                let _ = response_tx.send(pending);
            }
            ManagerCommand::GetConfigOverrides { limit, response_tx } => {
                let overrides = self.get_config_recommendations(limit).await;
                let _ = response_tx.send(overrides);
            }
        }
    }

    /// Read persisted config-override recommendations from storage.
    ///
    /// The read side of the advisory store written by
    /// [`Self::persist_config_recommendations`]. Returns an empty list on a
    /// storage error (logged) rather than failing the command — this is a
    /// read-only inspection path. Newest first, capped by `limit`.
    async fn get_config_recommendations(&self, limit: Option<u32>) -> Vec<ConfigRecommendation> {
        let mut records = match self.storage.get_all_config_overrides().await {
            Ok(records) => records,
            Err(e) => {
                tracing::error!(error = %e, "Failed to read config overrides");
                return Vec::new();
            }
        };
        records.sort_by_key(|r| std::cmp::Reverse(r.updated_at));
        if let Some(limit) = limit {
            records.truncate(limit as usize);
        }
        records
            .into_iter()
            .map(ConfigRecommendation::from)
            .collect()
    }

    /// Approve a persisted diagnosis: reconstruct its proposed actions, execute
    /// them, record outcomes, and mark the diagnosis `Executed`/`Failed`.
    ///
    /// Operates on the DB-backed `diagnoses` table (the same store
    /// [`Self::get_pending_diagnoses`] surfaces), so the id a caller approves is
    /// the id they saw, and a diagnosis persisted before a restart can still be
    /// approved.
    async fn handle_approve(&mut self, diagnosis_id: &str) -> Result<ApproveResult, String> {
        let diagnosis = self.load_pending_diagnosis(diagnosis_id).await?;
        let actions: Vec<SelfImprovementAction> =
            serde_json::from_str(&diagnosis.suggested_action_json)
                .map_err(|e| format!("Could not parse stored actions: {e}"))?;

        let (exec_results, learn_results) = self.system.execute_approved(actions);

        for exec in &exec_results {
            if let Err(e) = self.persist_action_outcome(diagnosis_id, exec).await {
                tracing::error!(error = %e, "Failed to persist approved action outcome");
            }
        }

        let succeeded = exec_results.iter().filter(|r| r.success).count();
        let outcomes: Vec<ActionStatus> = exec_results
            .iter()
            .map(|exec| self.action_outcome(exec))
            .collect();
        let new_status = Self::diagnosis_status_from(&outcomes);
        if let Err(e) = self
            .storage
            .update_diagnosis_status(diagnosis_id, new_status)
            .await
        {
            tracing::error!(error = %e, "Failed to update diagnosis status after approve");
        }

        self.state.total_actions_executed += succeeded as u64;
        self.refresh_pending_count().await;
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

    /// Reject a persisted diagnosis: record a rejection lesson per proposed
    /// action and mark the diagnosis `Rejected`. No actions execute.
    async fn handle_reject(
        &mut self,
        diagnosis_id: &str,
        reason: Option<&str>,
    ) -> Result<(), String> {
        let diagnosis = self.load_pending_diagnosis(diagnosis_id).await?;

        if let Ok(actions) =
            serde_json::from_str::<Vec<SelfImprovementAction>>(&diagnosis.suggested_action_json)
        {
            for action in &actions {
                self.system.record_rejection(action, reason);
            }
        }

        self.storage
            .update_diagnosis_status(diagnosis_id, DiagnosisStatus::Rejected)
            .await
            .map_err(|e| e.to_string())?;

        self.state.total_actions_rejected += 1;
        self.refresh_pending_count().await;
        self.update_status();
        Ok(())
    }

    /// Load a diagnosis and require it to be `Pending` (the only state that can
    /// be approved or rejected).
    async fn load_pending_diagnosis(&self, diagnosis_id: &str) -> Result<DiagnosisRecord, String> {
        let diagnosis = self
            .storage
            .get_diagnosis(diagnosis_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Diagnosis not found: {diagnosis_id}"))?;
        if diagnosis.status != DiagnosisStatus::Pending {
            return Err(format!(
                "Diagnosis {diagnosis_id} is not pending (status: {:?})",
                diagnosis.status
            ));
        }
        Ok(diagnosis)
    }

    fn handle_rollback(&mut self, action_id: &str) -> Result<(), String> {
        let result = self.system.rollback(action_id);
        if result.is_ok() {
            self.state.total_actions_rolled_back += 1;
            self.update_status();
        }
        result
    }

    /// List persisted `Pending` diagnoses (the DB-backed approval queue).
    ///
    /// Reads the `diagnoses` table rather than the in-memory pending list, so a
    /// diagnosis survives a restart and the id returned here is the id
    /// [`Self::handle_approve`] accepts. One entry per diagnosis (cycle), keyed
    /// by the diagnosis id.
    async fn get_pending_diagnoses(&self, limit: Option<u32>) -> Vec<PendingDiagnosis> {
        let diagnoses = match self.storage.get_pending_diagnoses().await {
            Ok(diagnoses) => diagnoses,
            Err(e) => {
                tracing::error!(error = %e, "Failed to read pending diagnoses");
                return Vec::new();
            }
        };
        let limit = limit.unwrap_or(100) as usize;
        diagnoses
            .into_iter()
            .take(limit)
            .map(diagnosis_to_pending)
            .collect()
    }

    /// Refresh the cached `Pending` diagnosis count from storage.
    async fn refresh_pending_count(&mut self) {
        self.state.pending_diagnoses = self
            .storage
            .get_pending_diagnoses()
            .await
            .map_or(self.state.pending_diagnoses, |d| d.len());
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
            pending_diagnoses: self.state.pending_diagnoses,
            total_actions_executed: self.state.total_actions_executed,
            total_actions_rolled_back: self.state.total_actions_rolled_back,
            total_actions_rejected: self.state.total_actions_rejected,
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
    async fn test_learner_stats_persist_across_restart() {
        use super::super::learner::ActionTypeStats;
        use super::super::types::ActionType;

        // Two managers sharing ONE database simulates a process restart.
        let sqlite = SqliteStorage::new_in_memory().await.unwrap();
        let storage_a = Arc::new(SelfImprovementStorage::new(sqlite.pool.clone()));
        let storage_b = Arc::new(SelfImprovementStorage::new(sqlite.pool.clone()));

        let (mut manager_a, _h1) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            storage_a,
        );

        // The first run accumulates effectiveness, then persists it.
        let mut stats = HashMap::new();
        stats.insert(
            ActionType::ConfigAdjust,
            ActionTypeStats {
                total_executions: 6,
                successful: 5,
                avg_reward: 0.6,
                total_expected: 0.5,
                total_actual: 0.6,
            },
        );
        manager_a.system.seed_learner_stats(stats);
        manager_a.persist_learner_stats().await;

        // A fresh manager (the "restart") starts empty, then restores from storage.
        let (mut manager_b, _h2) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            storage_b,
        );
        assert!(manager_b.system.learner_stats().is_empty());

        manager_b.load_learner_stats().await;

        let restored = manager_b.system.learner_stats();
        let cfg = restored
            .get(&ActionType::ConfigAdjust)
            .expect("config_adjust effectiveness restored after restart");
        assert_eq!(cfg.total_executions, 6);
        assert_eq!(cfg.successful, 5);
        assert!((cfg.avg_reward - 0.6).abs() < f64::EPSILON);
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
    async fn test_record_cycle_result_updates_outcome_counters() {
        use super::super::monitor::MonitorResult;
        use super::super::types::SystemMetrics;

        let (mut manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            create_test_storage().await,
        );

        let cycle = |error: Option<String>| -> Result<CycleResult, ModeError> {
            Ok(CycleResult {
                monitor_result: MonitorResult {
                    metrics: SystemMetrics::new(1.0, 0.0, 0, std::collections::HashMap::new()),
                    triggers: vec![],
                    action_recommended: false,
                },
                analysis_result: None,
                execution_results: vec![],
                learning_results: vec![],
                blocked: false,
                error,
            })
        };

        // A clean cycle is a success and advances total_cycles — the manual
        // trigger path previously left these at zero.
        manager.record_cycle_result(&cycle(None));
        assert_eq!(manager.state.total_cycles, 1);
        assert_eq!(manager.state.successful_cycles, 1);
        assert_eq!(manager.state.failed_cycles, 0);

        // A cycle carrying an error is counted as a failure.
        manager.record_cycle_result(&cycle(Some("boom".to_string())));
        assert_eq!(manager.state.total_cycles, 2);
        assert_eq!(manager.state.failed_cycles, 1);

        let status = manager.build_status();
        assert_eq!(status.total_cycles, 2);
        assert_eq!(status.successful_cycles, 1);
        assert_eq!(status.failed_cycles, 1);
    }

    #[tokio::test]
    async fn test_cycle_with_all_failed_actions_counts_as_failed() {
        use super::super::executor::ExecutionResult;
        use super::super::monitor::MonitorResult;
        use super::super::types::{ActionType, SelfImprovementAction, SystemMetrics};

        let (mut manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            create_test_storage().await,
        );

        let cycle: Result<CycleResult, ModeError> = Ok(CycleResult {
            monitor_result: MonitorResult {
                metrics: SystemMetrics::new(1.0, 0.0, 0, std::collections::HashMap::new()),
                triggers: vec![],
                action_recommended: true,
            },
            analysis_result: None,
            // One proposed action that failed to execute.
            execution_results: vec![ExecutionResult {
                action: SelfImprovementAction::new(
                    "a1",
                    ActionType::ConfigAdjust,
                    "desc",
                    "rationale",
                    0.1,
                ),
                success: false,
                message: "No parameters provided".to_string(),
                measured_improvement: None,
            }],
            learning_results: vec![],
            blocked: false,
            error: None,
        });

        manager.record_cycle_result(&cycle);

        // A cycle whose every action failed is a failure, not a success — this
        // is what previously let the circuit breaker open while the manager
        // still reported a successful cycle.
        assert_eq!(manager.state.failed_cycles, 1);
        assert_eq!(manager.state.successful_cycles, 0);
        assert_eq!(manager.state.total_actions_executed, 0);
    }

    #[tokio::test]
    async fn test_persist_cycle_audit_writes_diagnosis_and_actions() {
        use super::super::analyzer::AnalysisResult;
        use super::super::executor::ExecutionResult;
        use super::super::monitor::MonitorResult;
        use super::super::types::{ActionType, SystemMetrics};

        let storage = create_test_storage().await;
        let (manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            Arc::clone(&storage),
        );

        let action = SelfImprovementAction::new("a1", ActionType::ConfigAdjust, "d", "r", 0.1);
        let cycle: Result<CycleResult, ModeError> = Ok(CycleResult {
            monitor_result: MonitorResult {
                metrics: SystemMetrics::new(0.9, 50.0, 12, std::collections::HashMap::new()),
                triggers: vec![],
                action_recommended: true,
            },
            analysis_result: Some(AnalysisResult {
                actions: vec![action.clone()],
                summary: "cycle summary".to_string(),
                confidence: 0.8,
            }),
            execution_results: vec![ExecutionResult {
                action,
                success: true,
                message: "applied".to_string(),
                measured_improvement: None,
            }],
            learning_results: vec![],
            blocked: false,
            error: None,
        });

        manager.persist_cycle_audit(&cycle).await.expect("persist");

        // Advisory default: a successful config_adjust only recorded a
        // recommendation (nothing applied), so the accurate status is
        // Recommended, not Executed/Completed.
        let diags = storage
            .get_diagnoses_by_status(DiagnosisStatus::Recommended)
            .await
            .expect("diagnoses");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].description, "cycle summary");

        let actions = storage
            .get_actions_by_outcome(ActionStatus::Recommended, 10)
            .await
            .expect("actions");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].diagnosis_id, diags[0].id);
    }

    #[tokio::test]
    async fn test_persist_cycle_audit_records_config_recommendations() {
        use super::super::analyzer::AnalysisResult;
        use super::super::executor::ExecutionResult;
        use super::super::monitor::MonitorResult;
        use super::super::types::SystemMetrics;

        let storage = create_test_storage().await;
        let (manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            Arc::clone(&storage),
        );

        // A successful config_adjust carrying a real Config field.
        let action = SelfImprovementAction::new("a1", ActionType::ConfigAdjust, "d", "r", 0.1)
            .with_parameters(serde_json::json!({ "request_timeout_ms": 45000 }));
        let cycle: Result<CycleResult, ModeError> = Ok(CycleResult {
            monitor_result: MonitorResult {
                metrics: SystemMetrics::new(0.9, 50.0, 12, std::collections::HashMap::new()),
                triggers: vec![],
                action_recommended: true,
            },
            analysis_result: Some(AnalysisResult {
                actions: vec![action.clone()],
                summary: "s".to_string(),
                confidence: 0.8,
            }),
            execution_results: vec![ExecutionResult {
                action,
                success: true,
                message: "applied".to_string(),
                measured_improvement: None,
            }],
            learning_results: vec![],
            blocked: false,
            error: None,
        });

        manager.persist_cycle_audit(&cycle).await.expect("persist");

        // The successful action's concrete change is recorded as an advisory
        // override keyed by the real Config field — attributed to the action,
        // and NOT applied to the live server.
        let override_row = storage
            .get_config_override("request_timeout_ms")
            .await
            .expect("query")
            .expect("override recorded");
        assert_eq!(override_row.value_json, "45000");
        // Attributed to the si_actions row that produced it (FK target).
        let action_id = override_row
            .applied_by_action
            .expect("override attributed to an action");
        // Advisory default: the action's recorded outcome is Recommended.
        let actions = storage
            .get_actions_by_outcome(ActionStatus::Recommended, 10)
            .await
            .expect("actions");
        assert!(actions.iter().any(|a| a.id == action_id));
    }

    #[tokio::test]
    async fn test_failed_action_records_no_config_recommendation() {
        use super::super::analyzer::AnalysisResult;
        use super::super::executor::ExecutionResult;
        use super::super::monitor::MonitorResult;
        use super::super::types::SystemMetrics;

        let storage = create_test_storage().await;
        let (manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            Arc::clone(&storage),
        );

        let action = SelfImprovementAction::new("a1", ActionType::ConfigAdjust, "d", "r", 0.1)
            .with_parameters(serde_json::json!({ "request_timeout_ms": 45000 }));
        let cycle: Result<CycleResult, ModeError> = Ok(CycleResult {
            monitor_result: MonitorResult {
                metrics: SystemMetrics::new(0.9, 50.0, 12, std::collections::HashMap::new()),
                triggers: vec![],
                action_recommended: true,
            },
            analysis_result: Some(AnalysisResult {
                actions: vec![action.clone()],
                summary: "s".to_string(),
                confidence: 0.8,
            }),
            execution_results: vec![ExecutionResult {
                action,
                success: false,
                message: "boom".to_string(),
                measured_improvement: None,
            }],
            learning_results: vec![],
            blocked: false,
            error: None,
        });

        manager.persist_cycle_audit(&cycle).await.expect("persist");

        // A failed action leaves no recommendation behind.
        assert!(storage
            .get_config_override("request_timeout_ms")
            .await
            .expect("query")
            .is_none());
    }

    // Build a single-successful-action cycle for outcome tests.
    fn cycle_with_one_success(action: SelfImprovementAction) -> Result<CycleResult, ModeError> {
        use super::super::analyzer::AnalysisResult;
        use super::super::executor::ExecutionResult;
        use super::super::monitor::MonitorResult;
        use super::super::types::SystemMetrics;
        Ok(CycleResult {
            monitor_result: MonitorResult {
                metrics: SystemMetrics::new(0.9, 50.0, 12, std::collections::HashMap::new()),
                triggers: vec![],
                action_recommended: true,
            },
            analysis_result: Some(AnalysisResult {
                actions: vec![action.clone()],
                summary: "s".to_string(),
                confidence: 0.8,
            }),
            execution_results: vec![ExecutionResult {
                action,
                success: true,
                message: "ok".to_string(),
                measured_improvement: None,
            }],
            learning_results: vec![],
            blocked: false,
            error: None,
        })
    }

    #[tokio::test]
    async fn test_apply_mode_config_action_is_completed() {
        // With override application enabled, a successful config_adjust reaches
        // the live server (at restart), so it is genuinely Completed / Executed.
        let storage = create_test_storage().await;
        let config = SelfImprovementConfig {
            apply_config_overrides: true,
            ..Default::default()
        };
        let (manager, _handle) = SelfImprovementManager::new(
            config,
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            Arc::clone(&storage),
        );

        let action = SelfImprovementAction::new("a1", ActionType::ConfigAdjust, "d", "r", 0.1)
            .with_parameters(serde_json::json!({ "request_timeout_ms": 45000 }));
        manager
            .persist_cycle_audit(&cycle_with_one_success(action))
            .await
            .expect("persist");

        assert_eq!(
            storage
                .get_diagnoses_by_status(DiagnosisStatus::Executed)
                .await
                .expect("diags")
                .len(),
            1
        );
        assert_eq!(
            storage
                .get_actions_by_outcome(ActionStatus::Completed, 10)
                .await
                .expect("actions")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn test_log_observation_is_completed_in_advisory_mode() {
        // LogObservation genuinely completes (it logs) regardless of apply mode.
        let storage = create_test_storage().await;
        let (manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            Arc::clone(&storage),
        );

        let action = SelfImprovementAction::new("a1", ActionType::LogObservation, "d", "r", 0.1);
        manager
            .persist_cycle_audit(&cycle_with_one_success(action))
            .await
            .expect("persist");

        assert_eq!(
            storage
                .get_actions_by_outcome(ActionStatus::Completed, 10)
                .await
                .expect("actions")
                .len(),
            1
        );
        assert_eq!(
            storage
                .get_diagnoses_by_status(DiagnosisStatus::Executed)
                .await
                .expect("diags")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn test_get_config_recommendations_reads_sorts_and_limits() {
        use super::super::storage::ConfigOverrideRecord;

        let storage = create_test_storage().await;
        let (manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            Arc::clone(&storage),
        );

        let ts = |s: &str| {
            chrono::DateTime::parse_from_rfc3339(s)
                .expect("parse ts")
                .with_timezone(&Utc)
        };
        // applied_by_action = None sidesteps the si_actions foreign key here;
        // this test exercises the read/sort/limit path, not attribution.
        storage
            .upsert_config_override(&ConfigOverrideRecord {
                key: "max_retries".to_string(),
                value_json: "5".to_string(),
                applied_by_action: None,
                updated_at: ts("2026-06-01T00:00:00Z"),
            })
            .await
            .expect("seed older");
        storage
            .upsert_config_override(&ConfigOverrideRecord {
                key: "request_timeout_ms".to_string(),
                value_json: "45000".to_string(),
                applied_by_action: None,
                updated_at: ts("2026-06-02T00:00:00Z"),
            })
            .await
            .expect("seed newer");

        // Newest first, JSON value parsed (not the raw string).
        let recs = manager.get_config_recommendations(None).await;
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].key, "request_timeout_ms");
        assert_eq!(recs[0].value, serde_json::json!(45000));
        assert_eq!(recs[1].key, "max_retries");

        // Limit caps the result.
        let limited = manager.get_config_recommendations(Some(1)).await;
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].key, "request_timeout_ms");
    }

    #[test]
    fn test_config_recommendation_from_record_non_json_value() {
        use super::super::storage::ConfigOverrideRecord;

        // A value that is not valid JSON falls back to a string rather than
        // being dropped.
        let rec = ConfigOverrideRecord {
            key: "k".to_string(),
            value_json: "not json {".to_string(),
            applied_by_action: Some("act-1".to_string()),
            updated_at: Utc::now(),
        };
        let mapped = ConfigRecommendation::from(rec);
        assert_eq!(
            mapped.value,
            serde_json::Value::String("not json {".to_string())
        );
        assert_eq!(mapped.applied_by_action.as_deref(), Some("act-1"));
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

        let pending = manager.get_pending_diagnoses(Some(5)).await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_manager_get_pending_diagnoses_no_limit() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let pending = manager.get_pending_diagnoses(None).await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_manager_handle_reject_not_found() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let result = manager.handle_reject("nonexistent", Some("reason")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_manager_handle_approve_not_found() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let result = manager.handle_approve("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // Seed a persisted Pending diagnosis carrying one config_adjust action.
    async fn seed_pending_diagnosis(
        storage: &SelfImprovementStorage,
        id: &str,
        status: DiagnosisStatus,
    ) {
        use super::super::types::ActionType;
        let action = SelfImprovementAction::new("act-1", ActionType::ConfigAdjust, "d", "r", 0.1)
            .with_parameters(serde_json::json!({ "request_timeout_ms": 45000 }));
        let record = DiagnosisRecord {
            id: id.to_string(),
            trigger_type: "cycle".to_string(),
            trigger_json: "{}".to_string(),
            severity: Severity::Warning,
            description: "seeded".to_string(),
            suspected_cause: None,
            suggested_action_json: serde_json::to_string(&vec![action]).expect("serialize"),
            action_rationale: None,
            status,
            created_at: Utc::now(),
        };
        storage
            .insert_diagnosis(&record)
            .await
            .expect("insert diagnosis");
    }

    #[tokio::test]
    async fn test_pending_diagnosis_is_listed_and_approvable_by_id() {
        let storage = create_test_storage().await;
        let (mut manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            Arc::clone(&storage),
        );

        seed_pending_diagnosis(&storage, "diag-1", DiagnosisStatus::Pending).await;

        // Listed from the DB, keyed by the diagnosis id the caller will approve.
        let pending = manager.get_pending_diagnoses(None).await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "diag-1");
        assert_eq!(pending[0].action_type, "ConfigAdjust");

        // Approving that same id executes the proposed action.
        let result = manager.handle_approve("diag-1").await.expect("approve");
        assert_eq!(result.execution_results.len(), 1);
        assert!(result.execution_results[0].success);

        // No longer pending; the override and action outcome were persisted; the
        // cached count refreshed to 0. Advisory default: nothing was applied to
        // live config, so the accurate status is Recommended, not Executed.
        let diag = storage
            .get_diagnosis("diag-1")
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(diag.status, DiagnosisStatus::Recommended);
        assert!(manager.get_pending_diagnoses(None).await.is_empty());
        assert!(storage
            .get_config_override("request_timeout_ms")
            .await
            .expect("query")
            .is_some());
        assert_eq!(manager.state.pending_diagnoses, 0);
    }

    #[tokio::test]
    async fn test_pending_diagnosis_rejectable_by_id() {
        let storage = create_test_storage().await;
        let (mut manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            Arc::clone(&storage),
        );

        seed_pending_diagnosis(&storage, "diag-2", DiagnosisStatus::Pending).await;

        manager
            .handle_reject("diag-2", Some("unsafe"))
            .await
            .expect("reject");

        let diag = storage
            .get_diagnosis("diag-2")
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(diag.status, DiagnosisStatus::Rejected);
        assert_eq!(manager.state.total_actions_rejected, 1);
        // No execution occurred, so nothing was written to config_overrides.
        assert!(storage
            .get_config_override("request_timeout_ms")
            .await
            .expect("query")
            .is_none());
    }

    #[tokio::test]
    async fn test_approve_rejects_non_pending_diagnosis() {
        let storage = create_test_storage().await;
        let (mut manager, _handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            create_mock_client(),
            Arc::new(MetricsCollector::new()),
            Arc::clone(&storage),
        );

        // Already executed — approving again must be refused, not re-run.
        seed_pending_diagnosis(&storage, "diag-3", DiagnosisStatus::Executed).await;

        let err = manager
            .handle_approve("diag-3")
            .await
            .expect_err("must refuse non-pending");
        assert!(err.contains("not pending"));
    }

    #[tokio::test]
    async fn test_manager_handle_rollback_not_found() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let result = manager.handle_rollback("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_manager_update_status() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (manager, handle) = SelfImprovementManager::new(config, client, metrics, storage);

        // Initial status
        let status1 = handle.status_rx.borrow().clone();
        assert!(status1.running);
        assert_eq!(status1.total_cycles, 0);

        // Update status should send to receiver
        manager.update_status();

        // Status should still be same (nothing changed in state)
        let status2 = handle.status_rx.borrow().clone();
        assert!(status2.running);
        assert_eq!(status2.total_cycles, 0);
    }

    #[tokio::test]
    async fn test_manager_run_with_immediate_shutdown() {
        let config = SelfImprovementConfig {
            cycle_interval_secs: 1,
            ..Default::default()
        };
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (manager, handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Spawn the manager
        let manager_handle = tokio::spawn(async move {
            manager.run(shutdown_rx).await;
        });

        // Give it a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify it's running
        let status = handle.status_rx.borrow().clone();
        assert!(status.running);

        // Send shutdown signal
        shutdown_tx.send(true).unwrap();

        // Wait for manager to finish
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), manager_handle).await;
    }

    #[tokio::test]
    async fn test_manager_command_get_status() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let (response_tx, response_rx) = oneshot::channel();
        manager
            .handle_command(ManagerCommand::GetStatus { response_tx })
            .await;

        let status = response_rx.await.unwrap();
        assert!(status.running);
        assert_eq!(status.circuit_state, "closed");
    }

    #[tokio::test]
    async fn test_manager_command_get_pending() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let (response_tx, response_rx) = oneshot::channel();
        manager
            .handle_command(ManagerCommand::GetPending {
                limit: Some(10),
                response_tx,
            })
            .await;

        let pending = response_rx.await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_manager_command_trigger_cycle() {
        let config = SelfImprovementConfig::default();
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
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let (response_tx, response_rx) = oneshot::channel();
        manager
            .handle_command(ManagerCommand::TriggerCycle { response_tx })
            .await;

        let result = response_rx.await.unwrap();
        assert!(result.is_ok());

        // Should have incremented cycle count
        assert_eq!(manager.state.total_cycles, 1);
    }

    #[tokio::test]
    async fn test_manager_command_approve_not_found() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let (response_tx, response_rx) = oneshot::channel();
        manager
            .handle_command(ManagerCommand::Approve {
                diagnosis_id: "nonexistent".to_string(),
                response_tx,
            })
            .await;

        let result = response_rx.await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_command_reject_not_found() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let (response_tx, response_rx) = oneshot::channel();
        manager
            .handle_command(ManagerCommand::Reject {
                diagnosis_id: "nonexistent".to_string(),
                reason: Some("test reason".to_string()),
                response_tx,
            })
            .await;

        let result = response_rx.await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_command_rollback_not_found() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let (response_tx, response_rx) = oneshot::channel();
        manager
            .handle_command(ManagerCommand::Rollback {
                action_id: "nonexistent".to_string(),
                response_tx,
            })
            .await;

        let result = response_rx.await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_run_cycle() {
        let config = SelfImprovementConfig::default();
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
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (mut manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        // Run a cycle directly
        manager.run_cycle().await;

        // Should have updated state
        assert_eq!(manager.state.total_cycles, 1);
        assert!(manager.state.last_cycle_at.is_some());
        // Success or failure depends on analysis
        assert!(manager.state.successful_cycles + manager.state.failed_cycles >= 1);
    }

    #[tokio::test]
    async fn test_manager_should_run_cycle_with_enough_invocations() {
        use crate::metrics::MetricEvent;

        let config = SelfImprovementConfig {
            min_invocations_for_analysis: 5,
            ..Default::default()
        };
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        // Record some invocations
        for _ in 0..10 {
            metrics.record(MetricEvent {
                mode: "test_tool".to_string(),
                operation: None,
                latency_ms: 100,
                success: true,
                timestamp: 1234567890,
                problem_type: None,
                quality_rating: None,
                validation_consistent: None,
                converged: None,
            });
        }

        let (manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        // Now should run cycle
        assert!(manager.should_run_cycle());
    }

    #[tokio::test]
    async fn test_manager_circuit_state_in_status() {
        let config = SelfImprovementConfig::default();
        let client = create_mock_client();
        let metrics = Arc::new(MetricsCollector::new());
        let storage = create_test_storage().await;

        let (manager, _handle) = SelfImprovementManager::new(config, client, metrics, storage);

        let status = manager.build_status();
        // Circuit starts closed
        assert_eq!(status.circuit_state, "closed");
    }

    #[test]
    fn test_approve_result_debug_and_clone() {
        let result = ApproveResult {
            diagnosis_id: "test".to_string(),
            execution_results: vec![],
            learning_results: vec![],
        };

        let cloned = result.clone();
        assert_eq!(cloned.diagnosis_id, "test");

        let debug = format!("{result:?}");
        assert!(debug.contains("ApproveResult"));
    }

    #[test]
    fn test_execution_result_summary_debug_and_clone() {
        let summary = ExecutionResultSummary {
            action_id: "act-1".to_string(),
            success: true,
            message: "Done".to_string(),
        };

        let cloned = summary.clone();
        assert_eq!(cloned.action_id, "act-1");

        let debug = format!("{summary:?}");
        assert!(debug.contains("ExecutionResultSummary"));
    }

    #[test]
    fn test_learning_result_summary_debug_and_clone() {
        let summary = LearningResultSummary {
            action_id: "act-2".to_string(),
            lesson: "Test lesson".to_string(),
            reward: 0.75,
        };

        let cloned = summary.clone();
        assert_eq!(cloned.lesson, "Test lesson");

        let debug = format!("{summary:?}");
        assert!(debug.contains("LearningResultSummary"));
    }

    #[test]
    fn test_manager_status_serialization() {
        let status = ManagerStatus {
            running: true,
            circuit_state: "open".to_string(),
            total_cycles: 5,
            successful_cycles: 4,
            failed_cycles: 1,
            pending_diagnoses: 2,
            total_actions_executed: 10,
            total_actions_rolled_back: 1,
            total_actions_rejected: 0,
            last_cycle_at: Some(1234567890),
            learning_summary: LearningSummaryData {
                total_lessons: 3,
                average_reward: 0.8,
            },
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"running\":true"));

        let parsed: ManagerStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_cycles, 5);
    }

    #[test]
    fn test_pending_diagnosis_serialization() {
        let pending = PendingDiagnosis {
            id: "diag-1".to_string(),
            action_type: "ConfigAdjust".to_string(),
            description: "Test action".to_string(),
            rationale: "Test reason".to_string(),
            expected_improvement: 0.15,
            created_at: 1234567890,
        };

        let json = serde_json::to_string(&pending).unwrap();
        assert!(json.contains("\"id\":\"diag-1\""));

        let parsed: PendingDiagnosis = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.description, "Test action");
    }

    #[test]
    fn test_learning_summary_data_serialization() {
        let summary = LearningSummaryData {
            total_lessons: 10,
            average_reward: 0.65,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let parsed: LearningSummaryData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_lessons, 10);
    }

    #[test]
    fn test_manager_command_debug() {
        let (response_tx, _) = oneshot::channel::<ManagerStatus>();
        let cmd = ManagerCommand::GetStatus { response_tx };
        let debug = format!("{cmd:?}");
        assert!(debug.contains("GetStatus"));
    }
}
