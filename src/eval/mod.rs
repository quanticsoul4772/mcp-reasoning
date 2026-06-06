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
//!
//! Later PRs add the real-mode solver + runner and the reward-function rewrite
//! that consumes these primitives.

pub mod report;
pub mod scorer;
pub mod stats;
pub mod task;

pub use report::EvalReport;
pub use scorer::{ExactMatch, Score, Scorer};
pub use task::{AnswerKind, Dataset, DatasetError, EvalTask};
