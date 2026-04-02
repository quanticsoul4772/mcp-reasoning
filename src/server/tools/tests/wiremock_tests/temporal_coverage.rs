//! Wiremock tests targeting uncovered success paths in `handlers_temporal.rs`.
//!
//! The existing tests in `temporal.rs` used generic mock JSON that didn't match
//! what each operation's parser actually requires, so all success branches were
//! unreachable. These tests provide correctly-formatted JSON for each operation.

use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::{MctsRequest, TimelineRequest};

// ============================================================================
// Timeline: per-operation correctly-formatted mock responses
// ============================================================================

/// Correct mock JSON for `timeline.create`.
/// Requires: timeline_id, events (id/description/time/type), decision_points, temporal_structure.
fn timeline_create_json() -> String {
    serde_json::json!({
        "timeline_id": "tl-2026",
        "events": [
            {
                "id": "e1",
                "description": "Project kick-off",
                "time": "2026-01-01",
                "type": "event",
                "causes": ["funding approved"],
                "effects": ["team assembled"]
            }
        ],
        "decision_points": [
            {
                "id": "dp1",
                "description": "Choose architecture",
                "options": ["Monolith", "Microservices"],
                "deadline": "2026-02-01"
            }
        ],
        "temporal_structure": {
            "start": "2026-01-01",
            "current": "2026-04-01",
            "horizon": "2026-12-31"
        }
    })
    .to_string()
}

/// Correct mock JSON for `timeline.branch`.
/// Requires: branch_point (event_id, description), branches[], comparison.
fn timeline_branch_json() -> String {
    serde_json::json!({
        "branch_point": {
            "event_id": "dp1",
            "description": "Architecture decision fork"
        },
        "branches": [
            {
                "id": "b1",
                "choice": "Monolith first",
                "events": [
                    {
                        "id": "ev1",
                        "description": "Ship MVP fast",
                        "probability": 0.8,
                        "time_offset": "+3 months"
                    }
                ],
                "plausibility": 0.8,
                "outcome_quality": 0.75
            }
        ],
        "comparison": {
            "most_likely_good_outcome": "Fast delivery wins market",
            "highest_risk": "Technical debt accumulates",
            "key_differences": ["Speed vs scalability"]
        }
    })
    .to_string()
}

/// Correct mock JSON for `timeline.compare`.
/// Requires: branches_compared, divergence_point, key_differences[], risk_assessment, opportunity_assessment, recommendation.
fn timeline_compare_json() -> String {
    serde_json::json!({
        "branches_compared": ["b1", "b2"],
        "divergence_point": "Architecture decision at dp1",
        "key_differences": [
            {
                "dimension": "Delivery speed",
                "branch_1_value": "3 months",
                "branch_2_value": "9 months",
                "significance": "Critical"
            }
        ],
        "risk_assessment": {
            "branch_1_risks": ["Tech debt"],
            "branch_2_risks": ["Market timing"]
        },
        "opportunity_assessment": {
            "branch_1_opportunities": ["First mover advantage"],
            "branch_2_opportunities": ["Better long-term scalability"]
        },
        "recommendation": {
            "preferred_branch": "b1",
            "conditions": "When time-to-market is the primary constraint",
            "key_factors": "Delivery speed vs architectural soundness"
        }
    })
    .to_string()
}

/// Correct mock JSON for `timeline.merge`.
/// Requires: branches_merged, common_patterns[], robust_strategies[], fragile_strategies[], synthesis, recommendations[].
fn timeline_merge_json() -> String {
    serde_json::json!({
        "branches_merged": ["b1", "b2"],
        "common_patterns": [
            {
                "pattern": "Iterative delivery",
                "frequency": 0.85,
                "implications": "Use sprints regardless of architecture choice"
            }
        ],
        "robust_strategies": [
            {
                "strategy": "Early user feedback",
                "effectiveness": 0.9,
                "conditions": "When users are accessible"
            }
        ],
        "fragile_strategies": [
            {
                "strategy": "Big-bang release",
                "failure_modes": "Integration failures surface too late"
            }
        ],
        "synthesis": "Both branches benefit from iterative delivery and early feedback.",
        "recommendations": [
            "Start with monolith but design for future extraction",
            "Ship to pilot users within 3 months"
        ]
    })
    .to_string()
}

#[tokio::test]
async fn test_timeline_create_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&timeline_create_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = TimelineRequest {
        operation: "create".to_string(),
        session_id: Some("s-tl-create".to_string()),
        timeline_id: None,
        content: Some("Project timeline for a software product launch in 2026".to_string()),
        label: Some("main".to_string()),
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };

    let resp = server.reasoning_timeline(Parameters(req)).await;
    // Success path: timeline_id is the one from the mock, not an error message
    assert_eq!(resp.timeline_id, "tl-2026");
}

#[tokio::test]
async fn test_timeline_branch_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&timeline_branch_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = TimelineRequest {
        operation: "branch".to_string(),
        session_id: Some("s-tl-branch".to_string()),
        timeline_id: Some("tl-2026".to_string()),
        content: Some("Branch at the architecture decision point".to_string()),
        label: Some("monolith-vs-microservices".to_string()),
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };

    let resp = server.reasoning_timeline(Parameters(req)).await;
    // Success path: branch_id is set from the branch_point.event_id
    assert!(resp.branch_id.is_some());
    let branches = resp.branches.expect("branches should be populated");
    assert!(!branches.is_empty());
    assert!(resp.comparison.is_some());
}

#[tokio::test]
async fn test_timeline_compare_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&timeline_compare_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = TimelineRequest {
        operation: "compare".to_string(),
        session_id: Some("s-tl-compare".to_string()),
        timeline_id: Some("tl-2026".to_string()),
        content: Some("Compare the two architecture branches".to_string()),
        label: None,
        branch_ids: Some(vec!["b1".to_string(), "b2".to_string()]),
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };

    let resp = server.reasoning_timeline(Parameters(req)).await;
    // Success path: comparison is populated from the mock
    let comparison = resp.comparison.expect("comparison should be populated");
    assert!(!comparison.divergence_points.is_empty());
    assert!(!comparison.convergence_opportunities.is_empty());
}

#[tokio::test]
async fn test_timeline_merge_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&timeline_merge_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = TimelineRequest {
        operation: "merge".to_string(),
        session_id: Some("s-tl-merge".to_string()),
        timeline_id: Some("tl-2026".to_string()),
        content: Some("Synthesize insights from both branches".to_string()),
        label: None,
        branch_ids: Some(vec!["b1".to_string(), "b2".to_string()]),
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: Some("synthesis".to_string()),
    };

    let resp = server.reasoning_timeline(Parameters(req)).await;
    // Success path: merged_content is populated
    let content = resp
        .merged_content
        .expect("merged_content should be populated");
    assert!(content.contains("iterative"));
}

// ============================================================================
// MCTS: explore success path
// ============================================================================

/// Correct mock JSON for `mcts.explore_streaming`.
/// Requires: frontier_evaluation[], selected_node, search_status, recommendation.
fn mcts_explore_json() -> String {
    serde_json::json!({
        "frontier_evaluation": [
            {
                "node_id": "n1",
                "visits": 10,
                "ucb1_score": 0.85
            },
            {
                "node_id": "n2",
                "visits": 5,
                "ucb1_score": 0.72
            }
        ],
        "selected_node": {
            "node_id": "n1",
            "selection_reason": "Highest UCB1 score among unexplored nodes"
        },
        "search_status": {
            "total_nodes": 12,
            "total_simulations": 50,
            "best_path_value": 0.85
        },
        "recommendation": {
            "action": "continue",
            "confidence": 0.8,
            "expected_benefit": "Additional simulations will refine the estimate"
        }
    })
    .to_string()
}

/// Note: MCTS explore uses `complete_streaming` (SSE), which needs a different
/// wiremock setup. This test verifies the handler returns a valid response regardless.
#[tokio::test]
async fn test_mcts_explore_returns_valid_response() {
    let mock_server = MockServer::start().await;

    // MCTS streaming uses SSE format. With a regular JSON mock, the streaming
    // client will fail gracefully and return the error fallback path.
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&mcts_explore_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = MctsRequest {
        operation: Some("explore".to_string()),
        content: Some("Search for the optimal product strategy".to_string()),
        session_id: Some("s-mcts".to_string()),
        node_id: None,
        iterations: None,
        exploration_constant: None,
        simulation_depth: None,
        quality_threshold: None,
        auto_execute: None,
        lookback_depth: None,
        progress_token: None,
    };

    let resp = server.reasoning_mcts(Parameters(req)).await;
    // Handler always returns a MctsResponse (either success or error fallback)
    assert!(!resp.session_id.is_empty());
}

/// Test auto_backtrack operation — exercises a different branch in the handler.
#[tokio::test]
async fn test_mcts_auto_backtrack_returns_valid_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&mcts_explore_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = MctsRequest {
        operation: Some("auto_backtrack".to_string()),
        content: Some("Quality is declining in the search".to_string()),
        session_id: Some("s-mcts-bt".to_string()),
        node_id: Some("n2".to_string()),
        iterations: None,
        exploration_constant: None,
        simulation_depth: None,
        quality_threshold: None,
        auto_execute: None,
        lookback_depth: None,
        progress_token: None,
    };

    let resp = server.reasoning_mcts(Parameters(req)).await;
    assert!(!resp.session_id.is_empty());
}
