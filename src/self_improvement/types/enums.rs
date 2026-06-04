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
