//! Eval runner: drive a [`Solver`] over a [`Dataset`], score each output, and
//! aggregate into an [`EvalReport`] plus a JSON-serializable per-task record.
//!
//! Solver errors (API failures and the like) are **infrastructure failures, not
//! quality signals**: they are recorded per task and counted in
//! [`RunOutcome::solver_errors`], but excluded from the report so a transient
//! outage cannot masquerade as a quality regression. The report is computed over
//! the items that actually produced a scoreable answer.

use serde::Serialize;

use crate::eval::report::EvalReport;
use crate::eval::scorer::Scorer;
use crate::eval::solver::Solver;
use crate::eval::task::Dataset;

/// Per-task outcome in a run.
#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    /// The task's id.
    pub task_id: String,
    /// The answer extracted from the solver output, if any.
    pub extracted: Option<String>,
    /// `1.0` correct, `0.0` otherwise; `0.0` for a solver error (see `error`).
    pub score: f64,
    /// Whether extraction failed (distinct from a wrong answer).
    pub extraction_failed: bool,
    /// The solver error message, if the solve failed. When set, this task is
    /// excluded from the aggregated report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The full result of a run: per-task records, a count of solver failures, and
/// the aggregated report over the scoreable items.
#[derive(Debug, Clone, Serialize)]
pub struct RunOutcome {
    /// The mode that was run.
    pub mode: String,
    /// Per-task records, in dataset order.
    pub results: Vec<TaskResult>,
    /// Number of tasks whose solver call failed (excluded from `report`).
    pub solver_errors: usize,
    /// Aggregated statistics over the scoreable items. `None` when fewer than
    /// two items produced a scoreable answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<EvalReport>,
}

impl RunOutcome {
    /// Serialize the outcome as pretty JSON.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`serde_json::Error`] if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Run `solver` over every task in `dataset`, scoring each output with `scorer`.
///
/// Never panics and never short-circuits: a solver error on one task is recorded
/// and the run continues.
pub async fn run_eval(dataset: &Dataset, solver: &dyn Solver, scorer: &dyn Scorer) -> RunOutcome {
    let mut results = Vec::with_capacity(dataset.len());
    let mut report_tasks = Vec::new();
    let mut report_scores = Vec::new();
    let mut solver_errors = 0usize;

    for task in dataset.tasks() {
        match solver.solve(task).await {
            Ok(output) => {
                let score = scorer.score(task, &output.text);
                results.push(TaskResult {
                    task_id: task.id.clone(),
                    extracted: score.extracted.clone(),
                    score: score.value,
                    extraction_failed: score.extraction_failed,
                    error: None,
                });
                report_tasks.push(task.clone());
                report_scores.push(score);
            }
            Err(e) => {
                solver_errors += 1;
                results.push(TaskResult {
                    task_id: task.id.clone(),
                    extracted: None,
                    score: 0.0,
                    extraction_failed: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    RunOutcome {
        mode: solver.mode().to_string(),
        results,
        solver_errors,
        report: EvalReport::with_defaults(&report_tasks, &report_scores),
    }
}
