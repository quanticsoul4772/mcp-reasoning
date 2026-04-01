use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::*;

#[tokio::test]
async fn test_linear_success_path() {
    let mock_server = MockServer::start().await;

    let response_json = serde_json::json!({
        "analysis": "Detailed reasoning analysis",
        "confidence": 0.85,
        "next_step": "Continue with more analysis"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = LinearRequest {
        content: "Analyze this problem".to_string(),
        session_id: None,
        confidence: Some(ConfidenceThreshold::try_from(0.8).unwrap()),
        timeout_ms: None,
    };

    let resp = server.reasoning_linear(Parameters(req)).await;
    // Should succeed with mocked response
    assert!(!resp.thought_id.is_empty() || !resp.content.is_empty());
}

#[tokio::test]
async fn test_tree_all_operations() {
    let mock_server = MockServer::start().await;

    // Test create operation
    let create_json = serde_json::json!({
        "branches": [
            {"id": "b1", "content": "Branch 1", "score": 0.8},
            {"id": "b2", "content": "Branch 2", "score": 0.7}
        ],
        "recommendation": "Explore branch 1 first"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&create_json.to_string())),
        )
        .expect(1..)
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    // Test create
    let create_req = TreeRequest {
        operation: Some("create".to_string()),
        content: Some("Explore this topic".to_string()),
        session_id: Some("s1".to_string()),
        branch_id: None,
        num_branches: Some(2),
        completed: None,
    };
    let resp = server.reasoning_tree(Parameters(create_req)).await;
    assert_eq!(resp.session_id, "s1");

    // Test list
    let list_req = TreeRequest {
        operation: Some("list".to_string()),
        content: None,
        session_id: Some("s1".to_string()),
        branch_id: None,
        num_branches: None,
        completed: None,
    };
    let resp = server.reasoning_tree(Parameters(list_req)).await;
    assert_eq!(resp.session_id, "s1");

    // Test focus
    let focus_req = TreeRequest {
        operation: Some("focus".to_string()),
        content: None,
        session_id: Some("s1".to_string()),
        branch_id: Some("b1".to_string()),
        num_branches: None,
        completed: None,
    };
    let resp = server.reasoning_tree(Parameters(focus_req)).await;
    assert_eq!(resp.session_id, "s1");

    // Test complete
    let complete_req = TreeRequest {
        operation: Some("complete".to_string()),
        content: None,
        session_id: Some("s1".to_string()),
        branch_id: Some("b1".to_string()),
        num_branches: None,
        completed: Some(true),
    };
    let resp = server.reasoning_tree(Parameters(complete_req)).await;
    assert_eq!(resp.session_id, "s1");

    // Test unknown operation
    let unknown_req = TreeRequest {
        operation: Some("unknown".to_string()),
        content: None,
        session_id: Some("s1".to_string()),
        branch_id: None,
        num_branches: None,
        completed: None,
    };
    let resp = server.reasoning_tree(Parameters(unknown_req)).await;
    assert!(resp.recommendation.unwrap().contains("Unknown operation"));
}

#[tokio::test]
async fn test_tree_summarize_operation() {
    let mock_server = MockServer::start().await;

    // Both the create call and the summarize LLM call use the same mock
    let branches_json = serde_json::json!({
        "branches": [
            {"id": "b1", "content": "Branch 1 analysis", "score": 0.8},
            {"id": "b2", "content": "Branch 2 analysis", "score": 0.7}
        ],
        "recommendation": "Branch 1 is strongest"
    });

    // The summarize LLM call returns a tree_complete_prompt-formatted response
    let summarize_json = serde_json::json!({
        "key_findings": ["Finding 1: branch 1 is strongest", "Finding 2: both branches converge"],
        "best_insights": ["Insight: the core approach is sound"],
        "synthesis": "Both branches confirm the hypothesis with high confidence.",
        "unresolved": ["Edge case under unusual conditions"],
        "confidence": 0.88
    });

    // First call: create (returns branch structure)
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&branches_json.to_string())),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second call: summarize (returns synthesis)
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&summarize_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    // Create a session first
    let create_req = TreeRequest {
        operation: Some("create".to_string()),
        content: Some("Explore this topic".to_string()),
        session_id: Some("s-summarize".to_string()),
        branch_id: None,
        num_branches: Some(2),
        completed: None,
    };
    let create_resp = server.reasoning_tree(Parameters(create_req)).await;
    assert_eq!(create_resp.session_id, "s-summarize");

    // Now summarize
    let summarize_req = TreeRequest {
        operation: Some("summarize".to_string()),
        content: None,
        session_id: Some("s-summarize".to_string()),
        branch_id: None,
        num_branches: None,
        completed: None,
    };
    let resp = server.reasoning_tree(Parameters(summarize_req)).await;
    assert_eq!(resp.session_id, "s-summarize");
    // synthesis and key_findings should be populated
    assert!(resp.synthesis.is_some());
    assert!(resp.key_findings.is_some());
    assert!(resp.best_insights.is_some());
    let key_findings = resp.key_findings.unwrap();
    assert!(!key_findings.is_empty());
}

#[tokio::test]
async fn test_divergent_success_path() {
    let mock_server = MockServer::start().await;

    let response_json = serde_json::json!({
        "perspectives": [
            {"viewpoint": "Optimistic", "content": "Positive outlook", "novelty_score": 0.8},
            {"viewpoint": "Pessimistic", "content": "Cautionary view", "novelty_score": 0.7}
        ],
        "challenged_assumptions": ["Assumption 1"],
        "synthesis": "Combined insight"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = DivergentRequest {
        content: "Analyze from multiple perspectives".to_string(),
        session_id: Some("s1".to_string()),
        num_perspectives: Some(2),
        challenge_assumptions: Some(true),
        force_rebellion: Some(true),
        progress_token: None,
    };

    let resp = server.reasoning_divergent(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_reflection_all_operations() {
    let mock_server = MockServer::start().await;

    let process_json = serde_json::json!({
        "analysis": {
            "strengths": ["Clear logic"],
            "weaknesses": ["Needs more evidence"]
        },
        "improvements": [
            {"suggestion": "Add examples", "priority": 1}
        ],
        "refined_reasoning": "Improved version",
        "confidence_improvement": 0.15
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&process_json.to_string())),
        )
        .expect(1..)
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    // Test process
    let process_req = ReflectionRequest {
        operation: Some("process".to_string()),
        content: Some("Reasoning to improve".to_string()),
        thought_id: None,
        session_id: Some("s1".to_string()),
        max_iterations: Some(3),
        quality_threshold: Some(0.8),
        progress_token: None,
    };
    let resp = server.reasoning_reflection(Parameters(process_req)).await;
    assert!(resp.quality_score >= 0.0);

    // Test evaluate
    let eval_json = serde_json::json!({
        "session_assessment": {
            "overall_quality": 0.8,
            "coherence": 0.85,
            "reasoning_depth": 0.75
        },
        "strongest_elements": ["Logic", "Structure"],
        "areas_for_improvement": ["More examples"],
        "recommendations": ["Add case studies"]
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&eval_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let evaluate_req = ReflectionRequest {
        operation: Some("evaluate".to_string()),
        content: None,
        thought_id: None,
        session_id: Some("s1".to_string()),
        max_iterations: None,
        quality_threshold: None,
        progress_token: None,
    };
    let resp = server.reasoning_reflection(Parameters(evaluate_req)).await;
    assert!(resp.quality_score >= 0.0);

    // Test unknown operation
    let unknown_req = ReflectionRequest {
        operation: Some("unknown".to_string()),
        content: None,
        thought_id: None,
        session_id: Some("s1".to_string()),
        max_iterations: None,
        quality_threshold: None,
        progress_token: None,
    };
    let resp = server.reasoning_reflection(Parameters(unknown_req)).await;
    assert!(resp
        .weaknesses
        .unwrap()
        .iter()
        .any(|w| w.contains("Unknown")));
}

#[tokio::test]
async fn test_checkpoint_all_operations() {
    let mock_server = MockServer::start().await;

    // No API calls needed for checkpoint - it's storage-only
    let server = create_mocked_server(&mock_server).await;

    // First create a session
    let create_req = CheckpointRequest {
        operation: "create".to_string(),
        session_id: "s1".to_string(),
        checkpoint_id: None,
        name: Some("cp1".to_string()),
        description: Some("Test checkpoint".to_string()),
        new_direction: None,
    };
    let resp = server.reasoning_checkpoint(Parameters(create_req)).await;
    assert_eq!(resp.session_id, "s1");

    // List checkpoints
    let list_req = CheckpointRequest {
        operation: "list".to_string(),
        session_id: "s1".to_string(),
        checkpoint_id: None,
        name: None,
        description: None,
        new_direction: None,
    };
    let resp = server.reasoning_checkpoint(Parameters(list_req)).await;
    assert_eq!(resp.session_id, "s1");

    // Restore (will fail since no actual checkpoint, but exercises code path)
    let restore_req = CheckpointRequest {
        operation: "restore".to_string(),
        session_id: "s1".to_string(),
        checkpoint_id: Some("cp-nonexistent".to_string()),
        name: None,
        description: None,
        new_direction: Some("New direction".to_string()),
    };
    let resp = server.reasoning_checkpoint(Parameters(restore_req)).await;
    // Will have error in restored_state since checkpoint doesn't exist
    assert!(resp.restored_state.is_some());

    // Unknown operation
    let unknown_req = CheckpointRequest {
        operation: "unknown".to_string(),
        session_id: "s1".to_string(),
        checkpoint_id: None,
        name: None,
        description: None,
        new_direction: None,
    };
    let resp = server.reasoning_checkpoint(Parameters(unknown_req)).await;
    assert!(resp.restored_state.is_some());
}

#[tokio::test]
async fn test_auto_success_path() {
    let mock_server = MockServer::start().await;

    let response_json = serde_json::json!({
        "selected_mode": "tree",
        "reasoning": "Content suggests branching exploration",
        "characteristics": ["complex", "multi-path"],
        "suggested_parameters": {"num_branches": 3},
        "alternative_mode": {"mode": "linear", "reason": "Simpler option"}
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = AutoRequest {
        content: "Complex problem with multiple paths".to_string(),
        hints: Some(vec!["exploration".to_string()]),
        session_id: Some("s1".to_string()),
        execute: None,
    };

    let resp = server.reasoning_auto(Parameters(req)).await;
    assert!(!resp.selected_mode.is_empty());
}

// ============================================================================
// ConfidenceThreshold type tests — invalid states are unrepresentable
// ============================================================================

#[test]
fn test_confidence_threshold_valid_range() {
    // Boundary values
    assert!(ConfidenceThreshold::try_from(0.0).is_ok());
    assert!(ConfidenceThreshold::try_from(1.0).is_ok());
    assert!(ConfidenceThreshold::try_from(0.5).is_ok());
    assert!(ConfidenceThreshold::try_from(0.999).is_ok());
}

#[test]
fn test_confidence_threshold_rejects_nan() {
    // NaN used to silently disable the threshold (NaN < x == false always).
    // Now it's rejected at construction.
    assert!(ConfidenceThreshold::try_from(f64::NAN).is_err());
    let err = ConfidenceThreshold::try_from(f64::NAN).unwrap_err();
    assert!(err.contains("finite"), "error should mention finite: {err}");
}

#[test]
fn test_confidence_threshold_rejects_negative() {
    // Negative values used to silently disable the threshold.
    assert!(ConfidenceThreshold::try_from(-0.001).is_err());
    assert!(ConfidenceThreshold::try_from(-1.0).is_err());
}

#[test]
fn test_confidence_threshold_rejects_out_of_range() {
    // Values > 1.0 or +∞ used to reject all responses silently.
    assert!(ConfidenceThreshold::try_from(1.001).is_err());
    assert!(ConfidenceThreshold::try_from(f64::INFINITY).is_err());
}

#[test]
fn test_confidence_threshold_json_round_trip() {
    // Serializes as a plain f64, deserializes with validation.
    let t = ConfidenceThreshold::try_from(0.75).unwrap();
    let json = serde_json::to_string(&t).unwrap();
    assert_eq!(json, "0.75");

    let back: ConfidenceThreshold = serde_json::from_str("0.75").unwrap();
    assert!((back.value() - 0.75).abs() < f64::EPSILON);
}

#[test]
fn test_confidence_threshold_json_rejects_invalid() {
    // Bad values rejected at deserialization — before reaching any handler.
    assert!(serde_json::from_str::<ConfidenceThreshold>("1.5").is_err());
    assert!(serde_json::from_str::<ConfidenceThreshold>("-0.1").is_err());
}

#[test]
fn test_linear_request_confidence_json_rejects_out_of_range() {
    // Regression: a LinearRequest with an invalid confidence value in JSON
    // should be rejected at deserialization, never reaching the handler.
    let bad_json = r#"{"content":"test","confidence":1.5}"#;
    let result = serde_json::from_str::<LinearRequest>(bad_json);
    assert!(
        result.is_err(),
        "confidence:1.5 should be rejected at parse time"
    );
}
