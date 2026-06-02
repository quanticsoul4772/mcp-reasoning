//! Wiremock tests for the streaming handler success arms (mcts, counterfactual).
//!
//! These handlers call `complete_streaming`, so a plain JSON mock makes the
//! client fall back to the error path. Returning a real SSE body via
//! [`super::anthropic_sse_response`] drives the success arms — and lets us
//! assert the verification fires (and catches injected errors) end-to-end.

use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{body_string_contains, method, path};
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

/// Consistent UCB1 and an argmax selection of `b`, but backpropagation never
/// updated the selected node `b` — only `root`. Isolates the coherence check.
fn mcts_explore_backprop_incoherent() -> String {
    serde_json::json!({
        "frontier_evaluation": [
            {"node_id": "a", "visits": 8, "average_value": 0.6, "exploration_bonus": 0.2, "ucb1_score": 0.8},
            {"node_id": "b", "visits": 2, "average_value": 0.4, "exploration_bonus": 0.55, "ucb1_score": 0.95}
        ],
        "selected_node": {"node_id": "b", "selection_reason": "Highest UCB1 (0.95)"},
        "expansion": {"new_nodes": [
            {"id": "b1", "content": "Refine option b", "simulated_value": 0.7}
        ]},
        "backpropagation": {"updated_nodes": ["root"], "value_changes": {}},
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
        exploration_constant: None,
        simulation_depth: None,
        quality_threshold: None,
        auto_execute: None,
        lookback_depth: None,
        thinking: None,
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
    // Top two are within 0.15 UCB1 and best value is 0.7 → keep exploring.
    let convergence = resp.convergence.expect("convergence surfaced");
    assert!(!convergence.converged);
    assert!(convergence.reason.contains("keep exploring"));

    // The verification + convergence outcomes are recorded into metrics for
    // transparency, not just returned in the response.
    let mcts_metrics = server
        .state
        .metrics
        .summary()
        .by_mode
        .remove("mcts")
        .expect("mcts metrics recorded");
    assert_eq!(mcts_metrics.verifications, 1);
    assert_eq!(mcts_metrics.verification_failures, 0);
    assert_eq!(mcts_metrics.convergence_checks, 1);
    assert_eq!(mcts_metrics.converged, 0);
}

/// Explore body where the top node dominates the runner-up by 0.45 UCB1.
fn mcts_explore_dominant() -> String {
    serde_json::json!({
        "frontier_evaluation": [
            {"node_id": "a", "visits": 8, "average_value": 0.7, "exploration_bonus": 0.25, "ucb1_score": 0.95},
            {"node_id": "b", "visits": 5, "average_value": 0.3, "exploration_bonus": 0.2, "ucb1_score": 0.5}
        ],
        "selected_node": {"node_id": "a", "selection_reason": "Highest UCB1 (0.95)"},
        "expansion": {"new_nodes": [
            {"id": "a1", "content": "Deepen option a", "simulated_value": 0.7}
        ]},
        "backpropagation": {"updated_nodes": ["a", "root"], "value_changes": {"a": 0.1, "root": 0.02}},
        "search_status": {"total_nodes": 6, "total_simulations": 30, "best_path_value": 0.7}
    })
    .to_string()
}

#[tokio::test]
async fn test_mcts_explore_streaming_fast_mode_succeeds() {
    let mock_server = MockServer::start().await;
    mount_sse(&mock_server, &mcts_explore_consistent()).await;
    let server = create_mocked_server(&mock_server).await;

    // A "standard" thinking budget still drives the success arm end-to-end.
    let mut req = mcts_req();
    req.thinking = Some("standard".to_string());
    let resp = server.reasoning_mcts(Parameters(req)).await;
    assert_eq!(resp.frontier.expect("frontier surfaced").len(), 2);
}

#[tokio::test]
async fn test_mcts_explore_streaming_converges_on_dominant_candidate() {
    let mock_server = MockServer::start().await;
    mount_sse(&mock_server, &mcts_explore_dominant()).await;
    let server = create_mocked_server(&mock_server).await;

    let resp = server.reasoning_mcts(Parameters(mcts_req())).await;
    let convergence = resp.convergence.expect("convergence surfaced");
    assert!(convergence.converged);
    assert!((convergence.top_gap - 0.45).abs() < 1e-9);
    assert!(convergence.reason.contains("leads the runner-up"));
}

#[tokio::test]
async fn test_mcts_explore_streaming_flags_incoherent_backprop() {
    let mock_server = MockServer::start().await;
    mount_sse(&mock_server, &mcts_explore_backprop_incoherent()).await;
    let server = create_mocked_server(&mock_server).await;

    let resp = server.reasoning_mcts(Parameters(mcts_req())).await;
    let validation = resp.validation.expect("validation present");
    assert!(!validation.consistent);
    assert!(validation
        .warnings
        .iter()
        .any(|w| w.contains("does not include the selected node")));
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

/// A consistent backtrack response: declining trend over recent values with a
/// decline magnitude that matches the peak-to-trough range.
fn mcts_backtrack_consistent() -> String {
    serde_json::json!({
        "quality_assessment": {
            "recent_values": [0.7, 0.65, 0.5, 0.4],
            "trend": "declining",
            "decline_magnitude": 0.3
        },
        "backtrack_decision": {
            "should_backtrack": true,
            "reason": "Sustained quality decline",
            "backtrack_to": "node_3",
            "depth_reduction": 2
        },
        "alternative_actions": [
            {"action": "prune", "rationale": "Remove low-value branches"}
        ],
        "recommendation": {
            "action": "backtrack",
            "confidence": 0.8,
            "expected_benefit": "Recover from local minimum"
        }
    })
    .to_string()
}

/// A backtrack response whose current quality (0.4) sits below a 0.5 floor yet
/// declines to backtrack — the false-negative the quality_threshold check flags.
fn mcts_backtrack_ignores_threshold() -> String {
    serde_json::json!({
        "quality_assessment": {
            "recent_values": [0.8, 0.6, 0.4],
            "trend": "declining",
            "decline_magnitude": 0.4
        },
        "backtrack_decision": {
            "should_backtrack": false,
            "reason": "Model claims the dip is recoverable"
        },
        "alternative_actions": [
            {"action": "continue", "rationale": "Push on"}
        ],
        "recommendation": {
            "action": "continue",
            "confidence": 0.6,
            "expected_benefit": "Avoid losing progress"
        }
    })
    .to_string()
}

/// End-to-end: a backtrack decision that ignores the caller's `quality_threshold`
/// surfaces an inconsistency warning through the handler's validation.
#[tokio::test]
async fn test_mcts_auto_backtrack_flags_ignored_quality_threshold() {
    let mock_server = MockServer::start().await;
    mount_sse(&mock_server, &mcts_backtrack_ignores_threshold()).await;
    let server = create_mocked_server(&mock_server).await;

    let mut req = mcts_req();
    req.operation = Some("auto_backtrack".to_string());
    req.quality_threshold = Some(0.5);
    let resp = server.reasoning_mcts(Parameters(req)).await;

    let validation = resp.validation.expect("validation present");
    assert!(!validation.consistent);
    assert!(validation
        .warnings
        .iter()
        .any(|w| w.contains("below the requested quality_threshold")));
}

/// The explore arm only responds when the outbound prompt carries the
/// caller-supplied `exploration_constant`. A missing injection would leave no
/// matching mock, dropping the handler to the error fallback (`frontier == None`)
/// and failing the `expect` — so a passing test proves the param reached the model.
#[tokio::test]
async fn test_mcts_explore_streaming_injects_exploration_constant() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(body_string_contains("exploration constant C = 2"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(anthropic_sse_response(&mcts_explore_consistent())),
        )
        .mount(&mock_server)
        .await;
    let server = create_mocked_server(&mock_server).await;

    let mut req = mcts_req();
    req.exploration_constant = Some(2.0);
    let resp = server.reasoning_mcts(Parameters(req)).await;

    // Success arm reached => the param-bearing prompt matched the mock.
    let frontier = resp.frontier.expect("frontier surfaced (param injected)");
    assert_eq!(frontier.len(), 2);
}

/// Same end-to-end proof for the auto_backtrack arm and `quality_threshold`.
#[tokio::test]
async fn test_mcts_auto_backtrack_streaming_injects_quality_threshold() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(body_string_contains("below 0.3"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(anthropic_sse_response(&mcts_backtrack_consistent())),
        )
        .mount(&mock_server)
        .await;
    let server = create_mocked_server(&mock_server).await;

    let mut req = mcts_req();
    req.operation = Some("auto_backtrack".to_string());
    req.quality_threshold = Some(0.3);
    let resp = server.reasoning_mcts(Parameters(req)).await;

    let suggestion = resp
        .backtrack_suggestion
        .expect("backtrack suggestion surfaced (param injected)");
    assert!(suggestion.should_backtrack);
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
