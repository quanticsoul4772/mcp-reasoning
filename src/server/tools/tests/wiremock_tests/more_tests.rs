use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::*;

#[tokio::test]
async fn test_evidence_unknown_type() {
    let mock_server = MockServer::start().await;

    // No mock needed - unknown type returns early
    let server = create_mocked_server(&mock_server).await;

    let unknown_req = EvidenceRequest {
        evidence_type: Some("unknown_type".to_string()),
        claim: Some("Claim".to_string()),
        hypothesis: None,
        prior: None,
        context: None,
        session_id: None,
    };
    let resp = server.reasoning_evidence(Parameters(unknown_req)).await;
    assert!(resp.synthesis.unwrap().contains("Unknown"));
}

#[tokio::test]
async fn test_metrics_all_queries() {
    let mock_server = MockServer::start().await;

    // Metrics don't require API calls
    let server = create_mocked_server(&mock_server).await;

    // Test summary
    let summary_req = MetricsRequest {
        query: "summary".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(summary_req)).await;
    let _ = resp.summary;

    // Test by_mode
    let by_mode_req = MetricsRequest {
        query: "by_mode".to_string(),
        mode_name: Some("linear".to_string()),
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(by_mode_req)).await;
    let _ = resp.mode_stats;

    // Test invocations
    let invocations_req = MetricsRequest {
        query: "invocations".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: Some(true),
        limit: Some(10),
    };
    let resp = server.reasoning_metrics(Parameters(invocations_req)).await;
    let _ = resp.invocations;

    // Test fallbacks
    let fallbacks_req = MetricsRequest {
        query: "fallbacks".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(fallbacks_req)).await;
    let _ = resp.summary;

    // Test config
    let config_req = MetricsRequest {
        query: "config".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(config_req)).await;
    let _ = resp.config;

    // Test unknown query
    let unknown_req = MetricsRequest {
        query: "unknown".to_string(),
        mode_name: None,
        tool_name: None,
        session_id: None,
        success_only: None,
        limit: None,
    };
    let resp = server.reasoning_metrics(Parameters(unknown_req)).await;
    let _ = resp.summary;
}

#[tokio::test]
async fn test_preset_run_valid() {
    let mock_server = MockServer::start().await;

    // Test running a valid preset
    let json = serde_json::json!({
        "analysis": "Quick analysis result",
        "confidence": 0.8
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    // Run quick_analysis preset
    let run_req = PresetRequest {
        operation: "run".to_string(),
        preset_id: Some("quick_analysis".to_string()),
        category: None,
        inputs: Some(serde_json::json!({"content": "Analyze this"})),
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_preset(Parameters(run_req)).await;
    // Will have execution result or error
    let _ = resp.execution_result;
}

#[tokio::test]
async fn test_timeline_success_paths() {
    let mock_server = MockServer::start().await;

    // Test create with proper response
    let create_json = serde_json::json!({
        "timeline_id": "tl_123",
        "events": [
            {"timestamp": "2024-01-01", "event": "Start", "significance": "high"}
        ],
        "analysis": "Timeline created"
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

    // Create timeline
    let create_req = TimelineRequest {
        operation: "create".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: None,
        content: Some("Event history".to_string()),
        label: Some("main".to_string()),
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(create_req)).await;
    // Check that we get a response
    let _ = resp.timeline_id;

    // Test branch operation
    let branch_json = serde_json::json!({
        "branch_id": "br_456",
        "timeline_id": "tl_123",
        "divergence_point": "2024-01-15",
        "events": []
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&branch_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let branch_req = TimelineRequest {
        operation: "branch".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: Some("tl_123".to_string()),
        content: Some("Alternative history".to_string()),
        label: Some("alternative".to_string()),
        branch_ids: None,
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(branch_req)).await;
    let _ = resp.branch_id;

    // Test compare operation
    let compare_json = serde_json::json!({
        "comparison": {
            "common_events": ["Start"],
            "divergences": [{"point": "Day 5", "branch_a": "X", "branch_b": "Y"}],
            "analysis": "Branches diverge at Day 5"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&compare_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let compare_req = TimelineRequest {
        operation: "compare".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: Some("tl_123".to_string()),
        content: None,
        label: None,
        branch_ids: Some(vec!["br_1".to_string(), "br_2".to_string()]),
        source_branch_id: None,
        target_branch_id: None,
        merge_strategy: None,
    };
    let resp = server.reasoning_timeline(Parameters(compare_req)).await;
    let _ = resp.comparison;

    // Test merge operation
    let merge_json = serde_json::json!({
        "merged_timeline_id": "tl_merged",
        "events": [{"timestamp": "2024-01-01", "event": "Merged event"}],
        "conflicts_resolved": 1,
        "analysis": "Merge successful"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&merge_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let merge_req = TimelineRequest {
        operation: "merge".to_string(),
        session_id: Some("s1".to_string()),
        timeline_id: Some("tl_123".to_string()),
        content: None,
        label: None,
        branch_ids: None,
        source_branch_id: Some("br_1".to_string()),
        target_branch_id: Some("br_2".to_string()),
        merge_strategy: Some("integrate".to_string()),
    };
    let resp = server.reasoning_timeline(Parameters(merge_req)).await;
    let _ = resp.merged_content;
}
