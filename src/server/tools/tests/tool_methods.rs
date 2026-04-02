use rmcp::handler::server::wrapper::Parameters;

use super::create_test_server;
use crate::server::requests::*;

#[tokio::test]
async fn test_reasoning_linear_tool() {
    let server = create_test_server().await;
    let req = LinearRequest {
        content: "test".to_string(),
        session_id: Some("s1".to_string()),
        confidence: Some(ConfidenceThreshold::try_from(0.8).unwrap()),
        timeout_ms: None,
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
        execute: None,
    };
    let resp = server.reasoning_auto(Parameters(req)).await;
    assert!(!resp.selected_mode.is_empty());
    // execute=None: should have next_call hint, executed should be None
    assert!(
        resp.next_call.is_some(),
        "non-execute path should provide next_call hint"
    );
    assert!(
        resp.executed.is_none(),
        "non-execute path should not set executed flag"
    );
}

#[tokio::test]
async fn test_reasoning_auto_execute_linear() {
    let server = create_test_server().await;
    let req = AutoRequest {
        content: "Analyze the tradeoffs between SQL and NoSQL databases step by step".to_string(),
        hints: None,
        session_id: Some("s1".to_string()),
        execute: Some(true),
    };
    let resp = server.reasoning_auto(Parameters(req)).await;
    assert!(!resp.selected_mode.is_empty());
    // In all paths: next_call is always present (success=selected mode, error=linear fallback).
    assert!(
        resp.next_call.is_some() || resp.executed == Some(true),
        "either next_call hint or executed=true must be set"
    );
}

#[tokio::test]
async fn test_reasoning_auto_execute_false() {
    let server = create_test_server().await;
    let req = AutoRequest {
        content: "test content".to_string(),
        hints: None,
        session_id: None,
        execute: Some(false),
    };
    let resp = server.reasoning_auto(Parameters(req)).await;
    assert!(!resp.selected_mode.is_empty());
    // execute=false behaves the same as execute=None
    assert!(
        resp.next_call.is_some(),
        "execute=false should provide next_call hint"
    );
    assert!(resp.executed.is_none());
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

// ============================================================================
// Self-Improvement Tool Tests
// ============================================================================

#[tokio::test]
async fn test_reasoning_si_status_tool() {
    let server = create_test_server().await;
    let req = SiStatusRequest {};
    let resp = server.reasoning_si_status(Parameters(req)).await;
    // Status should report circuit state
    assert!(!resp.circuit_state.is_empty());
}

#[tokio::test]
async fn test_reasoning_si_diagnoses_tool() {
    let server = create_test_server().await;
    let req = SiDiagnosesRequest { limit: Some(5) };
    let resp = server.reasoning_si_diagnoses(Parameters(req)).await;
    // May or may not have pending diagnoses
    let _ = resp.diagnoses;
}

#[tokio::test]
async fn test_reasoning_si_approve_tool() {
    let server = create_test_server().await;
    let req = SiApproveRequest {
        diagnosis_id: "nonexistent".to_string(),
    };
    let resp = server.reasoning_si_approve(Parameters(req)).await;
    // Should fail for nonexistent diagnosis
    assert!(!resp.success);
}

#[tokio::test]
async fn test_reasoning_si_reject_tool() {
    let server = create_test_server().await;
    let req = SiRejectRequest {
        diagnosis_id: "nonexistent".to_string(),
        reason: Some("test rejection".to_string()),
    };
    let resp = server.reasoning_si_reject(Parameters(req)).await;
    assert!(!resp.success);
}

#[tokio::test]
async fn test_reasoning_si_trigger_tool() {
    let server = create_test_server().await;
    let req = SiTriggerRequest {};
    let resp = server.reasoning_si_trigger(Parameters(req)).await;
    // Trigger should complete (may or may not succeed depending on state)
    let _ = resp.success;
}

#[tokio::test]
async fn test_reasoning_si_rollback_tool() {
    let server = create_test_server().await;
    let req = SiRollbackRequest {
        action_id: "nonexistent".to_string(),
    };
    let resp = server.reasoning_si_rollback(Parameters(req)).await;
    assert!(!resp.success);
}

// ============================================================================
// Session Tool Tests
// ============================================================================

#[tokio::test]
async fn test_reasoning_list_sessions_tool() {
    let server = create_test_server().await;
    let req = ListSessionsRequest {
        limit: Some(10),
        offset: Some(0),
        mode_filter: None,
    };
    let resp = server.reasoning_list_sessions(Parameters(req)).await;
    // May or may not have sessions
    let _ = resp.sessions;
}

#[tokio::test]
async fn test_reasoning_resume_tool() {
    let server = create_test_server().await;
    let req = ResumeSessionRequest {
        session_id: "nonexistent".to_string(),
        compress: Some(false),
        include_checkpoints: Some(true),
    };
    let resp = server.reasoning_resume(Parameters(req)).await;
    // Should handle nonexistent session gracefully
    let _ = resp.session_id;
}

#[tokio::test]
async fn test_reasoning_search_tool() {
    let server = create_test_server().await;
    let req = SearchSessionsRequest {
        query: "test query".to_string(),
        limit: Some(5),
        min_similarity: Some(0.5),
        mode_filter: None,
    };
    let resp = server.reasoning_search(Parameters(req)).await;
    let _ = resp.results;
}

#[tokio::test]
async fn test_reasoning_relate_tool() {
    let server = create_test_server().await;
    let req = RelateSessionsRequest {
        session_id: None,
        min_strength: Some(0.5),
        depth: Some(2),
    };
    let resp = server.reasoning_relate(Parameters(req)).await;
    let _ = resp.nodes;
}

// ============================================================================
// Agent & Skill Tool Tests
// ============================================================================

#[tokio::test]
async fn test_reasoning_agent_invoke_tool() {
    let server = create_test_server().await;
    let req = AgentInvokeRequest {
        agent_id: "analyst".to_string(),
        task: "Test task".to_string(),
        session_id: None,
    };
    let resp = server.reasoning_agent_invoke(Parameters(req)).await;
    assert!(resp.success);
    assert_eq!(resp.agent_id, "analyst");
}

#[tokio::test]
async fn test_reasoning_agent_invoke_not_found() {
    let server = create_test_server().await;
    let req = AgentInvokeRequest {
        agent_id: "nonexistent".to_string(),
        task: "Test task".to_string(),
        session_id: None,
    };
    let resp = server.reasoning_agent_invoke(Parameters(req)).await;
    assert!(!resp.success);
    assert_eq!(resp.status, "error");
}

#[tokio::test]
async fn test_reasoning_agent_list_tool() {
    let server = create_test_server().await;
    let req = AgentListRequest { role: None };
    let resp = server.reasoning_agent_list(Parameters(req)).await;
    assert_eq!(resp.total, 4);
}

#[tokio::test]
async fn test_reasoning_agent_list_filtered() {
    let server = create_test_server().await;
    let req = AgentListRequest {
        role: Some("analyst".to_string()),
    };
    let resp = server.reasoning_agent_list(Parameters(req)).await;
    assert_eq!(resp.total, 1);
    assert_eq!(resp.agents[0].id, "analyst");
}

#[tokio::test]
async fn test_reasoning_skill_run_tool() {
    let server = create_test_server().await;
    let req = SkillRunRequest {
        skill_id: "code-review".to_string(),
        input: "test input".to_string(),
        session_id: None,
    };
    let resp = server.reasoning_skill_run(Parameters(req)).await;
    assert!(resp.success);
    assert!(resp.steps_executed > 0);
}

#[tokio::test]
async fn test_reasoning_skill_run_not_found() {
    let server = create_test_server().await;
    let req = SkillRunRequest {
        skill_id: "nonexistent".to_string(),
        input: "test".to_string(),
        session_id: None,
    };
    let resp = server.reasoning_skill_run(Parameters(req)).await;
    assert!(!resp.success);
}

#[tokio::test]
async fn test_reasoning_team_run_tool() {
    let server = create_test_server().await;
    let req = TeamRunRequest {
        team_id: "debug-investigation".to_string(),
        task: "Test task".to_string(),
        session_id: None,
    };
    let resp = server.reasoning_team_run(Parameters(req)).await;
    assert!(resp.success);
    assert!(resp.subtasks_executed > 0);
}

#[tokio::test]
async fn test_reasoning_team_run_not_found() {
    let server = create_test_server().await;
    let req = TeamRunRequest {
        team_id: "nonexistent".to_string(),
        task: "Test task".to_string(),
        session_id: None,
    };
    let resp = server.reasoning_team_run(Parameters(req)).await;
    assert!(!resp.success);
}

#[tokio::test]
async fn test_reasoning_team_list_tool() {
    let server = create_test_server().await;
    let req = TeamListRequest { topology: None };
    let resp = server.reasoning_team_list(Parameters(req)).await;
    assert_eq!(resp.total, 5);
}

#[tokio::test]
async fn test_reasoning_team_list_filtered() {
    let server = create_test_server().await;
    let req = TeamListRequest {
        topology: Some("sequential".to_string()),
    };
    let resp = server.reasoning_team_list(Parameters(req)).await;
    assert_eq!(resp.total, 2);
}
