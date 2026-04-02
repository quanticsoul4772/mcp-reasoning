//! Wiremock tests covering uncovered success paths in `handlers_basic.rs`:
//!
//! - Auto with execute=true (linear and divergent execution branches)
//! - Tree focus and complete success paths (using real branch IDs from a prior create)
//! - Meta handler success path with API-based classification

use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::{AutoRequest, MetaRequest, TreeRequest};

// ============================================================================
// Auto execute=true: linear and divergent
// ============================================================================

/// Combined JSON satisfying both auto-detection (selected_mode, confidence, reasoning)
/// and linear execution (analysis, confidence) parsers.
fn auto_then_linear_json() -> String {
    serde_json::json!({
        "selected_mode": "linear",
        "reasoning": "Sequential step-by-step analysis is most appropriate",
        "confidence": 0.88,
        "characteristics": ["sequential", "well-defined steps"],
        "suggested_parameters": {},
        "analysis": "Here is the detailed sequential reasoning.",
        "next_step": "Review and apply the conclusions"
    })
    .to_string()
}

/// Combined JSON satisfying both auto-detection and divergent (perspectives) parsers.
fn auto_then_divergent_json() -> String {
    serde_json::json!({
        "selected_mode": "divergent",
        "reasoning": "Multiple perspectives will surface important trade-offs",
        "confidence": 0.82,
        "characteristics": ["multi-faceted", "open-ended"],
        "suggested_parameters": {},
        "perspectives": [
            {"viewpoint": "Technical", "content": "Focus on implementation", "novelty_score": 0.85},
            {"viewpoint": "Business", "content": "Focus on value", "novelty_score": 0.75}
        ]
    })
    .to_string()
}

/// Combined JSON where auto detects a complex mode (tree) at high confidence.
/// When execute=true and mode is complex, the handler returns a next_call hint.
fn auto_then_complex_mode_json() -> String {
    serde_json::json!({
        "selected_mode": "tree",
        "reasoning": "Tree exploration needed for complex decision space",
        "confidence": 0.9,
        "characteristics": ["complex", "branching"],
        "suggested_parameters": {}
    })
    .to_string()
}

#[tokio::test]
async fn test_auto_execute_true_linear() {
    let mock_server = MockServer::start().await;

    // Both auto-detection call and linear execution call return the same combined JSON
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&auto_then_linear_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = AutoRequest {
        content: "Analyze this step by step".to_string(),
        hints: None,
        session_id: None,
        execute: Some(true),
    };

    let resp = server.reasoning_auto(Parameters(req)).await;
    // execute=true + linear auto-detection → executes linear, sets executed=Some(true)
    assert_eq!(resp.selected_mode, "linear");
    assert_eq!(resp.executed, Some(true));
    assert!(!resp.result.is_null());
    // next_call should be None when execute=true fires the mode directly
    assert!(resp.next_call.is_none());
}

#[tokio::test]
async fn test_auto_execute_true_divergent() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_then_divergent_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = AutoRequest {
        content: "Explore multiple perspectives on this design decision".to_string(),
        hints: None,
        session_id: None,
        execute: Some(true),
    };

    let resp = server.reasoning_auto(Parameters(req)).await;
    // execute=true + divergent auto-detection → executes divergent
    assert_eq!(resp.selected_mode, "divergent");
    assert_eq!(resp.executed, Some(true));
    assert!(!resp.result.is_null());
}

#[tokio::test]
async fn test_auto_execute_true_complex_mode_returns_next_call() {
    let mock_server = MockServer::start().await;

    // Auto detects "tree" — complex mode that needs extra parameters
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&auto_then_complex_mode_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = AutoRequest {
        content: "Explore this complex decision space".to_string(),
        hints: None,
        session_id: None,
        execute: Some(true),
    };

    let resp = server.reasoning_auto(Parameters(req)).await;
    // execute=true + complex mode → can't execute, returns next_call hint
    assert_eq!(resp.selected_mode, "tree");
    // next_call hint should be set to guide the user to the next step
    let next = resp
        .next_call
        .expect("complex mode with execute=true returns next_call");
    assert!(!next.tool.is_empty());
}

// ============================================================================
// Tree focus success path (requires a real branch_id from a prior create)
// ============================================================================

/// Combined JSON satisfying both tree create (branches) and focus (exploration, insights) parsers.
fn tree_create_and_focus_json() -> String {
    serde_json::json!({
        "branches": [
            {
                "title": "Approach A",
                "description": "Take the iterative path",
                "score": 0.85,
                "initial_thought": "Start with the simplest viable option"
            },
            {
                "title": "Approach B",
                "description": "Take the comprehensive path",
                "score": 0.7,
                "initial_thought": "Design the complete system upfront"
            }
        ],
        "recommendation": "Explore Approach A first (highest score)",
        "exploration": "Approach A allows rapid feedback and course-correction.",
        "insights": ["Fail fast principles apply here", "MVP first reduces risk"],
        "confidence": 0.85
    })
    .to_string()
}

/// Combined JSON for tree complete operation (marks a branch as done).
fn tree_complete_json() -> String {
    serde_json::json!({
        "branches": [
            {"title": "Approach A", "description": "Completed exploration", "score": 0.9}
        ],
        "recommendation": "Approach A fully explored — proceed to synthesis",
        "exploration": "Thorough analysis completed.",
        "insights": ["Approach A is clearly superior"],
        "confidence": 0.95
    })
    .to_string()
}

#[tokio::test]
async fn test_tree_focus_success_path() {
    let mock_server = MockServer::start().await;

    // All API calls return the combined JSON
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&tree_create_and_focus_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    // Step 1: Create a tree — branches get stored with generated IDs
    let create_req = TreeRequest {
        operation: Some("create".to_string()),
        content: Some("Should we use a monolith or microservices architecture?".to_string()),
        session_id: Some("s-tree-focus".to_string()),
        branch_id: None,
        num_branches: Some(2),
        completed: None,
    };
    let create_resp = server.reasoning_tree(Parameters(create_req)).await;

    // Step 2: Get the real branch_id from the create response
    let branches = create_resp.branches.expect("create should return branches");
    assert!(
        !branches.is_empty(),
        "must have at least one branch to focus on"
    );
    let real_branch_id = branches[0].id.clone();

    // Step 3: Focus on that branch using its real ID
    let focus_req = TreeRequest {
        operation: Some("focus".to_string()),
        content: None,
        session_id: Some("s-tree-focus".to_string()),
        branch_id: Some(real_branch_id.clone()),
        num_branches: None,
        completed: None,
    };
    let focus_resp = server.reasoning_tree(Parameters(focus_req)).await;

    // Success path: branch_id echoed back, no error in recommendation
    assert_eq!(
        focus_resp.branch_id.as_deref(),
        Some(real_branch_id.as_str())
    );
    let rec = focus_resp.recommendation.as_deref().unwrap_or("");
    assert!(!rec.contains("focus failed"), "unexpected error: {rec}");
}

#[tokio::test]
async fn test_tree_complete_success_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&tree_complete_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    // Step 1: Create branches first
    let create_req = TreeRequest {
        operation: Some("create".to_string()),
        content: Some("Evaluate the three candidate solutions".to_string()),
        session_id: Some("s-tree-complete".to_string()),
        branch_id: None,
        num_branches: Some(2),
        completed: None,
    };
    let create_resp = server.reasoning_tree(Parameters(create_req)).await;
    let branches = create_resp.branches.expect("create must return branches");
    let real_branch_id = branches[0].id.clone();

    // Step 2: Mark a branch as complete using its real ID
    let complete_req = TreeRequest {
        operation: Some("complete".to_string()),
        content: None,
        session_id: Some("s-tree-complete".to_string()),
        branch_id: Some(real_branch_id),
        num_branches: None,
        completed: Some(true),
    };
    let complete_resp = server.reasoning_tree(Parameters(complete_req)).await;

    // Success path: session_id preserved, no error message in recommendation
    assert_eq!(complete_resp.session_id, "s-tree-complete");
    let rec = complete_resp.recommendation.as_deref().unwrap_or("");
    assert!(!rec.contains("complete failed"), "unexpected error: {rec}");
}

// ============================================================================
// Meta handler success path with API-based classification
// ============================================================================

/// Mock JSON for MetaMode::classify_problem (no problem_type_hint provided).
fn meta_classify_json() -> String {
    serde_json::json!({
        "problem_type": "sequential_analysis",
        "reasoning": "The problem requires clear step-by-step evaluation"
    })
    .to_string()
}

#[tokio::test]
async fn test_meta_success_path_with_api_classification() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&meta_classify_json())),
        )
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;
    let req = MetaRequest {
        content: "Analyze the sequential steps for deploying a new service".to_string(),
        problem_type_hint: None, // No hint → API classifies the problem
        min_confidence: None,
    };

    let resp = server.reasoning_meta(Parameters(req)).await;
    // API classifies successfully, but no effectiveness data → falls back to "auto"
    // Key: the Ok path is reached (not an error path)
    assert!(!resp.problem_type.is_empty());
    assert!(!resp.reasoning.is_empty());
    // With empty metrics, fallback_to_auto should be true
    assert!(resp.fallback_to_auto);
    assert_eq!(resp.selected_tool, "auto");
}
