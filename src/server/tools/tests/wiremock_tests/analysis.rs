use rmcp::handler::server::wrapper::Parameters;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::*;

#[tokio::test]
async fn test_graph_all_operations() {
    let mock_server = MockServer::start().await;

    // Setup mock for various graph operations
    let init_json = serde_json::json!({
        "root": {"id": "n1", "content": "Root node", "score": 1.0}
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&init_json.to_string())),
        )
        .expect(1..)
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    // Test init
    let init_req = GraphRequest {
        operation: "init".to_string(),
        session_id: "s1".to_string(),
        content: Some("Problem to explore".to_string()),
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(init_req)).await;
    assert_eq!(resp.session_id, "s1");

    // Test generate
    let generate_json = serde_json::json!({
        "children": [
            {"id": "n2", "content": "Child 1", "score": 0.8},
            {"id": "n3", "content": "Child 2", "score": 0.7}
        ]
    });
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&generate_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let generate_req = GraphRequest {
        operation: "generate".to_string(),
        session_id: "s1".to_string(),
        content: Some("Generate continuations".to_string()),
        problem: None,
        node_id: Some("n1".to_string()),
        node_ids: None,
        k: Some(2),
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(generate_req)).await;
    assert_eq!(resp.session_id, "s1");

    // Test unknown operation
    let unknown_req = GraphRequest {
        operation: "unknown".to_string(),
        session_id: "s1".to_string(),
        content: None,
        problem: None,
        node_id: None,
        node_ids: None,
        k: None,
        threshold: None,
        terminal_node_ids: None,
    };
    let resp = server.reasoning_graph(Parameters(unknown_req)).await;
    assert!(resp
        .aggregated_insight
        .unwrap()
        .contains("Invalid operation"));
}

#[tokio::test]
async fn test_detect_all_types() {
    let mock_server = MockServer::start().await;

    // Test biases
    let biases_json = serde_json::json!({
        "biases_detected": [
            {
                "bias": "confirmation bias",
                "severity": "medium",
                "evidence": "Selected confirming evidence",
                "impact": "May miss counterexamples",
                "debiasing": "Consider opposing views"
            }
        ],
        "overall_assessment": {
            "bias_count": 1,
            "most_severe": "confirmation bias",
            "reasoning_quality": 0.7,
            "overall_analysis": "Some bias detected"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&biases_json.to_string())),
        )
        .expect(1..)
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let biases_req = DetectRequest {
        detect_type: "biases".to_string(),
        content: Some("Argument with potential bias".to_string()),
        thought_id: None,
        session_id: Some("s1".to_string()),
        check_types: None,
        check_formal: None,
        check_informal: None,
    };
    let resp = server.reasoning_detect(Parameters(biases_req)).await;
    assert!(resp.summary.is_some());

    // Test fallacies
    let fallacies_json = serde_json::json!({
        "fallacies_detected": [
            {
                "fallacy": "ad hominem",
                "category": "informal",
                "passage": "The argument",
                "explanation": "Attacks the person",
                "correction": "Address the argument"
            }
        ],
        "argument_structure": {
            "premises": ["Premise 1"],
            "conclusion": "Conclusion",
            "structure_type": "deductive",
            "validity": "invalid"
        },
        "overall_assessment": {
            "fallacy_count": 1,
            "most_critical": "ad hominem",
            "argument_strength": 0.4,
            "overall_analysis": "Weak argument"
        }
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&fallacies_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let fallacies_req = DetectRequest {
        detect_type: "fallacies".to_string(),
        content: Some("Argument with fallacy".to_string()),
        thought_id: None,
        session_id: Some("s1".to_string()),
        check_types: None,
        check_formal: Some(true),
        check_informal: Some(true),
    };
    let resp = server.reasoning_detect(Parameters(fallacies_req)).await;
    assert!(resp.summary.is_some());

    // Test unknown type
    let unknown_req = DetectRequest {
        detect_type: "unknown".to_string(),
        content: Some("Content".to_string()),
        thought_id: None,
        session_id: None,
        check_types: None,
        check_formal: None,
        check_informal: None,
    };
    let resp = server.reasoning_detect(Parameters(unknown_req)).await;
    assert!(resp.summary.unwrap().contains("Unknown"));
}

#[tokio::test]
async fn test_decision_all_types() {
    let mock_server = MockServer::start().await;

    // Test weighted
    let weighted_json = serde_json::json!({
        "criteria": [
            {"name": "Cost", "weight": 0.4, "type": "cost"},
            {"name": "Quality", "weight": 0.6, "type": "benefit"}
        ],
        "decision_matrix": [
            {"option": "A", "scores": {"Cost": 0.8, "Quality": 0.9}},
            {"option": "B", "scores": {"Cost": 0.6, "Quality": 0.7}}
        ],
        "weighted_totals": {"A": 0.86, "B": 0.66},
        "ranking": [
            {"option": "A", "score": 0.86, "rank": 1},
            {"option": "B", "score": 0.66, "rank": 2}
        ],
        "sensitivity_notes": "A is preferred"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&weighted_json.to_string())),
        )
        .expect(1..)
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let weighted_req = DecisionRequest {
        decision_type: Some("weighted".to_string()),
        question: Some("Which option?".to_string()),
        options: Some(vec!["A".to_string(), "B".to_string()]),
        topic: None,
        context: None,
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_decision(Parameters(weighted_req)).await;
    assert!(!resp.recommendation.is_empty() || resp.recommendation.contains("ERROR"));

    // Test pairwise
    let pairwise_json = serde_json::json!({
        "comparisons": [
            {"option_a": "A", "option_b": "B", "preferred": "A", "strength": "strong", "rationale": "Better"}
        ],
        "pairwise_matrix": [[0, 1], [0, 0]],
        "ranking": [
            {"option": "A", "wins": 1, "rank": 1},
            {"option": "B", "wins": 0, "rank": 2}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(anthropic_response(&pairwise_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let pairwise_req = DecisionRequest {
        decision_type: Some("pairwise".to_string()),
        question: Some("Compare options".to_string()),
        options: Some(vec!["A".to_string(), "B".to_string()]),
        topic: None,
        context: None,
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_decision(Parameters(pairwise_req)).await;
    let _ = resp.recommendation;

    // Test unknown type (defaults to weighted)
    let default_req = DecisionRequest {
        decision_type: None,
        question: Some("Question".to_string()),
        options: None,
        topic: None,
        context: None,
        session_id: None,
    };
    let resp = server.reasoning_decision(Parameters(default_req)).await;
    let _ = resp.recommendation;
}

#[tokio::test]
async fn test_evidence_all_types() {
    let mock_server = MockServer::start().await;

    // Test assess
    let assess_json = serde_json::json!({
        "evidence_pieces": [
            {
                "content": "Primary source",
                "credibility_score": 0.9,
                "source_tier": "primary",
                "corroborated_by": [1]
            }
        ],
        "corroboration_matrix": [[1]],
        "overall_credibility": 0.85,
        "synthesis": "Strong evidence"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&assess_json.to_string())),
        )
        .expect(1..)
        .mount(&mock_server)
        .await;

    let server = create_mocked_server(&mock_server).await;

    let assess_req = EvidenceRequest {
        evidence_type: Some("assess".to_string()),
        claim: Some("The claim".to_string()),
        hypothesis: None,
        prior: None,
        context: Some("Context".to_string()),
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_evidence(Parameters(assess_req)).await;
    assert!(resp.overall_credibility >= 0.0);

    // Test probabilistic
    let prob_json = serde_json::json!({
        "prior": 0.5,
        "likelihood_ratio": 2.0,
        "posterior": 0.67,
        "confidence_interval": {"lower": 0.5, "upper": 0.8},
        "hypothesis": "The hypothesis",
        "sensitivity": "Moderate"
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(anthropic_response(&prob_json.to_string())),
        )
        .mount(&mock_server)
        .await;

    let prob_req = EvidenceRequest {
        evidence_type: Some("probabilistic".to_string()),
        claim: None,
        hypothesis: Some("Hypothesis".to_string()),
        prior: Some(0.5),
        context: None,
        session_id: Some("s1".to_string()),
    };
    let resp = server.reasoning_evidence(Parameters(prob_req)).await;
    assert!(resp.overall_credibility >= 0.0);

    // Test unknown type (defaults to assess)
    let default_req = EvidenceRequest {
        evidence_type: None,
        claim: Some("Claim".to_string()),
        hypothesis: None,
        prior: None,
        context: None,
        session_id: None,
    };
    let resp = server.reasoning_evidence(Parameters(default_req)).await;
    assert!(resp.overall_credibility >= 0.0);
}
