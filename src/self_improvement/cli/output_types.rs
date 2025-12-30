//! CLI output types for self-improvement system.

use serde::{Deserialize, Serialize};

/// Status output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusOutput {
    /// Whether the system is enabled.
    pub enabled: bool,
    /// Whether the system is paused.
    pub paused: bool,
    /// Pause remaining duration (if paused).
    pub pause_remaining: Option<String>,
    /// Circuit breaker state.
    pub circuit_breaker_state: String,
    /// Total invocations processed.
    pub total_invocations: u64,
    /// Total diagnoses created.
    pub total_diagnoses: u64,
    /// Total actions executed.
    pub total_actions: u64,
    /// Pending diagnoses count.
    pub pending_diagnoses: u64,
    /// Current error rate.
    pub current_error_rate: f64,
    /// Current latency P95.
    pub current_latency_p95: i64,
    /// Current quality score.
    pub current_quality_score: f64,
    /// Last cycle time.
    pub last_cycle_at: Option<String>,
}

/// History output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryOutput {
    /// Action records.
    pub actions: Vec<ActionHistoryEntry>,
    /// Total count (may be more than returned).
    pub total_count: u64,
}

/// Action history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHistoryEntry {
    /// Action ID.
    pub id: String,
    /// Diagnosis ID.
    pub diagnosis_id: String,
    /// Action type.
    pub action_type: String,
    /// Outcome.
    pub outcome: String,
    /// Execution time.
    pub execution_time_ms: i64,
    /// Created at.
    pub created_at: String,
    /// Reward (if learning completed).
    pub reward: Option<f64>,
}

/// Diagnostics output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsOutput {
    /// System health.
    pub health: HealthDiagnostics,
    /// Recent errors.
    pub recent_errors: Vec<String>,
    /// Resource usage.
    pub resources: ResourceDiagnostics,
    /// Performance metrics.
    pub performance: PerformanceDiagnostics,
}

/// Health diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDiagnostics {
    /// Overall health status.
    pub status: String,
    /// Health score (0.0 to 1.0).
    pub score: f64,
    /// Issues detected.
    pub issues: Vec<String>,
}

/// Resource diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDiagnostics {
    /// Memory usage.
    pub memory_mb: f64,
    /// Active connections.
    pub active_connections: u32,
    /// Queue depth.
    pub queue_depth: u32,
}

/// Performance diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDiagnostics {
    /// Average cycle time.
    pub avg_cycle_time_ms: f64,
    /// Average analysis time.
    pub avg_analysis_time_ms: f64,
    /// Average execution time.
    pub avg_execution_time_ms: f64,
}

/// Config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOutput {
    /// Monitor configuration.
    pub monitor: MonitorConfigOutput,
    /// Analyzer configuration.
    pub analyzer: AnalyzerConfigOutput,
    /// Executor configuration.
    pub executor: ExecutorConfigOutput,
    /// Learner configuration.
    pub learner: LearnerConfigOutput,
    /// Circuit breaker configuration.
    pub circuit_breaker: CircuitBreakerConfigOutput,
    /// Applied overrides.
    pub overrides: Vec<ConfigOverrideOutput>,
}

/// Monitor config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfigOutput {
    /// Check interval.
    pub check_interval_secs: u64,
    /// Minimum samples.
    pub min_samples: u64,
    /// Error rate threshold.
    pub error_rate_threshold: f64,
    /// Latency threshold.
    pub latency_threshold_ms: i64,
    /// Quality threshold.
    pub quality_threshold: f64,
}

/// Analyzer config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfigOutput {
    /// Model used.
    pub model: String,
    /// Max tokens.
    pub max_tokens: u32,
    /// Minimum severity.
    pub min_severity: String,
}

/// Executor config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfigOutput {
    /// Cooldown between actions.
    pub cooldown_secs: u64,
    /// Rate limit per hour.
    pub rate_limit_per_hour: u32,
    /// Auto-approve enabled.
    pub auto_approve: bool,
}

/// Learner config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnerConfigOutput {
    /// Observation window.
    pub observation_window_secs: u64,
    /// Minimum samples for learning.
    pub min_samples: u64,
}

/// Circuit breaker config output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfigOutput {
    /// Failure threshold.
    pub failure_threshold: u32,
    /// Reset timeout.
    pub reset_timeout_secs: u64,
    /// Half-open max.
    pub half_open_max: u32,
}

/// Config override output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOverrideOutput {
    /// Override key.
    pub key: String,
    /// Override value.
    pub value: String,
    /// Applied by action ID.
    pub applied_by: Option<String>,
    /// Updated at.
    pub updated_at: String,
}

/// Circuit breaker output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerOutput {
    /// Current state.
    pub state: String,
    /// Consecutive failures.
    pub consecutive_failures: u32,
    /// Last failure time.
    pub last_failure_at: Option<String>,
    /// Time until reset (if open).
    pub reset_in: Option<String>,
    /// Total trips.
    pub total_trips: u64,
}

/// Baselines output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselinesOutput {
    /// Global baselines.
    pub global: GlobalBaselinesOutput,
    /// Per-tool baselines.
    pub tools: Vec<ToolBaselinesOutput>,
}

/// Global baselines output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBaselinesOutput {
    /// Error rate baseline.
    pub error_rate: f64,
    /// Latency P95 baseline.
    pub latency_p95_ms: i64,
    /// Quality score baseline.
    pub quality_score: f64,
    /// Sample count.
    pub sample_count: u64,
    /// Is valid.
    pub is_valid: bool,
}

/// Tool baselines output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolBaselinesOutput {
    /// Tool name.
    pub tool_name: String,
    /// Error rate baseline.
    pub error_rate: f64,
    /// Latency baseline.
    pub latency_ms: f64,
    /// Quality baseline.
    pub quality_score: f64,
    /// Sample count.
    pub sample_count: u64,
}
