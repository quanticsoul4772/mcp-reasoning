//! Integration tests for tool handlers with wiremock.
//!
//! These tests verify the tool handlers work correctly with mocked API responses.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreadable_literal
)]

use mcp_reasoning::anthropic::{AnthropicClient, ClientConfig};
use mcp_reasoning::config::{Config, SecretString};
use mcp_reasoning::metrics::MetricsCollector;
use mcp_reasoning::self_improvement::ManagerHandle;
use mcp_reasoning::server::AppState;
use mcp_reasoning::storage::SqliteStorage;
use serde_json::json;
use std::sync::Arc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Test Utilities
// ============================================================================

/// Create Anthropic API success response body.
fn anthropic_response(text: &str) -> serde_json::Value {
    json!({
        "id": "msg_test",
        "content": [{"type": "text", "text": text}],
        "model": "claude-3",
        "usage": {"input_tokens": 100, "output_tokens": 50},
        "stop_reason": "end_turn"
    })
}

/// Create test `AppState` with mocked Anthropic client.
async fn create_test_state(server: &MockServer) -> AppState {
    let storage = SqliteStorage::new_in_memory().await.unwrap();
    let metrics = Arc::new(MetricsCollector::new());
    let si_handle = ManagerHandle::for_testing();
    let client_config = ClientConfig::default()
        .with_base_url(server.uri())
        .with_max_retries(0)
        .with_timeout_ms(5_000);
    let client = AnthropicClient::new("test-api-key", client_config).unwrap();
    let config = Config {
        api_key: SecretString::new("test-key"),
        database_path: ":memory:".to_string(),
        log_level: "info".to_string(),
        request_timeout_ms: 30000,
        request_timeout_deep_ms: 60000,
        request_timeout_maximum_ms: 120000,
        max_retries: 3,
        model: "claude-sonnet-4-20250514".to_string(),
    };

    let metadata_builder = mcp_reasoning::metadata::MetadataBuilder::new(
        Arc::new(mcp_reasoning::metadata::TimingDatabase::new(Arc::new(
            storage.clone(),
        ))),
        Arc::new(mcp_reasoning::metadata::PresetIndex::build()),
        30000,
    );
    let (progress_tx, _rx) = tokio::sync::broadcast::channel(100);
    AppState::new(
        storage,
        client,
        config,
        metrics,
        si_handle,
        metadata_builder,
        progress_tx,
    )
}

// ============================================================================
// Linear Mode Tests
// ============================================================================

#[tokio::test]
async fn test_linear_mode_success() {
    use mcp_reasoning::modes::LinearMode;

    let server = MockServer::start().await;

    // Mock successful API response with valid JSON
    let response_json = json!({
        "analysis": "This is a thoughtful analysis of the content.",
        "confidence": 0.85,
        "next_step": "Consider exploring related topics."
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = LinearMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.process("Test content", None, None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert_eq!(
        response.content,
        "This is a thoughtful analysis of the content."
    );
    assert!((response.confidence - 0.85).abs() < 0.001);
    assert_eq!(
        response.next_step,
        Some("Consider exploring related topics.".to_string())
    );
}

#[tokio::test]
async fn test_linear_mode_missing_analysis() {
    use mcp_reasoning::modes::LinearMode;

    let server = MockServer::start().await;

    // Mock response missing "analysis" field
    let response_json = json!({
        "confidence": 0.85,
        "next_step": "Do something"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = LinearMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.process("Test content", None, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("analysis"));
}

#[tokio::test]
async fn test_linear_mode_invalid_confidence() {
    use mcp_reasoning::modes::LinearMode;

    let server = MockServer::start().await;

    // Mock response with invalid confidence > 1.0
    let response_json = json!({
        "analysis": "Some analysis",
        "confidence": 1.5
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = LinearMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.process("Test content", None, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("confidence"));
}

#[tokio::test]
async fn test_linear_mode_below_min_confidence() {
    use mcp_reasoning::modes::LinearMode;

    let server = MockServer::start().await;

    // Mock response with confidence below threshold
    let response_json = json!({
        "analysis": "Low confidence analysis",
        "confidence": 0.3
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = LinearMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.process("Test content", None, Some(0.5)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("below minimum"));
}

// ============================================================================
// Tree Mode Tests
// ============================================================================

#[tokio::test]
async fn test_tree_mode_create_success() {
    use mcp_reasoning::modes::TreeMode;

    let server = MockServer::start().await;

    let response_json = json!({
        "branches": [
            {"id": "b1", "content": "Branch 1 exploration", "score": 0.8, "status": "active"},
            {"id": "b2", "content": "Branch 2 exploration", "score": 0.7, "status": "active"}
        ],
        "recommendation": "Start with branch 1 as it has higher potential."
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mut mode = TreeMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.create("Test exploration topic", None, Some(2)).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert!(response.branches.is_some());
    let branches = response.branches.unwrap();
    assert_eq!(branches.len(), 2);
}

// ============================================================================
// Divergent Mode Tests
// ============================================================================

#[tokio::test]
async fn test_divergent_mode_success() {
    use mcp_reasoning::modes::DivergentMode;

    let server = MockServer::start().await;

    let response_json = json!({
        "perspectives": [
            {"viewpoint": "Technical", "content": "From a technical standpoint...", "novelty_score": 0.7},
            {"viewpoint": "Business", "content": "From a business perspective...", "novelty_score": 0.6}
        ],
        "challenged_assumptions": ["Assumption 1", "Assumption 2"],
        "synthesis": "Combining these perspectives reveals..."
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = DivergentMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode
        .process("Test topic", None, Some(2), false, false)
        .await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert_eq!(response.perspectives.len(), 2);
    assert!(response.synthesis.is_some());
}

// ============================================================================
// Auto Mode Tests
// ============================================================================

#[tokio::test]
async fn test_auto_mode_selection() {
    use mcp_reasoning::modes::AutoMode;

    let server = MockServer::start().await;

    let response_json = json!({
        "selected_mode": "linear",
        "reasoning": "This is a straightforward analysis task.",
        "characteristics": ["sequential", "analytical"],
        "suggested_parameters": {"max_tokens": 4096}
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = AutoMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.select("Analyze this simple problem", None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert_eq!(response.selected_mode.to_string(), "linear");
}

// ============================================================================
// Reflection Mode Tests
// ============================================================================

#[tokio::test]
async fn test_reflection_mode_process() {
    use mcp_reasoning::modes::ReflectionMode;

    let server = MockServer::start().await;

    // JSON structure must match reflection/parsing.rs + mod.rs expectations:
    // - analysis: { strengths: [], weaknesses: [] }
    // - improvements: [{ issue, suggestion, priority }]
    // - refined_reasoning: string
    let response_json = json!({
        "analysis": {
            "strengths": ["Clear structure", "Good examples"],
            "weaknesses": ["Could be more concise"]
        },
        "improvements": [
            {"issue": "Lack of conciseness", "suggestion": "Remove redundant phrases", "priority": "high"}
        ],
        "refined_reasoning": "Improved version of the reasoning with better structure and conciseness."
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = ReflectionMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.process("Some reasoning to improve", None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert!(!response.analysis.strengths.is_empty());
    assert!(!response.improvements.is_empty());
}

// ============================================================================
// Detect Mode Tests
// ============================================================================

#[tokio::test]
async fn test_detect_mode_biases() {
    use mcp_reasoning::modes::DetectMode;

    let server = MockServer::start().await;

    // JSON structure must match detect/parsing.rs expectations:
    // - biases_detected: [{ bias, evidence, severity, impact, debiasing }]
    // - overall_assessment: { bias_count, most_severe, reasoning_quality }
    // - debiased_version: string
    let response_json = json!({
        "biases_detected": [
            {
                "bias": "confirmation bias",
                "evidence": "Only supporting evidence cited",
                "severity": "high",
                "impact": "Skews conclusions",
                "debiasing": "Seek disconfirming evidence"
            }
        ],
        "overall_assessment": {
            "bias_count": 1,
            "most_severe": "confirmation bias",
            "reasoning_quality": 0.6
        },
        "debiased_version": "A more balanced analysis would consider..."
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = DetectMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.biases("Some biased reasoning to analyze", None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert!(!response.biases_detected.is_empty());
    assert_eq!(response.overall_assessment.bias_count, 1);
}

#[tokio::test]
async fn test_detect_mode_fallacies() {
    use mcp_reasoning::modes::DetectMode;

    let server = MockServer::start().await;

    // JSON structure must match detect/parsing.rs expectations:
    // - fallacies_detected: [{ fallacy, category, passage, explanation, correction }]
    // - argument_structure: { premises, conclusion, validity }
    // - overall_assessment: { fallacy_count, argument_strength, most_critical }
    let response_json = json!({
        "fallacies_detected": [
            {
                "fallacy": "ad hominem",
                "category": "informal",
                "passage": "He's wrong because he's biased",
                "explanation": "Attacks person not argument",
                "correction": "Address the argument directly"
            }
        ],
        "argument_structure": {
            "premises": ["Premise 1", "Premise 2"],
            "conclusion": "Therefore conclusion",
            "validity": "partially_valid"
        },
        "overall_assessment": {
            "fallacy_count": 1,
            "argument_strength": 0.5,
            "most_critical": "ad hominem"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = DetectMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.fallacies("Some argument with fallacies", None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert!(!response.fallacies_detected.is_empty());
    assert_eq!(response.overall_assessment.fallacy_count, 1);
}

// ============================================================================
// Evidence Mode Tests
// ============================================================================

#[tokio::test]
async fn test_evidence_mode_assess() {
    use mcp_reasoning::modes::EvidenceMode;

    let server = MockServer::start().await;

    // JSON structure must match evidence/parsing.rs expectations:
    // - evidence_pieces: [{ summary, source_type, credibility: {...}, quality: {...} }]
    // - overall_assessment: { evidential_support, key_strengths, key_weaknesses, gaps }
    // - confidence_in_conclusion
    let response_json = json!({
        "evidence_pieces": [
            {
                "summary": "Study shows correlation",
                "source_type": "primary",
                "credibility": {
                    "expertise": 0.9,
                    "objectivity": 0.8,
                    "corroboration": 0.7,
                    "recency": 0.85,
                    "overall": 0.81
                },
                "quality": {
                    "relevance": 0.9,
                    "strength": 0.75,
                    "representativeness": 0.8,
                    "overall": 0.82
                }
            }
        ],
        "overall_assessment": {
            "evidential_support": 0.8,
            "key_strengths": ["Strong primary sources"],
            "key_weaknesses": ["Limited sample size"],
            "gaps": ["Missing longitudinal data"]
        },
        "confidence_in_conclusion": 0.75
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = EvidenceMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.assess("Evidence to assess for claim X", None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert!(!response.evidence_pieces.is_empty());
}

#[tokio::test]
async fn test_evidence_mode_probabilistic() {
    use mcp_reasoning::modes::EvidenceMode;

    let server = MockServer::start().await;

    // JSON structure must match evidence/parsing.rs expectations:
    // - hypothesis: string
    // - prior: { probability, basis }
    // - evidence_analysis: [{ evidence, likelihood_if_true, likelihood_if_false, bayes_factor }]
    // - posterior: { probability, calculation }
    // - belief_update: { direction, magnitude, interpretation }
    // - sensitivity: string
    let response_json = json!({
        "hypothesis": "The treatment is effective",
        "prior": {
            "probability": 0.3,
            "basis": "Base rate from historical data"
        },
        "evidence_analysis": [
            {
                "evidence": "Positive test result",
                "likelihood_if_true": 0.9,
                "likelihood_if_false": 0.1,
                "bayes_factor": 9.0
            }
        ],
        "posterior": {
            "probability": 0.79,
            "calculation": "P(H|E) = P(E|H)P(H) / P(E)"
        },
        "belief_update": {
            "direction": "increase",
            "magnitude": "strong",
            "interpretation": "Evidence strongly supports hypothesis"
        },
        "sensitivity": "Results sensitive to prior probability assumptions"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = EvidenceMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode
        .probabilistic(
            "Hypothesis: The treatment is effective. Evidence: Positive test result.",
            None,
        )
        .await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert!(response.posterior.probability > response.prior.probability);
}

// ============================================================================
// Decision Mode Tests
// ============================================================================

#[tokio::test]
async fn test_decision_mode_weighted() {
    use mcp_reasoning::modes::DecisionMode;

    let server = MockServer::start().await;

    // JSON structure must match decision/parsing.rs + mod.rs expectations:
    // - options: [string]
    // - criteria: [{ name, weight, description }]
    // - scores: { option -> { criterion -> score }}
    // - weighted_totals: { option -> total }
    // - ranking: [{ option, score, rank }]
    // - sensitivity_notes: string
    let response_json = json!({
        "options": ["Option A", "Option B"],
        "criteria": [
            {"name": "Cost", "weight": 0.4, "description": "Cost efficiency"},
            {"name": "Quality", "weight": 0.6, "description": "Quality level"}
        ],
        "scores": {
            "Option A": {"Cost": 0.8, "Quality": 0.6},
            "Option B": {"Cost": 0.5, "Quality": 0.9}
        },
        "weighted_totals": {"Option A": 0.68, "Option B": 0.74},
        "ranking": [
            {"option": "Option B", "score": 0.74, "rank": 1},
            {"option": "Option A", "score": 0.68, "rank": 2}
        ],
        "sensitivity_notes": "Results are sensitive to quality weight"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = DecisionMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode
        .weighted("Choose between A and B based on cost and quality", None)
        .await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert_eq!(response.ranking.len(), 2);
    assert_eq!(response.ranking[0].option, "Option B");
}

// ============================================================================
// Graph Mode Tests
// ============================================================================

#[tokio::test]
async fn test_graph_mode_init() {
    use mcp_reasoning::modes::GraphMode;

    let server = MockServer::start().await;

    // JSON structure must match graph/parsing.rs expectations:
    // - root: { id, content, score, type }
    // - expansion_directions: [{ direction, potential }]
    // - graph_metadata: { complexity, estimated_depth }
    let response_json = json!({
        "root": {
            "id": "root-1",
            "content": "Main topic",
            "score": 0.9,
            "type": "root"
        },
        "expansion_directions": [
            {"direction": "Technical exploration", "potential": 0.8},
            {"direction": "Business analysis", "potential": 0.7}
        ],
        "graph_metadata": {
            "complexity": "medium",
            "estimated_depth": 3
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = GraphMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.init("Test topic for graph exploration", None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert_eq!(response.root.content, "Main topic");
}

// ============================================================================
// Timeline Mode Tests
// ============================================================================

#[tokio::test]
async fn test_timeline_mode_create() {
    use mcp_reasoning::modes::TimelineMode;

    let server = MockServer::start().await;

    // JSON structure must match timeline/parsing.rs + mod.rs expectations:
    // - timeline_id: string
    // - events: [{ id, description, time, type }]
    // - decision_points: [{ id, description, options, deadline }]
    // - temporal_structure: { start, current, horizon }
    let response_json = json!({
        "timeline_id": "tl-1",
        "events": [
            {"id": "e1", "description": "Event 1", "time": "T0", "type": "event"},
            {"id": "e2", "description": "Event 2", "time": "T1", "type": "state"}
        ],
        "decision_points": [
            {"id": "d1", "description": "Key decision", "options": ["A", "B"], "deadline": "T2"}
        ],
        "temporal_structure": {
            "start": "T0",
            "current": "T1",
            "horizon": "T5"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = TimelineMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.create("Scenario to analyze temporally", None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert!(!response.events.is_empty());
}

// ============================================================================
// MCTS Mode Tests
// ============================================================================

#[tokio::test]
async fn test_mcts_mode_explore() {
    use mcp_reasoning::modes::MctsMode;

    let server = MockServer::start().await;

    // JSON structure must match mcts/parsing.rs expectations:
    // - frontier_evaluation: [{ node_id, visits, average_value, ucb1_score, exploration_bonus }]
    // - selected_node: { node_id, selection_reason }
    // - expansion: { new_nodes: [{ id, content, simulated_value }] }
    // - backpropagation: { updated_nodes, value_changes }
    // - search_status: { total_nodes, total_simulations, best_path_value }
    let response_json = json!({
        "frontier_evaluation": [
            {"node_id": "n1", "visits": 5, "average_value": 0.7, "ucb1_score": 0.8, "exploration_bonus": 0.1}
        ],
        "selected_node": {
            "node_id": "n1",
            "selection_reason": "Highest UCB1 score"
        },
        "expansion": {
            "new_nodes": [
                {"id": "n2", "content": "New exploration path", "simulated_value": 0.5}
            ]
        },
        "backpropagation": {
            "updated_nodes": ["n1"],
            "value_changes": {"n1": 0.1}
        },
        "search_status": {
            "total_nodes": 10,
            "total_simulations": 50,
            "best_path_value": 0.85
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = MctsMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.explore("Current search state", None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert!(!response.expansion.new_nodes.is_empty());
}

// ============================================================================
// Counterfactual Mode Tests
// ============================================================================

#[tokio::test]
async fn test_counterfactual_mode_analyze() {
    use mcp_reasoning::modes::CounterfactualMode;

    let server = MockServer::start().await;

    // JSON structure must match counterfactual.rs types (uses serde deserialization):
    // - causal_question: { statement, ladder_rung, variables: { cause, effect, intervention }}
    // - causal_model: { nodes, edges: [{ from, to, type }], confounders }
    // - analysis: { association_level: { observed_correlation, interpretation },
    //               intervention_level: { causal_effect, mechanism },
    //               counterfactual_level: { scenario, outcome, confidence }}
    // - conclusions: { causal_claim, strength, caveats, actionable_insight }
    let response_json = json!({
        "causal_question": {
            "statement": "What if X had been different?",
            "ladder_rung": "counterfactual",
            "variables": {
                "cause": "X",
                "effect": "Y",
                "intervention": "Change X"
            }
        },
        "causal_model": {
            "nodes": ["X", "Y", "Z"],
            "edges": [
                {"from": "X", "to": "Y", "type": "direct"}
            ],
            "confounders": ["Z"]
        },
        "analysis": {
            "association_level": {
                "observed_correlation": 0.7,
                "interpretation": "Strong positive correlation observed"
            },
            "intervention_level": {
                "causal_effect": 0.5,
                "mechanism": "Direct influence through pathway"
            },
            "counterfactual_level": {
                "scenario": "If X had been lower",
                "outcome": "Y would have been lower",
                "confidence": 0.75
            }
        },
        "conclusions": {
            "causal_claim": "X causes Y",
            "strength": "moderate",
            "caveats": ["Confounding possible"],
            "actionable_insight": "Consider adjusting X to influence Y"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&response_json.to_string())),
        )
        .mount(&server)
        .await;

    let state = create_test_state(&server).await;
    let mode = CounterfactualMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    let result = mode.analyze("What if X had been different?", None).await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    let response = result.unwrap();
    assert_eq!(
        response.causal_question.statement,
        "What if X had been different?"
    );
}

// ============================================================================
// Checkpoint Mode Tests
// ============================================================================

#[tokio::test]
async fn test_checkpoint_mode_create_and_list() {
    use mcp_reasoning::modes::{CheckpointContext, CheckpointMode};
    use mcp_reasoning::traits::StorageTrait;

    let server = MockServer::start().await;
    let state = create_test_state(&server).await;

    let mode = CheckpointMode::new(Arc::clone(&state.storage), Arc::clone(&state.client));

    // Create a session first
    let session = state
        .storage
        .get_or_create_session(Some("test-session".to_string()))
        .await
        .unwrap();

    // Create a checkpoint
    let context = CheckpointContext::new(
        vec!["Finding 1".to_string()],
        "current focus",
        vec!["Question 1".to_string()],
    );

    let result = mode
        .create(
            &session.id,
            "test-checkpoint",
            Some("Test description"),
            context,
            "Resume here",
        )
        .await;
    assert!(result.is_ok(), "Expected success, got: {result:?}");

    // List checkpoints
    let list_result = mode.list(&session.id).await;
    assert!(list_result.is_ok());
    let list = list_result.unwrap();
    assert!(!list.checkpoints.is_empty());
}

// ============================================================================
// ReasoningServer Tool Handler Tests
// ============================================================================

mod server_tests {
    use super::*;
    use mcp_reasoning::server::{
        AutoRequest, CheckpointRequest, CounterfactualRequest, DecisionRequest, DetectRequest,
        DivergentRequest, EvidenceRequest, GraphRequest, LinearRequest, MctsRequest,
        MetricsRequest, PresetRequest, ReasoningServer, ReflectionRequest, TimelineRequest,
        TreeRequest,
    };

    /// Helper to create a `ReasoningServer` for testing.
    async fn create_server(mock_server: &MockServer) -> ReasoningServer {
        let state = create_test_state(mock_server).await;
        ReasoningServer::new(Arc::new(state))
    }

    // Linear handler tests
    #[tokio::test]
    async fn test_server_reasoning_linear() {
        let server = MockServer::start().await;

        let response_json = json!({
            "analysis": "Deep analysis of the topic",
            "confidence": 0.85,
            "next_step": "Consider implications"
        });

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(anthropic_response(&response_json.to_string())),
            )
            .mount(&server)
            .await;

        let _reasoning_server = create_server(&server).await;

        // ReasoningServer is created successfully
        // The actual tool handlers are tested via mode tests above
    }

    // Test request types serialization/deserialization
    #[test]
    fn test_linear_request_with_all_fields() {
        let req = LinearRequest {
            content: "test content".to_string(),
            session_id: Some("session-1".to_string()),
            confidence: Some(0.8),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("test content"));
        assert!(json.contains("session-1"));
        assert!(json.contains("0.8"));

        let deserialized: LinearRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, req.content);
    }

    #[test]
    fn test_tree_request_with_all_operations() {
        let create_req = TreeRequest {
            operation: Some("create".to_string()),
            content: Some("Topic to explore".to_string()),
            session_id: Some("s1".to_string()),
            branch_id: None,
            num_branches: Some(3),
            completed: None,
        };
        let json = serde_json::to_string(&create_req).unwrap();
        assert!(json.contains("create"));

        let focus_req = TreeRequest {
            operation: Some("focus".to_string()),
            content: None,
            session_id: Some("s1".to_string()),
            branch_id: Some("b1".to_string()),
            num_branches: None,
            completed: None,
        };
        let json = serde_json::to_string(&focus_req).unwrap();
        assert!(json.contains("focus"));
        assert!(json.contains("b1"));
    }

    #[test]
    fn test_divergent_request_serialization() {
        let req = DivergentRequest {
            content: "Analyze this".to_string(),
            session_id: None,
            num_perspectives: Some(4),
            challenge_assumptions: Some(true),
            force_rebellion: Some(false),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Analyze this"));
        assert!(json.contains("num_perspectives"));
        assert!(json.contains("challenge_assumptions"));
    }

    #[test]
    fn test_reflection_request_operations() {
        let process_req = ReflectionRequest {
            operation: Some("process".to_string()),
            content: Some("Reasoning to reflect on".to_string()),
            thought_id: None,
            session_id: None,
            max_iterations: Some(3),
            quality_threshold: Some(0.8),
        };
        let json = serde_json::to_string(&process_req).unwrap();
        assert!(json.contains("process"));
        assert!(json.contains("max_iterations"));
    }

    #[test]
    fn test_auto_request_serialization() {
        let req = AutoRequest {
            content: "Content for auto-routing".to_string(),
            session_id: None,
            hints: Some(vec!["technical".to_string(), "complex".to_string()]),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("auto-routing"));
        assert!(json.contains("hints"));
    }

    #[test]
    fn test_graph_request_all_operations() {
        let init_req = GraphRequest {
            operation: "init".to_string(),
            session_id: "s1".to_string(),
            content: Some("Problem to explore".to_string()),
            node_id: None,
            node_ids: None,
            k: None,
            problem: None,
            threshold: None,
            terminal_node_ids: None,
        };
        let json = serde_json::to_string(&init_req).unwrap();
        assert!(json.contains("init"));

        let generate_req = GraphRequest {
            operation: "generate".to_string(),
            session_id: "s1".to_string(),
            content: None,
            node_id: Some("n1".to_string()),
            node_ids: None,
            k: Some(3),
            problem: Some("The problem".to_string()),
            threshold: None,
            terminal_node_ids: None,
        };
        let json = serde_json::to_string(&generate_req).unwrap();
        assert!(json.contains("generate"));
    }

    #[test]
    fn test_detect_request_types() {
        let biases_req = DetectRequest {
            detect_type: "biases".to_string(),
            content: Some("Content to analyze for biases".to_string()),
            session_id: None,
            thought_id: None,
            check_formal: None,
            check_informal: Some(true),
            check_types: Some(vec!["confirmation".to_string()]),
        };
        let json = serde_json::to_string(&biases_req).unwrap();
        assert!(json.contains("biases"));
    }

    #[test]
    fn test_decision_request_types() {
        let weighted_req = DecisionRequest {
            decision_type: Some("weighted".to_string()),
            question: Some("Which option is best?".to_string()),
            options: Some(vec!["A".to_string(), "B".to_string()]),
            context: Some("Business context".to_string()),
            topic: None,
            session_id: None,
        };
        let json = serde_json::to_string(&weighted_req).unwrap();
        assert!(json.contains("weighted"));

        let perspectives_req = DecisionRequest {
            decision_type: Some("perspectives".to_string()),
            question: None,
            options: None,
            context: None,
            topic: Some("Stakeholder analysis".to_string()),
            session_id: None,
        };
        let json = serde_json::to_string(&perspectives_req).unwrap();
        assert!(json.contains("perspectives"));
    }

    #[test]
    fn test_evidence_request_types() {
        let assess_req = EvidenceRequest {
            evidence_type: Some("assess".to_string()),
            claim: Some("The claim to evaluate".to_string()),
            hypothesis: None,
            prior: None,
            context: Some("Scientific context".to_string()),
            session_id: None,
        };
        let json = serde_json::to_string(&assess_req).unwrap();
        assert!(json.contains("assess"));

        let prob_req = EvidenceRequest {
            evidence_type: Some("probabilistic".to_string()),
            claim: None,
            hypothesis: Some("Hypothesis to update".to_string()),
            prior: Some(0.3),
            context: None,
            session_id: None,
        };
        let json = serde_json::to_string(&prob_req).unwrap();
        assert!(json.contains("probabilistic"));
    }

    #[test]
    fn test_timeline_request_operations() {
        let create_req = TimelineRequest {
            operation: "create".to_string(),
            session_id: None,
            timeline_id: None,
            content: Some("Scenario to analyze".to_string()),
            label: None,
            branch_ids: None,
            source_branch_id: None,
            target_branch_id: None,
            merge_strategy: None,
        };
        let json = serde_json::to_string(&create_req).unwrap();
        assert!(json.contains("create"));

        let branch_req = TimelineRequest {
            operation: "branch".to_string(),
            session_id: None,
            timeline_id: Some("tl-1".to_string()),
            content: Some("Branch content".to_string()),
            label: Some("Alternative path".to_string()),
            branch_ids: None,
            source_branch_id: None,
            target_branch_id: None,
            merge_strategy: None,
        };
        let json = serde_json::to_string(&branch_req).unwrap();
        assert!(json.contains("branch"));
    }

    #[test]
    fn test_mcts_request_operations() {
        let explore_req = MctsRequest {
            operation: Some("explore".to_string()),
            content: Some("Search state".to_string()),
            session_id: None,
            node_id: None,
            iterations: Some(10),
            simulation_depth: Some(5),
            exploration_constant: Some(1.41),
            quality_threshold: None,
            lookback_depth: None,
            auto_execute: None,
        };
        let json = serde_json::to_string(&explore_req).unwrap();
        assert!(json.contains("explore"));
        assert!(json.contains("iterations"));

        let backtrack_req = MctsRequest {
            operation: Some("auto_backtrack".to_string()),
            content: None,
            session_id: Some("s1".to_string()),
            node_id: None,
            iterations: None,
            simulation_depth: None,
            exploration_constant: None,
            quality_threshold: Some(0.5),
            lookback_depth: Some(3),
            auto_execute: Some(true),
        };
        let json = serde_json::to_string(&backtrack_req).unwrap();
        assert!(json.contains("auto_backtrack"));
    }

    #[test]
    fn test_counterfactual_request() {
        let req = CounterfactualRequest {
            scenario: "The original scenario".to_string(),
            intervention: "What if X changed".to_string(),
            session_id: None,
            analysis_depth: Some("counterfactual".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("scenario"));
        assert!(json.contains("intervention"));
    }

    #[test]
    fn test_checkpoint_request_operations() {
        let create_req = CheckpointRequest {
            operation: "create".to_string(),
            session_id: "s1".to_string(),
            name: Some("Checkpoint 1".to_string()),
            description: Some("Description".to_string()),
            checkpoint_id: None,
            new_direction: None,
        };
        let json = serde_json::to_string(&create_req).unwrap();
        assert!(json.contains("create"));

        let restore_req = CheckpointRequest {
            operation: "restore".to_string(),
            session_id: "s1".to_string(),
            name: None,
            description: None,
            checkpoint_id: Some("cp1".to_string()),
            new_direction: Some("New exploration direction".to_string()),
        };
        let json = serde_json::to_string(&restore_req).unwrap();
        assert!(json.contains("restore"));
    }

    #[test]
    fn test_preset_request_operations() {
        let list_req = PresetRequest {
            operation: "list".to_string(),
            preset_id: None,
            inputs: None,
            category: Some("analysis".to_string()),
            session_id: None,
        };
        let json = serde_json::to_string(&list_req).unwrap();
        assert!(json.contains("list"));

        let run_req = PresetRequest {
            operation: "run".to_string(),
            preset_id: Some("comprehensive_analysis".to_string()),
            inputs: Some(serde_json::json!({"topic": "Test topic"})),
            category: None,
            session_id: None,
        };
        let json = serde_json::to_string(&run_req).unwrap();
        assert!(json.contains("run"));
    }

    #[test]
    fn test_metrics_request_queries() {
        let summary_req = MetricsRequest {
            query: "summary".to_string(),
            mode_name: None,
            tool_name: None,
            session_id: None,
            limit: None,
            success_only: None,
        };
        let json = serde_json::to_string(&summary_req).unwrap();
        assert!(json.contains("summary"));

        let by_mode_req = MetricsRequest {
            query: "by_mode".to_string(),
            mode_name: Some("linear".to_string()),
            tool_name: None,
            session_id: None,
            limit: Some(100),
            success_only: Some(true),
        };
        let json = serde_json::to_string(&by_mode_req).unwrap();
        assert!(json.contains("by_mode"));
    }
}
