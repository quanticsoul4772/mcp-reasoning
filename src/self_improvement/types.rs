//! Self-improvement system types.
//!
//! Core types for the 4-phase optimization loop:
//! - Monitor: Metric collection
//! - Analyze: LLM diagnosis
//! - Execute: Action application
//! - Learn: Lesson extraction

use serde::{Deserialize, Serialize};

/// Type of improvement action.
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

/// Status of an improvement action.
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

/// A proposed or executed improvement action.
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
    pub const fn approve(&mut self) {
        self.status = ActionStatus::Approved;
    }

    /// Mark action as executing.
    pub const fn start_execution(&mut self) {
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
    pub const fn rollback(&mut self) {
        self.status = ActionStatus::RolledBack;
    }
}

/// System-wide metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Overall success rate (0.0-1.0).
    pub success_rate: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Total invocations.
    pub total_invocations: u64,
    /// Per-mode success rates.
    pub mode_success_rates: std::collections::HashMap<String, f64>,
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
        mode_success_rates: std::collections::HashMap<String, f64>,
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

/// Severity level for detected issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Low severity - minor issue.
    Low,
    /// Medium severity - notable issue.
    Medium,
    /// High severity - significant issue.
    High,
    /// Critical severity - immediate attention needed.
    Critical,
}

/// A trigger metric that indicates potential issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerMetric {
    /// Metric name.
    pub name: String,
    /// Current value.
    pub value: f64,
    /// Threshold that was crossed.
    pub threshold: f64,
    /// Severity of the issue.
    pub severity: Severity,
    /// Description of the issue.
    pub description: String,
}

impl TriggerMetric {
    /// Create a new trigger metric.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_display() {
        assert_eq!(ActionType::ConfigAdjust.to_string(), "config_adjust");
        assert_eq!(ActionType::PromptTune.to_string(), "prompt_tune");
        assert_eq!(ActionType::ThresholdAdjust.to_string(), "threshold_adjust");
        assert_eq!(ActionType::LogObservation.to_string(), "log_observation");
    }

    #[test]
    fn test_action_new() {
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
        assert!((action.expected_improvement - 0.15).abs() < f64::EPSILON);
        assert!(action.actual_improvement.is_none());
    }

    #[test]
    fn test_action_expected_improvement_clamped() {
        let action = SelfImprovementAction::new(
            "a",
            ActionType::ConfigAdjust,
            "d",
            "r",
            1.5, // Above max
        );
        assert!((action.expected_improvement - 1.0).abs() < f64::EPSILON);

        let action2 = SelfImprovementAction::new(
            "a",
            ActionType::ConfigAdjust,
            "d",
            "r",
            -0.5, // Below min
        );
        assert!((action2.expected_improvement - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_action_with_parameters() {
        let action = SelfImprovementAction::new("a", ActionType::PromptTune, "d", "r", 0.1)
            .with_parameters(serde_json::json!({"prompt": "new_prompt"}));

        assert!(action.parameters.is_some());
    }

    #[test]
    fn test_action_lifecycle() {
        let mut action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", 0.1);

        assert_eq!(action.status, ActionStatus::Proposed);

        action.approve();
        assert_eq!(action.status, ActionStatus::Approved);

        action.start_execution();
        assert_eq!(action.status, ActionStatus::Executing);

        action.complete(0.12);
        assert_eq!(action.status, ActionStatus::Completed);
        assert!((action.actual_improvement.unwrap() - 0.12).abs() < f64::EPSILON);
        assert!(action.executed_at.is_some());
    }

    #[test]
    fn test_action_fail() {
        let mut action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", 0.1);

        action.approve();
        action.start_execution();
        action.fail();

        assert_eq!(action.status, ActionStatus::Failed);
        assert!(action.executed_at.is_some());
    }

    #[test]
    fn test_action_rollback() {
        let mut action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", 0.1);

        action.approve();
        action.start_execution();
        action.complete(0.05);
        action.rollback();

        assert_eq!(action.status, ActionStatus::RolledBack);
    }

    #[test]
    fn test_system_metrics_new() {
        let mut mode_rates = std::collections::HashMap::new();
        mode_rates.insert("linear".to_string(), 0.95);
        mode_rates.insert("tree".to_string(), 0.85);

        let metrics = SystemMetrics::new(0.9, 150.0, 1000, mode_rates);

        assert!((metrics.success_rate - 0.9).abs() < f64::EPSILON);
        assert!((metrics.avg_latency_ms - 150.0).abs() < f64::EPSILON);
        assert_eq!(metrics.total_invocations, 1000);
        assert_eq!(metrics.mode_success_rates.len(), 2);
    }

    #[test]
    fn test_system_metrics_clamping() {
        let metrics = SystemMetrics::new(1.5, -10.0, 100, std::collections::HashMap::new());

        assert!((metrics.success_rate - 1.0).abs() < f64::EPSILON);
        assert!((metrics.avg_latency_ms - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lesson_new() {
        let lesson = Lesson::new("lesson-1", "action-1", "Increasing timeout helps", 0.5);

        assert_eq!(lesson.id, "lesson-1");
        assert_eq!(lesson.action_id, "action-1");
        assert!((lesson.reward - 0.5).abs() < f64::EPSILON);
        assert!(lesson.applicable_contexts.is_empty());
    }

    #[test]
    fn test_lesson_reward_clamped() {
        let lesson = Lesson::new("l", "a", "i", 2.0);
        assert!((lesson.reward - 1.0).abs() < f64::EPSILON);

        let lesson2 = Lesson::new("l", "a", "i", -2.0);
        assert!((lesson2.reward - (-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lesson_with_contexts() {
        let lesson = Lesson::new("l", "a", "i", 0.5)
            .with_contexts(vec!["high_load".to_string(), "timeout".to_string()]);

        assert_eq!(lesson.applicable_contexts.len(), 2);
    }

    #[test]
    fn test_trigger_metric_new() {
        let trigger = TriggerMetric::new(
            "error_rate",
            0.15,
            0.10,
            Severity::High,
            "Error rate exceeded threshold",
        );

        assert_eq!(trigger.name, "error_rate");
        assert!((trigger.value - 0.15).abs() < f64::EPSILON);
        assert!((trigger.threshold - 0.10).abs() < f64::EPSILON);
        assert_eq!(trigger.severity, Severity::High);
    }

    #[test]
    fn test_severity_serialize() {
        assert_eq!(
            serde_json::to_string(&Severity::Critical).unwrap(),
            "\"critical\""
        );
    }

    #[test]
    fn test_action_serialize() {
        let action = SelfImprovementAction::new("a", ActionType::ConfigAdjust, "d", "r", 0.1);
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"action_type\":\"config_adjust\""));
        assert!(json.contains("\"status\":\"proposed\""));
    }
}
