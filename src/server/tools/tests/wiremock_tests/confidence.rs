//! Wiremock tests for the `reasoning_confidence_route` handler.
//!
//! These tests exercise the routing branches (direct / escalated_to_tree / budget_override)
//! and the execution branches (linear / divergent / tree) using a mocked Anthropic API.
//!
//! Design: the mock returns a combined JSON that satisfies *both* the auto-detection
//! parser (needs `selected_mode`, `confidence`, `reasoning`) and the execution parsers
//! (linear needs `analysis`, tree needs `branches`, divergent needs `perspectives`).

use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::ConfidenceRouteRequest;

// ============================================================================
// Helpers
// ============================================================================

/// A combined JSON that satisfies both the auto-detection parser and the
/// linear execution parser. `selected_mode` and `confidence` drive routing;
/// `analysis` satisfies linear mode parsing.
fn auto_linear_json(selected_mode: &str, confidence: f64) -> String {
    serde_json::json!({
        "selected_mode": selected_mode,
        "reasoning": "Sequential analysis is most appropriate here",
        "confidence": confidence,
        "characteristics": ["sequential", "clear steps"],
        "suggested_parameters": {},
        "analysis": "This is the reasoning result from the selected strategy.",
        "next_step": "Review the output and proceed accordingly"
    })
    .to_string()
}

/// Combined JSON for auto-detection + tree execution.
/// `branches` satisfies tree mode "create" parsing.
fn auto_tree_json(selected_mode: &str, confidence: f64) -> String {
    serde_json::json!({
        "selected_mode": selected_mode,
        "reasoning": "Tree exploration is appropriate",
        "confidence": confidence,
        "characteristics": ["complex", "multi-path"],
        "suggested_parameters": {},
        "branches": [
            {"id": "b1", "content": "Approach A", "score": 0.8},
            {"id": "b2", "content": "Approach B", "score": 0.7}
        ],
        "recommendation": "Explore branch b1 first"
    })
    .to_string()
}

/// Combined JSON for auto-detection + divergent execution.
fn auto_divergent_json(confidence: f64) -> String {
    serde_json::json!({
        "selected_mode": "divergent",
        "reasoning": "Multiple perspectives will surface blind spots",
        "confidence": confidence,
        "characteristics": ["multi-perspective", "open-ended"],
        "suggested_parameters": {},
        "perspectives": [
            {"viewpoint": "Optimistic", "content": "Positive outlook", "novelty_score": 0.8},
            {"viewpoint": "Pessimistic", "content": "Cautionary view", "novelty_score": 0.7}
        ]
    })
    .to_string()
}

// ============================================================================
// Routing: direct (high confidence)
// ============================================================================

#[tokio::test]
async fn test_confidence_route_high_confidence_executes_linear_directly() {
    let mock_server = MockServer::start().await;

    // confidence 0.88 >= default threshold 0.75 → direct → execute linear
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_linear_json("linear", 0.88))),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = ConfidenceRouteRequest {
        content: "Analyze the tradeoffs step by step".to_string(),
        session_id: None,
        high_confidence_threshold: None, // default 0.75
        budget: None,
    };

    let resp = server.reasoning_confidence_route(Parameters(req)).await;
    assert_eq!(resp.executed_mode, "linear");
    assert_eq!(resp.routing_decision, "direct");
    assert!((resp.routing_confidence - 0.88).abs() < 1e-6);
    assert!(resp.routing_reason.contains("0.88"));
    assert!(!resp.result.is_null());
}

#[tokio::test]
async fn test_confidence_route_high_confidence_divergent_executes_directly() {
    let mock_server = MockServer::start().await;

    // confidence 0.9 >= threshold 0.75, auto suggests "divergent" → execute divergent
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&auto_divergent_json(0.9))),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = ConfidenceRouteRequest {
        content: "What are the multiple perspectives on this?".to_string(),
        session_id: None,
        high_confidence_threshold: None,
        budget: None,
    };

    let resp = server.reasoning_confidence_route(Parameters(req)).await;
    assert_eq!(resp.executed_mode, "divergent");
    assert_eq!(resp.routing_decision, "direct");
    assert_eq!(resp.auto_suggested_mode, "divergent");
}

#[tokio::test]
async fn test_confidence_route_complex_mode_falls_back_to_linear() {
    let mock_server = MockServer::start().await;

    // auto suggests "mcts" (complex, needs parameters) at high confidence → linear fallback
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_linear_json("mcts", 0.9))),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = ConfidenceRouteRequest {
        content: "Explore this problem space".to_string(),
        session_id: None,
        high_confidence_threshold: None,
        budget: None,
    };

    let resp = server.reasoning_confidence_route(Parameters(req)).await;
    assert_eq!(resp.executed_mode, "linear");
    assert_eq!(resp.routing_decision, "direct_fallback");
    assert_eq!(resp.auto_suggested_mode, "mcts");
    assert!(resp.routing_reason.contains("mcts"));
}

// ============================================================================
// Routing: escalated_to_tree (low confidence)
// ============================================================================

#[tokio::test]
async fn test_confidence_route_low_confidence_escalates_to_tree() {
    let mock_server = MockServer::start().await;

    // confidence 0.55 < default threshold 0.75 → escalate to tree
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_tree_json("linear", 0.55))),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = ConfidenceRouteRequest {
        content: "I need to decide between several complex options".to_string(),
        session_id: None,
        high_confidence_threshold: None,
        budget: None,
    };

    let resp = server.reasoning_confidence_route(Parameters(req)).await;
    assert_eq!(resp.executed_mode, "tree");
    assert_eq!(resp.routing_decision, "escalated_to_tree");
    assert!((resp.routing_confidence - 0.55).abs() < 1e-6);
    assert!(resp.routing_reason.contains("0.55"));
    // next_call should guide user to focus a branch
    let next = resp
        .next_call
        .expect("tree escalation should include next_call");
    assert_eq!(next.tool, "reasoning_tree");
    assert!(next.args.get("operation").is_some());
}

#[tokio::test]
async fn test_confidence_route_custom_threshold_triggers_tree() {
    let mock_server = MockServer::start().await;

    // confidence 0.88 is high, but custom threshold 0.95 means it's still "low" → tree
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_tree_json("linear", 0.88))),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = ConfidenceRouteRequest {
        content: "Analyze this thoroughly".to_string(),
        session_id: None,
        high_confidence_threshold: Some(0.95), // high bar
        budget: None,
    };

    let resp = server.reasoning_confidence_route(Parameters(req)).await;
    assert_eq!(resp.executed_mode, "tree");
    assert_eq!(resp.routing_decision, "escalated_to_tree");
    assert!(resp.routing_reason.contains("0.95")); // threshold mentioned
}

// ============================================================================
// Routing: budget_override
// ============================================================================

#[tokio::test]
async fn test_confidence_route_budget_low_forces_linear() {
    let mock_server = MockServer::start().await;

    // budget=low → always linear, regardless of confidence
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_linear_json("tree", 0.9))),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = ConfidenceRouteRequest {
        content: "Quick analysis needed".to_string(),
        session_id: None,
        high_confidence_threshold: None,
        budget: Some("low".to_string()),
    };

    let resp = server.reasoning_confidence_route(Parameters(req)).await;
    assert_eq!(resp.executed_mode, "linear");
    assert_eq!(resp.routing_decision, "budget_override");
    assert!(resp.routing_reason.contains("low"));
}

#[tokio::test]
async fn test_confidence_route_budget_high_forces_tree() {
    let mock_server = MockServer::start().await;

    // budget=high → always tree, regardless of confidence
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_tree_json("linear", 0.95))),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = ConfidenceRouteRequest {
        content: "Thoroughly explore this complex problem".to_string(),
        session_id: None,
        high_confidence_threshold: None,
        budget: Some("high".to_string()),
    };

    let resp = server.reasoning_confidence_route(Parameters(req)).await;
    assert_eq!(resp.executed_mode, "tree");
    assert_eq!(resp.routing_decision, "budget_override");
    assert!(resp.routing_reason.contains("high"));
    let next = resp.next_call.expect("tree always returns next_call");
    assert_eq!(next.tool, "reasoning_tree");
}

#[tokio::test]
async fn test_confidence_route_budget_auto_is_default() {
    let mock_server = MockServer::start().await;

    // budget="auto" explicitly → same as None (uses confidence routing)
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_linear_json("linear", 0.8))),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = ConfidenceRouteRequest {
        content: "Standard analysis".to_string(),
        session_id: None,
        high_confidence_threshold: None,
        budget: Some("auto".to_string()),
    };

    let resp = server.reasoning_confidence_route(Parameters(req)).await;
    // 0.8 >= 0.75, linear suggested → direct
    assert_eq!(resp.executed_mode, "linear");
    assert_eq!(resp.routing_decision, "direct");
}

// ============================================================================
// Response structure validation
// ============================================================================

#[tokio::test]
async fn test_confidence_route_response_fields_populated() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_linear_json("linear", 0.85))),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = ConfidenceRouteRequest {
        content: "Verify all response fields are populated".to_string(),
        session_id: Some("s-verify".to_string()),
        high_confidence_threshold: Some(0.75),
        budget: None,
    };

    let resp = server.reasoning_confidence_route(Parameters(req)).await;
    // All fields should be populated
    assert!(!resp.executed_mode.is_empty());
    assert!(!resp.auto_suggested_mode.is_empty());
    assert!(resp.routing_confidence > 0.0);
    assert!(!resp.routing_decision.is_empty());
    assert!(!resp.routing_reason.is_empty());
    assert!(!resp.result.is_null());
}
