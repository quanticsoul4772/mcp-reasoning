//! Plan step (spec 001, T019): rank recurring defects and cap how many are
//! proposed per cycle.
//!
//! Severity is FR-014 (class weight × recurrence × blast radius, bounded to `[0,1]`).
//! Selection ranks by frequency × severity and keeps the top K (FR-013). Drift
//! defects are never proposed (FR-012). Fix-confidence (FR-015) is an outcome of
//! attempting a fix, so it is applied post-attempt and is not part of selection.

use std::collections::HashSet;

use super::types::{DefectRecord, FailureClass};

/// Minimum distinct components exhibiting a failure class before it is treated as
/// model/provider drift rather than a localized code defect (D3, FR-012).
pub const DEFAULT_DRIFT_THRESHOLD: u32 = 3;

/// Split recurring defects into `(code_defects, drift_defects)` (FR-012, D3).
///
/// A defect is drift when it is already classed `Drift` OR its failure class is
/// broad across `>= drift_threshold` distinct components (a model swap, not a code
/// bug). Drift defects are routed to the drift response (alert + record), never to
/// the repair path. The `code_defects` are the localized ones eligible to propose.
#[must_use]
pub fn partition_drift(
    recurring: &[DefectRecord],
    drift_threshold: u32,
) -> (Vec<DefectRecord>, Vec<DefectRecord>) {
    recurring.iter().cloned().partition(|d| {
        !(d.failure_class == FailureClass::Drift
            || is_drift_class(recurring, d.failure_class, drift_threshold))
    })
}

/// Number of distinct components exhibiting `class` across `defects` — the blast
/// radius used by [`severity`] and drift detection (FR-014, FR-012).
#[must_use]
pub fn blast_radius(defects: &[DefectRecord], class: FailureClass) -> u32 {
    let set: HashSet<&str> = defects
        .iter()
        .filter(|d| d.failure_class == class)
        .map(|d| d.component.as_str())
        .collect();
    u32::try_from(set.len()).unwrap_or(u32::MAX)
}

/// True when `class` appears broadly across components at once — a drift
/// candidate, not a code defect (FR-012, D3).
///
/// `threshold` = minimum distinct components to call it drift. The model-version
/// correlation (FR-017/T041) refines this when that signal is available.
#[must_use]
pub fn is_drift_class(defects: &[DefectRecord], class: FailureClass, threshold: u32) -> bool {
    class != FailureClass::Drift && blast_radius(defects, class) >= threshold.max(1)
}

/// Classify a defect: `Drift` when its failure class is broad across components
/// (FR-012), otherwise its recorded class. Drift is routed away from the repair
/// path by the caller.
#[must_use]
pub fn classify(
    defects: &[DefectRecord],
    defect: &DefectRecord,
    drift_threshold: u32,
) -> FailureClass {
    if is_drift_class(defects, defect.failure_class, drift_threshold) {
        FailureClass::Drift
    } else {
        defect.failure_class
    }
}

/// Class weight for severity (FR-014): schema ≥ parse ≥ drift.
fn class_weight(c: FailureClass) -> f64 {
    match c {
        FailureClass::Schema => 1.0,
        FailureClass::Parse => 0.7,
        FailureClass::Drift => 0.0,
    }
}

/// Occurrence count at which the recurrence factor saturates.
const RECUR_SAT: f64 = 10.0;
/// Distinct-component count at which the blast factor saturates.
const BLAST_SAT: f64 = 5.0;

/// Bounded `[0,1]` severity (FR-014): a class-weighted blend of recurrence and
/// blast radius.
///
/// `blast_radius` = number of distinct components exhibiting the same failure
/// class. Monotonic in both occurrences and blast radius; never collapses to ~0
/// for a real (non-drift) defect.
#[must_use]
pub fn severity(defect: &DefectRecord, blast_radius: u32) -> f64 {
    let recur = (f64::from(defect.occurrences) / RECUR_SAT).min(1.0);
    let blast = (f64::from(blast_radius.max(1)) / BLAST_SAT).min(1.0);
    let blend = 0.3f64.mul_add(blast, 0.3f64.mul_add(recur, 0.4));
    (class_weight(defect.failure_class) * blend).clamp(0.0, 1.0)
}

/// Rank recurring defects by frequency × severity and keep the top `k`
/// (FR-013). Drift defects are excluded (FR-012). `blast_radius_of` supplies the
/// blast radius for each defect.
#[must_use]
pub fn rank_and_cap(
    recurring: &[DefectRecord],
    blast_radius_of: impl Fn(&DefectRecord) -> u32,
    k: usize,
) -> Vec<DefectRecord> {
    let mut scored: Vec<(f64, DefectRecord)> = recurring
        .iter()
        .filter(|d| d.failure_class != FailureClass::Drift)
        .map(|d| {
            let score = f64::from(d.occurrences) * severity(d, blast_radius_of(d));
            (score, d.clone())
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored.into_iter().take(k).map(|(_, d)| d).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn defect(component: &str, class: FailureClass, occ: u32) -> DefectRecord {
        let mut d = DefectRecord::observe(component, class, "x", 1);
        for _ in 1..occ {
            d.record_occurrence(2);
        }
        d
    }

    #[test]
    fn severity_schema_ge_parse_gt_drift() {
        let s = severity(&defect("a", FailureClass::Schema, 3), 1);
        let p = severity(&defect("a", FailureClass::Parse, 3), 1);
        let dr = severity(&defect("a", FailureClass::Drift, 3), 1);
        assert!(s > p);
        assert!(p > dr);
        assert!((dr - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn severity_monotonic_in_recurrence_and_blast() {
        let lo = severity(&defect("a", FailureClass::Parse, 2), 1);
        let more_recur = severity(&defect("a", FailureClass::Parse, 8), 1);
        let more_blast = severity(&defect("a", FailureClass::Parse, 2), 4);
        assert!(more_recur > lo);
        assert!(more_blast > lo);
        assert!((0.0..=1.0).contains(&more_recur));
    }

    #[test]
    fn rank_caps_and_orders_by_score() {
        let defects = vec![
            defect("a", FailureClass::Parse, 2),
            defect("b", FailureClass::Schema, 9),
            defect("c", FailureClass::Parse, 5),
        ];
        let ranked = rank_and_cap(&defects, |_| 1, 2);
        assert_eq!(ranked.len(), 2);
        // Schema with 9 occurrences scores highest.
        assert_eq!(ranked[0].component, "b");
    }

    #[test]
    fn blast_radius_counts_distinct_components() {
        let defects = vec![
            defect("a", FailureClass::Parse, 1),
            defect("b", FailureClass::Parse, 1),
            defect("a", FailureClass::Schema, 1),
        ];
        assert_eq!(blast_radius(&defects, FailureClass::Parse), 2);
        assert_eq!(blast_radius(&defects, FailureClass::Schema), 1);
    }

    #[test]
    fn classify_drift_when_broad_else_code_defect() {
        // Localized parse failure (1 component) → stays Parse (code defect).
        let localized = vec![defect("a", FailureClass::Parse, 3)];
        assert_eq!(classify(&localized, &localized[0], 3), FailureClass::Parse);
        // Broad parse failures across 3 components → Drift.
        let broad = vec![
            defect("a", FailureClass::Parse, 1),
            defect("b", FailureClass::Parse, 1),
            defect("c", FailureClass::Parse, 1),
        ];
        assert_eq!(classify(&broad, &broad[0], 3), FailureClass::Drift);
        assert!(is_drift_class(&broad, FailureClass::Parse, 3));
        assert!(!is_drift_class(&localized, FailureClass::Parse, 3));
    }

    #[test]
    fn partition_drift_routes_broad_and_literal_drift_away() {
        let defects = vec![
            // Broad parse failure across 3 components → structural drift (D3).
            defect("a", FailureClass::Parse, 1),
            defect("b", FailureClass::Parse, 1),
            defect("c", FailureClass::Parse, 1),
            // A localized schema defect → a real code defect.
            defect("solo", FailureClass::Schema, 4),
            // An already-Drift-classed defect → drift.
            defect("x", FailureClass::Drift, 5),
        ];
        let (code, drift) = partition_drift(&defects, 3);
        // Only the localized schema defect is a code defect.
        assert_eq!(code.len(), 1);
        assert_eq!(code[0].component, "solo");
        // The three broad-parse + one literal-Drift are routed to drift.
        assert_eq!(drift.len(), 4);
        assert!(drift.iter().all(|d| d.component != "solo"));
    }

    #[test]
    fn rank_excludes_drift() {
        let defects = vec![
            defect("a", FailureClass::Drift, 20),
            defect("b", FailureClass::Parse, 3),
        ];
        let ranked = rank_and_cap(&defects, |_| 1, 5);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].component, "b");
    }
}
