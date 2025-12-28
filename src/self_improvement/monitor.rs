//! Self-improvement monitoring.
//!
//! Phase 1 of the 4-phase loop: Collect metrics and detect issues.

use super::types::{Severity, SystemMetrics, TriggerMetric};
use crate::metrics::{MetricsCollector, MetricsSummary};
use std::collections::HashMap;

/// Configuration for monitoring thresholds.
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Minimum success rate before triggering (0.0-1.0).
    pub min_success_rate: f64,
    /// Maximum average latency in milliseconds.
    pub max_avg_latency_ms: f64,
    /// Minimum invocations before analysis.
    pub min_invocations: u64,
    /// Per-mode success rate threshold.
    pub mode_success_threshold: f64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            min_success_rate: 0.8,
            max_avg_latency_ms: 5000.0,
            min_invocations: 10,
            mode_success_threshold: 0.7,
        }
    }
}

/// Baseline metrics for comparison.
#[derive(Debug, Clone, Default)]
pub struct Baseline {
    /// Expected success rate.
    pub success_rate: f64,
    /// Expected average latency.
    pub avg_latency_ms: f64,
    /// Per-mode expected success rates.
    pub mode_success_rates: HashMap<String, f64>,
    /// Number of samples used to calculate baseline.
    pub sample_count: u64,
}

impl Baseline {
    /// Create a new baseline from a metrics summary.
    #[must_use]
    pub fn from_summary(summary: &MetricsSummary) -> Self {
        let mode_success_rates: HashMap<String, f64> = summary
            .by_mode
            .iter()
            .map(|(mode, stats)| (mode.clone(), stats.success_rate))
            .collect();

        let avg_latency = if summary.by_mode.is_empty() {
            0.0
        } else {
            summary
                .by_mode
                .values()
                .map(|s| s.avg_latency_ms)
                .sum::<f64>()
                / summary.by_mode.len() as f64
        };

        Self {
            success_rate: summary.overall_success_rate,
            avg_latency_ms: avg_latency,
            mode_success_rates,
            sample_count: summary.total_invocations,
        }
    }
}

/// Monitoring results.
#[derive(Debug, Clone)]
pub struct MonitorResult {
    /// Current system metrics.
    pub metrics: SystemMetrics,
    /// Triggered issues.
    pub triggers: Vec<TriggerMetric>,
    /// Whether action is recommended.
    pub action_recommended: bool,
}

/// Monitor for the self-improvement system.
#[derive(Debug)]
pub struct Monitor {
    config: MonitorConfig,
    baseline: Option<Baseline>,
}

impl Monitor {
    /// Create a new monitor with the given configuration.
    #[must_use]
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            baseline: None,
        }
    }

    /// Create a monitor with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(MonitorConfig::default())
    }

    /// Set the baseline for comparison.
    pub fn set_baseline(&mut self, baseline: Baseline) {
        self.baseline = Some(baseline);
    }

    /// Calculate and set baseline from metrics collector.
    pub fn calculate_baseline(&mut self, collector: &MetricsCollector) {
        let summary = collector.summary();
        self.baseline = Some(Baseline::from_summary(&summary));
    }

    /// Check metrics and detect issues.
    #[must_use]
    pub fn check(&self, collector: &MetricsCollector) -> MonitorResult {
        let summary = collector.summary();
        let mut triggers = Vec::new();

        // Skip analysis if insufficient data
        if summary.total_invocations < self.config.min_invocations {
            return MonitorResult {
                metrics: self.summary_to_metrics(&summary),
                triggers,
                action_recommended: false,
            };
        }

        // Check overall success rate
        if summary.overall_success_rate < self.config.min_success_rate {
            let severity =
                self.calculate_severity(summary.overall_success_rate, self.config.min_success_rate);
            triggers.push(TriggerMetric::new(
                "overall_success_rate",
                summary.overall_success_rate,
                self.config.min_success_rate,
                severity,
                format!(
                    "Overall success rate {:.1}% is below threshold {:.1}%",
                    summary.overall_success_rate * 100.0,
                    self.config.min_success_rate * 100.0
                ),
            ));
        }

        // Check per-mode success rates
        for (mode, stats) in &summary.by_mode {
            if stats.success_rate < self.config.mode_success_threshold {
                let severity =
                    self.calculate_severity(stats.success_rate, self.config.mode_success_threshold);
                triggers.push(TriggerMetric::new(
                    format!("mode_{mode}_success_rate"),
                    stats.success_rate,
                    self.config.mode_success_threshold,
                    severity,
                    format!(
                        "Mode '{mode}' success rate {:.1}% is below threshold {:.1}%",
                        stats.success_rate * 100.0,
                        self.config.mode_success_threshold * 100.0
                    ),
                ));
            }

            // Check latency
            if stats.avg_latency_ms > self.config.max_avg_latency_ms {
                let severity = self.calculate_latency_severity(stats.avg_latency_ms);
                triggers.push(TriggerMetric::new(
                    format!("mode_{mode}_latency"),
                    stats.avg_latency_ms,
                    self.config.max_avg_latency_ms,
                    severity,
                    format!(
                        "Mode '{mode}' average latency {:.0}ms exceeds threshold {:.0}ms",
                        stats.avg_latency_ms, self.config.max_avg_latency_ms
                    ),
                ));
            }
        }

        // Check deviation from baseline if available
        if let Some(baseline) = &self.baseline {
            self.check_baseline_deviation(&summary, baseline, &mut triggers);
        }

        let action_recommended =
            !triggers.is_empty() && triggers.iter().any(|t| t.severity != Severity::Low);

        MonitorResult {
            metrics: self.summary_to_metrics(&summary),
            triggers,
            action_recommended,
        }
    }

    fn summary_to_metrics(&self, summary: &MetricsSummary) -> SystemMetrics {
        let avg_latency = if summary.by_mode.is_empty() {
            0.0
        } else {
            summary
                .by_mode
                .values()
                .map(|s| s.avg_latency_ms)
                .sum::<f64>()
                / summary.by_mode.len() as f64
        };

        let mode_success_rates: HashMap<String, f64> = summary
            .by_mode
            .iter()
            .map(|(mode, stats)| (mode.clone(), stats.success_rate))
            .collect();

        SystemMetrics::new(
            summary.overall_success_rate,
            avg_latency,
            summary.total_invocations,
            mode_success_rates,
        )
    }

    fn calculate_severity(&self, value: f64, threshold: f64) -> Severity {
        let deviation = (threshold - value) / threshold;
        if deviation > 0.5 {
            Severity::Critical
        } else if deviation > 0.3 {
            Severity::High
        } else if deviation > 0.15 {
            Severity::Medium
        } else {
            Severity::Low
        }
    }

    fn calculate_latency_severity(&self, latency_ms: f64) -> Severity {
        let ratio = latency_ms / self.config.max_avg_latency_ms;
        if ratio > 3.0 {
            Severity::Critical
        } else if ratio > 2.0 {
            Severity::High
        } else if ratio > 1.5 {
            Severity::Medium
        } else {
            Severity::Low
        }
    }

    fn check_baseline_deviation(
        &self,
        summary: &MetricsSummary,
        baseline: &Baseline,
        triggers: &mut Vec<TriggerMetric>,
    ) {
        // Check success rate deviation from baseline
        if baseline.success_rate > 0.0 {
            let deviation =
                (baseline.success_rate - summary.overall_success_rate) / baseline.success_rate;
            if deviation > 0.2 {
                triggers.push(TriggerMetric::new(
                    "success_rate_deviation",
                    summary.overall_success_rate,
                    baseline.success_rate,
                    if deviation > 0.4 {
                        Severity::High
                    } else {
                        Severity::Medium
                    },
                    format!(
                        "Success rate dropped {:.1}% from baseline",
                        deviation * 100.0
                    ),
                ));
            }
        }

        // Check latency deviation from baseline
        let current_latency = if summary.by_mode.is_empty() {
            0.0
        } else {
            summary
                .by_mode
                .values()
                .map(|s| s.avg_latency_ms)
                .sum::<f64>()
                / summary.by_mode.len() as f64
        };

        if baseline.avg_latency_ms > 0.0 {
            let deviation = (current_latency - baseline.avg_latency_ms) / baseline.avg_latency_ms;
            if deviation > 0.5 {
                triggers.push(TriggerMetric::new(
                    "latency_deviation",
                    current_latency,
                    baseline.avg_latency_ms,
                    if deviation > 1.0 {
                        Severity::High
                    } else {
                        Severity::Medium
                    },
                    format!(
                        "Average latency increased {:.1}% from baseline",
                        deviation * 100.0
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricEvent;

    #[test]
    fn test_monitor_config_default() {
        let config = MonitorConfig::default();
        assert!((config.min_success_rate - 0.8).abs() < f64::EPSILON);
        assert!((config.max_avg_latency_ms - 5000.0).abs() < f64::EPSILON);
        assert_eq!(config.min_invocations, 10);
    }

    #[test]
    fn test_baseline_from_summary() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        collector.record(MetricEvent::new("linear", 200, true));
        collector.record(MetricEvent::new("tree", 150, false));

        let summary = collector.summary();
        let baseline = Baseline::from_summary(&summary);

        assert_eq!(baseline.sample_count, 3);
        assert!(baseline.success_rate > 0.0);
        assert!(baseline.mode_success_rates.contains_key("linear"));
    }

    #[test]
    fn test_monitor_insufficient_data() {
        let monitor = Monitor::with_defaults();
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));

        let result = monitor.check(&collector);
        assert!(result.triggers.is_empty());
        assert!(!result.action_recommended);
    }

    #[test]
    fn test_monitor_success_rate_trigger() {
        let config = MonitorConfig {
            min_success_rate: 0.8,
            min_invocations: 5,
            ..Default::default()
        };
        let monitor = Monitor::new(config);
        let collector = MetricsCollector::new();

        // 3 success, 7 failures = 30% success rate
        for _ in 0..3 {
            collector.record(MetricEvent::new("linear", 100, true));
        }
        for _ in 0..7 {
            collector.record(MetricEvent::new("linear", 100, false));
        }

        let result = monitor.check(&collector);
        assert!(!result.triggers.is_empty());
        assert!(result.action_recommended);

        let success_trigger = result
            .triggers
            .iter()
            .find(|t| t.name == "overall_success_rate");
        assert!(success_trigger.is_some());
    }

    #[test]
    fn test_monitor_latency_trigger() {
        let config = MonitorConfig {
            max_avg_latency_ms: 1000.0,
            min_invocations: 5,
            ..Default::default()
        };
        let monitor = Monitor::new(config);
        let collector = MetricsCollector::new();

        for _ in 0..10 {
            collector.record(MetricEvent::new("linear", 2000, true));
        }

        let result = monitor.check(&collector);
        let latency_trigger = result.triggers.iter().find(|t| t.name.contains("latency"));
        assert!(latency_trigger.is_some());
    }

    #[test]
    fn test_monitor_mode_specific_trigger() {
        let config = MonitorConfig {
            mode_success_threshold: 0.8,
            min_invocations: 5,
            ..Default::default()
        };
        let monitor = Monitor::new(config);
        let collector = MetricsCollector::new();

        // Linear mode: 100% success
        for _ in 0..5 {
            collector.record(MetricEvent::new("linear", 100, true));
        }
        // Tree mode: 0% success
        for _ in 0..5 {
            collector.record(MetricEvent::new("tree", 100, false));
        }

        let result = monitor.check(&collector);
        let tree_trigger = result.triggers.iter().find(|t| t.name.contains("tree"));
        assert!(tree_trigger.is_some());
    }

    #[test]
    fn test_monitor_baseline_deviation() {
        let mut monitor = Monitor::with_defaults();
        let collector = MetricsCollector::new();

        // Set high baseline
        let mut baseline = Baseline::default();
        baseline.success_rate = 0.95;
        baseline.avg_latency_ms = 100.0;
        baseline.sample_count = 100;
        monitor.set_baseline(baseline);

        // Record poor performance (below baseline)
        for _ in 0..20 {
            collector.record(MetricEvent::new("linear", 500, false));
        }

        let result = monitor.check(&collector);
        let deviation_trigger = result
            .triggers
            .iter()
            .find(|t| t.name.contains("deviation"));
        assert!(deviation_trigger.is_some());
    }

    #[test]
    fn test_calculate_baseline() {
        let mut monitor = Monitor::with_defaults();
        let collector = MetricsCollector::new();

        for _ in 0..10 {
            collector.record(MetricEvent::new("linear", 100, true));
        }

        monitor.calculate_baseline(&collector);
        assert!(monitor.baseline.is_some());

        let baseline = monitor.baseline.as_ref().unwrap();
        assert!((baseline.success_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_severity_calculation() {
        let monitor = Monitor::with_defaults();

        // Critical: >50% deviation
        let severity = monitor.calculate_severity(0.3, 0.8);
        assert_eq!(severity, Severity::Critical);

        // High: 30-50% deviation
        let severity = monitor.calculate_severity(0.5, 0.8);
        assert_eq!(severity, Severity::High);

        // Medium: 15-30% deviation
        let severity = monitor.calculate_severity(0.65, 0.8);
        assert_eq!(severity, Severity::Medium);

        // Low: <15% deviation
        let severity = monitor.calculate_severity(0.75, 0.8);
        assert_eq!(severity, Severity::Low);
    }

    #[test]
    fn test_latency_severity_calculation() {
        let config = MonitorConfig {
            max_avg_latency_ms: 1000.0,
            ..Default::default()
        };
        let monitor = Monitor::new(config);

        assert_eq!(
            monitor.calculate_latency_severity(3500.0),
            Severity::Critical
        );
        assert_eq!(monitor.calculate_latency_severity(2500.0), Severity::High);
        assert_eq!(monitor.calculate_latency_severity(1600.0), Severity::Medium);
        assert_eq!(monitor.calculate_latency_severity(1200.0), Severity::Low);
    }

    #[test]
    fn test_monitor_result_metrics() {
        let config = MonitorConfig {
            min_invocations: 5,
            ..Default::default()
        };
        let monitor = Monitor::new(config);
        let collector = MetricsCollector::new();

        for _ in 0..10 {
            collector.record(MetricEvent::new("linear", 100, true));
        }

        let result = monitor.check(&collector);
        assert_eq!(result.metrics.total_invocations, 10);
        assert!((result.metrics.success_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_no_action_for_low_severity() {
        let config = MonitorConfig {
            min_success_rate: 0.8,
            min_invocations: 5,
            ..Default::default()
        };
        let monitor = Monitor::new(config);
        let collector = MetricsCollector::new();

        // 75% success rate - just below threshold, low severity
        for _ in 0..15 {
            collector.record(MetricEvent::new("linear", 100, true));
        }
        for _ in 0..5 {
            collector.record(MetricEvent::new("linear", 100, false));
        }

        let result = monitor.check(&collector);
        // Triggers exist but should be low severity
        let all_low = result.triggers.iter().all(|t| t.severity == Severity::Low);
        if !result.triggers.is_empty() && all_low {
            assert!(!result.action_recommended);
        }
    }
}
