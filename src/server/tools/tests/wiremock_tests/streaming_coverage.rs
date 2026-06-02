//! Wiremock tests for the streaming handler success arms (mcts, counterfactual).
//!
//! These handlers call `complete_streaming`, so a plain JSON mock makes the
//! client fall back to the error path. Returning a real SSE body via
//! [`super::anthropic_sse_response`] drives the success arms — and lets us
//! assert the verification fires (and catches injected errors) end-to-end.

use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_sse_response, create_mocked_server};
use crate::server::requests::{CounterfactualRequest, MctsRequest};

/// MCTS explore response with internally consistent UCB1: each ucb1_score equals
/// average_value + exploration_bonus, and `selected_node` is the argmax.
fn mcts_explore_consistent() -> String {
    serde_json::json!({
        "frontier_evaluation": [
            {"node_id": "a", "visits": 8, "average_value": 0.6, "exploration_bonus": 0.2, "ucb1_score": 0.8},
            {"node_id": "b", "visits": 2, "average_value": 0.4, "exploration_bonus": 0.55, "ucb1_score": 0.95}
        ],
        "selected_node": {"node_id": "b", "selection_reason": "Highest UCB1 (0.95)"},
        "expansion": {"new_nodes": [
            {"id": "b1", "content": "Refine option b", "simulated_value": 0.7}
        ]},
        "backpropagation": {"updated_nodes": ["b", "root"], "value_changes": {"b": 0.1, "root": 0.02}},
        "search_status": {"total_nodes": 6, "total_simulations": 30, "best_path_value": 0.7}
    })
    .to_string()
}

async fn mount_sse(mock_server: &MockServer, body_text: &str) {
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(anthropic_sse_response(body_text)),
        )
        .mount(mock_server)
        .await;
}

fn mcts_req() -> MctsRequest {
    MctsRequest {
        operation: Some("explore".to_string()),
        content: Some("Explore strategies".to_string()),
        session_id: Some("s-stream-mcts".to_string()),
        node_id: None,
        iterations: None,
        exploration_constant: None,
        simulation_depth: None,
        quality_threshold: None,
        auto_execute: None,
        lookback_depth: None,
        progress_token: None,
    }
}

#[tokio::test]
async fn test_mcts_explore_streaming_success_surfaces_breakdown() {
    let mock_server = MockServer::start().await;
    mount_sse(&mock_server, &mcts_explore_consistent()).await;
    let server = create_mocked_server(&mock_server).await;

    let resp = server.reasoning_mcts(Parameters(mcts_req())).await;
    // Success arm: the rich fields are surfaced (not the error fallback).
    let frontier = resp.frontier.expect("frontier surfaced");
    assert_eq!(frontier.len(), 2);
    let selected = resp.selected_node.expect("selected node surfaced");
    assert_eq!(selected.node_id, "b");
    let expanded = resp.expanded_nodes.expect("expanded nodes surfaced");
    assert_eq!(expanded[0].content, "Refine option b");
    let validation = resp.validation.expect("validation present");
    assert!(validation.consistent, "warnings: {:?}", validation.warnings);
}

#[tokio::test]
async fn test_mcts_explore_streaming_flags_non_argmax_selection() {
    let mock_server = MockServer::start().await;
    // Same frontier, but the model "selects" a — even though b has the higher UCB1.
    let bad = serde_json::json!({
        "frontier_evaluation": [
            {"node_id": "a", "visits": 8, "average_value": 0.6, "exploration_bonus": 0.2, "ucb1_score": 0.8},
            {"node_id": "b", "visits": 2, "average_value": 0.4, "exploration_bonus": 0.55, "ucb1_score": 0.95}
        ],
        "selected_node": {"node_id": "a", "selection_reason": "(wrong) picked a"},
        "expansion": {"new_nodes": [{"id": "a1", "content": "Refine a", "simulated_value": 0.5}]},
        "backpropagation": {"updated_nodes": ["a"], "value_changes": {"a": 0.05}},
        "search_status": {"total_nodes": 6, "total_simulations": 30, "best_path_value": 0.7}
    })
    .to_string();
    mount_sse(&mock_server, &bad).await;
    let server = create_mocked_server(&mock_server).await;

    let resp = server.reasoning_mcts(Parameters(mcts_req())).await;
    let validation = resp.validation.expect("validation present");
    assert!(!validation.consistent);
    assert!(validation
        .warnings
        .iter()
        .any(|w| w.contains("highest-UCB1")));
}

#[tokio::test]
async fn test_counterfactual_streaming_success_surfaces_ladder() {
    let mock_server = MockServer::start().await;
    // Consistent causal model: cause X, effect Y, confounder Z linked to both.
    let json = serde_json::json!({
        "causal_question": {
            "statement": "Does X cause Y?",
            "ladder_rung": "counterfactual",
            "variables": {"cause": "X", "effect": "Y", "intervention": "remove X"}
        },
        "causal_model": {
            "nodes": ["X", "Y", "Z"],
            "edges": [
                {"from": "X", "to": "Y", "type": "direct"},
                {"from": "Z", "to": "X", "type": "confounded"},
                {"from": "Z", "to": "Y", "type": "confounded"}
            ],
            "confounders": ["Z"]
        },
        "analysis": {
            "association_level": {"observed_correlation": 0.7, "interpretation": "Correlated but confounded by Z"},
            "intervention_level": {"causal_effect": 0.4, "mechanism": "X directly raises Y"},
            "counterfactual_level": {"scenario": "If X were removed", "outcome": "Y would be lower", "confidence": 0.6}
        },
        "conclusions": {
            "causal_claim": "X contributes about 0.4 to Y",
            "strength": "moderate",
            "caveats": ["Z confounds the raw correlation"],
            "actionable_insight": "Run an A/B test isolating X"
        }
    })
    .to_string();
    mount_sse(&mock_server, &json).await;
    let server = create_mocked_server(&mock_server).await;

    let req = CounterfactualRequest {
        scenario: "X happened alongside Z".to_string(),
        intervention: "remove X".to_string(),
        analysis_depth: None,
        session_id: Some("s-stream-cf".to_string()),
        progress_token: None,
    };
    let resp = server.reasoning_counterfactual(Parameters(req)).await;

    // Both previously-dropped rungs are surfaced.
    let assoc = resp.association.expect("association rung surfaced");
    assert!((assoc.observed_correlation - 0.7).abs() < 1e-9);
    let interv = resp.intervention.expect("intervention rung surfaced");
    assert!((interv.causal_effect - 0.4).abs() < 1e-9);
    assert_eq!(resp.ladder_rung.as_deref(), Some("counterfactual"));
    let model = resp.causal_model.expect("causal model surfaced");
    assert_eq!(model.nodes.len(), 3);
    let validation = resp.validation.expect("validation present");
    assert!(validation.consistent, "warnings: {:?}", validation.warnings);
}
