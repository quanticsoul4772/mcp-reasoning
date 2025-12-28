//! Self-improvement system types.
//!
//! Core types for the 4-phase optimization loop (DESIGN.md Section 14):
//! - Monitor: Metric collection and baseline tracking
//! - Analyze: LLM-powered diagnosis
//! - Execute: Action execution with safety
//! - Learn: Reward calculation and lesson synthesis

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// Type Aliases (Future use - R8/R9 database schema)
// ============================================================================

/// Unique identifier for a diagnosis.
pub type DiagnosisId = String;

/// Unique identifier for an action.
#[allow(dead_code)]
pub type ActionId = String;

// ============================================================================
// Severity (DESIGN.md 14.2)
// ============================================================================

/// Severity levels for detected issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Severity {
    /// Minor deviation, no action needed.
    Info = 0,
    /// Moderate deviation, consider action.
    Warning = 1,
    /// Significant deviation, action recommended.
    High = 2,
    /// Severe deviation, immediate action required.
    Critical = 3,
}

impl Severity {
    /// Create severity from deviation percentage.
    #[must_use]
    pub fn from_deviation(deviation_pct: f64) -> Self {
        match deviation_pct {
            d if d >= 100.0 => Self::Critical,
            d if d >= 50.0 => Self::High,
            d if d >= 25.0 => Self::Warning,
            _ => Self::Info,
        }
    }

    /// Get numeric value for comparison.
    #[must_use]
    pub const fn value(self) -> u8 {
        self as u8
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

// ============================================================================
// TriggerMetric (DESIGN.md 14.2)
// ============================================================================

/// What triggered the diagnosis - type-safe metric variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerMetric {
    /// Error rate exceeded threshold.
    ErrorRate {
        /// Observed error rate (0.0 to 1.0).
        observed: f64,
        /// Baseline error rate for comparison.
        baseline: f64,
        /// Threshold that was exceeded.
        threshold: f64,
    },
    /// Latency (P95) exceeded threshold.
    Latency {
        /// Observed P95 latency in milliseconds.
        observed_p95_ms: i64,
        /// Baseline latency for comparison.
        baseline_ms: i64,
        /// Threshold that was exceeded.
        threshold_ms: i64,
    },
    /// Quality score dropped below minimum.
    QualityScore {
        /// Observed quality score (0.0 to 1.0).
        observed: f64,
        /// Baseline quality score for comparison.
        baseline: f64,
        /// Minimum acceptable quality score.
        minimum: f64,
    },
}

impl TriggerMetric {
    /// Calculate deviation percentage from baseline.
    #[must_use]
    pub fn deviation_pct(&self) -> f64 {
        match self {
            Self::ErrorRate {
                observed, baseline, ..
            } => {
                if *baseline == 0.0 {
                    if *observed > 0.0 {
                        100.0
                    } else {
                        0.0
                    }
                } else {
                    ((observed - baseline) / baseline) * 100.0
                }
            }
            Self::Latency {
                observed_p95_ms,
                baseline_ms,
                ..
            } => {
                if *baseline_ms == 0 {
                    if *observed_p95_ms > 0 {
                        100.0
                    } else {
                        0.0
                    }
                } else {
                    ((*observed_p95_ms - *baseline_ms) as f64 / *baseline_ms as f64) * 100.0
                }
            }
            Self::QualityScore {
                observed, baseline, ..
            } => {
                if *baseline == 0.0 {
                    if *observed < 1.0 {
                        100.0
                    } else {
                        0.0
                    }
                } else {
                    // Inverted: lower quality is worse
                    ((baseline - observed) / baseline) * 100.0
                }
            }
        }
    }

    /// Get severity based on deviation.
    #[must_use]
    pub fn severity(&self) -> Severity {
        Severity::from_deviation(self.deviation_pct().abs())
    }

    /// Check if threshold is exceeded.
    #[must_use]
    pub fn is_triggered(&self) -> bool {
        match self {
            Self::ErrorRate {
                observed,
                threshold,
                ..
            } => observed > threshold,
            Self::Latency {
                observed_p95_ms,
                threshold_ms,
                ..
            } => observed_p95_ms > threshold_ms,
            Self::QualityScore {
                observed, minimum, ..
            } => observed < minimum,
        }
    }

    /// Get metric type name.
    #[must_use]
    pub fn metric_type(&self) -> &'static str {
        match self {
            Self::ErrorRate { .. } => "error_rate",
            Self::Latency { .. } => "latency",
            Self::QualityScore { .. } => "quality_score",
        }
    }
}

// ============================================================================
// ParamValue (DESIGN.md 14.2) - Future use (R8/R9)
// ============================================================================

/// Parameter value types for type-safe configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ParamValue {
    /// Integer value.
    Integer(i64),
    /// Floating-point value.
    Float(f64),
    /// String value.
    String(String),
    /// Duration in milliseconds.
    DurationMs(u64),
    /// Boolean value.
    Boolean(bool),
}

impl ParamValue {
    /// Create an integer param value.
    #[must_use]
    pub const fn integer(v: i64) -> Self {
        Self::Integer(v)
    }

    /// Create a float param value.
    #[must_use]
    pub fn float(v: f64) -> Self {
        Self::Float(v)
    }

    /// Create a string param value.
    #[must_use]
    pub fn string(v: impl Into<String>) -> Self {
        Self::String(v.into())
    }

    /// Create a duration param value.
    #[must_use]
    pub const fn duration_ms(v: u64) -> Self {
        Self::DurationMs(v)
    }

    /// Create a boolean param value.
    #[must_use]
    pub const fn boolean(v: bool) -> Self {
        Self::Boolean(v)
    }
}

impl std::fmt::Display for ParamValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "{v}"),
            Self::DurationMs(v) => write!(f, "{v}ms"),
            Self::Boolean(v) => write!(f, "{v}"),
        }
    }
}

// ============================================================================
// ConfigScope (DESIGN.md 14.2) - Future use (R8/R9)
// ============================================================================

/// Scope for configuration parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ConfigScope {
    /// Applies to all modes and tools.
    #[default]
    Global,
    /// Applies to a specific reasoning mode.
    Mode(String),
    /// Applies to a specific tool.
    Tool(String),
}

impl std::fmt::Display for ConfigScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Mode(m) => write!(f, "mode:{m}"),
            Self::Tool(t) => write!(f, "tool:{t}"),
        }
    }
}

impl ConfigScope {
    /// Known valid reasoning mode names.
    const VALID_MODES: &'static [&'static str] = &[
        "linear",
        "tree",
        "divergent",
        "reflection",
        "checkpoint",
        "auto",
        "graph",
        "detect",
        "decision",
        "evidence",
        "timeline",
        "mcts",
        "counterfactual",
    ];

    /// Validate that Mode/Tool variants contain known values.
    ///
    /// - `Mode` must be a known reasoning mode (case-insensitive)
    /// - `Tool` must follow the pattern `reasoning_<mode>`
    /// - `Global` is always valid
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, `Err(reason)` if invalid.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Global => Ok(()),
            Self::Mode(mode_str) => {
                let normalized = mode_str.to_lowercase();
                if Self::VALID_MODES.contains(&normalized.as_str()) {
                    Ok(())
                } else {
                    Err(format!(
                        "Unknown mode '{}'. Valid modes: {}",
                        mode_str,
                        Self::VALID_MODES.join(", ")
                    ))
                }
            }
            Self::Tool(tool_str) => {
                // Tools should follow pattern: reasoning_<mode>
                match tool_str.strip_prefix("reasoning_") {
                    Some(mode_part) if Self::VALID_MODES.contains(&mode_part) => Ok(()),
                    Some(_) => Err(format!(
                        "Unknown tool '{}'. Tool name should be reasoning_<mode>",
                        tool_str
                    )),
                    None => Err(format!(
                        "Invalid tool format '{}'. Expected 'reasoning_<mode>'",
                        tool_str
                    )),
                }
            }
        }
    }
}

// ============================================================================
// ResourceType (DESIGN.md 14.2) - Future use (R8/R9)
// ============================================================================

/// Resource types that can be scaled by the self-improvement system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// Maximum concurrent API requests.
    MaxConcurrentRequests,
    /// Database connection pool size.
    ConnectionPoolSize,
    /// In-memory cache size.
    CacheSize,
    /// Request timeout in milliseconds.
    TimeoutMs,
    /// Maximum retry attempts.
    MaxRetries,
    /// Delay between retries in milliseconds.
    RetryDelayMs,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxConcurrentRequests => write!(f, "max_concurrent_requests"),
            Self::ConnectionPoolSize => write!(f, "connection_pool_size"),
            Self::CacheSize => write!(f, "cache_size"),
            Self::TimeoutMs => write!(f, "timeout_ms"),
            Self::MaxRetries => write!(f, "max_retries"),
            Self::RetryDelayMs => write!(f, "retry_delay_ms"),
        }
    }
}

// ============================================================================
// SuggestedAction (DESIGN.md 14.2) - Future use (R8/R9)
// ============================================================================

/// Actions the system can take (ALL must be reversible).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SuggestedAction {
    /// Adjust a configuration parameter.
    AdjustParam {
        /// The configuration key to adjust.
        key: String,
        /// The previous value.
        old_value: ParamValue,
        /// The new value to set.
        new_value: ParamValue,
        /// The scope of the adjustment.
        scope: ConfigScope,
    },
    /// Scale a resource.
    ScaleResource {
        /// The resource to scale.
        resource: ResourceType,
        /// The previous resource value.
        old_value: u32,
        /// The new resource value.
        new_value: u32,
    },
    /// No action needed, but revisit later.
    NoOp {
        /// Reason for taking no action.
        reason: String,
        /// Duration to wait before revisiting.
        #[serde(with = "duration_serde")]
        revisit_after: Duration,
    },
}

impl SuggestedAction {
    /// Create a param adjustment action.
    #[must_use]
    pub fn adjust_param(
        key: impl Into<String>,
        old_value: ParamValue,
        new_value: ParamValue,
        scope: ConfigScope,
    ) -> Self {
        Self::AdjustParam {
            key: key.into(),
            old_value,
            new_value,
            scope,
        }
    }

    /// Create a resource scaling action.
    #[must_use]
    pub const fn scale_resource(resource: ResourceType, old_value: u32, new_value: u32) -> Self {
        Self::ScaleResource {
            resource,
            old_value,
            new_value,
        }
    }

    /// Create a no-op action.
    #[must_use]
    pub fn no_op(reason: impl Into<String>, revisit_after: Duration) -> Self {
        Self::NoOp {
            reason: reason.into(),
            revisit_after,
        }
    }

    /// Check if this is a no-op action.
    #[must_use]
    pub const fn is_no_op(&self) -> bool {
        matches!(self, Self::NoOp { .. })
    }

    /// Get action type name.
    #[must_use]
    pub fn action_type(&self) -> &'static str {
        match self {
            Self::AdjustParam { .. } => "adjust_param",
            Self::ScaleResource { .. } => "scale_resource",
            Self::NoOp { .. } => "no_op",
        }
    }
}

/// Serde module for Duration serialization.
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

// ============================================================================
// DiagnosisStatus (DESIGN.md 14.2) - Future use (R8/R9)
// ============================================================================

/// Status of a diagnosis in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisStatus {
    /// Diagnosis created, awaiting review.
    Pending,
    /// Diagnosis approved for execution.
    Approved,
    /// Diagnosis rejected, will not execute.
    Rejected,
    /// Action executed successfully.
    Executed,
    /// Action execution failed.
    Failed,
    /// Action was rolled back.
    RolledBack,
}

impl std::fmt::Display for DiagnosisStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
            Self::Executed => write!(f, "executed"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled_back"),
        }
    }
}

// ============================================================================
// SelfDiagnosis (DESIGN.md 14.2) - Future use (R8/R9)
// ============================================================================

/// Complete diagnosis report from the Analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfDiagnosis {
    /// Unique diagnosis identifier.
    pub id: DiagnosisId,
    /// When the diagnosis was created.
    pub created_at: DateTime<Utc>,
    /// What triggered this diagnosis.
    pub trigger: TriggerMetric,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable description of the issue.
    pub description: String,
    /// Suspected root cause (LLM-generated).
    pub suspected_cause: Option<String>,
    /// Recommended action to take.
    pub suggested_action: SuggestedAction,
    /// Rationale for the suggested action (LLM-generated).
    pub action_rationale: Option<String>,
    /// Current status.
    pub status: DiagnosisStatus,
}

impl SelfDiagnosis {
    /// Create a new diagnosis.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        trigger: TriggerMetric,
        description: impl Into<String>,
        suggested_action: SuggestedAction,
    ) -> Self {
        let severity = trigger.severity();
        Self {
            id: id.into(),
            created_at: Utc::now(),
            trigger,
            severity,
            description: description.into(),
            suspected_cause: None,
            suggested_action,
            action_rationale: None,
            status: DiagnosisStatus::Pending,
        }
    }

    /// Add suspected cause.
    #[must_use]
    pub fn with_suspected_cause(mut self, cause: impl Into<String>) -> Self {
        self.suspected_cause = Some(cause.into());
        self
    }

    /// Add action rationale.
    #[must_use]
    pub fn with_action_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.action_rationale = Some(rationale.into());
        self
    }

    /// Approve this diagnosis for execution.
    pub fn approve(&mut self) {
        self.status = DiagnosisStatus::Approved;
    }

    /// Reject this diagnosis.
    pub fn reject(&mut self) {
        self.status = DiagnosisStatus::Rejected;
    }

    /// Mark as executed.
    pub fn mark_executed(&mut self) {
        self.status = DiagnosisStatus::Executed;
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self) {
        self.status = DiagnosisStatus::Failed;
    }

    /// Mark as rolled back.
    pub fn mark_rolled_back(&mut self) {
        self.status = DiagnosisStatus::RolledBack;
    }
}

// ============================================================================
// Reward Types (DESIGN.md 14.2) - Future use (R8/R9)
// ============================================================================

/// Breakdown of reward components.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct RewardBreakdown {
    /// Component from error rate improvement.
    pub error_rate_component: f64,
    /// Component from latency improvement.
    pub latency_component: f64,
    /// Component from quality improvement.
    pub quality_component: f64,
}

impl RewardBreakdown {
    /// Create a new reward breakdown.
    #[must_use]
    pub const fn new(error_rate: f64, latency: f64, quality: f64) -> Self {
        Self {
            error_rate_component: error_rate,
            latency_component: latency,
            quality_component: quality,
        }
    }

    /// Calculate weighted total.
    #[must_use]
    pub fn weighted_total(&self, weights: &RewardWeights) -> f64 {
        self.error_rate_component * weights.error_rate
            + self.latency_component * weights.latency
            + self.quality_component * weights.quality
    }
}

/// Weights for reward calculation based on trigger type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardWeights {
    /// Weight for error rate component.
    pub error_rate: f64,
    /// Weight for latency component.
    pub latency: f64,
    /// Weight for quality component.
    pub quality: f64,
}

impl RewardWeights {
    /// Get weights optimized for the trigger type.
    #[must_use]
    pub fn for_trigger(trigger: &TriggerMetric) -> Self {
        match trigger {
            TriggerMetric::ErrorRate { .. } => Self {
                error_rate: 0.6,
                latency: 0.2,
                quality: 0.2,
            },
            TriggerMetric::Latency { .. } => Self {
                error_rate: 0.2,
                latency: 0.6,
                quality: 0.2,
            },
            TriggerMetric::QualityScore { .. } => Self {
                error_rate: 0.2,
                latency: 0.2,
                quality: 0.6,
            },
        }
    }
}

impl Default for RewardWeights {
    fn default() -> Self {
        Self {
            error_rate: 0.34,
            latency: 0.33,
            quality: 0.33,
        }
    }
}

/// Normalized reward for comparing improvements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedReward {
    /// Reward value (-1.0 to 1.0, positive = improvement).
    pub value: f64,
    /// Breakdown by component.
    pub breakdown: RewardBreakdown,
    /// Confidence based on sample size (0.0 to 1.0).
    pub confidence: f64,
}

impl NormalizedReward {
    /// Create a new normalized reward.
    #[must_use]
    pub fn new(value: f64, breakdown: RewardBreakdown, confidence: f64) -> Self {
        Self {
            value: value.clamp(-1.0, 1.0),
            breakdown,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Calculate reward from metrics snapshots.
    #[must_use]
    pub fn calculate(
        trigger: &TriggerMetric,
        pre_metrics: &MetricsSnapshot,
        post_metrics: &MetricsSnapshot,
        sample_count: u64,
    ) -> Self {
        let weights = RewardWeights::for_trigger(trigger);

        // Calculate component improvements
        let error_improvement = if pre_metrics.error_rate > 0.0 {
            (pre_metrics.error_rate - post_metrics.error_rate) / pre_metrics.error_rate
        } else if post_metrics.error_rate > 0.0 {
            -1.0
        } else {
            0.0
        };

        let latency_improvement = if pre_metrics.latency_p95_ms > 0 {
            (pre_metrics.latency_p95_ms - post_metrics.latency_p95_ms) as f64
                / pre_metrics.latency_p95_ms as f64
        } else if post_metrics.latency_p95_ms > 0 {
            -1.0
        } else {
            0.0
        };

        let quality_improvement = if pre_metrics.quality_score > 0.0 {
            (post_metrics.quality_score - pre_metrics.quality_score) / pre_metrics.quality_score
        } else if post_metrics.quality_score > 0.0 {
            1.0
        } else {
            0.0
        };

        let breakdown = RewardBreakdown::new(
            error_improvement.clamp(-1.0, 1.0),
            latency_improvement.clamp(-1.0, 1.0),
            quality_improvement.clamp(-1.0, 1.0),
        );

        let value = breakdown.weighted_total(&weights);

        // Confidence based on sample size (asymptotic to 1.0)
        let confidence = 1.0 - 1.0 / (1.0 + sample_count as f64 / 100.0);

        Self::new(value, breakdown, confidence)
    }

    /// Check if reward indicates improvement.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.value > 0.0
    }

    /// Check if reward indicates degradation.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.value < 0.0
    }

    /// Check if reward is significant (above noise threshold).
    #[must_use]
    pub fn is_significant(&self, threshold: f64) -> bool {
        self.value.abs() > threshold && self.confidence > 0.5
    }
}

// ============================================================================
// MetricsSnapshot (DESIGN.md 14.3) - Future use (R8/R9)
// ============================================================================

/// Snapshot of system metrics at a point in time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Error rate (0.0 to 1.0).
    pub error_rate: f64,
    /// P95 latency in milliseconds.
    pub latency_p95_ms: i64,
    /// Average quality score (0.0 to 1.0).
    pub quality_score: f64,
    /// Total invocations in this period.
    pub invocation_count: u64,
    /// Timestamp of snapshot.
    pub timestamp: DateTime<Utc>,
    /// Per-tool metrics.
    pub tool_metrics: HashMap<String, ToolMetrics>,
}

impl MetricsSnapshot {
    /// Create a new metrics snapshot.
    #[must_use]
    pub fn new(
        error_rate: f64,
        latency_p95_ms: i64,
        quality_score: f64,
        invocation_count: u64,
    ) -> Self {
        Self {
            error_rate: error_rate.clamp(0.0, 1.0),
            latency_p95_ms: latency_p95_ms.max(0),
            quality_score: quality_score.clamp(0.0, 1.0),
            invocation_count,
            timestamp: Utc::now(),
            tool_metrics: HashMap::new(),
        }
    }
}

/// Per-tool metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetrics {
    /// Error rate for this tool.
    pub error_rate: f64,
    /// Average latency for this tool.
    pub avg_latency_ms: i64,
    /// Invocation count for this tool.
    pub invocation_count: u64,
}

// ============================================================================
// Baselines (DESIGN.md 14.3) - Future use (R8/R9)
// ============================================================================

/// Baseline values for comparison.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Baselines {
    /// Baseline error rate.
    pub error_rate: f64,
    /// Baseline P95 latency.
    pub latency_p95_ms: i64,
    /// Baseline quality score.
    pub quality_score: f64,
    /// Sample count used to calculate baselines.
    pub sample_count: u64,
    /// When baselines were last updated.
    pub updated_at: DateTime<Utc>,
}

impl Baselines {
    /// Create new baselines.
    #[must_use]
    pub fn new(
        error_rate: f64,
        latency_p95_ms: i64,
        quality_score: f64,
        sample_count: u64,
    ) -> Self {
        Self {
            error_rate,
            latency_p95_ms,
            quality_score,
            sample_count,
            updated_at: Utc::now(),
        }
    }
}

// ============================================================================
// Legacy Types (kept for backward compatibility during transition)
// ============================================================================

/// Type of improvement action (legacy - use SuggestedAction instead).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Adjust a configuration parameter.
    ConfigAdjust,
    /// Modify prompt templates.
    PromptTune,
    /// Adjust mode routing thresholds.
    ThresholdAdjust,
    /// Log an observation for future reference.
    LogObservation,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigAdjust => write!(f, "config_adjust"),
            Self::PromptTune => write!(f, "prompt_tune"),
            Self::ThresholdAdjust => write!(f, "threshold_adjust"),
            Self::LogObservation => write!(f, "log_observation"),
        }
    }
}

/// Status of an improvement action (legacy - use DiagnosisStatus instead).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// Action is proposed but not yet approved.
    Proposed,
    /// Action is approved and ready to execute.
    Approved,
    /// Action is currently being executed.
    Executing,
    /// Action completed successfully.
    Completed,
    /// Action failed during execution.
    Failed,
    /// Action was rolled back.
    RolledBack,
}

impl std::fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proposed => write!(f, "proposed"),
            Self::Approved => write!(f, "approved"),
            Self::Executing => write!(f, "executing"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled_back"),
        }
    }
}

/// A proposed or executed improvement action (legacy - use SelfDiagnosis instead).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementAction {
    /// Unique action identifier.
    pub id: String,
    /// Type of action.
    pub action_type: ActionType,
    /// Human-readable description.
    pub description: String,
    /// Current status.
    pub status: ActionStatus,
    /// Rationale for this action.
    pub rationale: String,
    /// Expected improvement (0.0-1.0).
    pub expected_improvement: f64,
    /// Actual improvement after execution (if completed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_improvement: Option<f64>,
    /// Action-specific parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// Rollback data (if action is reversible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_data: Option<serde_json::Value>,
    /// Timestamp when created.
    pub created_at: u64,
    /// Timestamp when executed (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_at: Option<u64>,
}

impl SelfImprovementAction {
    /// Create a new proposed action.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        action_type: ActionType,
        description: impl Into<String>,
        rationale: impl Into<String>,
        expected_improvement: f64,
    ) -> Self {
        Self {
            id: id.into(),
            action_type,
            description: description.into(),
            status: ActionStatus::Proposed,
            rationale: rationale.into(),
            expected_improvement: expected_improvement.clamp(0.0, 1.0),
            actual_improvement: None,
            parameters: None,
            rollback_data: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            executed_at: None,
        }
    }

    /// Add parameters to the action.
    #[must_use]
    pub fn with_parameters(mut self, params: serde_json::Value) -> Self {
        self.parameters = Some(params);
        self
    }

    /// Mark action as approved.
    pub fn approve(&mut self) {
        self.status = ActionStatus::Approved;
    }

    /// Mark action as executing.
    pub fn start_execution(&mut self) {
        self.status = ActionStatus::Executing;
    }

    /// Mark action as completed with actual improvement.
    pub fn complete(&mut self, actual_improvement: f64) {
        self.status = ActionStatus::Completed;
        self.actual_improvement = Some(actual_improvement.clamp(0.0, 1.0));
        self.executed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
    }

    /// Mark action as failed.
    pub fn fail(&mut self) {
        self.status = ActionStatus::Failed;
        self.executed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
    }

    /// Mark action as rolled back.
    pub fn rollback(&mut self) {
        self.status = ActionStatus::RolledBack;
    }
}

/// System-wide metrics snapshot (legacy - use MetricsSnapshot instead).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Overall success rate (0.0-1.0).
    pub success_rate: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Total invocations.
    pub total_invocations: u64,
    /// Per-mode success rates.
    pub mode_success_rates: HashMap<String, f64>,
    /// Timestamp of snapshot.
    pub timestamp: u64,
}

impl SystemMetrics {
    /// Create a new metrics snapshot.
    #[must_use]
    pub fn new(
        success_rate: f64,
        avg_latency_ms: f64,
        total_invocations: u64,
        mode_success_rates: HashMap<String, f64>,
    ) -> Self {
        Self {
            success_rate: success_rate.clamp(0.0, 1.0),
            avg_latency_ms: avg_latency_ms.max(0.0),
            total_invocations,
            mode_success_rates,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

/// Legacy trigger metric struct (for backward compatibility with monitor/analyzer).
/// Use the TriggerMetric enum for new code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyTriggerMetric {
    /// Metric name.
    pub name: String,
    /// Current value.
    pub value: f64,
    /// Threshold value.
    pub threshold: f64,
    /// Severity level.
    pub severity: Severity,
    /// Description of the issue.
    pub description: String,
}

impl LegacyTriggerMetric {
    /// Create a new legacy trigger metric.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        value: f64,
        threshold: f64,
        severity: Severity,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value,
            threshold,
            severity,
            description: description.into(),
        }
    }
}

/// A lesson learned from an improvement action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    /// Unique lesson identifier.
    pub id: String,
    /// The action that led to this lesson.
    pub action_id: String,
    /// What was learned.
    pub insight: String,
    /// Calculated reward (-1.0 to 1.0).
    pub reward: f64,
    /// Applicable contexts.
    pub applicable_contexts: Vec<String>,
    /// Timestamp.
    pub created_at: u64,
}

impl Lesson {
    /// Create a new lesson.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        action_id: impl Into<String>,
        insight: impl Into<String>,
        reward: f64,
    ) -> Self {
        Self {
            id: id.into(),
            action_id: action_id.into(),
            insight: insight.into(),
            reward: reward.clamp(-1.0, 1.0),
            applicable_contexts: Vec::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Add applicable contexts.
    #[must_use]
    pub fn with_contexts(mut self, contexts: Vec<String>) -> Self {
        self.applicable_contexts = contexts;
        self
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Severity tests
    #[test]
    fn test_severity_from_deviation() {
        assert_eq!(Severity::from_deviation(0.0), Severity::Info);
        assert_eq!(Severity::from_deviation(10.0), Severity::Info);
        assert_eq!(Severity::from_deviation(25.0), Severity::Warning);
        assert_eq!(Severity::from_deviation(49.0), Severity::Warning);
        assert_eq!(Severity::from_deviation(50.0), Severity::High);
        assert_eq!(Severity::from_deviation(99.0), Severity::High);
        assert_eq!(Severity::from_deviation(100.0), Severity::Critical);
        assert_eq!(Severity::from_deviation(200.0), Severity::Critical);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }

    #[test]
    fn test_severity_value() {
        assert_eq!(Severity::Info.value(), 0);
        assert_eq!(Severity::Warning.value(), 1);
        assert_eq!(Severity::High.value(), 2);
        assert_eq!(Severity::Critical.value(), 3);
    }

    // TriggerMetric tests
    #[test]
    fn test_trigger_metric_error_rate_deviation() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.15,
            baseline: 0.10,
            threshold: 0.12,
        };
        assert!((trigger.deviation_pct() - 50.0).abs() < 0.01);
        // 50% deviation is right at the boundary, might be Warning or High due to floating point
        assert!(trigger.severity() >= Severity::Warning);
        assert!(trigger.is_triggered());
    }

    #[test]
    fn test_trigger_metric_latency_deviation() {
        let trigger = TriggerMetric::Latency {
            observed_p95_ms: 200,
            baseline_ms: 100,
            threshold_ms: 150,
        };
        assert!((trigger.deviation_pct() - 100.0).abs() < 0.01);
        assert_eq!(trigger.severity(), Severity::Critical);
        assert!(trigger.is_triggered());
    }

    #[test]
    fn test_trigger_metric_quality_deviation() {
        let trigger = TriggerMetric::QualityScore {
            observed: 0.7,
            baseline: 0.9,
            minimum: 0.8,
        };
        // (0.9 - 0.7) / 0.9 * 100 = 22.2%
        assert!(trigger.deviation_pct() > 20.0);
        assert!(trigger.is_triggered());
    }

    #[test]
    fn test_trigger_metric_zero_baseline() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.1,
            baseline: 0.0,
            threshold: 0.05,
        };
        assert!((trigger.deviation_pct() - 100.0).abs() < 0.01);
    }

    // ParamValue tests
    #[test]
    fn test_param_value_display() {
        assert_eq!(ParamValue::integer(42).to_string(), "42");
        assert_eq!(ParamValue::boolean(true).to_string(), "true");
        assert_eq!(ParamValue::duration_ms(1000).to_string(), "1000ms");
    }

    #[test]
    fn test_param_value_serialize() {
        let value = ParamValue::Integer(42);
        let json = serde_json::to_string(&value).unwrap();
        assert!(json.contains("integer"));
        assert!(json.contains("42"));
    }

    // ResourceType tests
    #[test]
    fn test_resource_type_display() {
        assert_eq!(ResourceType::TimeoutMs.to_string(), "timeout_ms");
        assert_eq!(ResourceType::MaxRetries.to_string(), "max_retries");
    }

    // SuggestedAction tests
    #[test]
    fn test_suggested_action_adjust_param() {
        let action = SuggestedAction::adjust_param(
            "timeout",
            ParamValue::duration_ms(30000),
            ParamValue::duration_ms(60000),
            ConfigScope::Global,
        );
        assert!(!action.is_no_op());
        assert_eq!(action.action_type(), "adjust_param");
    }

    #[test]
    fn test_suggested_action_scale_resource() {
        let action = SuggestedAction::scale_resource(ResourceType::MaxRetries, 3, 5);
        assert!(!action.is_no_op());
        assert_eq!(action.action_type(), "scale_resource");
    }

    #[test]
    fn test_suggested_action_no_op() {
        let action = SuggestedAction::no_op("Within acceptable range", Duration::from_secs(3600));
        assert!(action.is_no_op());
        assert_eq!(action.action_type(), "no_op");
    }

    // SelfDiagnosis tests
    #[test]
    fn test_self_diagnosis_new() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.1,
            threshold: 0.15,
        };
        let action = SuggestedAction::scale_resource(ResourceType::MaxRetries, 3, 5);
        let diagnosis = SelfDiagnosis::new("diag-1", trigger, "High error rate", action);

        assert_eq!(diagnosis.id, "diag-1");
        assert_eq!(diagnosis.status, DiagnosisStatus::Pending);
        assert_eq!(diagnosis.severity, Severity::Critical);
    }

    #[test]
    fn test_self_diagnosis_lifecycle() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.15,
            baseline: 0.1,
            threshold: 0.12,
        };
        let action = SuggestedAction::no_op("Test", Duration::from_secs(60));
        let mut diagnosis = SelfDiagnosis::new("d", trigger, "test", action);

        assert_eq!(diagnosis.status, DiagnosisStatus::Pending);

        diagnosis.approve();
        assert_eq!(diagnosis.status, DiagnosisStatus::Approved);

        diagnosis.mark_executed();
        assert_eq!(diagnosis.status, DiagnosisStatus::Executed);
    }

    // NormalizedReward tests
    #[test]
    fn test_normalized_reward_new() {
        let breakdown = RewardBreakdown::new(0.1, 0.2, 0.3);
        let reward = NormalizedReward::new(0.5, breakdown, 0.8);

        assert!((reward.value - 0.5).abs() < 0.01);
        assert!((reward.confidence - 0.8).abs() < 0.01);
        assert!(reward.is_positive());
        assert!(!reward.is_negative());
    }

    #[test]
    fn test_normalized_reward_clamping() {
        let breakdown = RewardBreakdown::default();
        let reward = NormalizedReward::new(2.0, breakdown.clone(), 1.5);
        assert!((reward.value - 1.0).abs() < 0.01);
        assert!((reward.confidence - 1.0).abs() < 0.01);

        let reward2 = NormalizedReward::new(-2.0, breakdown, -0.5);
        assert!((reward2.value - (-1.0)).abs() < 0.01);
        assert!((reward2.confidence - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_normalized_reward_calculate() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.1,
            threshold: 0.15,
        };
        let pre = MetricsSnapshot::new(0.2, 200, 0.8, 100);
        let post = MetricsSnapshot::new(0.1, 150, 0.85, 100);

        let reward = NormalizedReward::calculate(&trigger, &pre, &post, 100);

        assert!(reward.is_positive());
        assert!(reward.confidence > 0.4);
    }

    #[test]
    fn test_reward_weights_for_trigger() {
        let error_trigger = TriggerMetric::ErrorRate {
            observed: 0.1,
            baseline: 0.05,
            threshold: 0.08,
        };
        let weights = RewardWeights::for_trigger(&error_trigger);
        assert!((weights.error_rate - 0.6).abs() < 0.01);

        let latency_trigger = TriggerMetric::Latency {
            observed_p95_ms: 200,
            baseline_ms: 100,
            threshold_ms: 150,
        };
        let weights = RewardWeights::for_trigger(&latency_trigger);
        assert!((weights.latency - 0.6).abs() < 0.01);
    }

    // Legacy type tests (kept for backward compatibility)
    #[test]
    fn test_action_type_display() {
        assert_eq!(ActionType::ConfigAdjust.to_string(), "config_adjust");
        assert_eq!(ActionType::PromptTune.to_string(), "prompt_tune");
    }

    #[test]
    fn test_legacy_action_new() {
        let action = SelfImprovementAction::new(
            "action-1",
            ActionType::ConfigAdjust,
            "Increase timeout",
            "Too many timeouts observed",
            0.15,
        );

        assert_eq!(action.id, "action-1");
        assert_eq!(action.action_type, ActionType::ConfigAdjust);
        assert_eq!(action.status, ActionStatus::Proposed);
    }

    #[test]
    fn test_legacy_action_lifecycle() {
        let mut action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", 0.1);

        action.approve();
        assert_eq!(action.status, ActionStatus::Approved);

        action.start_execution();
        assert_eq!(action.status, ActionStatus::Executing);

        action.complete(0.12);
        assert_eq!(action.status, ActionStatus::Completed);
    }

    #[test]
    fn test_system_metrics_new() {
        let mut mode_rates = HashMap::new();
        mode_rates.insert("linear".to_string(), 0.95);

        let metrics = SystemMetrics::new(0.9, 150.0, 1000, mode_rates);
        assert!((metrics.success_rate - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lesson_new() {
        let lesson = Lesson::new("lesson-1", "action-1", "Increasing timeout helps", 0.5);
        assert_eq!(lesson.id, "lesson-1");
        assert!((lesson.reward - 0.5).abs() < f64::EPSILON);
    }

    // ConfigScope tests
    #[test]
    fn test_config_scope_display() {
        assert_eq!(ConfigScope::Global.to_string(), "global");
        assert_eq!(
            ConfigScope::Mode("linear".into()).to_string(),
            "mode:linear"
        );
        assert_eq!(
            ConfigScope::Tool("reasoning_linear".into()).to_string(),
            "tool:reasoning_linear"
        );
    }

    #[test]
    fn test_config_scope_validate_global() {
        assert!(ConfigScope::Global.validate().is_ok());
    }

    #[test]
    fn test_config_scope_validate_known_modes() {
        for mode in ConfigScope::VALID_MODES {
            let scope = ConfigScope::Mode(mode.to_string());
            assert!(scope.validate().is_ok(), "Mode '{mode}' should be valid");
        }
    }

    #[test]
    fn test_config_scope_validate_mode_case_insensitive() {
        assert!(ConfigScope::Mode("LINEAR".into()).validate().is_ok());
        assert!(ConfigScope::Mode("Linear".into()).validate().is_ok());
        assert!(ConfigScope::Mode("tree".into()).validate().is_ok());
    }

    #[test]
    fn test_config_scope_validate_unknown_mode() {
        let result = ConfigScope::Mode("unknown".into()).validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown mode"));
    }

    #[test]
    fn test_config_scope_validate_known_tools() {
        for mode in ConfigScope::VALID_MODES {
            let tool_name = format!("reasoning_{mode}");
            let scope = ConfigScope::Tool(tool_name.clone());
            assert!(
                scope.validate().is_ok(),
                "Tool '{tool_name}' should be valid"
            );
        }
    }

    #[test]
    fn test_config_scope_validate_invalid_tool_format() {
        let result = ConfigScope::Tool("invalid_tool".into()).validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid tool format"));
    }

    #[test]
    fn test_config_scope_validate_unknown_tool_mode() {
        let result = ConfigScope::Tool("reasoning_unknown".into()).validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }

    // DiagnosisStatus tests
    #[test]
    fn test_diagnosis_status_display() {
        assert_eq!(DiagnosisStatus::Pending.to_string(), "pending");
        assert_eq!(DiagnosisStatus::Executed.to_string(), "executed");
        assert_eq!(DiagnosisStatus::RolledBack.to_string(), "rolled_back");
    }

    // ========== Additional tests for 100% coverage ==========

    // Severity Display test
    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Info.to_string(), "info");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::High.to_string(), "high");
        assert_eq!(Severity::Critical.to_string(), "critical");
    }

    // TriggerMetric::metric_type tests
    #[test]
    fn test_trigger_metric_type() {
        let error = TriggerMetric::ErrorRate {
            observed: 0.1,
            baseline: 0.05,
            threshold: 0.08,
        };
        assert_eq!(error.metric_type(), "error_rate");

        let latency = TriggerMetric::Latency {
            observed_p95_ms: 100,
            baseline_ms: 50,
            threshold_ms: 75,
        };
        assert_eq!(latency.metric_type(), "latency");

        let quality = TriggerMetric::QualityScore {
            observed: 0.8,
            baseline: 0.9,
            minimum: 0.85,
        };
        assert_eq!(quality.metric_type(), "quality_score");
    }

    // TriggerMetric not triggered cases
    #[test]
    fn test_trigger_metric_not_triggered() {
        // Error rate below threshold
        let error = TriggerMetric::ErrorRate {
            observed: 0.05,
            baseline: 0.05,
            threshold: 0.10,
        };
        assert!(!error.is_triggered());

        // Latency below threshold
        let latency = TriggerMetric::Latency {
            observed_p95_ms: 50,
            baseline_ms: 50,
            threshold_ms: 100,
        };
        assert!(!latency.is_triggered());

        // Quality above minimum
        let quality = TriggerMetric::QualityScore {
            observed: 0.9,
            baseline: 0.85,
            minimum: 0.8,
        };
        assert!(!quality.is_triggered());
    }

    // TriggerMetric zero baseline edge cases
    #[test]
    fn test_trigger_metric_zero_baseline_latency() {
        // Zero baseline with positive observed
        let latency = TriggerMetric::Latency {
            observed_p95_ms: 100,
            baseline_ms: 0,
            threshold_ms: 50,
        };
        assert!((latency.deviation_pct() - 100.0).abs() < 0.01);

        // Zero baseline with zero observed
        let latency_zero = TriggerMetric::Latency {
            observed_p95_ms: 0,
            baseline_ms: 0,
            threshold_ms: 50,
        };
        assert!((latency_zero.deviation_pct() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_trigger_metric_zero_baseline_quality() {
        // Zero baseline with observed < 1.0
        let quality = TriggerMetric::QualityScore {
            observed: 0.8,
            baseline: 0.0,
            minimum: 0.5,
        };
        assert!((quality.deviation_pct() - 100.0).abs() < 0.01);

        // Zero baseline with observed = 1.0
        let quality_full = TriggerMetric::QualityScore {
            observed: 1.0,
            baseline: 0.0,
            minimum: 0.5,
        };
        assert!((quality_full.deviation_pct() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_trigger_metric_error_rate_zero_baseline_zero_observed() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.0,
            baseline: 0.0,
            threshold: 0.05,
        };
        assert!((trigger.deviation_pct() - 0.0).abs() < 0.01);
    }

    // ParamValue constructor tests
    #[test]
    fn test_param_value_float() {
        let val = ParamValue::float(3.14);
        assert_eq!(val.to_string(), "3.14");
    }

    #[test]
    fn test_param_value_string() {
        let val = ParamValue::string("hello");
        assert_eq!(val.to_string(), "hello");
    }

    #[test]
    fn test_param_value_float_display() {
        let val = ParamValue::Float(2.5);
        assert_eq!(val.to_string(), "2.5");
    }

    #[test]
    fn test_param_value_string_display() {
        let val = ParamValue::String("test".into());
        assert_eq!(val.to_string(), "test");
    }

    // SelfDiagnosis builder methods
    #[test]
    fn test_self_diagnosis_with_suspected_cause() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.1,
            threshold: 0.15,
        };
        let action = SuggestedAction::no_op("Test", Duration::from_secs(60));
        let diagnosis =
            SelfDiagnosis::new("d", trigger, "test", action).with_suspected_cause("API timeout");

        assert_eq!(diagnosis.suspected_cause, Some("API timeout".to_string()));
    }

    #[test]
    fn test_self_diagnosis_with_action_rationale() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.1,
            threshold: 0.15,
        };
        let action = SuggestedAction::no_op("Test", Duration::from_secs(60));
        let diagnosis = SelfDiagnosis::new("d", trigger, "test", action)
            .with_action_rationale("Increase retries");

        assert_eq!(
            diagnosis.action_rationale,
            Some("Increase retries".to_string())
        );
    }

    #[test]
    fn test_self_diagnosis_reject() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.1,
            threshold: 0.15,
        };
        let action = SuggestedAction::no_op("Test", Duration::from_secs(60));
        let mut diagnosis = SelfDiagnosis::new("d", trigger, "test", action);

        diagnosis.reject();
        assert_eq!(diagnosis.status, DiagnosisStatus::Rejected);
    }

    #[test]
    fn test_self_diagnosis_mark_failed() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.1,
            threshold: 0.15,
        };
        let action = SuggestedAction::no_op("Test", Duration::from_secs(60));
        let mut diagnosis = SelfDiagnosis::new("d", trigger, "test", action);

        diagnosis.mark_failed();
        assert_eq!(diagnosis.status, DiagnosisStatus::Failed);
    }

    #[test]
    fn test_self_diagnosis_mark_rolled_back() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.1,
            threshold: 0.15,
        };
        let action = SuggestedAction::no_op("Test", Duration::from_secs(60));
        let mut diagnosis = SelfDiagnosis::new("d", trigger, "test", action);

        diagnosis.mark_rolled_back();
        assert_eq!(diagnosis.status, DiagnosisStatus::RolledBack);
    }

    // NormalizedReward::is_significant tests
    #[test]
    fn test_normalized_reward_is_significant() {
        let breakdown = RewardBreakdown::new(0.3, 0.3, 0.3);
        let reward = NormalizedReward::new(0.5, breakdown, 0.8);

        assert!(reward.is_significant(0.1));
        assert!(!reward.is_significant(0.6));
    }

    #[test]
    fn test_normalized_reward_not_significant_low_confidence() {
        let breakdown = RewardBreakdown::new(0.3, 0.3, 0.3);
        let reward = NormalizedReward::new(0.5, breakdown, 0.4);

        // High value but low confidence
        assert!(!reward.is_significant(0.1));
    }

    // RewardBreakdown::weighted_total test
    #[test]
    fn test_reward_breakdown_weighted_total() {
        let breakdown = RewardBreakdown::new(0.5, 0.3, 0.2);
        let weights = RewardWeights {
            error_rate: 0.5,
            latency: 0.3,
            quality: 0.2,
        };

        let total = breakdown.weighted_total(&weights);
        // 0.5*0.5 + 0.3*0.3 + 0.2*0.2 = 0.25 + 0.09 + 0.04 = 0.38
        assert!((total - 0.38).abs() < 0.01);
    }

    // RewardWeights::default test
    #[test]
    fn test_reward_weights_default() {
        let weights = RewardWeights::default();
        assert!((weights.error_rate - 0.34).abs() < 0.01);
        assert!((weights.latency - 0.33).abs() < 0.01);
        assert!((weights.quality - 0.33).abs() < 0.01);
    }

    // RewardWeights::for_trigger with QualityScore
    #[test]
    fn test_reward_weights_for_quality_trigger() {
        let quality_trigger = TriggerMetric::QualityScore {
            observed: 0.7,
            baseline: 0.9,
            minimum: 0.8,
        };
        let weights = RewardWeights::for_trigger(&quality_trigger);
        assert!((weights.quality - 0.6).abs() < 0.01);
        assert!((weights.error_rate - 0.2).abs() < 0.01);
        assert!((weights.latency - 0.2).abs() < 0.01);
    }

    // ToolMetrics tests
    #[test]
    fn test_tool_metrics_default() {
        let metrics = ToolMetrics::default();
        assert!((metrics.error_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(metrics.avg_latency_ms, 0);
        assert_eq!(metrics.invocation_count, 0);
    }

    // Baselines tests
    #[test]
    fn test_baselines_new() {
        let baselines = Baselines::new(0.05, 100, 0.9, 1000);
        assert!((baselines.error_rate - 0.05).abs() < f64::EPSILON);
        assert_eq!(baselines.latency_p95_ms, 100);
        assert!((baselines.quality_score - 0.9).abs() < f64::EPSILON);
        assert_eq!(baselines.sample_count, 1000);
    }

    #[test]
    fn test_baselines_default() {
        let baselines = Baselines::default();
        assert!((baselines.error_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(baselines.latency_p95_ms, 0);
        assert!((baselines.quality_score - 0.0).abs() < f64::EPSILON);
        assert_eq!(baselines.sample_count, 0);
    }

    // LegacyTriggerMetric tests
    #[test]
    fn test_legacy_trigger_metric_new() {
        let metric = LegacyTriggerMetric::new(
            "error_rate",
            0.15,
            0.10,
            Severity::High,
            "Error rate exceeded",
        );
        assert_eq!(metric.name, "error_rate");
        assert!((metric.value - 0.15).abs() < f64::EPSILON);
        assert!((metric.threshold - 0.10).abs() < f64::EPSILON);
        assert_eq!(metric.severity, Severity::High);
        assert_eq!(metric.description, "Error rate exceeded");
    }

    // ActionStatus display tests
    #[test]
    fn test_action_status_display() {
        assert_eq!(ActionStatus::Proposed.to_string(), "proposed");
        assert_eq!(ActionStatus::Approved.to_string(), "approved");
        assert_eq!(ActionStatus::Executing.to_string(), "executing");
        assert_eq!(ActionStatus::Completed.to_string(), "completed");
        assert_eq!(ActionStatus::Failed.to_string(), "failed");
        assert_eq!(ActionStatus::RolledBack.to_string(), "rolled_back");
    }

    // Lesson::with_contexts test
    #[test]
    fn test_lesson_with_contexts() {
        let lesson = Lesson::new("lesson-1", "action-1", "Increasing timeout helps", 0.5)
            .with_contexts(vec!["high_load".into(), "api_timeout".into()]);

        assert_eq!(lesson.applicable_contexts.len(), 2);
        assert_eq!(lesson.applicable_contexts[0], "high_load");
        assert_eq!(lesson.applicable_contexts[1], "api_timeout");
    }

    // SelfImprovementAction additional methods
    #[test]
    fn test_legacy_action_with_parameters() {
        let action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", 0.1)
            .with_parameters(serde_json::json!({"key": "value"}));

        assert!(action.parameters.is_some());
        assert_eq!(action.parameters.unwrap()["key"], "value");
    }

    #[test]
    fn test_legacy_action_fail() {
        let mut action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", 0.1);
        action.approve();
        action.start_execution();
        action.fail();

        assert_eq!(action.status, ActionStatus::Failed);
        assert!(action.executed_at.is_some());
    }

    #[test]
    fn test_legacy_action_rollback() {
        let mut action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", 0.1);
        action.approve();
        action.start_execution();
        action.complete(0.12);
        action.rollback();

        assert_eq!(action.status, ActionStatus::RolledBack);
    }

    // ActionType additional display tests
    #[test]
    fn test_action_type_display_all() {
        assert_eq!(ActionType::ConfigAdjust.to_string(), "config_adjust");
        assert_eq!(ActionType::PromptTune.to_string(), "prompt_tune");
        assert_eq!(ActionType::ThresholdAdjust.to_string(), "threshold_adjust");
        assert_eq!(ActionType::LogObservation.to_string(), "log_observation");
    }

    // ResourceType display all variants
    #[test]
    fn test_resource_type_display_all() {
        assert_eq!(
            ResourceType::MaxConcurrentRequests.to_string(),
            "max_concurrent_requests"
        );
        assert_eq!(
            ResourceType::ConnectionPoolSize.to_string(),
            "connection_pool_size"
        );
        assert_eq!(ResourceType::CacheSize.to_string(), "cache_size");
        assert_eq!(ResourceType::TimeoutMs.to_string(), "timeout_ms");
        assert_eq!(ResourceType::MaxRetries.to_string(), "max_retries");
        assert_eq!(ResourceType::RetryDelayMs.to_string(), "retry_delay_ms");
    }

    // DiagnosisStatus display all variants
    #[test]
    fn test_diagnosis_status_display_all() {
        assert_eq!(DiagnosisStatus::Pending.to_string(), "pending");
        assert_eq!(DiagnosisStatus::Approved.to_string(), "approved");
        assert_eq!(DiagnosisStatus::Rejected.to_string(), "rejected");
        assert_eq!(DiagnosisStatus::Executed.to_string(), "executed");
        assert_eq!(DiagnosisStatus::Failed.to_string(), "failed");
        assert_eq!(DiagnosisStatus::RolledBack.to_string(), "rolled_back");
    }

    // SuggestedAction serialization tests (for duration_serde coverage)
    #[test]
    fn test_suggested_action_no_op_serialization() {
        let action = SuggestedAction::no_op("Within acceptable range", Duration::from_secs(3600));
        let json = serde_json::to_string(&action).unwrap();

        assert!(json.contains("no_op"));
        assert!(json.contains("3600"));

        // Round-trip deserialization
        let deserialized: SuggestedAction = serde_json::from_str(&json).unwrap();
        assert!(deserialized.is_no_op());
    }

    #[test]
    fn test_suggested_action_adjust_param_serialization() {
        let action = SuggestedAction::adjust_param(
            "timeout",
            ParamValue::duration_ms(30000),
            ParamValue::duration_ms(60000),
            ConfigScope::Mode("linear".into()),
        );
        let json = serde_json::to_string(&action).unwrap();

        assert!(json.contains("adjust_param"));
        assert!(json.contains("timeout"));

        // Round-trip deserialization
        let deserialized: SuggestedAction = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.is_no_op());
        assert_eq!(deserialized.action_type(), "adjust_param");
    }

    #[test]
    fn test_suggested_action_scale_resource_serialization() {
        let action = SuggestedAction::scale_resource(ResourceType::MaxRetries, 3, 5);
        let json = serde_json::to_string(&action).unwrap();

        assert!(json.contains("scale_resource"));
        assert!(json.contains("max_retries"));

        // Round-trip deserialization
        let deserialized: SuggestedAction = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.is_no_op());
        assert_eq!(deserialized.action_type(), "scale_resource");
    }

    // NormalizedReward::calculate edge cases
    #[test]
    fn test_normalized_reward_calculate_zero_pre_error() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.1,
            threshold: 0.15,
        };
        // Pre error rate is 0
        let pre = MetricsSnapshot::new(0.0, 100, 0.8, 100);
        let post = MetricsSnapshot::new(0.1, 80, 0.85, 100);

        let reward = NormalizedReward::calculate(&trigger, &pre, &post, 100);
        // Error went from 0 to 0.1, so error component is -1.0
        assert!(reward.breakdown.error_rate_component < 0.0);
    }

    #[test]
    fn test_normalized_reward_calculate_zero_pre_latency() {
        let trigger = TriggerMetric::Latency {
            observed_p95_ms: 200,
            baseline_ms: 100,
            threshold_ms: 150,
        };
        // Pre latency is 0
        let pre = MetricsSnapshot::new(0.1, 0, 0.8, 100);
        let post = MetricsSnapshot::new(0.1, 100, 0.8, 100);

        let reward = NormalizedReward::calculate(&trigger, &pre, &post, 100);
        // Latency went from 0 to 100, so latency component is -1.0
        assert!(reward.breakdown.latency_component < 0.0);
    }

    #[test]
    fn test_normalized_reward_calculate_zero_pre_quality() {
        let trigger = TriggerMetric::QualityScore {
            observed: 0.7,
            baseline: 0.9,
            minimum: 0.8,
        };
        // Pre quality is 0
        let pre = MetricsSnapshot::new(0.1, 100, 0.0, 100);
        let post = MetricsSnapshot::new(0.1, 100, 0.8, 100);

        let reward = NormalizedReward::calculate(&trigger, &pre, &post, 100);
        // Quality went from 0 to 0.8, so quality component is 1.0
        assert!(reward.breakdown.quality_component > 0.0);
    }

    #[test]
    fn test_normalized_reward_is_negative() {
        let breakdown = RewardBreakdown::new(-0.3, -0.3, -0.3);
        let reward = NormalizedReward::new(-0.5, breakdown, 0.8);

        assert!(!reward.is_positive());
        assert!(reward.is_negative());
    }

    // MetricsSnapshot clamping
    #[test]
    fn test_metrics_snapshot_clamping() {
        let snapshot = MetricsSnapshot::new(1.5, -100, 2.0, 100);
        // Error rate clamped to 1.0
        assert!((snapshot.error_rate - 1.0).abs() < f64::EPSILON);
        // Latency clamped to 0
        assert_eq!(snapshot.latency_p95_ms, 0);
        // Quality clamped to 1.0
        assert!((snapshot.quality_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metrics_snapshot_negative_error_rate() {
        let snapshot = MetricsSnapshot::new(-0.5, 100, 0.8, 100);
        // Error rate clamped to 0.0
        assert!((snapshot.error_rate - 0.0).abs() < f64::EPSILON);
    }

    // ConfigScope serialization
    #[test]
    fn test_config_scope_serialization() {
        let global = ConfigScope::Global;
        let json = serde_json::to_string(&global).unwrap();
        assert!(json.contains("global"));

        let mode = ConfigScope::Mode("linear".into());
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("mode"));
        assert!(json.contains("linear"));

        let tool = ConfigScope::Tool("reasoning_linear".into());
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("tool"));
        assert!(json.contains("reasoning_linear"));
    }

    // TriggerMetric serialization
    #[test]
    fn test_trigger_metric_serialization() {
        let error = TriggerMetric::ErrorRate {
            observed: 0.15,
            baseline: 0.10,
            threshold: 0.12,
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("error_rate"));

        let latency = TriggerMetric::Latency {
            observed_p95_ms: 200,
            baseline_ms: 100,
            threshold_ms: 150,
        };
        let json = serde_json::to_string(&latency).unwrap();
        assert!(json.contains("latency"));

        let quality = TriggerMetric::QualityScore {
            observed: 0.8,
            baseline: 0.9,
            minimum: 0.85,
        };
        let json = serde_json::to_string(&quality).unwrap();
        assert!(json.contains("quality_score"));
    }

    // SelfDiagnosis serialization
    #[test]
    fn test_self_diagnosis_serialization() {
        let trigger = TriggerMetric::ErrorRate {
            observed: 0.2,
            baseline: 0.1,
            threshold: 0.15,
        };
        let action = SuggestedAction::scale_resource(ResourceType::MaxRetries, 3, 5);
        let diagnosis = SelfDiagnosis::new("diag-1", trigger, "High error rate", action);

        let json = serde_json::to_string(&diagnosis).unwrap();
        assert!(json.contains("diag-1"));
        assert!(json.contains("High error rate"));
        assert!(json.contains("pending"));
    }

    // Legacy types serialization
    #[test]
    fn test_system_metrics_serialization() {
        let mut mode_rates = HashMap::new();
        mode_rates.insert("linear".to_string(), 0.95);
        let metrics = SystemMetrics::new(0.9, 150.0, 1000, mode_rates);

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("success_rate"));
        assert!(json.contains("linear"));
    }

    #[test]
    fn test_lesson_serialization() {
        let lesson = Lesson::new("lesson-1", "action-1", "Insight", 0.5)
            .with_contexts(vec!["context1".into()]);

        let json = serde_json::to_string(&lesson).unwrap();
        assert!(json.contains("lesson-1"));
        assert!(json.contains("Insight"));
        assert!(json.contains("context1"));
    }

    #[test]
    fn test_self_improvement_action_serialization() {
        let action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "desc", "rat", 0.15)
            .with_parameters(serde_json::json!({"key": "value"}));

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("config_adjust"));
        assert!(json.contains("desc"));
        assert!(json.contains("key"));
    }

    // Expected improvement clamping
    #[test]
    fn test_legacy_action_expected_improvement_clamping() {
        let action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", 1.5);
        assert!((action.expected_improvement - 1.0).abs() < f64::EPSILON);

        let action2 = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", -0.5);
        assert!((action2.expected_improvement - 0.0).abs() < f64::EPSILON);
    }

    // Lesson reward clamping
    #[test]
    fn test_lesson_reward_clamping() {
        let lesson = Lesson::new("l", "a", "i", 1.5);
        assert!((lesson.reward - 1.0).abs() < f64::EPSILON);

        let lesson2 = Lesson::new("l", "a", "i", -1.5);
        assert!((lesson2.reward - (-1.0)).abs() < f64::EPSILON);
    }
}
