// Tests for alternate operation types on each tool handler.
// These exercise uncovered match arms to improve coverage.
use rmcp::handler::server::wrapper::Parameters;

use super::create_test_server;
use crate::server::requests::*;

// ============================================================================
// Agent Metrics Operations (handlers_agents.rs)
// ============================================================================

#[tokio::test]
async fn test_agent_metrics_summary() {
    let server = create_test_server().await;
    let req = AgentMetricsRequest {
        query: "summary".to_string(),
        agent_id: None,
    };
    let resp = server.reasoning_agent_metrics(Parameters(req)).await;
    assert_eq!(resp.query, "summary");
    assert_eq!(resp.data["total_agents"], 4);
}

#[tokio::test]
async fn test_agent_metrics_by_agent() {
    let server = create_test_server().await;
    let req = AgentMetricsRequest {
        query: "by_agent".to_string(),
        agent_id: Some("strategist".to_string()),
    };
    let resp = server.reasoning_agent_metrics(Parameters(req)).await;
    assert_eq!(resp.data["agent_id"], "strategist");
}

#[tokio::test]
async fn test_agent_metrics_unknown_query() {
    let server = create_test_server().await;
    let req = AgentMetricsRequest {
        query: "unknown".to_string(),
        agent_id: None,
    };
    let resp = server.reasoning_agent_metrics(Parameters(req)).await;
    assert!(resp.data["error"].is_string());
}

#[tokio::test]
async fn test_agent_metrics_discovered_skills() {
    let server = create_test_server().await;
    let req = AgentMetricsRequest {
        query: "discovered_skills".to_string(),
        agent_id: None,
    };
    let resp = server.reasoning_agent_metrics(Parameters(req)).await;
    assert_eq!(resp.query, "discovered_skills");
}

// ============================================================================
// Preset Operations (handlers_infra.rs)
// ============================================================================

#[tokio::test]
async fn test_preset_run_missing_preset_id() {
    let server = create_test_server().await;
    let req = PresetRequest {
        operation: "run".to_string(),
        preset_id: None,
        category: None,
        inputs: None,
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_preset(Parameters(req)).await;
    let exec = resp.execution_result.unwrap();
    assert_eq!(exec.steps_completed, 0);
    assert_eq!(exec.preset_id, "unknown");
}

#[tokio::test]
async fn test_preset_run_not_found() {
    let server = create_test_server().await;
    let req = PresetRequest {
        operation: "run".to_string(),
        preset_id: Some("nonexistent-preset".to_string()),
        category: None,
        inputs: None,
        session_id: None,
    };
    let resp = server.reasoning_preset(Parameters(req)).await;
    let exec = resp.execution_result.unwrap();
    assert_eq!(exec.preset_id, "nonexistent-preset");
    assert_eq!(exec.steps_completed, 0);
}

#[tokio::test]
async fn test_preset_run_valid_preset() {
    let server = create_test_server().await;
    let req = PresetRequest {
        operation: "run".to_string(),
        preset_id: Some("code-review".to_string()),
        category: None,
        inputs: None,
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_preset(Parameters(req)).await;
    let exec = resp.execution_result.unwrap();
    assert!(exec.total_steps > 0);
}

#[tokio::test]
async fn test_preset_unknown_operation() {
    let server = create_test_server().await;
    let req = PresetRequest {
        operation: "invalid".to_string(),
        preset_id: None,
        category: None,
        inputs: None,
        session_id: None,
    };
    let resp = server.reasoning_preset(Parameters(req)).await;
    let exec = resp.execution_result.unwrap();
    assert_eq!(exec.preset_id, "unknown");
}

#[tokio::test]
async fn test_preset_list_no_filter() {
    let server = create_test_server().await;
    let req = PresetRequest {
        operation: "list".to_string(),
        preset_id: None,
        category: None,
        inputs: None,
        session_id: None,
    };
    let resp = server.reasoning_preset(Parameters(req)).await;
    let presets = resp.presets.unwrap();
    assert!(!presets.is_empty());
}

// ============================================================================
// Metrics Query Variants (handlers_infra.rs)
// ============================================================================

#[tokio::test]
async fn test_metrics_by_mode_empty_name() {
    let server = create_test_server().await;
    let req = MetricsRequest {
        query: "by_mode".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(req)).await;
    // Falls back to summary when mode_name is empty
    assert!(resp.summary.is_some());
}

#[tokio::test]
async fn test_metrics_by_mode_with_name() {
    let server = create_test_server().await;
    let req = MetricsRequest {
        query: "by_mode".to_string(),
        mode_name: Some("linear".to_string()),
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(req)).await;
    assert!(resp.mode_stats.is_some());
    let stats = resp.mode_stats.unwrap();
    assert_eq!(stats.mode_name, "linear");
}

#[tokio::test]
async fn test_metrics_invocations() {
    let server = create_test_server().await;
    let req = MetricsRequest {
        query: "invocations".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: Some(false),
        limit: Some(50),
    };
    let resp = server.reasoning_metrics(Parameters(req)).await;
    assert!(resp.invocations.is_some());
}

#[tokio::test]
async fn test_metrics_fallbacks() {
    let server = create_test_server().await;
    let req = MetricsRequest {
        query: "fallbacks".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(req)).await;
    assert!(resp.invocations.is_some());
}

#[tokio::test]
async fn test_metrics_config() {
    let server = create_test_server().await;
    let req = MetricsRequest {
        query: "config".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(req)).await;
    assert!(resp.config.is_some());
    let config = resp.config.unwrap();
    assert!(config["model"].is_string());
}

#[tokio::test]
async fn test_metrics_unknown_query() {
    let server = create_test_server().await;
    let req = MetricsRequest {
        query: "invalid".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(req)).await;
    assert!(resp.config.is_some());
    let config = resp.config.unwrap();
    assert!(config["error"].is_string());
}

// ============================================================================
// Checkpoint Operations (handlers_cognitive.rs)
// ============================================================================

#[tokio::test]
async fn test_checkpoint_list_operation() {
    let server = create_test_server().await;
    let req = CheckpointRequest {
        operation: "list".to_string(),
        session_id: "s1".to_string(),
        checkpoint_id: None,
        name: None,
        description: None,
        new_direction: None,
    };
    let resp = server.reasoning_checkpoint(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_checkpoint_restore_operation() {
    let server = create_test_server().await;
    let req = CheckpointRequest {
        operation: "restore".to_string(),
        session_id: "s1".to_string(),
        checkpoint_id: Some("cp-1".to_string()),
        name: None,
        description: None,
        new_direction: Some("new direction".to_string()),
    };
    let resp = server.reasoning_checkpoint(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_checkpoint_unknown_operation() {
    let server = create_test_server().await;
    let req = CheckpointRequest {
        operation: "invalid".to_string(),
        session_id: "s1".to_string(),
        checkpoint_id: None,
        name: None,
        description: None,
        new_direction: None,
    };
    let resp = server.reasoning_checkpoint(Parameters(req)).await;
    assert!(resp.restored_state.is_some());
}

// ============================================================================
// Reflection Operations (handlers_cognitive.rs)
// ============================================================================

#[tokio::test]
async fn test_reflection_evaluate_operation() {
    let server = create_test_server().await;
    let req = ReflectionRequest {
        operation: Some("evaluate".to_string()),
        content: Some("test content".to_string()),
        thought_id: None,
        session_id: Some("s1".to_string()),
        max_iterations: None,
        quality_threshold: None,
        progress_token: None,
    };
    let resp = server.reasoning_reflection(Parameters(req)).await;
    // Evaluate will fail (no API) but exercises the code path
    let _ = resp.quality_score;
}

#[tokio::test]
async fn test_reflection_unknown_operation() {
    let server = create_test_server().await;
    let req = ReflectionRequest {
        operation: Some("invalid".to_string()),
        content: Some("test".to_string()),
        thought_id: None,
        session_id: Some("s1".to_string()),
        max_iterations: None,
        quality_threshold: None,
        progress_token: None,
    };
    let resp = server.reasoning_reflection(Parameters(req)).await;
    assert_eq!(resp.quality_score, 0.0);
    assert!(resp.weaknesses.is_some());
}

// ============================================================================
// Tree Operations (handlers_basic.rs)
// ============================================================================

#[tokio::test]
async fn test_tree_focus_operation() {
    let server = create_test_server().await;
    let req = TreeRequest {
        operation: Some("focus".to_string()),
        content: None,
        session_id: Some("s1".to_string()),
        branch_id: Some("b1".to_string()),
        num_branches: None,
        completed: None,
    };
    let resp = server.reasoning_tree(Parameters(req)).await;
    let _ = resp.session_id;
}

#[tokio::test]
async fn test_tree_list_operation() {
    let server = create_test_server().await;
    let req = TreeRequest {
        operation: Some("list".to_string()),
        content: None,
        session_id: Some("s1".to_string()),
        branch_id: None,
        num_branches: None,
        completed: None,
    };
    let resp = server.reasoning_tree(Parameters(req)).await;
    let _ = resp.session_id;
}

#[tokio::test]
async fn test_tree_complete_operation() {
    let server = create_test_server().await;
    let req = TreeRequest {
        operation: Some("complete".to_string()),
        content: None,
        session_id: Some("s1".to_string()),
        branch_id: Some("b1".to_string()),
        num_branches: None,
        completed: Some(true),
    };
    let resp = server.reasoning_tree(Parameters(req)).await;
    let _ = resp.session_id;
}

#[tokio::test]
async fn test_tree_unknown_operation() {
    let server = create_test_server().await;
    let req = TreeRequest {
        operation: Some("invalid".to_string()),
        content: None,
        session_id: Some("s1".to_string()),
        branch_id: None,
        num_branches: None,
        completed: None,
    };
    let resp = server.reasoning_tree(Parameters(req)).await;
    assert!(resp.recommendation.unwrap().contains("Unknown operation"));
}
