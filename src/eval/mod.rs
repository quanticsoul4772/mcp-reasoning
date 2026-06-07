//! Eval harness for the reasoning modes.
//!
//! The harness turns "did this change help?" into a measured quantity with a
//! correct error bar. Its first and load-bearing job is to decide whether the
//! self-improvement loop is even worth rewiring: an affordable dataset's
//! Minimum Detectable Effect may exceed any effect the loop's changes produce,
//! in which case the honest outcome is to keep the harness as a measurement tool
//! and *not* close the loop around it.
//!
//! See `docs/design/EVAL_HARNESS_PLAN.md` for the staged plan.
//!
//! - PR1: the statistical foundation ([`stats`]).
//! - PR2: the data model ([`task`]), programmatic scoring ([`scorer`]), and the
//!   aggregated [`report`].
//! - PR3: the real-mode [`solver`] adapters and the [`runner`] that drives a
//!   solver over a dataset and aggregates a report. The opt-in `eval` binary is
//!   the live entry point; it is never run in normal CI.
//!
//! A later PR adds the reward-function rewrite that consumes these primitives.

pub mod report;
pub mod runner;
pub mod scorer;
pub mod solver;
pub mod stats;
pub mod task;

pub use report::EvalReport;
pub use runner::{run_eval, run_eval_with_progress, RunOutcome, TaskResult};
pub use scorer::{ExactMatch, Score, Scorer};
pub use solver::{LinearSolver, MockSolver, ReflectionSolver, Solver, SolverError, SolverOutput};
pub use task::{AnswerKind, Dataset, DatasetError, EvalTask};
