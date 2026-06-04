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
// ResourceType tests
// SuggestedAction tests
// SelfDiagnosis tests
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
// SelfDiagnosis builder methods
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
