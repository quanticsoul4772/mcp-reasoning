use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::*;

#[tokio::test]
async fn test_decision_topsis_and_perspectives() {
    let mock_server = MockServer::start().await;

    // Test topsis
    let topsis_json = serde_json::json!({
        "criteria": [
            {"name": "Cost", "weight": 0.4, "type": "cost"},
            {"name": "Quality", "weight": 0.6, "type": "benefit"}
        ],
        "normalized_matrix": [[0.8, 0.9], [0.6, 0.7]],
        "weighted_matrix": [[0.32, 0.54], [0.24, 0.42]],
        "ideal_positive": [0.24, 0.54],
        "ideal_negative": [0.32, 0.42],
        "distance_positive": {"A": 0.1, "B": 0.2},
        "distance_negative": {"A": 0.2, "B": 0.1},
        "relative_closeness": {"A": 0.67, "B": 0.33},
        "ranking": [
            {"option": "A", "closeness": 0.67, "rank": 1},
            {"option": "B", "closeness": 0.33, "rank": 2}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&topsis_json.to_string())),
        )
        .expect(1..)
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let topsis_req = DecisionRequest {
        decision_type: Some("topsis".to_string()),
        question: Some("Which option using TOPSIS?".to_string()),
        options: Some(vec!["A".to_string(), "B".to_string()]),
        topic: None,
        context: None,
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_decision(Parameters(topsis_req)).await;
    let _ = resp.recommendation;

    // Test perspectives
    let perspectives_json = serde_json::json!({
        "stakeholders": [
            {"name": "Customer", "perspective": "Quality focus", "interests": ["Quality"], "concerns": ["Price"], "influence_level": "high"},
            {"name": "Developer", "perspective": "Tech focus", "interests": ["Simplicity"], "concerns": ["Complexity"], "influence_level": "medium"},
            {"name": "Manager", "perspective": "Cost focus", "interests": ["Budget"], "concerns": ["Overruns"], "influence_level": "low"}
        ],
        "conflicts": [
            {"parties": ["Customer", "Manager"], "issue": "Budget vs quality", "severity": "medium", "resolution_approach": "Compromise"}
        ],
        "alignments": [
            {"parties": ["Customer", "Developer"], "common_ground": "User experience", "leverage_opportunity": "Focus on UX"}
        ],
        "balanced_recommendation": {
            "option": "Option A",
            "rationale": "Best balance of interests",
            "trade_offs": ["Some cost increase"]
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&perspectives_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let perspectives_req = DecisionRequest {
        decision_type: Some("perspectives".to_string()),
        question: None,
        options: None,
        topic: Some("Project stakeholder analysis".to_string()),
        context: None,
        session_id: Some("s1".to_string()),
    };
    let resp = server
        .reasoning_decision(Parameters(perspectives_req))
        .await;
    assert!(resp.stakeholder_map.is_some() || !resp.recommendation.is_empty());

    // Test unknown decision type
    let unknown_req = DecisionRequest {
        decision_type: Some("unknown_type".to_string()),
        question: Some("Question".to_string()),
        options: None,
        topic: None,
        context: None,
        session_id: None,
    };
    let resp = server.reasoning_decision(Parameters(unknown_req)).await;
    assert!(resp.recommendation.contains("ERROR") || resp.recommendation.contains("unknown"));
}

#[tokio::test]
async fn test_graph_score_operation() {
    let mock_server = MockServer::start().await;

    let score_json = serde_json::json!({
        "node_id": "n1",
        "score": 0.85,
        "factors": {"coherence": 0.9, "novelty": 0.8}
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&score_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let score_req = GraphRequest {
        operation: "score".to_string(),
        session_id: "s1".to_string(),
        content: Some("Evaluate this node".to_string()),
        problem: None,
        node_id: Some("n1".to_string()),
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(score_req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_aggregate_operation() {
    let mock_server = MockServer::start().await;

    let aggregate_json = serde_json::json!({
        "synthesis": {"content": "Combined insight from multiple nodes", "confidence": 0.8}
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&aggregate_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let aggregate_req = GraphRequest {
        operation: "aggregate".to_string(),
        session_id: "s1".to_string(),
        content: Some("Aggregate these insights".to_string()),
        problem: None,
        node_id: None,
        node_ids: Some(vec!["n1".to_string(), "n2".to_string()]),
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(aggregate_req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_refine_operation() {
    let mock_server = MockServer::start().await;

    let refine_json = serde_json::json!({
        "refined_node": {"id": "n1_refined", "content": "Improved reasoning", "score": 0.9}
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&refine_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let refine_req = GraphRequest {
        operation: "refine".to_string(),
        session_id: "s1".to_string(),
        content: Some("Refine this node".to_string()),
        problem: None,
        node_id: Some("n1".to_string()),
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(refine_req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_prune_operation() {
    let mock_server = MockServer::start().await;

    let prune_json = serde_json::json!({
        "prune_candidates": [
            {"id": "n3", "reason": "Low score", "score": 0.2},
            {"id": "n4", "reason": "Redundant", "score": 0.3}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&prune_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let prune_req = GraphRequest {
        operation: "prune".to_string(),
        session_id: "s1".to_string(),
        content: Some("Prune low value nodes".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: Some(0.5),
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(prune_req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_finalize_operation() {
    let mock_server = MockServer::start().await;

    let finalize_json = serde_json::json!({
        "conclusions": [
            {"conclusion": "Main finding 1", "confidence": 0.9},
            {"conclusion": "Main finding 2", "confidence": 0.85}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&finalize_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let finalize_req = GraphRequest {
        operation: "finalize".to_string(),
        session_id: "s1".to_string(),
        content: Some("Generate final conclusions".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: Some(vec!["n1".to_string(), "n2".to_string()]),
    };
    let resp = server.reasoning_graph(Parameters(finalize_req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_state_operation() {
    let mock_server = MockServer::start().await;

    let state_json = serde_json::json!({
        "structure": {
            "total_nodes": 10,
            "depth": 3,
            "pruned_count": 2,
            "active_branches": 4
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&state_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let state_req = GraphRequest {
        operation: "state".to_string(),
        session_id: "s1".to_string(),
        content: None,
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(state_req)).await;
    assert_eq!(resp.session_id, "s1");
}
