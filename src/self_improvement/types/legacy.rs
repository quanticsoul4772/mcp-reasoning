//! Legacy types for backward compatibility.
//!
//! These types are kept for backward compatibility during the transition
//! to the new type system. New code should use the types in other modules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::enums::Severity;

// ============================================================================
// ActionType (Legacy)
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

/// Error parsing an [`ActionType`] from its string form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseActionTypeError {
    /// The input that failed to parse.
    pub input: String,
}

impl std::fmt::Display for ParseActionTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown action type: '{}'", self.input)
    }
}

impl std::error::Error for ParseActionTypeError {}

impl std::str::FromStr for ActionType {
    type Err = ParseActionTypeError;

    /// Parse the lowercase string form produced by [`Display`]. The inverse of
    /// `to_string()`, used to rehydrate persisted action-type keys.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "config_adjust" => Ok(Self::ConfigAdjust),
            "prompt_tune" => Ok(Self::PromptTune),
            "threshold_adjust" => Ok(Self::ThresholdAdjust),
            "log_observation" => Ok(Self::LogObservation),
            _ => Err(ParseActionTypeError {
                input: s.to_string(),
            }),
        }
    }
}

// ============================================================================
// ActionStatus (Legacy)
// ============================================================================

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
    /// Action completed successfully — its effect reached the running system
    /// (a logged observation, or a config change applied to the live `Config`).
    Completed,
    /// Action ran and a recommendation was recorded, but nothing was applied to
    /// the running server (advisory mode). Accurate counterpart to `Completed` for
    /// config/threshold/prompt actions that only persist a recommendation.
    Recommended,
    /// Action failed during execution.
    Failed,
    /// Action was rolled back.
    RolledBack,
    /// Action was rejected before execution.
    Rejected,
}

impl std::fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proposed => write!(f, "proposed"),
            Self::Approved => write!(f, "approved"),
            Self::Executing => write!(f, "executing"),
            Self::Completed => write!(f, "completed"),
            Self::Recommended => write!(f, "recommended"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled_back"),
            Self::Rejected => write!(f, "rejected"),
        }
    }
}

// ============================================================================
// SelfImprovementAction (Legacy)
// ============================================================================

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
                .map_or(0, |d| d.as_secs()),
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
                .map_or(0, |d| d.as_secs()),
        );
    }

    /// Mark action as failed.
    pub fn fail(&mut self) {
        self.status = ActionStatus::Failed;
        self.executed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        );
    }

    /// Mark action as rolled back.
    pub fn rollback(&mut self) {
        self.status = ActionStatus::RolledBack;
    }
}

// ============================================================================
// SystemMetrics (Legacy)
// ============================================================================

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
                .map_or(0, |d| d.as_secs()),
        }
    }
}

// ============================================================================
// LegacyTriggerMetric
// ============================================================================

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

// ============================================================================
// Lesson
// ============================================================================

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
                .map_or(0, |d| d.as_secs()),
        }
    }

    /// Add applicable contexts.
    #[must_use]
    pub fn with_contexts(mut self, contexts: Vec<String>) -> Self {
        self.applicable_contexts = contexts;
        self
    }
}
