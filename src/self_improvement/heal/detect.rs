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

use super::redact::redact;
use super::types::{DefectRecord, DefectStatus, FailureClass, EXCERPT_MAX};

/// Default recurrence threshold (FR-003): N occurrences of a signature before a
/// defect is eligible for a proposal.
pub const DEFAULT_RECURRENCE_THRESHOLD: u32 = 3;

/// A defect signature's record plus its per-triggering-input occurrence counts
/// (spec 002, US2): the per-input map separates a stable-path code defect (one
/// input recurring) from input-induced noise (many distinct inputs).
#[derive(Debug)]
struct Tracked {
    record: DefectRecord,
    per_input: HashMap<String, u32>,
}

/// In-memory log of detected self-defects, keyed by recurrence signature.
#[derive(Debug)]
pub struct DefectLog {
    by_signature: RwLock<HashMap<String, Tracked>>,
    recurrence_threshold: u32,
}

/// Snapshot the per-input stats into the record so callers see `max_input_occurrences`
/// / `distinct_inputs` without holding the log lock.
fn sync_input_stats(record: &mut DefectRecord, per_input: &HashMap<String, u32>) {
    record.max_input_occurrences = per_input.values().copied().max().unwrap_or(0);
    record.distinct_inputs = u32::try_from(per_input.len()).unwrap_or(u32::MAX);
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
        let input_hash = redact(raw_input, EXCERPT_MAX).hash;
        let Ok(mut map) = self.by_signature.write() else {
            return DefectRecord::observe(component, class, raw_input, now);
        };
        if let Some(tracked) = map.get_mut(&sig) {
            tracked.record.record_occurrence(now);
            *tracked.per_input.entry(input_hash).or_insert(0) += 1;
            if tracked.record.status == DefectStatus::Observed
                && tracked.record.is_recurring(self.recurrence_threshold)
            {
                tracked.record.status = DefectStatus::Recurring;
            }
            sync_input_stats(&mut tracked.record, &tracked.per_input);
            tracked.record.clone()
        } else {
            let mut record = DefectRecord::observe(component, class, raw_input, now);
            let mut per_input = HashMap::new();
            per_input.insert(input_hash, 1u32);
            sync_input_stats(&mut record, &per_input);
            map.insert(
                sig,
                Tracked {
                    record: record.clone(),
                    per_input,
                },
            );
            record
        }
    }

    /// Snapshot of all recurring defects (status `Recurring`).
    #[must_use]
    pub fn recurring(&self) -> Vec<DefectRecord> {
        self.by_signature.read().map_or_else(
            |_| Vec::new(),
            |m| {
                m.values()
                    .filter(|t| t.record.status == DefectStatus::Recurring)
                    .map(|t| t.record.clone())
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
    fn varied_inputs_recur_but_are_not_propose_eligible() {
        // spec 002 US2: three DIFFERENT inputs of the same signature each fail once.
        // The defect recurs (aggregate), but no single input repeated → not eligible.
        let m = MetricsCollector::new();
        let log = DefectLog::new(3);
        log.observe(
            &m,
            "reasoning_linear/linear",
            FailureClass::Schema,
            "in-a",
            1,
        );
        log.observe(
            &m,
            "reasoning_linear/linear",
            FailureClass::Schema,
            "in-b",
            2,
        );
        let d = log.observe(
            &m,
            "reasoning_linear/linear",
            FailureClass::Schema,
            "in-c",
            3,
        );
        assert_eq!(d.status, DefectStatus::Recurring); // aggregate recurrence
        assert_eq!(d.max_input_occurrences, 1);
        assert_eq!(d.distinct_inputs, 3);
        assert!(
            !d.is_propose_eligible(3),
            "varied inputs must not be eligible"
        );
    }

    #[test]
    fn stable_input_recurrence_is_propose_eligible() {
        // The SAME input repeated three times → a stable, repeatable code path.
        let m = MetricsCollector::new();
        let log = DefectLog::new(3);
        log.observe(
            &m,
            "reasoning_linear/linear",
            FailureClass::Schema,
            "same",
            1,
        );
        log.observe(
            &m,
            "reasoning_linear/linear",
            FailureClass::Schema,
            "same",
            2,
        );
        let d = log.observe(
            &m,
            "reasoning_linear/linear",
            FailureClass::Schema,
            "same",
            3,
        );
        assert_eq!(d.max_input_occurrences, 3);
        assert_eq!(d.distinct_inputs, 1);
        assert!(
            d.is_propose_eligible(3),
            "stable-path recurrence must be eligible"
        );
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
