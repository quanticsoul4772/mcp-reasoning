//! Detection + recording machinery for US1 (spec 001).
//!
//! [`DefectLog`] records detected parse/schema failures, increments the metrics
//! counters (FR-001), and tracks recurrence by `(component, failure_class)`
//! signature within the process (FR-003). It is the recording layer the live
//! parse/param seam will call; threading a sink through `ModeCore` and every
//! mode (the hot path) is a separate cross-cutting change (tasks T011/T012).

use std::collections::HashMap;
use std::sync::RwLock;

use crate::metrics::MetricsCollector;

use super::types::{DefectRecord, DefectStatus, FailureClass};

/// Default recurrence threshold (FR-003): N occurrences of a signature before a
/// defect is eligible for a proposal.
pub const DEFAULT_RECURRENCE_THRESHOLD: u32 = 3;

/// In-memory log of detected self-defects, keyed by recurrence signature.
#[derive(Debug)]
pub struct DefectLog {
    by_signature: RwLock<HashMap<String, DefectRecord>>,
    recurrence_threshold: u32,
}

impl DefectLog {
    /// Create a log with an explicit recurrence threshold.
    #[must_use]
    pub fn new(recurrence_threshold: u32) -> Self {
        Self {
            by_signature: RwLock::new(HashMap::new()),
            recurrence_threshold: recurrence_threshold.max(1),
        }
    }

    /// Observe a failure: increment the metrics counter for `component`, upsert
    /// the `DefectRecord` (tracking recurrence), and return its current snapshot.
    /// Drift-class failures never touch the parse/schema counters.
    pub fn observe(
        &self,
        metrics: &MetricsCollector,
        component: &str,
        class: FailureClass,
        raw_input: &str,
        now: i64,
    ) -> DefectRecord {
        match class {
            FailureClass::Parse => metrics.record_parse_failure(component),
            FailureClass::Schema => metrics.record_schema_violation(component),
            FailureClass::Drift => {}
        }
        let sig = format!("{component}::{class}");
        let Ok(mut map) = self.by_signature.write() else {
            return DefectRecord::observe(component, class, raw_input, now);
        };
        if let Some(rec) = map.get_mut(&sig) {
            rec.record_occurrence(now);
            if rec.status == DefectStatus::Observed && rec.is_recurring(self.recurrence_threshold) {
                rec.status = DefectStatus::Recurring;
            }
            rec.clone()
        } else {
            let rec = DefectRecord::observe(component, class, raw_input, now);
            map.insert(sig, rec.clone());
            rec
        }
    }

    /// Snapshot of all recurring defects (status `Recurring`).
    #[must_use]
    pub fn recurring(&self) -> Vec<DefectRecord> {
        self.by_signature.read().map_or_else(
            |_| Vec::new(),
            |m| {
                m.values()
                    .filter(|d| d.status == DefectStatus::Recurring)
                    .cloned()
                    .collect()
            },
        )
    }

    /// Number of distinct defect signatures recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_signature.read().map_or(0, |m| m.len())
    }

    /// True if no defects have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn records_parse_and_increments_counter() {
        let m = MetricsCollector::new();
        let log = DefectLog::new(DEFAULT_RECURRENCE_THRESHOLD);
        let d = log.observe(
            &m,
            "reasoning_linear/linear",
            FailureClass::Parse,
            "bad json",
            1,
        );
        assert_eq!(d.status, DefectStatus::Observed);
        assert_eq!(m.parse_failure_count("reasoning_linear/linear"), 1);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn schema_is_distinct_from_parse() {
        let m = MetricsCollector::new();
        let log = DefectLog::new(3);
        log.observe(&m, "tool_a", FailureClass::Schema, "x", 1);
        assert_eq!(m.schema_violation_count("tool_a"), 1);
        assert_eq!(m.parse_failure_count("tool_a"), 0);
    }

    #[test]
    fn one_off_stays_observed() {
        let m = MetricsCollector::new();
        let log = DefectLog::new(3);
        log.observe(&m, "tool_a", FailureClass::Parse, "x", 1);
        assert!(log.recurring().is_empty());
    }

    #[test]
    fn promotes_to_recurring_at_threshold() {
        let m = MetricsCollector::new();
        let log = DefectLog::new(3);
        log.observe(&m, "tool_a", FailureClass::Parse, "x", 1);
        log.observe(&m, "tool_a", FailureClass::Parse, "x", 2);
        let d = log.observe(&m, "tool_a", FailureClass::Parse, "x", 3);
        assert_eq!(d.status, DefectStatus::Recurring);
        assert_eq!(d.occurrences, 3);
        assert_eq!(m.parse_failure_count("tool_a"), 3);
        assert_eq!(log.recurring().len(), 1);
    }

    #[test]
    fn totals_sum_across_components() {
        let m = MetricsCollector::new();
        let log = DefectLog::new(3);
        log.observe(&m, "tool_a", FailureClass::Parse, "x", 1);
        log.observe(&m, "tool_b", FailureClass::Parse, "y", 1);
        log.observe(&m, "tool_a", FailureClass::Schema, "z", 1);
        assert_eq!(m.total_parse_failures(), 2);
        assert_eq!(m.total_schema_violations(), 1);
    }

    #[test]
    fn drift_does_not_touch_counters() {
        let m = MetricsCollector::new();
        let log = DefectLog::new(3);
        log.observe(&m, "tool_a", FailureClass::Drift, "x", 1);
        assert_eq!(m.parse_failure_count("tool_a"), 0);
        assert_eq!(m.schema_violation_count("tool_a"), 0);
    }
}
