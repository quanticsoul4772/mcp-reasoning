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
    /// Action executed successfully and its effect reached the running system.
    Executed,
    /// Action ran but only a recommendation was recorded — nothing was applied
    /// to the running server (advisory mode).
    Recommended,
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
            Self::Recommended => write!(f, "recommended"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled_back"),
        }
    }
}
