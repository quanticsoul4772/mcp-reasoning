//! Opt-in detection sink (T011/T012).
//!
//! The seam a reasoning mode calls when its own output fails to parse or violates
//! its contract. Holding the shared metrics collector + defect log, it records
//! the failure (counter + recurrence) without the mode needing to know the
//! recording machinery. It is opt-in (a mode records only if a sink was
//! attached), so wiring it into a mode breaks no existing constructor or test.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::metrics::MetricsCollector;

use super::detect::DefectLog;
use super::types::FailureClass;

/// Records a single component's parse/schema failures into the shared metrics
/// collector and defect log.
#[derive(Clone)]
pub struct DefectSink {
    metrics: Arc<MetricsCollector>,
    log: Arc<DefectLog>,
    component: String,
}

impl std::fmt::Debug for DefectSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefectSink")
            .field("component", &self.component)
            .finish_non_exhaustive()
    }
}

impl DefectSink {
    /// Create a sink for `component` (e.g. `reasoning_linear/linear`).
    #[must_use]
    pub fn new(
        metrics: Arc<MetricsCollector>,
        log: Arc<DefectLog>,
        component: impl Into<String>,
    ) -> Self {
        Self {
            metrics,
            log,
            component: component.into(),
        }
    }

    /// Record a parse failure (malformed/unparseable output) for this component.
    pub fn parse_failure(&self, raw_input: &str) {
        self.log.observe(
            &self.metrics,
            &self.component,
            FailureClass::Parse,
            raw_input,
            now_millis(),
        );
    }

    /// Record a schema/contract violation for this component.
    pub fn schema_violation(&self, raw_input: &str) {
        self.log.observe(
            &self.metrics,
            &self.component,
            FailureClass::Schema,
            raw_input,
            now_millis(),
        );
    }
}

/// Current unix time in milliseconds, or 0 if the clock is before the epoch.
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::self_improvement::heal::DEFAULT_RECURRENCE_THRESHOLD;

    #[test]
    fn sink_records_parse_and_schema() {
        let metrics = Arc::new(MetricsCollector::new());
        let log = Arc::new(DefectLog::new(DEFAULT_RECURRENCE_THRESHOLD));
        let sink = DefectSink::new(
            Arc::clone(&metrics),
            Arc::clone(&log),
            "reasoning_linear/linear",
        );
        sink.parse_failure("not json");
        sink.schema_violation("{\"missing\": true}");
        assert_eq!(metrics.parse_failure_count("reasoning_linear/linear"), 1);
        assert_eq!(metrics.schema_violation_count("reasoning_linear/linear"), 1);
    }

    #[test]
    fn debug_redacts_to_component_only() {
        let sink = DefectSink::new(
            Arc::new(MetricsCollector::new()),
            Arc::new(DefectLog::new(DEFAULT_RECURRENCE_THRESHOLD)),
            "reasoning_tree/tree",
        );
        let shown = format!("{sink:?}");
        assert!(shown.contains("reasoning_tree/tree"));
        assert!(shown.contains("DefectSink"));
    }
}
