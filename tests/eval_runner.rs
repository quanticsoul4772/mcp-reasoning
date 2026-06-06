//! Tests for the eval runner and solvers ([`mcp_reasoning::eval::runner`],
//! [`mcp_reasoning::eval::solver`]).
//!
//! The `MockSolver` tests cover orchestration offline; the wiremock test proves
//! the real `LinearSolver` path end-to-end against a mocked Anthropic API.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use mcp_reasoning::eval::{run_eval, Dataset, ExactMatch, MockSolver};

const DS: &str = concat!(
    r#"{"id":"a","prompt":"p","target":"48","answer_kind":"numeric"}"#,
    "\n",
    r#"{"id":"b","prompt":"p","target":"99","answer_kind":"numeric"}"#,
);

#[tokio::test]
async fn mock_solver_scores_and_aggregates() {
    let ds = Dataset::from_jsonl(DS).unwrap();
    // Both tasks get "#### 48": a (target 48) correct, b (target 99) wrong.
    let solver = MockSolver::new("reasoning... #### 48");
    let outcome = run_eval(&ds, &solver, &ExactMatch::new()).await;

    assert_eq!(outcome.results.len(), 2);
    assert_eq!(outcome.solver_errors, 0);
    let r = outcome.report.expect("two scored items");
    assert_eq!(r.n, 2);
    assert!((r.mean_score - 0.5).abs() < 1e-12);
    assert_eq!(r.extraction_failure_rate, 0.0);
}

#[tokio::test]
async fn mock_solver_per_task_outputs() {
    let ds = Dataset::from_jsonl(DS).unwrap();
    let solver = MockSolver::new("no answer")
        .with_output("a", "#### 48")
        .with_output("b", "#### 99");
    let outcome = run_eval(&ds, &solver, &ExactMatch::new()).await;
    assert_eq!(outcome.report.unwrap().mean_score, 1.0);
}

#[tokio::test]
async fn solver_errors_are_excluded_from_the_report() {
    let ds = Dataset::from_jsonl(DS).unwrap();
    // 'b' errors; only 'a' is scored, so n < 2 → no report, but the error is
    // recorded and counted.
    let solver = MockSolver::new("#### 48").with_error("b");
    let outcome = run_eval(&ds, &solver, &ExactMatch::new()).await;

    assert_eq!(outcome.solver_errors, 1);
    let b = outcome
        .results
        .iter()
        .find(|r| r.task_id == "b")
        .expect("b recorded");
    assert!(b.error.is_some());
    assert!(b.extracted.is_none());
    assert!(
        outcome.report.is_none(),
        "one scored item cannot form a report"
    );
}

#[tokio::test]
async fn extraction_failure_rate_surfaces_in_the_report() {
    let ds = Dataset::from_jsonl(DS).unwrap();
    let solver = MockSolver::new("I cannot answer this");
    let outcome = run_eval(&ds, &solver, &ExactMatch::new()).await;
    let r = outcome.report.unwrap();
    assert!((r.extraction_failure_rate - 1.0).abs() < 1e-12);
    assert_eq!(r.mean_score, 0.0);
}

#[tokio::test]
async fn outcome_serializes_to_json() {
    let ds = Dataset::from_jsonl(DS).unwrap();
    let outcome = run_eval(&ds, &MockSolver::new("#### 48"), &ExactMatch::new()).await;
    let json = outcome.to_json().unwrap();
    for needle in ["\"mode\"", "\"results\"", "\"report\"", "\"mde\"", "\"n\""] {
        assert!(json.contains(needle), "missing {needle} in {json}");
    }
}

// ---- end-to-end: real LinearSolver against a mocked Anthropic API ----------

mod wiremock_path {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

    use mcp_reasoning::anthropic::{AnthropicClient, ClientConfig};
    use mcp_reasoning::eval::{run_eval, Dataset, ExactMatch, LinearSolver};
    use mcp_reasoning::storage::SqliteStorage;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn anthropic_envelope(text: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": text}],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 100, "output_tokens": 50}
        })
    }

    #[tokio::test]
    async fn linear_solver_runs_against_mocked_anthropic() {
        let mock = MockServer::start().await;

        // The mode parses a JSON object with `analysis`/`confidence`; the
        // analysis text carries the terminal-format answer the scorer extracts.
        let analysis = serde_json::json!({
            "analysis": "Reasoning about the problem. #### 48",
            "confidence": 0.9
        })
        .to_string();
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(anthropic_envelope(&analysis)))
            .mount(&mock)
            .await;

        let client_config = ClientConfig::new()
            .with_base_url(mock.uri())
            .with_max_retries(0);
        let client = AnthropicClient::new("test-key", client_config).unwrap();
        let storage = SqliteStorage::new(":memory:").await.unwrap();
        let solver = LinearSolver::new(storage, client);

        let ds = Dataset::from_jsonl(super::DS).unwrap();
        let outcome = run_eval(&ds, &solver, &ExactMatch::new()).await;

        assert_eq!(outcome.mode, "linear");
        assert_eq!(outcome.solver_errors, 0);
        let r = outcome.report.expect("two scored items");
        assert_eq!(r.n, 2);
        // Both answers are 48: a (target 48) correct, b (target 99) wrong.
        assert!((r.mean_score - 0.5).abs() < 1e-12);
    }
}
