//! Tests for the self-improvement measurement sensor
//! ([`mcp_reasoning::self_improvement::measure_delta`]).
//!
//! The sensor measures a real paired delta between a baseline and a changed
//! configuration over a held-out slice. Driven here with `MockSolver` so the
//! per-item scores — and therefore the delta — are deterministic.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use mcp_reasoning::eval::{Dataset, ExactMatch, MockSolver};
use mcp_reasoning::self_improvement::measure_delta;

const DS: &str = concat!(
    r#"{"id":"a","prompt":"p","target":"1","answer_kind":"numeric"}"#,
    "\n",
    r#"{"id":"b","prompt":"p","target":"2","answer_kind":"numeric"}"#,
    "\n",
    r#"{"id":"c","prompt":"p","target":"3","answer_kind":"numeric"}"#,
    "\n",
    r#"{"id":"d","prompt":"p","target":"4","answer_kind":"numeric"}"#,
);

#[tokio::test]
async fn measures_positive_paired_delta() {
    let ds = Dataset::from_jsonl(DS).unwrap();
    // Baseline gets everything wrong (#### 0); changed gets everything right.
    let baseline = MockSolver::new("#### 0");
    let changed = MockSolver::new("x")
        .with_output("a", "#### 1")
        .with_output("b", "#### 2")
        .with_output("c", "#### 3")
        .with_output("d", "#### 4");

    let delta = measure_delta(&ds, &baseline, &changed, &ExactMatch::new(), 0.05, 0.05)
        .await
        .expect("four paired items");

    assert_eq!(delta.n_paired, 4);
    // changed (1.0 mean) − baseline (0.0 mean) = +1.0.
    assert!((delta.estimate.mean - 1.0).abs() < 1e-12);
    assert!(delta.clears_mde, "a +1.0 delta clears the MDE");
}

#[tokio::test]
async fn zero_delta_does_not_clear_mde() {
    let ds = Dataset::from_jsonl(DS).unwrap();
    // Both configurations identical → every per-item difference is 0.
    let same = MockSolver::new("#### 0");
    let delta = measure_delta(&ds, &same, &same, &ExactMatch::new(), 0.05, 0.05)
        .await
        .expect("four paired items");

    assert_eq!(delta.estimate.mean, 0.0);
    assert!(
        !delta.clears_mde,
        "a zero delta cannot clear a positive MDE — this is the saturated/no-signal case"
    );
}

#[tokio::test]
async fn negative_delta_is_measured_and_fails_the_gate() {
    let ds = Dataset::from_jsonl(DS).unwrap();
    // Baseline right, changed wrong → the change regressed accuracy.
    let baseline = MockSolver::new("x")
        .with_output("a", "#### 1")
        .with_output("b", "#### 2")
        .with_output("c", "#### 3")
        .with_output("d", "#### 4");
    let changed = MockSolver::new("#### 0");

    let delta = measure_delta(&ds, &baseline, &changed, &ExactMatch::new(), 0.05, 0.05)
        .await
        .expect("four paired items");

    assert!((delta.estimate.mean + 1.0).abs() < 1e-12); // −1.0
    assert!(!delta.clears_mde);
}

#[tokio::test]
async fn solver_errors_drop_items_from_the_pairing() {
    let ds = Dataset::from_jsonl(DS).unwrap();
    // 'd' errors under the changed config → only a,b,c are paired.
    let baseline = MockSolver::new("#### 0");
    let changed = MockSolver::new("x")
        .with_output("a", "#### 1")
        .with_output("b", "#### 2")
        .with_output("c", "#### 3")
        .with_error("d");

    let delta = measure_delta(&ds, &baseline, &changed, &ExactMatch::new(), 0.05, 0.05)
        .await
        .expect("three paired items");
    assert_eq!(delta.n_paired, 3);
}

#[tokio::test]
async fn too_few_paired_items_returns_none() {
    let one = r#"{"id":"a","prompt":"p","target":"1","answer_kind":"numeric"}"#;
    let ds = Dataset::from_jsonl(one).unwrap();
    let s = MockSolver::new("#### 1");
    // A single item cannot form a paired-difference standard error.
    assert!(measure_delta(&ds, &s, &s, &ExactMatch::new(), 0.05, 0.05)
        .await
        .is_none());
}
