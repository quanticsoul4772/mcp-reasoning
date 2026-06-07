//! Real measurement sensor for the self-improvement loop.
//!
//! Replaces the executor's fabricated `measured_improvement` (a fixed multiple of
//! the *estimate*) with a real **measured paired delta**: run a held-out slice
//! under the baseline and the changed configuration, score both programmatically,
//! and take the per-item difference. The loop is then credited for what was
//! measured, not for a number derived from its own prediction.
//!
//! Pairing is item-aligned: only tasks that produced a score under *both*
//! configurations are compared, so the difference cancels per-item difficulty
//! (the variance-reduction lever that makes small effects detectable).
//!
//! This is the sensor; wiring it as the executor's measurement source in the live
//! loop is the integration seam. On its own it is pure orchestration over the
//! eval harness and is tested deterministically with `MockSolver`.

use crate::eval::scorer::Scorer;
use crate::eval::solver::Solver;
use crate::eval::stats::{self, Estimate};
use crate::eval::task::Dataset;

/// The measured effect of a change: the paired per-item delta (changed minus
/// baseline) with its standard error, and whether it clears the pre-registered MDE.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasuredDelta {
    /// Paired mean difference (changed − baseline) and its standard error.
    pub estimate: Estimate,
    /// Number of items measured under both configurations.
    pub n_paired: usize,
    /// Whether the lower confidence bound of the delta clears the pre-registered
    /// MDE — the accept gate. A change that does not clear it is not a win.
    pub clears_mde: bool,
}

/// Score every task under `solver`, aligned to dataset order. A solver error
/// yields `None` for that item so the two runs pair on exactly the items that
/// succeeded under both.
async fn score_items(
    dataset: &Dataset,
    solver: &dyn Solver,
    scorer: &dyn Scorer,
) -> Vec<Option<f64>> {
    let mut out = Vec::with_capacity(dataset.len());
    for task in dataset.tasks() {
        match solver.solve(task).await {
            Ok(output) => out.push(Some(scorer.score(task, &output.text).value)),
            Err(_) => out.push(None),
        }
    }
    out
}

/// Measure the paired delta between a baseline and a changed configuration over a
/// held-out slice.
///
/// `mde` is the pre-registered Minimum Detectable Effect; `alpha` the confidence
/// level for the gate. Returns `None` when fewer than two items were scored under
/// *both* configurations (a paired difference is undefined) or `alpha` is out of
/// range.
pub async fn measure_delta(
    dataset: &Dataset,
    baseline: &dyn Solver,
    changed: &dyn Solver,
    scorer: &dyn Scorer,
    mde: f64,
    alpha: f64,
) -> Option<MeasuredDelta> {
    let base = score_items(dataset, baseline, scorer).await;
    let changed_scores = score_items(dataset, changed, scorer).await;

    // Pair only items that produced a score under both configurations.
    let mut baseline_paired = Vec::new();
    let mut changed_paired = Vec::new();
    for (b, c) in base.iter().zip(changed_scores.iter()) {
        if let (Some(bv), Some(cv)) = (b, c) {
            baseline_paired.push(*bv);
            changed_paired.push(*cv);
        }
    }

    // changed − baseline: a positive delta means the change improved accuracy.
    let estimate = stats::paired_difference(&changed_paired, &baseline_paired)?;
    let clears_mde = stats::clears_mde(&estimate, mde, alpha)?;
    Some(MeasuredDelta {
        estimate,
        n_paired: baseline_paired.len(),
        clears_mde,
    })
}
