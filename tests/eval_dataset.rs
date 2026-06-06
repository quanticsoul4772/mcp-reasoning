//! Tests for the eval dataset model ([`mcp_reasoning::eval::task`]).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mcp_reasoning::eval::{AnswerKind, Dataset, DatasetError, EvalTask};

const TWO_LINES: &str = concat!(
    r#"{"id":"a","prompt":"2+2?","target":"4","answer_kind":"numeric"}"#,
    "\n",
    r#"{"id":"b","cluster_id":"g1","prompt":"cap of France?","target":"Paris","answer_kind":"exact","expected_mode":"linear","metadata":{"difficulty":"easy"}}"#,
);

#[test]
fn parses_jsonl_with_fields_and_defaults() {
    let ds = Dataset::from_jsonl(TWO_LINES).unwrap();
    assert_eq!(ds.len(), 2);
    assert!(!ds.is_empty());

    let a = &ds.tasks()[0];
    assert_eq!(a.id, "a");
    assert_eq!(a.answer_kind, AnswerKind::Numeric);
    // Omitted optionals default cleanly.
    assert!(a.cluster_id.is_none());
    assert!(a.expected_mode.is_none());
    assert!(a.metadata.is_empty());

    let b = &ds.tasks()[1];
    assert_eq!(b.cluster_id.as_deref(), Some("g1"));
    assert_eq!(b.expected_mode.as_deref(), Some("linear"));
    assert_eq!(b.answer_kind, AnswerKind::Exact);
    assert_eq!(
        b.metadata.get("difficulty").and_then(|v| v.as_str()),
        Some("easy")
    );
}

#[test]
fn skips_blank_lines() {
    let content = format!("\n{TWO_LINES}\n\n");
    let ds = Dataset::from_jsonl(&content).unwrap();
    assert_eq!(ds.len(), 2);
}

#[test]
fn empty_content_is_empty_error() {
    assert!(matches!(
        Dataset::from_jsonl("   \n\n"),
        Err(DatasetError::Empty)
    ));
}

#[test]
fn malformed_line_reports_one_based_line_number() {
    // Line 2 is malformed.
    let content = concat!(
        r#"{"id":"a","prompt":"p","target":"4","answer_kind":"numeric"}"#,
        "\n",
        r#"{"id":"b","prompt":"p" MISSING"#,
    );
    let err = Dataset::from_jsonl(content).unwrap_err();
    assert!(
        matches!(err, DatasetError::Parse { line: 2, .. }),
        "expected parse error on line 2, got {err:?}"
    );
}

#[test]
fn unknown_answer_kind_is_a_parse_error() {
    let content = r#"{"id":"a","prompt":"p","target":"4","answer_kind":"poetic"}"#;
    assert!(matches!(
        Dataset::from_jsonl(content),
        Err(DatasetError::Parse { line: 1, .. })
    ));
}

#[test]
fn from_jsonl_file_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ds.jsonl");
    std::fs::write(&path, TWO_LINES).unwrap();
    let ds = Dataset::from_jsonl_file(&path).unwrap();
    assert_eq!(ds.len(), 2);
}

#[test]
fn missing_file_is_io_error() {
    let err = Dataset::from_jsonl_file("does/not/exist.jsonl").unwrap_err();
    assert!(matches!(err, DatasetError::Io(_)));
}

#[test]
fn ships_a_loadable_seed_dataset() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("eval")
        .join("data")
        .join("seed_arithmetic.jsonl");
    let ds = Dataset::from_jsonl_file(&path).expect("seed dataset loads");
    assert!(ds.len() >= 5, "seed should have a usable slice");
    assert!(
        ds.tasks()
            .iter()
            .all(|t: &EvalTask| t.answer_kind == AnswerKind::Numeric),
        "seed items are programmatically scoreable"
    );
}
