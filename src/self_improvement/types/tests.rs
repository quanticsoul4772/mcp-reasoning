//! Tests for self-improvement types.

use super::*;
use std::collections::HashMap;

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
// ParamValue tests
// ResourceType tests
// SuggestedAction tests
// SelfDiagnosis tests
// NormalizedReward tests
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
// TriggerMetric not triggered cases
// TriggerMetric zero baseline edge cases
// ParamValue constructor tests
// SelfDiagnosis builder methods
// NormalizedReward::is_significant tests
// RewardBreakdown::weighted_total test
// RewardWeights::default test
// RewardWeights::for_trigger with QualityScore
// ToolMetrics tests
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
fn test_legacy_action_status_display() {
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
fn test_legacy_action_type_display_all() {
    assert_eq!(ActionType::ConfigAdjust.to_string(), "config_adjust");
    assert_eq!(ActionType::PromptTune.to_string(), "prompt_tune");
    assert_eq!(ActionType::ThresholdAdjust.to_string(), "threshold_adjust");
    assert_eq!(ActionType::LogObservation.to_string(), "log_observation");
}

// ResourceType display all variants
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
// NormalizedReward::calculate edge cases
// MetricsSnapshot clamping
// ConfigScope serialization
// TriggerMetric serialization
// SelfDiagnosis serialization
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
    let lesson =
        Lesson::new("lesson-1", "action-1", "Insight", 0.5).with_contexts(vec!["context1".into()]);

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
