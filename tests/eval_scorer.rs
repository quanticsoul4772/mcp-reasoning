//! Tests for programmatic scoring ([`mcp_reasoning::eval::scorer`]) and the
//! aggregated [`mcp_reasoning::eval::EvalReport`].

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use mcp_reasoning::eval::{AnswerKind, EvalReport, EvalTask, ExactMatch, Score, Scorer};

fn task(id: &str, target: &str, kind: AnswerKind, cluster: Option<&str>) -> EvalTask {
    EvalTask {
        id: id.to_string(),
        cluster_id: cluster.map(ToString::to_string),
        prompt: "p".to_string(),
        target: target.to_string(),
        expected_mode: None,
        answer_kind: kind,
        metadata: serde_json::Map::new(),
    }
}

fn num(target: &str) -> EvalTask {
    task("t", target, AnswerKind::Numeric, None)
}

fn exact(target: &str) -> EvalTask {
    task("t", target, AnswerKind::Exact, None)
}

// ---- numeric extraction ----------------------------------------------------

#[test]
fn numeric_strict_terminal_format() {
    let s = ExactMatch::new().score(&num("48"), "Reasoning here.\n#### 48");
    assert_eq!(s.value, 1.0);
    assert_eq!(s.extracted.as_deref(), Some("48"));
    assert!(!s.extraction_failed);
}

#[test]
fn numeric_flexible_last_number_without_marker() {
    let s = ExactMatch::new().score(&num("42"), "I worked it out: the answer is 42");
    assert_eq!(s.value, 1.0);
    assert_eq!(s.extracted.as_deref(), Some("42"));
}

#[test]
fn strict_filter_beats_flexible_when_marker_present() {
    // A distractor number appears after the marker's answer; the strict filter
    // must take the first number after ####, not the last number overall.
    let s = ExactMatch::new().score(&num("48"), "first guess 99 #### 48 then noise 100");
    assert_eq!(s.extracted.as_deref(), Some("48"));
    assert_eq!(s.value, 1.0);
}

#[test]
fn numeric_handles_commas_signs_and_decimals() {
    assert_eq!(
        ExactMatch::new().score(&num("1250"), "#### 1,250").value,
        1.0
    );
    assert_eq!(ExactMatch::new().score(&num("-5"), "#### -5").value, 1.0);
    assert_eq!(
        ExactMatch::new().score(&num("3.14"), "#### 3.14").value,
        1.0
    );
}

#[test]
fn numeric_normalizes_currency_percent_and_trailing_period() {
    assert_eq!(ExactMatch::new().score(&num("50"), "#### $50").value, 1.0);
    assert_eq!(ExactMatch::new().score(&num("50%"), "#### 50").value, 1.0);
    // Target carries a trailing period; extracted does not.
    assert_eq!(ExactMatch::new().score(&num("48."), "#### 48").value, 1.0);
}

#[test]
fn numeric_wrong_answer_scores_zero_but_does_not_fail_extraction() {
    let s = ExactMatch::new().score(&num("48"), "#### 49");
    assert_eq!(s.value, 0.0);
    assert!(!s.extraction_failed);
    assert_eq!(s.extracted.as_deref(), Some("49"));
}

#[test]
fn numeric_no_number_is_extraction_failure() {
    let s = ExactMatch::new().score(&num("48"), "I am not sure.");
    assert_eq!(s.value, 0.0);
    assert!(s.extraction_failed);
    assert!(s.extracted.is_none());
}

#[test]
fn numeric_falls_back_to_string_compare_for_unparseable_target() {
    // Target is not a number, extracted is: parse fails on one side, so the
    // comparison falls back to normalized string equality (and differs).
    let s = ExactMatch::new().score(&num("abc"), "#### 5");
    assert_eq!(s.value, 0.0);
    assert!(!s.extraction_failed);
}

// ---- exact extraction ------------------------------------------------------

#[test]
fn exact_strict_and_case_insensitive() {
    let s = ExactMatch::new().score(&exact("Paris"), "The capital is...\n#### paris");
    assert_eq!(s.value, 1.0);
    assert_eq!(s.extracted.as_deref(), Some("paris"));
}

#[test]
fn exact_strips_trailing_period() {
    assert_eq!(
        ExactMatch::new()
            .score(&exact("Paris"), "#### Paris.")
            .value,
        1.0
    );
}

#[test]
fn exact_flexible_last_nonblank_line() {
    let s = ExactMatch::new().score(&exact("Berlin"), "Thinking...\nBerlin\n\n");
    assert_eq!(s.value, 1.0);
    assert_eq!(s.extracted.as_deref(), Some("Berlin"));
}

#[test]
fn exact_empty_output_is_extraction_failure() {
    let s = ExactMatch::new().score(&exact("Berlin"), "   \n  ");
    assert!(s.extraction_failed);
    assert!(s.extracted.is_none());
}

// ---- report ----------------------------------------------------------------

fn score(value: f64, failed: bool) -> Score {
    Score {
        value,
        extracted: if failed { None } else { Some("x".to_string()) },
        extraction_failed: failed,
    }
}

#[test]
fn report_basic_unclustered() {
    let tasks = vec![num("1"), num("2"), num("3"), num("4")];
    let scores = vec![
        score(1.0, false),
        score(1.0, false),
        score(0.0, false),
        score(1.0, false),
    ];
    let rep = EvalReport::with_defaults(&tasks, &scores).unwrap();
    assert_eq!(rep.n, 4);
    assert_eq!(rep.mean_score, 0.75);
    assert!(rep.stderr > 0.0);
    assert!(rep.mde > 0.0 && rep.mde.is_finite());
    assert_eq!(rep.extraction_failure_rate, 0.0);
    assert!(rep.clustered_stderr.is_none());
}

#[test]
fn report_tracks_extraction_failure_rate() {
    let tasks = vec![num("1"), num("2"), num("3"), num("4")];
    let scores = vec![
        score(0.0, true),
        score(1.0, false),
        score(1.0, false),
        score(1.0, false),
    ];
    let rep = EvalReport::with_defaults(&tasks, &scores).unwrap();
    assert_eq!(rep.extraction_failure_rate, 0.25);
}

#[test]
fn report_computes_clustered_se_only_when_genuinely_clustered() {
    // Two clusters of two → clustering applies.
    let tasks = vec![
        task("a", "1", AnswerKind::Numeric, Some("g1")),
        task("b", "2", AnswerKind::Numeric, Some("g1")),
        task("c", "3", AnswerKind::Numeric, Some("g2")),
        task("d", "4", AnswerKind::Numeric, Some("g2")),
    ];
    let scores = vec![
        score(1.0, false),
        score(1.0, false),
        score(0.0, false),
        score(0.0, false),
    ];
    let rep = EvalReport::with_defaults(&tasks, &scores).unwrap();
    assert!(rep.clustered_stderr.is_some());
}

#[test]
fn report_skips_clustering_when_all_distinct_or_partly_unlabelled() {
    let scores = vec![score(1.0, false), score(0.0, false), score(1.0, false)];

    // Every item its own cluster → no genuine clustering.
    let distinct = vec![
        task("a", "1", AnswerKind::Numeric, Some("g1")),
        task("b", "2", AnswerKind::Numeric, Some("g2")),
        task("c", "3", AnswerKind::Numeric, Some("g3")),
    ];
    assert!(EvalReport::with_defaults(&distinct, &scores)
        .unwrap()
        .clustered_stderr
        .is_none());

    // One item missing a cluster id → not all tagged → skipped.
    let partial = vec![
        task("a", "1", AnswerKind::Numeric, Some("g1")),
        task("b", "2", AnswerKind::Numeric, Some("g1")),
        task("c", "3", AnswerKind::Numeric, None),
    ];
    assert!(EvalReport::with_defaults(&partial, &scores)
        .unwrap()
        .clustered_stderr
        .is_none());
}

#[test]
fn report_rejects_mismatch_and_too_few() {
    let tasks = vec![num("1"), num("2")];
    assert!(EvalReport::with_defaults(&tasks, &[score(1.0, false)]).is_none()); // length mismatch
    assert!(EvalReport::with_defaults(&[num("1")], &[score(1.0, false)]).is_none());
    // n < 2
}
