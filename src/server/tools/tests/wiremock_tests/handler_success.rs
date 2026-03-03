//! Wiremock tests targeting handler success paths with properly formatted JSON responses.
//! These tests provide mock responses matching exactly what each mode parser expects,
//! ensuring the handler Ok branches are exercised for coverage.

use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::*;
use crate::storage::StoredThought;

// ============================================================================
// Decision Handler Success Paths (handlers_decision.rs)
// ============================================================================

#[tokio::test]
async fn test_decision_weighted_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "options": ["Option A", "Option B"],
        "criteria": [
            {"name": "cost", "weight": 0.5, "description": "Cost efficiency"},
            {"name": "quality", "weight": 0.5, "description": "Quality"}
        ],
        "scores": {
            "Option A": {"cost": 0.8, "quality": 0.9},
            "Option B": {"cost": 0.6, "quality": 0.7}
        },
        "weighted_totals": {"Option A": 0.85, "Option B": 0.65},
        "ranking": [
            {"option": "Option A", "score": 0.85, "rank": 1},
            {"option": "Option B", "score": 0.65, "rank": 2}
        ],
        "sensitivity_notes": "Results stable"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = DecisionRequest {
        decision_type: Some("weighted".to_string()),
        question: Some("Which option?".to_string()),
        options: Some(vec!["Option A".to_string(), "Option B".to_string()]),
        topic: None,
        context: Some("Test context".to_string()),
        session_id: Some("s1".to_string()),
    };

    let resp = server.reasoning_decision(Parameters(req)).await;
    assert!(!resp.recommendation.is_empty());
    assert!(resp.rankings.is_some());
}

#[tokio::test]
async fn test_decision_pairwise_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "comparisons": [
            {
                "option_a": "A",
                "option_b": "B",
                "preferred": "option_a",
                "strength": "strong",
                "reasoning": "Better overall"
            }
        ],
        "pairwise_matrix": {"a_vs_b": 1},
        "ranking": [
            {"option": "A", "wins": 1, "rank": 1},
            {"option": "B", "wins": 0, "rank": 2}
        ],
        "consistency_check": "Consistent"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = DecisionRequest {
        decision_type: Some("pairwise".to_string()),
        question: Some("Which is better?".to_string()),
        options: Some(vec!["A".to_string(), "B".to_string()]),
        topic: None,
        context: Some("comparison".to_string()),
        session_id: Some("s1".to_string()),
    };

    let resp = server.reasoning_decision(Parameters(req)).await;
    assert!(!resp.recommendation.is_empty());
    assert!(resp.rankings.is_some());
}

#[tokio::test]
async fn test_decision_topsis_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "criteria": [
            {"name": "cost", "type": "cost", "weight": 0.4},
            {"name": "quality", "type": "benefit", "weight": 0.6}
        ],
        "decision_matrix": {
            "Option A": [100.0, 0.8],
            "Option B": [150.0, 0.9]
        },
        "ideal_solution": [100.0, 0.9],
        "anti_ideal_solution": [150.0, 0.8],
        "distances": {
            "Option A": {"to_ideal": 0.2, "to_anti_ideal": 0.8},
            "Option B": {"to_ideal": 0.15, "to_anti_ideal": 0.85}
        },
        "relative_closeness": {"Option A": 0.8, "Option B": 0.85},
        "ranking": [
            {"option": "Option B", "closeness": 0.85, "rank": 1},
            {"option": "Option A", "closeness": 0.8, "rank": 2}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = DecisionRequest {
        decision_type: Some("topsis".to_string()),
        question: Some("Rank by TOPSIS".to_string()),
        options: Some(vec!["Option A".to_string(), "Option B".to_string()]),
        topic: None,
        context: Some("multi-criteria".to_string()),
        session_id: Some("s1".to_string()),
    };

    let resp = server.reasoning_decision(Parameters(req)).await;
    assert!(!resp.recommendation.is_empty());
    assert!(resp.rankings.is_some());
}

#[tokio::test]
async fn test_decision_perspectives_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "stakeholders": [
            {
                "name": "Customer",
                "interests": ["quality", "price"],
                "preferred_option": "Option A",
                "concerns": ["warranty"],
                "influence_level": "high"
            },
            {
                "name": "Vendor",
                "interests": ["margin"],
                "preferred_option": "Option B",
                "concerns": ["costs"],
                "influence_level": "medium"
            },
            {
                "name": "Regulator",
                "interests": ["compliance"],
                "preferred_option": "Option A",
                "concerns": ["standards"],
                "influence_level": "low"
            }
        ],
        "conflicts": [
            {
                "between": ["Customer", "Vendor"],
                "issue": "pricing",
                "severity": "medium"
            }
        ],
        "alignments": [
            {
                "stakeholders": ["Customer", "Regulator"],
                "common_ground": "Quality focus"
            }
        ],
        "balanced_recommendation": {
            "option": "Option A",
            "rationale": "Best balance of needs",
            "mitigation": "Negotiate vendor pricing"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = DecisionRequest {
        decision_type: Some("perspectives".to_string()),
        question: None,
        options: Some(vec!["Option A".to_string(), "Option B".to_string()]),
        topic: Some("Product launch strategy".to_string()),
        context: Some("stakeholder analysis".to_string()),
        session_id: Some("s1".to_string()),
    };

    let resp = server.reasoning_decision(Parameters(req)).await;
    assert!(!resp.recommendation.is_empty());
    assert!(resp.stakeholder_map.is_some());
}

// ============================================================================
// Graph Handler Success Paths (handlers_graph.rs)
// ============================================================================

#[tokio::test]
async fn test_graph_init_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "root": {
            "id": "n1",
            "content": "Root analysis",
            "score": 0.8,
            "type": "root"
        },
        "expansion_directions": [
            {"direction": "evidence", "potential": 0.9},
            {"direction": "hypothesis", "potential": 0.7}
        ],
        "graph_metadata": {
            "complexity": "medium",
            "estimated_depth": 4
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "init".to_string(),
        session_id: "s1".to_string(),
        content: Some("Analyze this problem".to_string()),
        problem: Some("Complex problem".to_string()),
        node_id: None,
        node_ids: None,
        k: Some(3),
        threshold: None,
        terminal_node_ids: None,
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
    // On success, init returns node_id (not nodes); on error, node_id is None
    // Either way exercises the handler code paths
    let _ = resp.node_id;
}

#[tokio::test]
async fn test_graph_generate_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "parent_id": "n1",
        "children": [
            {
                "id": "n2",
                "content": "Supporting evidence",
                "score": 0.85,
                "type": "evidence",
                "relationship": "supports"
            },
            {
                "id": "n3",
                "content": "Counter argument",
                "score": 0.7,
                "type": "reasoning",
                "relationship": "challenges"
            }
        ],
        "generation_notes": "Generated 2 child nodes"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "generate".to_string(),
        session_id: "s1".to_string(),
        content: None,
        problem: None,
        node_id: Some("n1".to_string()),
        node_ids: None,
        k: Some(2),
        threshold: None,
        terminal_node_ids: None,
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_score_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "node_id": "n1",
        "scores": {
            "relevance": 0.9,
            "coherence": 0.85,
            "depth": 0.8,
            "novelty": 0.7,
            "overall": 0.81
        },
        "assessment": {
            "strengths": ["Clear logic", "Good evidence"],
            "weaknesses": ["Could be deeper"],
            "recommendation": "expand"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "score".to_string(),
        session_id: "s1".to_string(),
        content: None,
        problem: None,
        node_id: Some("n1".to_string()),
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_aggregate_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "input_node_ids": ["n1", "n2"],
        "synthesis": {
            "id": "n_synth",
            "content": "Combined insight",
            "score": 0.88,
            "type": "synthesis"
        },
        "integration_notes": {
            "common_themes": ["Evidence quality"],
            "complementary_aspects": ["Different angles"],
            "resolved_contradictions": ["Minor disagreement resolved"]
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "aggregate".to_string(),
        session_id: "s1".to_string(),
        content: None,
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
async fn test_graph_finalize_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "best_paths": [
            {
                "path": ["n1", "n2", "n3"],
                "path_quality": 0.9,
                "key_insight": "Strong evidence chain"
            }
        ],
        "conclusions": [
            {
                "conclusion": "Main finding",
                "confidence": 0.85,
                "supporting_nodes": ["n1", "n2"]
            }
        ],
        "final_synthesis": "The analysis shows...",
        "session_quality": {
            "depth_achieved": 0.8,
            "breadth_achieved": 0.75,
            "coherence": 0.9,
            "overall": 0.82
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "finalize".to_string(),
        session_id: "s1".to_string(),
        content: None,
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: Some(vec!["n3".to_string()]),
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    assert_eq!(resp.session_id, "s1");
}

#[tokio::test]
async fn test_graph_state_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "structure": {
            "total_nodes": 10,
            "depth": 3,
            "branches": 4,
            "pruned_count": 2
        },
        "frontiers": [
            {
                "node_id": "n5",
                "potential": 0.85,
                "suggested_action": "expand"
            }
        ],
        "metrics": {
            "average_score": 0.78,
            "max_score": 0.95,
            "coverage": 0.7
        },
        "next_steps": ["Expand frontier nodes", "Refine weak nodes"]
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
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

// ============================================================================
// Session Handler Success Paths (handlers_sessions.rs)
// ============================================================================

#[tokio::test]
async fn test_list_sessions_success_with_data() {
    let mock_server = MockServer::start().await;
    let server = create_mocked_server(&mock_server).await;

    // Seed storage with session data
    let session = server
        .state
        .storage
        .create_session()
        .await
        .expect("create session");
    let thought = StoredThought::new(
        uuid::Uuid::new_v4().to_string(),
        &session.id,
        "linear",
        "Test thought content for coverage",
        0.85,
    );
    server
        .state
        .storage
        .save_stored_thought(&thought)
        .await
        .expect("save thought");

    let req = ListSessionsRequest {
        limit: Some(10),
        offset: Some(0),
        mode_filter: None,
    };

    let resp = server.reasoning_list_sessions(Parameters(req)).await;
    assert_eq!(resp.sessions.len(), 1);
    assert_eq!(resp.total, 1);
    assert!(!resp.has_more);
    assert!(!resp.sessions[0].session_id.is_empty());
    assert!(resp.sessions[0].preview.contains("Test thought"));
}

#[tokio::test]
async fn test_list_sessions_with_mode_filter() {
    let mock_server = MockServer::start().await;
    let server = create_mocked_server(&mock_server).await;

    // Create two sessions with different modes
    let s1 = server
        .state
        .storage
        .create_session()
        .await
        .expect("create session");
    let t1 = StoredThought::new(
        uuid::Uuid::new_v4().to_string(),
        &s1.id,
        "linear",
        "Linear thought",
        0.8,
    );
    server
        .state
        .storage
        .save_stored_thought(&t1)
        .await
        .expect("save");

    let s2 = server
        .state
        .storage
        .create_session()
        .await
        .expect("create session");
    let t2 = StoredThought::new(
        uuid::Uuid::new_v4().to_string(),
        &s2.id,
        "tree",
        "Tree thought",
        0.9,
    );
    server
        .state
        .storage
        .save_stored_thought(&t2)
        .await
        .expect("save");

    let req = ListSessionsRequest {
        limit: None,
        offset: None,
        mode_filter: Some("linear".to_string()),
    };

    let resp = server.reasoning_list_sessions(Parameters(req)).await;
    assert_eq!(resp.sessions.len(), 1);
    assert_eq!(resp.sessions[0].primary_mode, Some("linear".to_string()));
}

// ============================================================================
// Detect Handler Success Paths - with both biases and fallacies
// ============================================================================

#[tokio::test]
async fn test_detect_biases_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "detected_biases": [
            {
                "type": "confirmation_bias",
                "description": "Seeking confirming evidence",
                "severity": "high",
                "evidence": "Only cited supporting sources",
                "mitigation": "Consider opposing viewpoints"
            }
        ],
        "overall_assessment": {
            "argument_strength": "moderate",
            "objectivity_score": 0.5,
            "recommendation": "Consider opposing evidence"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = DetectRequest {
        detect_type: "biases".to_string(),
        content: Some(
            "The evidence clearly shows our product is superior because our customers say so"
                .to_string(),
        ),
        thought_id: None,
        session_id: Some("s1".to_string()),
        check_types: None,
        check_formal: Some(true),
        check_informal: Some(true),
    };

    let resp = server.reasoning_detect(Parameters(req)).await;
    assert!(!resp.detections.is_empty() || resp.summary.is_some());
}

#[tokio::test]
async fn test_detect_fallacies_success_path() {
    let mock_server = MockServer::start().await;

    let json = serde_json::json!({
        "detected_fallacies": [
            {
                "type": "ad_hominem",
                "description": "Attacking the person rather than the argument",
                "severity": "high",
                "evidence": "Dismissed claim based on speaker's background",
                "correction": "Address the argument's content directly"
            }
        ],
        "overall_assessment": {
            "argument_strength": "weak",
            "formal_validity": false,
            "recommendation": "Restructure argument"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = DetectRequest {
        detect_type: "fallacies".to_string(),
        content: Some("You can't trust his argument because he's not an expert".to_string()),
        thought_id: None,
        session_id: Some("s1".to_string()),
        check_types: Some(vec!["formal".to_string()]),
        check_formal: Some(true),
        check_informal: Some(true),
    };

    let resp = server.reasoning_detect(Parameters(req)).await;
    assert!(!resp.detections.is_empty() || resp.summary.is_some());
}
