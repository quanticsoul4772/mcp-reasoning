// Tests for alternate operation types on decision, evidence, temporal, graph, and detect handlers.
// These exercise uncovered match arms to maximize code coverage.
use rmcp::handler::server::wrapper::Parameters;

use super::create_test_server;
use crate::server::requests::*;

// ============================================================================
// Decision Type Variants (handlers_decision.rs)
// ============================================================================

#[tokio::test]
async fn test_decision_pairwise_type() {
    let server = create_test_server().await;
    let req = DecisionRequest {
        decision_type: Some("pairwise".to_string()),
        question: Some("A or B?".to_string()),
        options: Some(vec!["A".to_string(), "B".to_string()]),
        topic: None,
        context: None,
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_decision(Parameters(req)).await;
    let _ = resp.recommendation;
}

#[tokio::test]
async fn test_decision_topsis_type() {
    let server = create_test_server().await;
    let req = DecisionRequest {
        decision_type: Some("topsis".to_string()),
        question: Some("rank options".to_string()),
        options: Some(vec!["X".to_string(), "Y".to_string(), "Z".to_string()]),
        topic: None,
        context: Some("evaluation context".to_string()),
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_decision(Parameters(req)).await;
    let _ = resp.recommendation;
}

#[tokio::test]
async fn test_decision_perspectives_type() {
    let server = create_test_server().await;
    let req = DecisionRequest {
        decision_type: Some("perspectives".to_string()),
        question: None,
        options: None,
        topic: Some("stakeholder analysis".to_string()),
        context: Some("project decision".to_string()),
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_decision(Parameters(req)).await;
    let _ = resp.recommendation;
}

#[tokio::test]
async fn test_decision_unknown_type() {
    let server = create_test_server().await;
    let req = DecisionRequest {
        decision_type: Some("invalid".to_string()),
        question: Some("test".to_string()),
        options: None,
        topic: None,
        context: None,
        session_id: None,
    };
    let resp = server.reasoning_decision(Parameters(req)).await;
    assert!(resp.recommendation.contains("ERROR"));
}

#[tokio::test]
async fn test_decision_no_options() {
    let server = create_test_server().await;
    let req = DecisionRequest {
        decision_type: Some("weighted".to_string()),
        question: Some("test question".to_string()),
        options: Some(vec![]),
        topic: None,
        context: None,
        session_id: None,
    };
    let resp = server.reasoning_decision(Parameters(req)).await;
    let _ = resp.recommendation;
}

// ============================================================================
// Evidence Type Variants (handlers_decision.rs)
// ============================================================================

#[tokio::test]
async fn test_evidence_probabilistic_type() {
    let server = create_test_server().await;
    let req = EvidenceRequest {
        evidence_type: Some("probabilistic".to_string()),
        claim: None,
        hypothesis: Some("hypothesis A".to_string()),
        context: Some("test context".to_string()),
        prior: Some(0.5),
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_evidence(Parameters(req)).await;
    let _ = resp.overall_credibility;
}

#[tokio::test]
async fn test_evidence_unknown_type() {
    let server = create_test_server().await;
    let req = EvidenceRequest {
        evidence_type: Some("invalid".to_string()),
        claim: Some("test".to_string()),
        hypothesis: None,
        context: None,
        prior: None,
        session_id: None,
    };
    let resp = server.reasoning_evidence(Parameters(req)).await;
    assert!(resp.synthesis.unwrap().contains("Unknown"));
}

// ============================================================================
// Timeline Operations (handlers_temporal.rs)
// ============================================================================

#[tokio::test]
async fn test_timeline_branch_operation() {
    let server = create_test_server().await;
    let req = TimelineRequest {
        operation: "branch".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: None,
        content: Some("branching point".to_string()),
        label: Some("alt".to_string()),
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(req)).await;
    let _ = resp.timeline_id;
}

#[tokio::test]
async fn test_timeline_compare_operation() {
    let server = create_test_server().await;
    let req = TimelineRequest {
        operation: "compare".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: None,
        content: Some("compare branches".to_string()),
        label: None,
        branch_ids: Some(vec!["b1".to_string(), "b2".to_string()]),
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(req)).await;
    let _ = resp.timeline_id;
}

#[tokio::test]
async fn test_timeline_merge_operation() {
    let server = create_test_server().await;
    let req = TimelineRequest {
        operation: "merge".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: None,
        content: Some("merge content".to_string()),
        label: None,
        branch_ids: None,
        source_branch_id: Some("src".to_string()),
        target_branch_id: Some("tgt".to_string()),
        merge_strategy: Some("best_of_both".to_string()),
    };
    let resp = server.reasoning_timeline(Parameters(req)).await;
    let _ = resp.timeline_id;
}

#[tokio::test]
async fn test_timeline_unknown_operation() {
    let server = create_test_server().await;
    let req = TimelineRequest {
        operation: "invalid".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: None,
        content: None,
        label: None,
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(req)).await;
    assert!(resp.timeline_id.contains("Unknown operation"));
}

// ============================================================================
// MCTS Operations (handlers_temporal.rs)
// ============================================================================

#[tokio::test]
async fn test_mcts_auto_backtrack_operation() {
    let server = create_test_server().await;
    let req = MctsRequest {
        operation: Some("auto_backtrack".to_string()),
        content: Some("test content".to_string()),
        session_id: Some("s1".to_string()),
        node_id: None,
        iterations: None,
        exploration_constant: None,
        simulation_depth: None,
        quality_threshold: Some(0.7),
        lookback_depth: Some(3),
        auto_execute: Some(false),
        progress_token: None,
    };
    let resp = server.reasoning_mcts(Parameters(req)).await;
    let _ = resp.session_id;
}

#[tokio::test]
async fn test_mcts_unknown_operation() {
    let server = create_test_server().await;
    let req = MctsRequest {
        operation: Some("invalid".to_string()),
        content: Some("test".to_string()),
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
    let resp = server.reasoning_mcts(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

// ============================================================================
// Graph Operations (handlers_graph.rs)
// ============================================================================

#[tokio::test]
async fn test_graph_generate_operation() {
    let server = create_test_server().await;
    let req = GraphRequest {
        operation: "generate".to_string(),
        session_id: "s1".to_string(),
        content: Some("generate nodes".to_string()),
        problem: None,
        node_id: Some("root".to_string()),
        node_ids: None,
        k: Some(3),
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_score_operation() {
    let server = create_test_server().await;
    let req = GraphRequest {
        operation: "score".to_string(),
        session_id: "s1".to_string(),
        content: Some("score this".to_string()),
        problem: None,
        node_id: Some("node-1".to_string()),
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_aggregate_operation() {
    let server = create_test_server().await;
    let req = GraphRequest {
        operation: "aggregate".to_string(),
        session_id: "s1".to_string(),
        content: Some("aggregate".to_string()),
        problem: None,
        node_id: None,
        node_ids: Some(vec!["n1".to_string(), "n2".to_string()]),
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_refine_operation() {
    let server = create_test_server().await;
    let req = GraphRequest {
        operation: "refine".to_string(),
        session_id: "s1".to_string(),
        content: Some("refine this".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_prune_operation() {
    let server = create_test_server().await;
    let req = GraphRequest {
        operation: "prune".to_string(),
        session_id: "s1".to_string(),
        content: Some("prune low quality".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: Some(0.5),
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_finalize_operation() {
    let server = create_test_server().await;
    let req = GraphRequest {
        operation: "finalize".to_string(),
        session_id: "s1".to_string(),
        content: Some("finalize".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: Some(vec!["t1".to_string()]),
    };
    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_state_operation() {
    let server = create_test_server().await;
    let req = GraphRequest {
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
    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_unknown_operation() {
    let server = create_test_server().await;
    let req = GraphRequest {
        operation: "invalid".to_string(),
        session_id: "s1".to_string(),
        content: None,
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(req)).await;
    assert!(resp.aggregated_insight.unwrap().contains("failed"));
}

// ============================================================================
// Detect Operations (handlers_graph.rs)
// ============================================================================

#[tokio::test]
async fn test_detect_fallacies_type() {
    let server = create_test_server().await;
    let req = DetectRequest {
        detect_type: "fallacies".to_string(),
        content: Some("This is a bad argument because I said so".to_string()),
        thought_id: None,
        session_id: Some("s1".to_string()),
        check_types: None,
        check_formal: Some(true),
        check_informal: Some(true),
    };
    let resp = server.reasoning_detect(Parameters(req)).await;
    let _ = resp.detections;
}

#[tokio::test]
async fn test_detect_unknown_type() {
    let server = create_test_server().await;
    let req = DetectRequest {
        detect_type: "invalid".to_string(),
        content: Some("test".to_string()),
        thought_id: None,
        session_id: None,
        check_types: None,
        check_formal: None,
        check_informal: None,
    };
    let resp = server.reasoning_detect(Parameters(req)).await;
    assert!(resp.summary.unwrap().contains("Unknown"));
}
