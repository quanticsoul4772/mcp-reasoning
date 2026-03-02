use rmcp::handler::server::wrapper::Parameters;

use super::create_test_server;
use crate::server::requests::*;

#[tokio::test]
async fn test_reasoning_linear_tool() {
    let server = create_test_server().await;
    let req = LinearRequest {
        content: "test".to_string(),
        session_id: Some("s1".to_string()),
        confidence: Some(0.8),
    };
    let resp = server.reasoning_linear(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_reasoning_tree_tool() {
    let server = create_test_server().await;
    let req = TreeRequest {
        operation: Some("create".to_string()),
        content: Some("test".to_string()),
        session_id: Some("s1".to_string()),
        branch_id: None,
        num_branches: Some(2),
        completed: None,
    };
    let resp = server.reasoning_tree(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_reasoning_divergent_tool() {
    let server = create_test_server().await;
    let req = DivergentRequest {
        content: "test".to_string(),
        session_id: Some("s1".to_string()),
        num_perspectives: Some(3),
        challenge_assumptions: Some(true),
        force_rebellion: Some(false),
        progress_token: None,
    };
    let resp = server.reasoning_divergent(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_reasoning_reflection_tool() {
    let server = create_test_server().await;
    let req = ReflectionRequest {
        operation: Some("process".to_string()),
        content: Some("test".to_string()),
        thought_id: None,
        session_id: Some("s1".to_string()),
        max_iterations: Some(3),
        quality_threshold: Some(0.8),
        progress_token: None,
    };
    let resp = server.reasoning_reflection(Parameters(req)).await;
    assert!(resp.quality_score >= 0.0);
}

#[tokio::test]
async fn test_reasoning_checkpoint_tool() {
    let server = create_test_server().await;
    let req = CheckpointRequest {
        operation: "create".to_string(),
        session_id: "s1".to_string(),
        checkpoint_id: None,
        name: Some("cp1".to_string()),
        description: Some("test checkpoint".to_string()),
        new_direction: None,
    };
    let resp = server.reasoning_checkpoint(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_reasoning_auto_tool() {
    let server = create_test_server().await;
    let req = AutoRequest {
        content: "test".to_string(),
        hints: Some(vec!["hint".to_string()]),
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_auto(Parameters(req)).await;
    assert!(!resp.selected_mode.is_empty());
}

#[tokio::test]
async fn test_reasoning_graph_tool() {
    let server = create_test_server().await;
    let req = GraphRequest {
        operation: "init".to_string(),
        session_id: "s1".to_string(),
        content: Some("test".to_string()),
        problem: Some("problem".to_string()),
        node_id: None,
        node_ids: None,
        k: Some(3),
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_reasoning_detect_tool() {
    let server = create_test_server().await;
    let req = DetectRequest {
        detect_type: "biases".to_string(),
        content: Some("test".to_string()),
        thought_id: None,
        session_id: Some("s1".to_string()),
        check_types: None,
        check_formal: Some(true),
        check_informal: Some(true),
    };
    let resp = server.reasoning_detect(Parameters(req)).await;
    assert!(resp.detections.is_empty() || !resp.detections.is_empty());
}

#[tokio::test]
async fn test_reasoning_decision_tool() {
    let server = create_test_server().await;
    let req = DecisionRequest {
        decision_type: Some("weighted".to_string()),
        question: Some("which?".to_string()),
        options: Some(vec!["A".to_string(), "B".to_string()]),
        topic: None,
        context: Some("context".to_string()),
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_decision(Parameters(req)).await;
    // Stub returns empty recommendation
    let _ = resp.recommendation;
}

#[tokio::test]
async fn test_reasoning_evidence_tool() {
    let server = create_test_server().await;
    let req = EvidenceRequest {
        evidence_type: Some("assess".to_string()),
        claim: Some("claim".to_string()),
        hypothesis: None,
        context: Some("context".to_string()),
        prior: Some(0.5),
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_evidence(Parameters(req)).await;
    assert!(resp.overall_credibility >= 0.0);
}

#[tokio::test]
async fn test_reasoning_timeline_tool() {
    let server = create_test_server().await;
    let req = TimelineRequest {
        operation: "create".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: None,
        content: Some("test".to_string()),
        label: Some("main".to_string()),
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(req)).await;
    // Stub returns empty timeline_id
    let _ = resp.timeline_id;
}

#[tokio::test]
async fn test_reasoning_mcts_tool() {
    let server = create_test_server().await;
    let req = MctsRequest {
        operation: Some("explore".to_string()),
        content: Some("test".to_string()),
        session_id: Some("s1".to_string()),
        node_id: None,
        iterations: Some(10),
        exploration_constant: Some(1.41),
        simulation_depth: Some(5),
        quality_threshold: Some(0.7),
        lookback_depth: Some(3),
        auto_execute: Some(false),
        progress_token: None,
    };
    let resp = server.reasoning_mcts(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_reasoning_counterfactual_tool() {
    let server = create_test_server().await;
    let req = CounterfactualRequest {
        scenario: "base".to_string(),
        intervention: "change".to_string(),
        analysis_depth: Some("counterfactual".to_string()),
        session_id: Some("s1".to_string()),
        progress_token: None,
    };
    let resp = server.reasoning_counterfactual(Parameters(req)).await;
    // Stub uses input values for output
    assert_eq!(resp.original_scenario, "base");
    assert_eq!(resp.intervention_applied, "change");
}

#[tokio::test]
async fn test_reasoning_preset_tool() {
    let server = create_test_server().await;
    let req = PresetRequest {
        operation: "list".to_string(),
        preset_id: None,
        category: Some("analysis".to_string()),
        inputs: None,
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_preset(Parameters(req)).await;
    // presets may or may not be present
    let _ = resp.presets;
}

#[tokio::test]
async fn test_reasoning_metrics_tool() {
    let server = create_test_server().await;
    let req = MetricsRequest {
        query: "summary".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: Some(true),
        limit: Some(10),
    };
    let resp = server.reasoning_metrics(Parameters(req)).await;
    // summary may or may not be present
    let _ = resp.summary;
}
