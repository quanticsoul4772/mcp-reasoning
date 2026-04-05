use rmcp::model::IntoContents;

use crate::server::responses::*;

#[test]
fn test_linear_response_into_contents() {
    let response = LinearResponse {
        thought_id: "t1".to_string(),
        session_id: "s1".to_string(),
        content: "reasoning content".to_string(),
        confidence: 0.85,
        next_step: Some("continue".to_string()),
        metadata: None,
        next_call: None,
    };
    let contents = response.clone().into_contents();
    assert_eq!(contents.len(), 1);
    // Verify it produces valid JSON content
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("thought_id"));
    assert!(json.contains("t1"));
}

#[test]
fn test_tree_response_into_contents() {
    let response = TreeResponse {
        session_id: "s1".to_string(),
        branch_id: Some("b1".to_string()),
        branches: Some(vec![Branch {
            id: "b1".to_string(),
            content: "branch content".to_string(),
            score: 0.9,
            status: "active".to_string(),
        }]),
        recommendation: Some("explore branch b1".to_string()),
        synthesis: None,
        key_findings: None,
        best_insights: None,
        metadata: None,
        next_call: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_divergent_response_into_contents() {
    let response = DivergentResponse {
        thought_id: "t1".to_string(),
        session_id: "s1".to_string(),
        perspectives: vec![Perspective {
            viewpoint: "optimistic".to_string(),
            content: "positive outlook".to_string(),
            novelty_score: 0.8,
        }],
        challenged_assumptions: Some(vec!["assumption1".to_string()]),
        synthesis: Some("unified insight".to_string()),
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_reflection_response_into_contents() {
    let response = ReflectionResponse {
        quality_score: 0.85,
        thought_id: Some("t1".to_string()),
        session_id: Some("s1".to_string()),
        iterations_used: Some(3),
        strengths: Some(vec!["logical".to_string()]),
        weaknesses: Some(vec!["needs more detail".to_string()]),
        recommendations: Some(vec!["add examples".to_string()]),
        refined_content: Some("improved reasoning".to_string()),
        coherence_score: Some(0.9),
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_checkpoint_response_into_contents() {
    let response = CheckpointResponse {
        session_id: "s1".to_string(),
        checkpoint_id: Some("cp1".to_string()),
        checkpoints: Some(vec![Checkpoint {
            id: "cp1".to_string(),
            name: "checkpoint 1".to_string(),
            description: Some("first checkpoint".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            thought_count: 5,
        }]),
        restored_state: None,
        next_call: None,
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_auto_response_into_contents() {
    let response = AutoResponse {
        selected_mode: "linear".to_string(),
        confidence: 0.9,
        rationale: "simple query".to_string(),
        result: serde_json::json!({"status": "ok"}),
        metadata: None,
        next_call: None,
        executed: None,
        skill_suggestion: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_graph_response_into_contents() {
    let response = GraphResponse {
        session_id: "s1".to_string(),
        node_id: Some("n1".to_string()),
        nodes: Some(vec![GraphNode {
            id: "n1".to_string(),
            content: "node content".to_string(),
            score: Some(0.85),
            depth: Some(1),
            parent_id: None,
        }]),
        aggregated_insight: Some("combined insight".to_string()),
        conclusions: Some(vec!["conclusion 1".to_string()]),
        state: Some(GraphState {
            total_nodes: 10,
            active_nodes: 8,
            max_depth: 3,
            pruned_count: 2,
        }),
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_detect_response_into_contents() {
    let response = DetectResponse {
        detections: vec![Detection {
            detection_type: "confirmation_bias".to_string(),
            category: Some("cognitive".to_string()),
            severity: "medium".to_string(),
            confidence: 0.8,
            evidence: "selective evidence".to_string(),
            explanation: "focusing on confirming data".to_string(),
            remediation: Some("consider counterexamples".to_string()),
        }],
        summary: Some("1 bias detected".to_string()),
        overall_quality: Some(0.7),
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_decision_response_into_contents() {
    let response = DecisionResponse {
        recommendation: "Option A".to_string(),
        rankings: Some(vec![RankedOption {
            option: "Option A".to_string(),
            score: 0.9,
            rank: 1,
        }]),
        stakeholder_map: Some(StakeholderMap {
            key_players: vec!["CEO".to_string()],
            keep_satisfied: vec!["Board".to_string()],
            keep_informed: vec!["Team".to_string()],
            minimal_effort: vec!["Others".to_string()],
        }),
        conflicts: Some(vec!["resource allocation".to_string()]),
        alignments: Some(vec!["shared goals".to_string()]),
        rationale: Some("highest weighted score".to_string()),
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_evidence_response_into_contents() {
    let response = EvidenceResponse {
        overall_credibility: 0.85,
        evidence_assessments: Some(vec![EvidenceAssessment {
            content: "primary source".to_string(),
            credibility_score: 0.9,
            source_tier: "tier1".to_string(),
            corroborated_by: Some(vec![1, 2]),
        }]),
        posterior: Some(0.75),
        prior: Some(0.5),
        likelihood_ratio: Some(3.0),
        entropy: Some(0.2),
        confidence_interval: Some(ConfidenceInterval {
            lower: 0.6,
            upper: 0.9,
        }),
        synthesis: Some("strong evidence".to_string()),
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_timeline_response_into_contents() {
    let response = TimelineResponse {
        timeline_id: "tl1".to_string(),
        branch_id: Some("br1".to_string()),
        branches: Some(vec![TimelineBranch {
            id: "br1".to_string(),
            label: Some("main".to_string()),
            content: "timeline content".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }]),
        comparison: Some(BranchComparison {
            divergence_points: vec!["point1".to_string()],
            quality_differences: serde_json::json!({"score": 0.1}),
            convergence_opportunities: vec!["merge here".to_string()],
        }),
        merged_content: None,
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_mcts_response_into_contents() {
    let response = MctsResponse {
        session_id: "s1".to_string(),
        best_path: Some(vec![MctsNode {
            node_id: "n1".to_string(),
            content: "node content".to_string(),
            visits: 10,
            ucb_score: 1.2,
        }]),
        iterations_completed: Some(50),
        backtrack_suggestion: Some(BacktrackSuggestion {
            should_backtrack: false,
            target_step: None,
            reason: None,
            quality_drop: None,
        }),
        executed: Some(false),
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_counterfactual_response_into_contents() {
    let response = CounterfactualResponse {
        counterfactual_outcome: "different result".to_string(),
        causal_chain: vec![CausalStep {
            step: 1,
            cause: "intervention".to_string(),
            effect: "outcome change".to_string(),
            probability: 0.8,
        }],
        session_id: Some("s1".to_string()),
        original_scenario: "base scenario".to_string(),
        intervention_applied: "change X".to_string(),
        analysis_depth: "counterfactual".to_string(),
        key_differences: vec!["difference 1".to_string()],
        confidence: 0.85,
        assumptions: vec!["assumption 1".to_string()],
        metadata: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_preset_response_into_contents() {
    let response = PresetResponse {
        presets: Some(vec![PresetInfo {
            id: "p1".to_string(),
            name: "Quick Analysis".to_string(),
            description: "Fast analysis preset".to_string(),
            category: "analysis".to_string(),
            required_inputs: vec!["content".to_string()],
        }]),
        execution_result: None,
        session_id: Some("s1".to_string()),
        metadata: None,
        next_call: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}

#[test]
fn test_metrics_response_into_contents() {
    let response = MetricsResponse {
        summary: Some(MetricsSummary {
            total_calls: 100,
            success_rate: 0.95,
            avg_latency_ms: 150.0,
            by_mode: serde_json::json!({"linear": 50, "tree": 30}),
        }),
        mode_stats: None,
        invocations: None,
        config: None,
    };
    let contents = response.into_contents();
    assert_eq!(contents.len(), 1);
}
