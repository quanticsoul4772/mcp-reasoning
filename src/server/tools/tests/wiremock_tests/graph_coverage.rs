//! Wiremock tests targeting uncovered success paths in `handlers_graph.rs`.
//!
//! The existing graph tests exercise `init` and `state` (and error paths), but
//! `generate`, `score`, `aggregate`, `refine`, `prune`, and `finalize` were only
//! hit via error paths because the mock JSON didn't match the parsers. These tests
//! provide correctly-formatted JSON for each of those six operations.

use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::GraphRequest;

// ============================================================================
// Per-operation correctly-formatted mock JSON
// ============================================================================

/// Correct mock JSON for `graph.generate`.
/// Requires: parent_id, children[]{id,content,score,type,relationship}, generation_notes.
fn graph_generate_json() -> String {
    serde_json::json!({
        "parent_id": "root-1",
        "children": [
            {
                "id": "c1",
                "content": "Consider the iterative approach first",
                "score": 0.85,
                "type": "reasoning",
                "relationship": "elaborates"
            },
            {
                "id": "c2",
                "content": "Supporting evidence from prior work",
                "score": 0.78,
                "type": "evidence",
                "relationship": "supports"
            }
        ],
        "generation_notes": "Generated two child nodes exploring the problem space"
    })
    .to_string()
}

/// Correct mock JSON for `graph.score`.
/// Requires: node_id, scores{relevance,coherence,depth,novelty,overall},
///           assessment{strengths[],weaknesses[],recommendation}.
fn graph_score_json() -> String {
    serde_json::json!({
        "node_id": "c1",
        "scores": {
            "relevance": 0.90,
            "coherence": 0.85,
            "depth": 0.80,
            "novelty": 0.75,
            "overall": 0.83
        },
        "assessment": {
            "strengths": ["Well-reasoned", "Clear logic"],
            "weaknesses": ["Could go deeper"],
            "recommendation": "expand"
        }
    })
    .to_string()
}

/// Correct mock JSON for `graph.aggregate`.
/// Requires: input_node_ids[], synthesis{id,content,score,type},
///           integration_notes{common_themes[],complementary_aspects[],resolved_contradictions[]}.
fn graph_aggregate_json() -> String {
    serde_json::json!({
        "input_node_ids": ["c1", "c2"],
        "synthesis": {
            "id": "s1",
            "content": "Synthesized insight combining iterative approach with evidence",
            "score": 0.90,
            "type": "synthesis"
        },
        "integration_notes": {
            "common_themes": ["iterative delivery", "risk reduction"],
            "complementary_aspects": ["speed vs quality tradeoff"],
            "resolved_contradictions": []
        }
    })
    .to_string()
}

/// Correct mock JSON for `graph.refine`.
/// Requires: original_node_id, critique{issues[],missing_elements[],unclear_aspects[]},
///           refined_node{id,content,score,type}, improvement_delta.
fn graph_refine_json() -> String {
    serde_json::json!({
        "original_node_id": "c1",
        "critique": {
            "issues": ["Too abstract without concrete steps"],
            "missing_elements": ["timeline", "resource requirements"],
            "unclear_aspects": []
        },
        "refined_node": {
            "id": "r1",
            "content": "Iterative approach with 2-week sprints and defined acceptance criteria",
            "score": 0.92,
            "type": "refined"
        },
        "improvement_delta": 0.07
    })
    .to_string()
}

/// Correct mock JSON for `graph.prune`.
/// Requires: prune_candidates[]{node_id,reason,confidence,impact},
///           preserve_nodes[], pruning_strategy.
fn graph_prune_json() -> String {
    serde_json::json!({
        "prune_candidates": [
            {
                "node_id": "c2",
                "reason": "low_score",
                "confidence": 0.85,
                "impact": "minor"
            }
        ],
        "preserve_nodes": ["c1", "r1", "s1"],
        "pruning_strategy": "Score threshold 0.75 applied; retained high-value nodes"
    })
    .to_string()
}

/// Correct mock JSON for `graph.finalize`.
/// Requires: best_paths[]{path[],path_quality,key_insight},
///           conclusions[]{conclusion,confidence,supporting_nodes[]},
///           final_synthesis, session_quality{depth_achieved,breadth_achieved,coherence,overall}.
fn graph_finalize_json() -> String {
    serde_json::json!({
        "best_paths": [
            {
                "path": ["root-1", "c1", "r1", "s1"],
                "path_quality": 0.90,
                "key_insight": "Iterative approach with evidence support is optimal"
            }
        ],
        "conclusions": [
            {
                "conclusion": "Adopt iterative delivery with 2-week sprints",
                "confidence": 0.95,
                "supporting_nodes": ["c1", "r1"]
            }
        ],
        "final_synthesis": "The graph exploration strongly supports an iterative approach.",
        "session_quality": {
            "depth_achieved": 0.85,
            "breadth_achieved": 0.75,
            "coherence": 0.90,
            "overall": 0.83
        }
    })
    .to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_graph_generate_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&graph_generate_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "generate".to_string(),
        session_id: "s-graph-gen".to_string(),
        content: Some("Explore approaches to reducing system latency".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    // Success path: nodes are populated from children
    let nodes = resp.nodes.expect("generate should return nodes");
    assert!(!nodes.is_empty());
    assert_eq!(nodes[0].id, "c1");
    assert!(nodes[0].score.is_some());
}

#[tokio::test]
async fn test_graph_score_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&graph_score_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "score".to_string(),
        session_id: "s-graph-score".to_string(),
        content: Some("Consider the iterative approach first".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    // Success path: node_id echoed back from the mock
    assert_eq!(resp.node_id.as_deref(), Some("c1"));
}

#[tokio::test]
async fn test_graph_aggregate_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&graph_aggregate_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "aggregate".to_string(),
        session_id: "s-graph-agg".to_string(),
        content: Some("c1: iterative approach\nc2: supporting evidence".to_string()),
        problem: None,
        node_id: None,
        node_ids: Some(vec!["c1".to_string(), "c2".to_string()]),
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    // Success path: aggregated_insight contains synthesis content
    let insight = resp
        .aggregated_insight
        .expect("aggregate should return aggregated_insight");
    assert!(insight.contains("iterative"));
}

#[tokio::test]
async fn test_graph_refine_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&graph_refine_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "refine".to_string(),
        session_id: "s-graph-refine".to_string(),
        content: Some("Consider the iterative approach first".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    // Success path: node_id is the refined node's id
    assert_eq!(resp.node_id.as_deref(), Some("r1"));
}

#[tokio::test]
async fn test_graph_prune_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&graph_prune_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "prune".to_string(),
        session_id: "s-graph-prune".to_string(),
        content: Some("Graph with multiple nodes to evaluate for pruning".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: Some(0.75),
        terminal_node_ids: None,
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    // Success path: state contains pruned_count from prune_candidates
    let state = resp.state.expect("prune should return state");
    assert_eq!(state.pruned_count, 1);
}

#[tokio::test]
async fn test_graph_finalize_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&graph_finalize_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = GraphRequest {
        operation: "finalize".to_string(),
        session_id: "s-graph-finalize".to_string(),
        content: Some("Full graph with all explored nodes and edges".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };

    let resp = server.reasoning_graph(Parameters(req)).await;
    // Success path: conclusions populated
    let conclusions = resp
        .conclusions
        .expect("finalize should return conclusions");
    assert!(!conclusions.is_empty());
    assert!(conclusions[0].contains("iterative"));
}
