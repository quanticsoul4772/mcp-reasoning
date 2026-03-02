use crate::server::requests::*;

#[test]
fn test_tree_request_deserialize() {
    let json = r#"{"operation": "create", "content": "test"}"#;
    let req: TreeRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.operation, Some("create".to_string()));
}

#[test]
fn test_divergent_request_deserialize() {
    let json = r#"{"content": "test", "force_rebellion": true}"#;
    let req: DivergentRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.force_rebellion, Some(true));
}

#[test]
fn test_reflection_request_deserialize() {
    let json = r#"{"operation": "evaluate", "session_id": "s1"}"#;
    let req: ReflectionRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.operation, Some("evaluate".to_string()));
}

#[test]
fn test_checkpoint_request_deserialize() {
    let json = r#"{"operation": "create", "session_id": "s1", "name": "cp1"}"#;
    let req: CheckpointRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.name, Some("cp1".to_string()));
}

#[test]
fn test_auto_request_deserialize() {
    let json = r#"{"content": "test", "hints": ["hint1"]}"#;
    let req: AutoRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.hints, Some(vec!["hint1".to_string()]));
}

#[test]
fn test_graph_request_deserialize() {
    let json = r#"{"operation": "init", "session_id": "s1", "k": 5}"#;
    let req: GraphRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.k, Some(5));
}

#[test]
fn test_detect_request_deserialize() {
    let json = r#"{"type": "biases", "check_formal": true}"#;
    let req: DetectRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.detect_type, "biases");
}

#[test]
fn test_decision_request_deserialize() {
    let json = r#"{"type": "weighted", "options": ["A", "B"]}"#;
    let req: DecisionRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.options, Some(vec!["A".to_string(), "B".to_string()]));
}

#[test]
fn test_evidence_request_deserialize() {
    let json = r#"{"type": "assess", "prior": 0.5}"#;
    let req: EvidenceRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.prior, Some(0.5));
}

#[test]
fn test_timeline_request_deserialize() {
    let json = r#"{"operation": "branch", "timeline_id": "tl1"}"#;
    let req: TimelineRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.timeline_id, Some("tl1".to_string()));
}

#[test]
fn test_mcts_request_deserialize() {
    let json = r#"{"operation": "explore", "iterations": 50}"#;
    let req: MctsRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.iterations, Some(50));
}

#[test]
fn test_counterfactual_request_deserialize() {
    let json = r#"{"scenario": "base", "intervention": "change"}"#;
    let req: CounterfactualRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.scenario, "base");
}

#[test]
fn test_preset_request_deserialize() {
    let json = r#"{"operation": "run", "preset_id": "p1"}"#;
    let req: PresetRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.preset_id, Some("p1".to_string()));
}

#[test]
fn test_metrics_request_deserialize() {
    let json = r#"{"query": "by_mode", "mode_name": "linear"}"#;
    let req: MetricsRequest = serde_json::from_str(json).expect("deserialize");
    assert_eq!(req.mode_name, Some("linear".to_string()));
}
