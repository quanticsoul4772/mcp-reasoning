use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::*;

#[tokio::test]
async fn test_timeline_all_operations() {
    let mock_server = MockServer::start().await;

    let create_json = serde_json::json!({
        "timeline_id": "tl1",
        "events": [
            {"timestamp": "t1", "event": "Event 1", "significance": "high"}
        ],
        "analysis": "Timeline analysis"
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
    let create_req = TimelineRequest {
        operation: "create".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: None,
        content: Some("Timeline content".to_string()),
        label: Some("main".to_string()),
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(create_req)).await;
    let _ = resp.timeline_id;

    // Test branch
    let branch_req = TimelineRequest {
        operation: "branch".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: Some("tl1".to_string()),
        content: Some("Branch content".to_string()),
        label: Some("alternative".to_string()),
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(branch_req)).await;
    let _ = resp.timeline_id;

    // Test compare
    let compare_req = TimelineRequest {
        operation: "compare".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: Some("tl1".to_string()),
        content: None,
        label: None,
        branch_ids: Some(vec!["b1".to_string(), "b2".to_string()]),
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(compare_req)).await;
    let _ = resp.timeline_id;

    // Test merge
    let merge_req = TimelineRequest {
        operation: "merge".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: Some("tl1".to_string()),
        content: None,
        label: None,
        branch_ids: None,
        source_branch_id: Some("b1".to_string()),
        target_branch_id: Some("b2".to_string()),
        merge_strategy: Some("integrate".to_string()),
    };
    let resp = server.reasoning_timeline(Parameters(merge_req)).await;
    let _ = resp.timeline_id;

    // Test unknown operation
    let unknown_req = TimelineRequest {
        operation: "unknown".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: None,
        content: None,
        label: None,
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(unknown_req)).await;
    // Should have error in some field
    let _ = resp.timeline_id;
}

#[tokio::test]
async fn test_mcts_all_operations() {
    let mock_server = MockServer::start().await;

    let explore_json = serde_json::json!({
        "best_path": [
            {"node_id": "n1", "content": "Step 1", "visits": 10, "ucb_score": 1.5}
        ],
        "iterations_completed": 50,
        "frontier_evaluation": [
            {"node_id": "n2", "score": 0.8}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&explore_json.to_string())),
        )
        .expect(1..)
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    // Test explore
    let explore_req = MctsRequest {
        operation: Some("explore".to_string()),
        content: Some("Problem to search".to_string()),
        session_id: Some("s1".to_string()),
        node_id: None,
        iterations: Some(50),
        exploration_constant: Some(1.41),
        simulation_depth: Some(5),
        quality_threshold: Some(0.7),
        lookback_depth: Some(3),
        auto_execute: Some(false),
        progress_token: None,
    };
    let resp = server.reasoning_mcts(Parameters(explore_req)).await;
    assert_eq!(resp.session_id, "s1");

    // Test auto_backtrack
    let backtrack_json = serde_json::json!({
        "should_backtrack": true,
        "target_step": 2,
        "reason": "Quality dropped",
        "quality_drop": 0.2
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&backtrack_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let backtrack_req = MctsRequest {
        operation: Some("auto_backtrack".to_string()),
        content: None,
        session_id: Some("s1".to_string()),
        node_id: None,
        iterations: None,
        exploration_constant: None,
        simulation_depth: None,
        quality_threshold: Some(0.7),
        lookback_depth: Some(3),
        auto_execute: Some(true),
        progress_token: None,
    };
    let resp = server.reasoning_mcts(Parameters(backtrack_req)).await;
    assert_eq!(resp.session_id, "s1");

    // Test unknown operation (defaults to explore)
    let default_req = MctsRequest {
        operation: None,
        content: Some("Content".to_string()),
        session_id: Some("s1".to_string()),
        node_id: None,
        iterations: None,
        exploration_constant: None,
        simulation_depth: None,
        quality_threshold: None,
        lookback_depth: None,
        auto_execute: None,
        progress_token: None,
    };
    let resp = server.reasoning_mcts(Parameters(default_req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_counterfactual_success_path() {
    let mock_server = MockServer::start().await;

    let response_json = serde_json::json!({
        "causal_model": {
            "nodes": ["A", "B", "C"],
            "edges": [
                {"from": "A", "to": "B", "edge_type": "causes", "strength": "strong"}
            ]
        },
        "ladder_rung": "intervention",
        "causal_chain": [
            {"step": 1, "cause": "Intervention", "effect": "Changed outcome", "probability": 0.8}
        ],
        "counterfactual_outcome": "Different result",
        "key_differences": ["Difference 1"],
        "confidence": 0.85,
        "assumptions": ["Assumption 1"]
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
    let req = CounterfactualRequest {
        scenario: "Original scenario".to_string(),
        intervention: "What if X changed?".to_string(),
        analysis_depth: Some("counterfactual".to_string()),
        session_id: Some("s1".to_string()),
        progress_token: None,
    };

    let resp = server.reasoning_counterfactual(Parameters(req)).await;
    assert_eq!(resp.original_scenario, "Original scenario");
    assert_eq!(resp.intervention_applied, "What if X changed?");
}

#[tokio::test]
async fn test_preset_all_operations() {
    let mock_server = MockServer::start().await;

    // Presets don't require API calls
    let server = create_mocked_server(&mock_server).await;

    // Test list
    let list_req = PresetRequest {
        operation: "list".to_string(),
        preset_id: None,
        category: Some("analysis".to_string()),
        inputs: None,
        session_id: None,
    };
    let resp = server.reasoning_preset(Parameters(list_req)).await;
    assert!(resp.presets.is_some());

    // Test run (will fail without valid preset but exercises code)
    let run_req = PresetRequest {
        operation: "run".to_string(),
        preset_id: Some("quick_analysis".to_string()),
        category: None,
        inputs: Some(serde_json::json!({"content": "Test content"})),
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_preset(Parameters(run_req)).await;
    // Either has execution result or presets
    let _ = resp.execution_result;

    // Test unknown operation
    let unknown_req = PresetRequest {
        operation: "unknown".to_string(),
        preset_id: None,
        category: None,
        inputs: None,
        session_id: None,
    };
    let resp = server.reasoning_preset(Parameters(unknown_req)).await;
    let _ = resp.presets;
}
