//! Aggregated eval report.
//!
//! The per-run summary required by the harness's v1 success criteria — `n`,
//! mean score, standard error, clustered SE where applicable, the
//! extraction-failure rate, and the MDE for that sample.

use std::collections::HashSet;

use serde::Serialize;

use crate::eval::scorer::Score;
use crate::eval::stats::{self};
use crate::eval::task::EvalTask;

/// Conventional significance level for the reported MDE.
pub const DEFAULT_ALPHA: f64 = 0.05;
/// Conventional power for the reported MDE.
pub const DEFAULT_POWER: f64 = 0.80;

/// Summary of one mode's run over a dataset.
#[allow(clippy::derive_partial_eq_without_eq)] // f64 fields preclude `Eq`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvalReport {
    /// Number of scored items.
    pub n: usize,
    /// Mean score (accuracy for binary scoring).
    pub mean_score: f64,
    /// Standard error of `mean_score` (CLT).
    pub stderr: f64,
    /// Clustered standard error, present only when the dataset has genuine
    /// clusters (every item tagged, and at least one cluster with >1 member).
    /// `None` otherwise — per the plan, the clustered-SE computation is deferred
    /// until a clustered dataset actually exists, even though the API supports it.
    pub clustered_stderr: Option<f64>,
    /// Fraction of items where no answer could be extracted. A first-class
    /// metric: a rising rate corrupts deltas while looking like a quality drop.
    pub extraction_failure_rate: f64,
    /// Minimum Detectable Effect at [`DEFAULT_ALPHA`]/[`DEFAULT_POWER`] for this
    /// `stderr`. If it exceeds the effect you pre-registered as meaningful, the
    /// honest conclusion is "this dataset cannot test this."
    pub mde: f64,
}

impl EvalReport {
    /// Build a report from item-aligned `tasks` and `scores`.
    ///
    /// Returns `None` on a length mismatch, fewer than 2 items, or out-of-range
    /// `alpha`/`power` (the standard error and MDE are undefined there).
    #[must_use]
    pub fn from_scores(
        tasks: &[EvalTask],
        scores: &[Score],
        alpha: f64,
        power: f64,
    ) -> Option<Self> {
        if tasks.len() != scores.len() {
            return None;
        }
        let values: Vec<f64> = scores.iter().map(|s| s.value).collect();
        let est = stats::mean_and_stderr(&values)?;
        let mde = stats::minimum_detectable_effect(est.stderr, alpha, power)?;

        let failures = scores.iter().filter(|s| s.extraction_failed).count();
        let extraction_failure_rate = failures as f64 / est.n as f64;

        Some(Self {
            n: est.n,
            mean_score: est.mean,
            stderr: est.stderr,
            clustered_stderr: clustered_if_applicable(tasks, &values),
            extraction_failure_rate,
            mde,
        })
    }

    /// [`EvalReport::from_scores`] at the conventional alpha/power.
    #[must_use]
    pub fn with_defaults(tasks: &[EvalTask], scores: &[Score]) -> Option<Self> {
        Self::from_scores(tasks, scores, DEFAULT_ALPHA, DEFAULT_POWER)
    }
}

/// Compute the clustered SE only when the dataset is genuinely clustered: every
/// task carries a `cluster_id` and there are fewer distinct clusters than items
/// (so at least one cluster holds more than one member). Otherwise `None` —
/// clustering a fully-independent set would just reproduce the CLT SE.
fn clustered_if_applicable(tasks: &[EvalTask], values: &[f64]) -> Option<f64> {
    let ids: Vec<&str> = tasks
        .iter()
        .map(|t| t.cluster_id.as_deref())
        .collect::<Option<Vec<&str>>>()?;
    let distinct: HashSet<&str> = ids.iter().copied().collect();
    if distinct.len() < 2 || distinct.len() == ids.len() {
        return None;
    }
    stats::clustered_stderr(values, &ids).map(|e| e.stderr)
}
