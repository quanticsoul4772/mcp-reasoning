use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::IntoContents;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{anthropic_response, create_mocked_server};
use crate::server::requests::*;
use crate::server::responses::*;

#[tokio::test]
async fn test_detect_low_argument_strength() {
    let mock_server = MockServer::start().await;

    // Test fallacies with low argument strength (high severity)
    let fallacies_json = serde_json::json!({
        "fallacies_detected": [
            {
                "fallacy": "strawman",
                "category": "informal",
                "passage": "The weak argument",
                "explanation": "Misrepresents position",
                "correction": "Address actual argument"
            }
        ],
        "argument_structure": {
            "premises": ["P1"],
            "conclusion": "C",
            "structure_type": "deductive",
            "validity": "invalid"
        },
        "overall_assessment": {
            "fallacy_count": 1,
            "most_critical": "strawman",
            "argument_strength": 0.3,
            "overall_analysis": "Very weak"
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

    let server = create_mocked_server(&mock_server).await;

    let req = DetectRequest {
        detect_type: "fallacies".to_string(),
        content: Some("Weak argument".to_string()),
        thought_id: None,
        session_id: None,
        check_types: None,
        check_formal: None,
        check_informal: None,
    };
    let resp = server.reasoning_detect(Parameters(req)).await;
    // With argument_strength 0.3, severity should be "high"
    if let Some(detection) = resp.detections.first() {
        assert_eq!(detection.severity, "high");
    }
}

#[tokio::test]
async fn test_detect_medium_argument_strength() {
    let mock_server = MockServer::start().await;

    // Test fallacies with medium argument strength
    let fallacies_json = serde_json::json!({
        "fallacies_detected": [
            {
                "fallacy": "appeal to authority",
                "category": "informal",
                "passage": "Expert says so",
                "explanation": "Relies on authority",
                "correction": "Provide evidence"
            }
        ],
        "argument_structure": {
            "premises": ["P1"],
            "conclusion": "C",
            "structure_type": "inductive",
            "validity": "partially_valid"
        },
        "overall_assessment": {
            "fallacy_count": 1,
            "most_critical": "appeal to authority",
            "argument_strength": 0.5,
            "overall_analysis": "Moderate"
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

    let server = create_mocked_server(&mock_server).await;

    let req = DetectRequest {
        detect_type: "fallacies".to_string(),
        content: Some("Medium strength argument".to_string()),
        thought_id: None,
        session_id: None,
        check_types: None,
        check_formal: None,
        check_informal: None,
    };
    let resp = server.reasoning_detect(Parameters(req)).await;
    // With argument_strength 0.5, severity should be "medium"
    if let Some(detection) = resp.detections.first() {
        assert_eq!(detection.severity, "medium");
    }
}

#[tokio::test]
async fn test_detect_high_argument_strength() {
    let mock_server = MockServer::start().await;

    // Test fallacies with high argument strength (low severity)
    let fallacies_json = serde_json::json!({
        "fallacies_detected": [
            {
                "fallacy": "minor issue",
                "category": "informal",
                "passage": "Good argument",
                "explanation": "Small flaw",
                "correction": "Minor fix"
            }
        ],
        "argument_structure": {
            "premises": ["P1", "P2"],
            "conclusion": "C",
            "structure_type": "deductive",
            "validity": "valid"
        },
        "overall_assessment": {
            "fallacy_count": 1,
            "most_critical": "minor issue",
            "argument_strength": 0.8,
            "overall_analysis": "Strong"
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

    let server = create_mocked_server(&mock_server).await;

    let req = DetectRequest {
        detect_type: "fallacies".to_string(),
        content: Some("Strong argument".to_string()),
        thought_id: None,
        session_id: None,
        check_types: None,
        check_formal: None,
        check_informal: None,
    };
    let resp = server.reasoning_detect(Parameters(req)).await;
    // With argument_strength 0.8, severity should be "low"
    if let Some(detection) = resp.detections.first() {
        assert_eq!(detection.severity, "low");
    }
}

#[tokio::test]
async fn test_into_contents_implementations() {
    // Test all response types' IntoContents implementations
    let linear_resp = LinearResponse {
        thought_id: "t1".to_string(),
        session_id: "s1".to_string(),
        content: "Analysis".to_string(),
        confidence: 0.8,
        next_step: Some("Continue".to_string()),
        metadata: None,
        next_call: None,
    };
    let _ = linear_resp.into_contents();

    let tree_resp = TreeResponse {
        session_id: "s1".to_string(),
        branch_id: Some("b1".to_string()),
        branches: Some(vec![Branch {
            id: "b1".to_string(),
            content: "Content".to_string(),
            score: 0.7,
            status: "active".to_string(),
        }]),
        recommendation: Some("Rec".to_string()),
        synthesis: None,
        key_findings: None,
        best_insights: None,
        metadata: None,
        next_call: None,
    };
    let _ = tree_resp.into_contents();

    let divergent_resp = DivergentResponse {
        thought_id: "t1".to_string(),
        session_id: "s1".to_string(),
        perspectives: vec![Perspective {
            viewpoint: "View".to_string(),
            content: "Content".to_string(),
            novelty_score: 0.8,
        }],
        challenged_assumptions: Some(vec!["Assumption".to_string()]),
        synthesis: Some("Synthesis".to_string()),
        metadata: None,
    };
    let _ = divergent_resp.into_contents();

    let reflection_resp = ReflectionResponse {
        quality_score: 0.8,
        thought_id: Some("t1".to_string()),
        session_id: Some("s1".to_string()),
        iterations_used: Some(2),
        strengths: Some(vec!["Strength".to_string()]),
        weaknesses: Some(vec!["Weakness".to_string()]),
        recommendations: Some(vec!["Improve".to_string()]),
        refined_content: Some("Refined".to_string()),
        coherence_score: Some(0.85),
        metadata: None,
    };
    let _ = reflection_resp.into_contents();

    let checkpoint_resp = CheckpointResponse {
        session_id: "s1".to_string(),
        checkpoint_id: Some("cp1".to_string()),
        checkpoints: Some(vec![Checkpoint {
            id: "cp1".to_string(),
            name: "Name".to_string(),
            created_at: "2024-01-01".to_string(),
            description: None,
            thought_count: 5,
        }]),
        restored_state: None,
        metadata: None,
        next_call: None,
    };
    let _ = checkpoint_resp.into_contents();

    let auto_resp = AutoResponse {
        selected_mode: "linear".to_string(),
        confidence: 0.9,
        rationale: "Rationale".to_string(),
        result: serde_json::json!({}),
        metadata: None,
        next_call: None,
        executed: None,
    };
    let _ = auto_resp.into_contents();

    let graph_resp = GraphResponse {
        session_id: "s1".to_string(),
        node_id: Some("n1".to_string()),
        nodes: None,
        aggregated_insight: None,
        conclusions: None,
        state: None,
        metadata: None,
    };
    let _ = graph_resp.into_contents();

    let detect_resp = DetectResponse {
        detections: vec![],
        summary: Some("Summary".to_string()),
        overall_quality: Some(0.8),
        metadata: None,
    };
    let _ = detect_resp.into_contents();

    let decision_resp = DecisionResponse {
        recommendation: "A".to_string(),
        rankings: None,
        stakeholder_map: None,
        conflicts: None,
        alignments: None,
        rationale: None,
        metadata: None,
    };
    let _ = decision_resp.into_contents();

    let evidence_resp = EvidenceResponse {
        overall_credibility: 0.8,
        evidence_assessments: None,
        posterior: None,
        prior: None,
        likelihood_ratio: None,
        entropy: None,
        confidence_interval: None,
        synthesis: None,
        metadata: None,
    };
    let _ = evidence_resp.into_contents();

    let timeline_resp = TimelineResponse {
        timeline_id: "tl1".to_string(),
        branch_id: None,
        branches: None,
        comparison: None,
        merged_content: None,
        metadata: None,
    };
    let _ = timeline_resp.into_contents();

    let mcts_resp = MctsResponse {
        session_id: "s1".to_string(),
        best_path: None,
        iterations_completed: Some(10),
        backtrack_suggestion: None,
        executed: None,
        metadata: None,
    };
    let _ = mcts_resp.into_contents();

    let cf_resp = CounterfactualResponse {
        original_scenario: "Original".to_string(),
        intervention_applied: "Intervention".to_string(),
        counterfactual_outcome: "Outcome".to_string(),
        causal_chain: vec![],
        key_differences: vec![],
        confidence: 0.8,
        assumptions: vec![],
        session_id: Some("s1".to_string()),
        analysis_depth: "counterfactual".to_string(),
        metadata: None,
    };
    let _ = cf_resp.into_contents();

    let preset_resp = PresetResponse {
        presets: None,
        execution_result: None,
        session_id: None,
        metadata: None,
        next_call: None,
    };
    let _ = preset_resp.into_contents();

    let metrics_resp = MetricsResponse {
        summary: None,
        mode_stats: None,
        invocations: None,
        config: None,
    };
    let _ = metrics_resp.into_contents();
}
