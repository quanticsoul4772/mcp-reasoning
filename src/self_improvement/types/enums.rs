//! Core enumeration types for the self-improvement system.
//!
//! This module contains all enum types used throughout the self-improvement
//! 4-phase optimization loop.

use serde::{Deserialize, Serialize};

// ============================================================================
// Type Aliases
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
// ParamValue (DESIGN.md 14.2)
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
// ConfigScope (DESIGN.md 14.2)
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
// ResourceType (DESIGN.md 14.2)
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
// DiagnosisStatus (DESIGN.md 14.2)
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
// ActionType (DESIGN.md 14.2)
// ============================================================================

/// Type of self-improvement action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Adjust configuration parameter.
    ConfigAdjust,
    /// Scale resources.
    ResourceScale,
    /// No action (monitoring only).
    NoOp,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigAdjust => write!(f, "config_adjust"),
            Self::ResourceScale => write!(f, "resource_scale"),
            Self::NoOp => write!(f, "no_op"),
        }
    }
}

// ============================================================================
// ActionStatus (DESIGN.md 14.2)
// ============================================================================

/// Status of a self-improvement action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// Action proposed, awaiting approval.
    Proposed,
    /// Action approved for execution.
    Approved,
    /// Action executed successfully.
    Executed,
    /// Action execution failed.
    Failed,
    /// Action was rolled back.
    RolledBack,
}

impl std::fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proposed => write!(f, "proposed"),
            Self::Approved => write!(f, "approved"),
            Self::Executed => write!(f, "executed"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled_back"),
        }
    }
}

// ============================================================================
// Duration Serde Helper
// ============================================================================

/// Serde module for Duration serialization.
pub mod duration_serde {
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
