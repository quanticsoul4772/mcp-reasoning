//! Metrics collection.
//!
//! This module provides:
//! - Usage metrics tracking per mode
//! - Latency measurements
//! - Success/failure rates
//! - Query interfaces for metrics data
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::metrics::{MetricsCollector, MetricEvent};
//!
//! let metrics = MetricsCollector::new();
//! metrics.record(MetricEvent::new("linear", 150, true));
//! metrics.record(MetricEvent::new("linear", 200, true));
//! metrics.record(MetricEvent::new("tree", 300, false));
//!
//! let summary = metrics.summary();
//! assert_eq!(summary.total_invocations, 3);
//! // 2 out of 3 succeeded = ~66.7%
//! assert!((summary.overall_success_rate - 0.666).abs() < 0.01);
//! // Per-mode stats are available
//! assert!(summary.by_mode.contains_key("linear"));
//! assert!(summary.by_mode.contains_key("tree"));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;

/// A single metric event recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEvent {
    /// Mode that was invoked.
    pub mode: String,
    /// Operation within the mode (if applicable).
    pub operation: Option<String>,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Whether the invocation succeeded.
    pub success: bool,
    /// Timestamp of the event (Unix epoch seconds).
    pub timestamp: u64,
}

impl MetricEvent {
    /// Create a new metric event.
    #[must_use]
    pub fn new(mode: impl Into<String>, latency_ms: u64, success: bool) -> Self {
        Self {
            mode: mode.into(),
            operation: None,
            latency_ms,
            success,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Create an event with an operation.
    #[must_use]
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }
}

/// Summary statistics for a mode.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModeSummary {
    /// Total invocations.
    pub total_invocations: u64,
    /// Successful invocations.
    pub successful: u64,
    /// Failed invocations.
    pub failed: u64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Minimum latency in milliseconds.
    pub min_latency_ms: u64,
    /// Maximum latency in milliseconds.
    pub max_latency_ms: u64,
    /// Success rate (0.0-1.0).
    pub success_rate: f64,
}

/// Overall metrics summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    /// Total invocations across all modes.
    pub total_invocations: u64,
    /// Overall success rate.
    pub overall_success_rate: f64,
    /// Per-mode summaries.
    pub by_mode: HashMap<String, ModeSummary>,
    /// Recent fallbacks (mode → fallback mode).
    pub recent_fallbacks: Vec<FallbackEvent>,
}

/// A fallback event when a mode fails and routes to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackEvent {
    /// Original mode that failed.
    pub from_mode: String,
    /// Mode that handled the fallback.
    pub to_mode: String,
    /// Reason for fallback.
    pub reason: String,
    /// Timestamp.
    pub timestamp: u64,
}

impl FallbackEvent {
    /// Create a new fallback event.
    #[must_use]
    pub fn new(
        from_mode: impl Into<String>,
        to_mode: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            from_mode: from_mode.into(),
            to_mode: to_mode.into(),
            reason: reason.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

/// Thread-safe metrics collector.
#[derive(Debug, Default)]
pub struct MetricsCollector {
    events: RwLock<Vec<MetricEvent>>,
    fallbacks: RwLock<Vec<FallbackEvent>>,
}

impl MetricsCollector {
    /// Create a new metrics collector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a metric event.
    pub fn record(&self, event: MetricEvent) {
        if let Ok(mut events) = self.events.write() {
            events.push(event);
        }
    }

    /// Record a fallback event.
    pub fn record_fallback(&self, fallback: FallbackEvent) {
        if let Ok(mut fallbacks) = self.fallbacks.write() {
            fallbacks.push(fallback);
        }
    }

    /// Get summary statistics.
    #[must_use]
    pub fn summary(&self) -> MetricsSummary {
        let events = self.events.read().map(|e| e.clone()).unwrap_or_default();
        let fallbacks = self.fallbacks.read().map(|f| f.clone()).unwrap_or_default();

        let mut by_mode: HashMap<String, Vec<&MetricEvent>> = HashMap::new();
        for event in &events {
            by_mode.entry(event.mode.clone()).or_default().push(event);
        }

        let mode_summaries: HashMap<String, ModeSummary> = by_mode
            .into_iter()
            .map(|(mode, mode_events)| {
                let total = mode_events.len() as u64;
                let successful = mode_events.iter().filter(|e| e.success).count() as u64;
                let failed = total - successful;

                let latencies: Vec<u64> = mode_events.iter().map(|e| e.latency_ms).collect();
                let avg_latency = if latencies.is_empty() {
                    0.0
                } else {
                    latencies.iter().sum::<u64>() as f64 / latencies.len() as f64
                };
                let min_latency = latencies.iter().copied().min().unwrap_or(0);
                let max_latency = latencies.iter().copied().max().unwrap_or(0);
                let success_rate = if total > 0 {
                    successful as f64 / total as f64
                } else {
                    0.0
                };

                (
                    mode,
                    ModeSummary {
                        total_invocations: total,
                        successful,
                        failed,
                        avg_latency_ms: avg_latency,
                        min_latency_ms: min_latency,
                        max_latency_ms: max_latency,
                        success_rate,
                    },
                )
            })
            .collect();

        let total_invocations = events.len() as u64;
        let total_successful = events.iter().filter(|e| e.success).count() as u64;
        let overall_success_rate = if total_invocations > 0 {
            total_successful as f64 / total_invocations as f64
        } else {
            1.0
        };

        MetricsSummary {
            total_invocations,
            overall_success_rate,
            by_mode: mode_summaries,
            recent_fallbacks: fallbacks,
        }
    }

    /// Get invocations for a specific mode.
    #[must_use]
    pub fn invocations_by_mode(&self, mode: &str) -> Vec<MetricEvent> {
        self.events
            .read()
            .map(|events| events.iter().filter(|e| e.mode == mode).cloned().collect())
            .unwrap_or_default()
    }

    /// Get invocations within a time range.
    #[must_use]
    pub fn invocations_in_range(&self, start: u64, end: u64) -> Vec<MetricEvent> {
        self.events
            .read()
            .map(|events| {
                events
                    .iter()
                    .filter(|e| e.timestamp >= start && e.timestamp <= end)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get recent fallbacks.
    #[must_use]
    pub fn fallbacks(&self) -> Vec<FallbackEvent> {
        self.fallbacks.read().map(|f| f.clone()).unwrap_or_default()
    }

    /// Clear all metrics (useful for testing).
    pub fn clear(&self) {
        if let Ok(mut events) = self.events.write() {
            events.clear();
        }
        if let Ok(mut fallbacks) = self.fallbacks.write() {
            fallbacks.clear();
        }
    }
}

/// Timer for measuring operation latency.
#[derive(Debug)]
pub struct Timer {
    start: Instant,
}

impl Timer {
    /// Start a new timer.
    #[must_use]
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed time in milliseconds.
    #[must_use]
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::start()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_event_new() {
        let event = MetricEvent::new("linear", 100, true);
        assert_eq!(event.mode, "linear");
        assert_eq!(event.latency_ms, 100);
        assert!(event.success);
        assert!(event.operation.is_none());
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_metric_event_with_operation() {
        let event = MetricEvent::new("tree", 200, false).with_operation("create");
        assert_eq!(event.mode, "tree");
        assert_eq!(event.operation, Some("create".to_string()));
        assert!(!event.success);
    }

    #[test]
    fn test_fallback_event_new() {
        let fallback = FallbackEvent::new("graph", "linear", "API timeout");
        assert_eq!(fallback.from_mode, "graph");
        assert_eq!(fallback.to_mode, "linear");
        assert_eq!(fallback.reason, "API timeout");
        assert!(fallback.timestamp > 0);
    }

    #[test]
    fn test_metrics_collector_record() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        collector.record(MetricEvent::new("linear", 150, true));
        collector.record(MetricEvent::new("tree", 200, false));

        let summary = collector.summary();
        assert_eq!(summary.total_invocations, 3);
        assert_eq!(summary.by_mode.len(), 2);
    }

    #[test]
    fn test_metrics_collector_summary() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        collector.record(MetricEvent::new("linear", 200, true));
        collector.record(MetricEvent::new("linear", 300, false));

        let summary = collector.summary();
        let linear_summary = summary.by_mode.get("linear").unwrap();

        assert_eq!(linear_summary.total_invocations, 3);
        assert_eq!(linear_summary.successful, 2);
        assert_eq!(linear_summary.failed, 1);
        assert!((linear_summary.avg_latency_ms - 200.0).abs() < f64::EPSILON);
        assert_eq!(linear_summary.min_latency_ms, 100);
        assert_eq!(linear_summary.max_latency_ms, 300);
        assert!((linear_summary.success_rate - 0.666_666_666_666_666_6).abs() < 0.01);
    }

    #[test]
    fn test_metrics_collector_fallbacks() {
        let collector = MetricsCollector::new();
        collector.record_fallback(FallbackEvent::new("graph", "linear", "timeout"));
        collector.record_fallback(FallbackEvent::new("mcts", "tree", "API error"));

        let fallbacks = collector.fallbacks();
        assert_eq!(fallbacks.len(), 2);
        assert_eq!(fallbacks[0].from_mode, "graph");
        assert_eq!(fallbacks[1].from_mode, "mcts");
    }

    #[test]
    fn test_invocations_by_mode() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        collector.record(MetricEvent::new("tree", 150, true));
        collector.record(MetricEvent::new("linear", 200, false));

        let linear_events = collector.invocations_by_mode("linear");
        assert_eq!(linear_events.len(), 2);

        let tree_events = collector.invocations_by_mode("tree");
        assert_eq!(tree_events.len(), 1);

        let unknown_events = collector.invocations_by_mode("unknown");
        assert!(unknown_events.is_empty());
    }

    #[test]
    fn test_metrics_collector_clear() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        collector.record_fallback(FallbackEvent::new("a", "b", "c"));

        assert_eq!(collector.summary().total_invocations, 1);
        assert_eq!(collector.fallbacks().len(), 1);

        collector.clear();

        assert_eq!(collector.summary().total_invocations, 0);
        assert!(collector.fallbacks().is_empty());
    }

    #[test]
    fn test_empty_summary() {
        let collector = MetricsCollector::new();
        let summary = collector.summary();

        assert_eq!(summary.total_invocations, 0);
        assert!((summary.overall_success_rate - 1.0).abs() < f64::EPSILON);
        assert!(summary.by_mode.is_empty());
        assert!(summary.recent_fallbacks.is_empty());
    }

    #[test]
    fn test_timer() {
        let timer = Timer::start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
    }

    #[test]
    fn test_timer_default() {
        let timer = Timer::default();
        let elapsed = timer.elapsed_ms();
        assert!(elapsed < 100); // Should be nearly instant
    }

    #[test]
    fn test_mode_summary_default() {
        let summary = ModeSummary::default();
        assert_eq!(summary.total_invocations, 0);
        assert_eq!(summary.successful, 0);
        assert_eq!(summary.failed, 0);
        assert!((summary.avg_latency_ms - 0.0).abs() < f64::EPSILON);
        assert_eq!(summary.min_latency_ms, 0);
        assert_eq!(summary.max_latency_ms, 0);
        assert!((summary.success_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_invocations_in_range() {
        let collector = MetricsCollector::new();

        // Create events with known timestamps
        let mut event1 = MetricEvent::new("linear", 100, true);
        event1.timestamp = 1000;
        let mut event2 = MetricEvent::new("tree", 150, true);
        event2.timestamp = 2000;
        let mut event3 = MetricEvent::new("linear", 200, false);
        event3.timestamp = 3000;

        collector.record(event1);
        collector.record(event2);
        collector.record(event3);

        let in_range = collector.invocations_in_range(1500, 2500);
        assert_eq!(in_range.len(), 1);
        assert_eq!(in_range[0].mode, "tree");

        let all = collector.invocations_in_range(0, 5000);
        assert_eq!(all.len(), 3);

        let none = collector.invocations_in_range(4000, 5000);
        assert!(none.is_empty());
    }

    #[test]
    fn test_metric_event_serialize() {
        let event = MetricEvent::new("linear", 100, true).with_operation("process");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"mode\":\"linear\""));
        assert!(json.contains("\"operation\":\"process\""));
        assert!(json.contains("\"latency_ms\":100"));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_metrics_summary_serialize() {
        let collector = MetricsCollector::new();
        collector.record(MetricEvent::new("linear", 100, true));
        let summary = collector.summary();

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_invocations\":1"));
        assert!(json.contains("\"by_mode\""));
    }
}
