//! Propose-eligibility attribution (spec 002, US2).
//!
//! Before a recurring defect may propose a fix, decide whether the evidence points
//! to a genuine *code* defect (a stable triggering path) versus input-induced noise
//! (varied inputs) or model drift. Fail-safe: anything not clearly a stable-path
//! code defect is held back — the loop never proposes on uncertainty (FR-006).

use super::types::DefectRecord;

/// Attribution verdict for a recurring code defect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EligibilityOutcome {
    /// Stable triggering path — proceed to localize/propose.
    Eligible,
    /// Recorded but held back from the propose path, with a reason (FR-006/FR-007).
    HeldBack(String),
    /// Correlates with a model-version change — route to the drift response (FR-005).
    Drift(String),
}

/// Classify a recurring code defect's eligibility (spec 002, FR-003/004/005/006).
///
/// `model_changed_in_window` means a model-version change overlaps this defect's
/// window (computed by the caller). Order: a model-correlated defect routes to
/// drift; otherwise a stable-path defect (`is_propose_eligible`) is eligible;
/// everything else (varied-input / ambiguous) is held back.
#[must_use]
pub fn classify_eligibility(
    defect: &DefectRecord,
    model_changed_in_window: bool,
    threshold: u32,
) -> EligibilityOutcome {
    if model_changed_in_window {
        return EligibilityOutcome::Drift(format!(
            "failure window overlaps a model-version change — likely model drift, not a code defect (signature {})",
            defect.signature()
        ));
    }
    if defect.is_propose_eligible(threshold) {
        return EligibilityOutcome::Eligible;
    }
    EligibilityOutcome::HeldBack(format!(
        "no stable triggering path: {} distinct inputs, max {} repeats (< {threshold}) — likely input-induced, not a code defect",
        defect.distinct_inputs, defect.max_input_occurrences
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::self_improvement::heal::FailureClass;

    /// A stable-path defect: `occ` repeats from a single input.
    fn stable(occ: u32) -> DefectRecord {
        let mut d = DefectRecord::observe("reasoning_linear/linear", FailureClass::Schema, "x", 1);
        d.max_input_occurrences = occ;
        d.distinct_inputs = 1;
        d
    }

    #[test]
    fn stable_path_is_eligible() {
        assert_eq!(
            classify_eligibility(&stable(4), false, 3),
            EligibilityOutcome::Eligible
        );
    }

    #[test]
    fn varied_input_is_held_back() {
        let mut d = stable(4);
        d.max_input_occurrences = 1;
        d.distinct_inputs = 4;
        assert!(matches!(
            classify_eligibility(&d, false, 3),
            EligibilityOutcome::HeldBack(_)
        ));
    }

    #[test]
    fn below_threshold_is_held_back() {
        // A single input repeated only twice (< threshold 3) → not eligible.
        assert!(matches!(
            classify_eligibility(&stable(2), false, 3),
            EligibilityOutcome::HeldBack(_)
        ));
    }

    #[test]
    fn model_change_routes_to_drift_even_on_stable_path() {
        assert!(matches!(
            classify_eligibility(&stable(9), true, 3),
            EligibilityOutcome::Drift(_)
        ));
    }
}
