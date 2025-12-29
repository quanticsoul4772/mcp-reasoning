//! JSON parsing helpers for MCTS mode.
//!
//! These functions extract structured data from LLM JSON responses.
//! Each parse_* function handles a specific MCTS component (frontier nodes,
//! expansion, backpropagation, quality assessment). Returns ModeError for failures.

use std::collections::HashMap;

use crate::error::ModeError;

use super::types::{
    AlternativeAction, AlternativeOption, Backpropagation, BacktrackDecision, Expansion,
    FrontierNode, NewNode, QualityAssessment, QualityTrend, Recommendation, RecommendedAction,
    SearchStatus, SelectedNode,
};

// ============================================================================
// Explore Parsing
// ============================================================================

/// Parse frontier nodes from JSON.
pub fn parse_frontier(json: &serde_json::Value) -> Result<Vec<FrontierNode>, ModeError> {
    let arr = json
        .get("frontier_evaluation")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "frontier_evaluation".to_string(),
        })?;

    arr.iter()
        .map(|n| {
            // Visit counts from JSON; values will be small counts that fit in u32
            #[allow(clippy::cast_possible_truncation)]
            let visits = n
                .get("visits")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| ModeError::MissingField {
                    field: "visits".to_string(),
                })? as u32;

            Ok(FrontierNode {
                node_id: get_str(n, "node_id")?,
                visits,
                average_value: get_f64(n, "average_value")?,
                ucb1_score: get_f64(n, "ucb1_score")?,
                exploration_bonus: get_f64(n, "exploration_bonus")?,
            })
        })
        .collect()
}

/// Parse selected node from JSON.
pub fn parse_selected(json: &serde_json::Value) -> Result<SelectedNode, ModeError> {
    let s = json
        .get("selected_node")
        .ok_or_else(|| ModeError::MissingField {
            field: "selected_node".to_string(),
        })?;

    Ok(SelectedNode {
        node_id: get_str(s, "node_id")?,
        selection_reason: get_str(s, "selection_reason")?,
    })
}

/// Parse expansion results from JSON.
pub fn parse_expansion(json: &serde_json::Value) -> Result<Expansion, ModeError> {
    let e = json
        .get("expansion")
        .ok_or_else(|| ModeError::MissingField {
            field: "expansion".to_string(),
        })?;

    let nodes_arr = e
        .get("new_nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "new_nodes".to_string(),
        })?;

    let new_nodes: Result<Vec<_>, _> = nodes_arr
        .iter()
        .map(|n| {
            Ok(NewNode {
                id: get_str(n, "id")?,
                content: get_str(n, "content")?,
                simulated_value: get_f64(n, "simulated_value")?,
            })
        })
        .collect();

    Ok(Expansion {
        new_nodes: new_nodes?,
    })
}

/// Parse backpropagation results from JSON.
pub fn parse_backpropagation(json: &serde_json::Value) -> Result<Backpropagation, ModeError> {
    let b = json
        .get("backpropagation")
        .ok_or_else(|| ModeError::MissingField {
            field: "backpropagation".to_string(),
        })?;

    let updated_nodes = get_string_array(b, "updated_nodes")?;

    let changes_obj = b
        .get("value_changes")
        .and_then(serde_json::Value::as_object);

    let value_changes = changes_obj.map_or_else(HashMap::new, |obj| {
        obj.iter()
            .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
            .collect()
    });

    Ok(Backpropagation {
        updated_nodes,
        value_changes,
    })
}

/// Parse search status from JSON.
pub fn parse_search_status(json: &serde_json::Value) -> Result<SearchStatus, ModeError> {
    let s = json
        .get("search_status")
        .ok_or_else(|| ModeError::MissingField {
            field: "search_status".to_string(),
        })?;

    // Node/simulation counts from LLM are small enough for u32
    #[allow(clippy::cast_possible_truncation)]
    let total_nodes = s
        .get("total_nodes")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ModeError::MissingField {
            field: "total_nodes".to_string(),
        })? as u32;

    #[allow(clippy::cast_possible_truncation)]
    let total_simulations = s
        .get("total_simulations")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ModeError::MissingField {
            field: "total_simulations".to_string(),
        })? as u32;

    Ok(SearchStatus {
        total_nodes,
        total_simulations,
        best_path_value: get_f64(s, "best_path_value")?,
    })
}

// ============================================================================
// Backtrack Parsing
// ============================================================================

/// Parse quality assessment from JSON.
pub fn parse_quality_assessment(json: &serde_json::Value) -> Result<QualityAssessment, ModeError> {
    let q = json
        .get("quality_assessment")
        .ok_or_else(|| ModeError::MissingField {
            field: "quality_assessment".to_string(),
        })?;

    let recent_values: Vec<f64> = q
        .get("recent_values")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "recent_values".to_string(),
        })?
        .iter()
        .filter_map(serde_json::Value::as_f64)
        .collect();

    let trend_str = get_str(q, "trend")?;
    let trend = match trend_str.to_lowercase().as_str() {
        "declining" => QualityTrend::Declining,
        "stable" => QualityTrend::Stable,
        "improving" => QualityTrend::Improving,
        _ => {
            return Err(ModeError::InvalidValue {
                field: "trend".to_string(),
                reason: format!("must be declining, stable, or improving, got {trend_str}"),
            })
        }
    };

    Ok(QualityAssessment {
        recent_values,
        trend,
        decline_magnitude: get_f64(q, "decline_magnitude")?,
    })
}

/// Parse backtrack decision from JSON.
pub fn parse_backtrack_decision(json: &serde_json::Value) -> Result<BacktrackDecision, ModeError> {
    let b = json
        .get("backtrack_decision")
        .ok_or_else(|| ModeError::MissingField {
            field: "backtrack_decision".to_string(),
        })?;

    let should_backtrack = b
        .get("should_backtrack")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| ModeError::MissingField {
            field: "should_backtrack".to_string(),
        })?;

    let backtrack_to = b
        .get("backtrack_to")
        .and_then(serde_json::Value::as_str)
        .map(String::from);

    // Depth reduction is a small integer (typically < 10)
    #[allow(clippy::cast_possible_truncation)]
    let depth_reduction = b
        .get("depth_reduction")
        .and_then(serde_json::Value::as_u64)
        .map(|v| v as u32);

    Ok(BacktrackDecision {
        should_backtrack,
        reason: get_str(b, "reason")?,
        backtrack_to,
        depth_reduction,
    })
}

/// Parse alternative actions from JSON.
pub fn parse_alternatives(json: &serde_json::Value) -> Result<Vec<AlternativeOption>, ModeError> {
    let arr = json
        .get("alternative_actions")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "alternative_actions".to_string(),
        })?;

    arr.iter()
        .map(|a| {
            let action_str = get_str(a, "action")?;
            let action = match action_str.to_lowercase().as_str() {
                "prune" => AlternativeAction::Prune,
                "refine" => AlternativeAction::Refine,
                "widen" => AlternativeAction::Widen,
                "continue" => AlternativeAction::Continue,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "action".to_string(),
                        reason: format!(
                            "must be prune, refine, widen, or continue, got {action_str}"
                        ),
                    })
                }
            };

            Ok(AlternativeOption {
                action,
                rationale: get_str(a, "rationale")?,
            })
        })
        .collect()
}

/// Parse recommendation from JSON.
pub fn parse_recommendation(json: &serde_json::Value) -> Result<Recommendation, ModeError> {
    let r = json
        .get("recommendation")
        .ok_or_else(|| ModeError::MissingField {
            field: "recommendation".to_string(),
        })?;

    let action_str = get_str(r, "action")?;
    let action = match action_str.to_lowercase().as_str() {
        "backtrack" => RecommendedAction::Backtrack,
        "continue" => RecommendedAction::Continue,
        "terminate" => RecommendedAction::Terminate,
        _ => {
            return Err(ModeError::InvalidValue {
                field: "action".to_string(),
                reason: format!("must be backtrack, continue, or terminate, got {action_str}"),
            })
        }
    };

    let confidence = get_f64(r, "confidence")?;
    if !(0.0..=1.0).contains(&confidence) {
        return Err(ModeError::InvalidValue {
            field: "confidence".to_string(),
            reason: format!("must be between 0.0 and 1.0, got {confidence}"),
        });
    }

    Ok(Recommendation {
        action,
        confidence,
        expected_benefit: get_str(r, "expected_benefit")?,
    })
}

// ============================================================================
// Utility Helpers
// ============================================================================

/// Get string field from JSON.
pub fn get_str(json: &serde_json::Value, field: &str) -> Result<String, ModeError> {
    json.get(field)
        .and_then(serde_json::Value::as_str)
        .map(String::from)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })
}

/// Get f64 field from JSON.
pub fn get_f64(json: &serde_json::Value, field: &str) -> Result<f64, ModeError> {
    json.get(field)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })
}

/// Get string array from JSON.
pub fn get_string_array(json: &serde_json::Value, field: &str) -> Result<Vec<String>, ModeError> {
    Ok(json
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;

    // Utility Helper Tests
    #[test]
    fn test_get_str_success() {
        let json = json!({"name": "test"});
        let result = get_str(&json, "name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_get_str_missing() {
        let json = json!({"other": "value"});
        let result = get_str(&json, "name");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_f64_success() {
        let json = json!({"value": 0.85});
        let result = get_f64(&json, "value");
        assert!(result.is_ok());
        assert!((result.unwrap() - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_get_f64_missing() {
        let json = json!({"other": 1.0});
        let result = get_f64(&json, "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_string_array_success() {
        let json = json!({"items": ["a", "b", "c"]});
        let result = get_string_array(&json, "items");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_get_string_array_missing() {
        let json = json!({"other": []});
        let result = get_string_array(&json, "items");
        assert!(result.is_err());
    }

    // Parse Frontier Tests
    #[test]
    fn test_parse_frontier_success() {
        let json = json!({
            "frontier_evaluation": [
                {
                    "node_id": "n1",
                    "visits": 10,
                    "average_value": 0.7,
                    "ucb1_score": 0.85,
                    "exploration_bonus": 0.15
                }
            ]
        });
        let result = parse_frontier(&json);
        assert!(result.is_ok());
        let frontier = result.unwrap();
        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0].node_id, "n1");
        assert_eq!(frontier[0].visits, 10);
        assert!((frontier[0].average_value - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_frontier_missing() {
        let json = json!({"other": []});
        let result = parse_frontier(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_frontier_empty() {
        let json = json!({"frontier_evaluation": []});
        let result = parse_frontier(&json);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // Parse Selected Tests
    #[test]
    fn test_parse_selected_success() {
        let json = json!({
            "selected_node": {
                "node_id": "n1",
                "selection_reason": "Highest UCB1 score"
            }
        });
        let result = parse_selected(&json);
        assert!(result.is_ok());
        let selected = result.unwrap();
        assert_eq!(selected.node_id, "n1");
        assert_eq!(selected.selection_reason, "Highest UCB1 score");
    }

    #[test]
    fn test_parse_selected_missing() {
        let json = json!({"other": {}});
        let result = parse_selected(&json);
        assert!(result.is_err());
    }

    // Parse Expansion Tests
    #[test]
    fn test_parse_expansion_success() {
        let json = json!({
            "expansion": {
                "new_nodes": [
                    {"id": "n2", "content": "New exploration", "simulated_value": 0.6},
                    {"id": "n3", "content": "Another path", "simulated_value": 0.7}
                ]
            }
        });
        let result = parse_expansion(&json);
        assert!(result.is_ok());
        let expansion = result.unwrap();
        assert_eq!(expansion.new_nodes.len(), 2);
        assert_eq!(expansion.new_nodes[0].id, "n2");
        assert!((expansion.new_nodes[1].simulated_value - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_expansion_missing() {
        let json = json!({"other": {}});
        let result = parse_expansion(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_expansion_empty_nodes() {
        let json = json!({
            "expansion": {"new_nodes": []}
        });
        let result = parse_expansion(&json);
        assert!(result.is_ok());
        assert!(result.unwrap().new_nodes.is_empty());
    }

    // Parse Backpropagation Tests
    #[test]
    fn test_parse_backpropagation_success() {
        let json = json!({
            "backpropagation": {
                "updated_nodes": ["n1", "n2"],
                "value_changes": {"n1": 0.1, "n2": -0.05}
            }
        });
        let result = parse_backpropagation(&json);
        assert!(result.is_ok());
        let bp = result.unwrap();
        assert_eq!(bp.updated_nodes.len(), 2);
        assert_eq!(bp.value_changes.len(), 2);
        assert!((bp.value_changes.get("n1").unwrap() - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_backpropagation_no_value_changes() {
        let json = json!({
            "backpropagation": {
                "updated_nodes": ["n1"]
            }
        });
        let result = parse_backpropagation(&json);
        assert!(result.is_ok());
        let bp = result.unwrap();
        assert!(bp.value_changes.is_empty());
    }

    #[test]
    fn test_parse_backpropagation_missing() {
        let json = json!({"other": {}});
        let result = parse_backpropagation(&json);
        assert!(result.is_err());
    }

    // Parse Search Status Tests
    #[test]
    fn test_parse_search_status_success() {
        let json = json!({
            "search_status": {
                "total_nodes": 100,
                "total_simulations": 500,
                "best_path_value": 0.85
            }
        });
        let result = parse_search_status(&json);
        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status.total_nodes, 100);
        assert_eq!(status.total_simulations, 500);
        assert!((status.best_path_value - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_search_status_missing() {
        let json = json!({"other": {}});
        let result = parse_search_status(&json);
        assert!(result.is_err());
    }

    // Parse Quality Assessment Tests
    #[test]
    fn test_parse_quality_assessment_declining() {
        let json = json!({
            "quality_assessment": {
                "recent_values": [0.8, 0.7, 0.6],
                "trend": "declining",
                "decline_magnitude": 0.2
            }
        });
        let result = parse_quality_assessment(&json);
        assert!(result.is_ok());
        let qa = result.unwrap();
        assert_eq!(qa.recent_values.len(), 3);
        assert!(matches!(qa.trend, QualityTrend::Declining));
        assert!((qa.decline_magnitude - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_quality_assessment_stable() {
        let json = json!({
            "quality_assessment": {
                "recent_values": [0.7, 0.7, 0.7],
                "trend": "stable",
                "decline_magnitude": 0.0
            }
        });
        let result = parse_quality_assessment(&json);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap().trend, QualityTrend::Stable));
    }

    #[test]
    fn test_parse_quality_assessment_improving() {
        let json = json!({
            "quality_assessment": {
                "recent_values": [0.6, 0.7, 0.8],
                "trend": "improving",
                "decline_magnitude": 0.0
            }
        });
        let result = parse_quality_assessment(&json);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap().trend, QualityTrend::Improving));
    }

    #[test]
    fn test_parse_quality_assessment_invalid_trend() {
        let json = json!({
            "quality_assessment": {
                "recent_values": [0.7],
                "trend": "unknown",
                "decline_magnitude": 0.0
            }
        });
        let result = parse_quality_assessment(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::InvalidValue { field, reason }) => {
                assert_eq!(field, "trend");
                assert!(reason.contains("unknown"));
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_parse_quality_assessment_missing() {
        let json = json!({"other": {}});
        let result = parse_quality_assessment(&json);
        assert!(result.is_err());
    }

    // Parse Backtrack Decision Tests
    #[test]
    fn test_parse_backtrack_decision_should_backtrack() {
        let json = json!({
            "backtrack_decision": {
                "should_backtrack": true,
                "reason": "Quality declining",
                "backtrack_to": "n5",
                "depth_reduction": 3
            }
        });
        let result = parse_backtrack_decision(&json);
        assert!(result.is_ok());
        let bd = result.unwrap();
        assert!(bd.should_backtrack);
        assert_eq!(bd.reason, "Quality declining");
        assert_eq!(bd.backtrack_to, Some("n5".to_string()));
        assert_eq!(bd.depth_reduction, Some(3));
    }

    #[test]
    fn test_parse_backtrack_decision_no_backtrack() {
        let json = json!({
            "backtrack_decision": {
                "should_backtrack": false,
                "reason": "Path still promising"
            }
        });
        let result = parse_backtrack_decision(&json);
        assert!(result.is_ok());
        let bd = result.unwrap();
        assert!(!bd.should_backtrack);
        assert!(bd.backtrack_to.is_none());
        assert!(bd.depth_reduction.is_none());
    }

    #[test]
    fn test_parse_backtrack_decision_missing() {
        let json = json!({"other": {}});
        let result = parse_backtrack_decision(&json);
        assert!(result.is_err());
    }

    // Parse Alternatives Tests
    #[test]
    fn test_parse_alternatives_success() {
        let json = json!({
            "alternative_actions": [
                {"action": "prune", "rationale": "Remove low-value branches"},
                {"action": "refine", "rationale": "Focus on promising area"},
                {"action": "widen", "rationale": "Explore more options"},
                {"action": "continue", "rationale": "Current path is good"}
            ]
        });
        let result = parse_alternatives(&json);
        assert!(result.is_ok());
        let alts = result.unwrap();
        assert_eq!(alts.len(), 4);
        assert!(matches!(alts[0].action, AlternativeAction::Prune));
        assert!(matches!(alts[1].action, AlternativeAction::Refine));
        assert!(matches!(alts[2].action, AlternativeAction::Widen));
        assert!(matches!(alts[3].action, AlternativeAction::Continue));
    }

    #[test]
    fn test_parse_alternatives_invalid_action() {
        let json = json!({
            "alternative_actions": [
                {"action": "unknown", "rationale": "Test"}
            ]
        });
        let result = parse_alternatives(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::InvalidValue { field, reason }) => {
                assert_eq!(field, "action");
                assert!(reason.contains("unknown"));
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_parse_alternatives_missing() {
        let json = json!({"other": []});
        let result = parse_alternatives(&json);
        assert!(result.is_err());
    }

    // Parse Recommendation Tests
    #[test]
    fn test_parse_recommendation_backtrack() {
        let json = json!({
            "recommendation": {
                "action": "backtrack",
                "confidence": 0.9,
                "expected_benefit": "Recover from declining path"
            }
        });
        let result = parse_recommendation(&json);
        assert!(result.is_ok());
        let rec = result.unwrap();
        assert!(matches!(rec.action, RecommendedAction::Backtrack));
        assert!((rec.confidence - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_recommendation_continue() {
        let json = json!({
            "recommendation": {
                "action": "continue",
                "confidence": 0.8,
                "expected_benefit": "Path is promising"
            }
        });
        let result = parse_recommendation(&json);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap().action,
            RecommendedAction::Continue
        ));
    }

    #[test]
    fn test_parse_recommendation_terminate() {
        let json = json!({
            "recommendation": {
                "action": "terminate",
                "confidence": 0.95,
                "expected_benefit": "Found optimal solution"
            }
        });
        let result = parse_recommendation(&json);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap().action,
            RecommendedAction::Terminate
        ));
    }

    #[test]
    fn test_parse_recommendation_invalid_action() {
        let json = json!({
            "recommendation": {
                "action": "invalid",
                "confidence": 0.5,
                "expected_benefit": "Test"
            }
        });
        let result = parse_recommendation(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::InvalidValue { field, .. }) => {
                assert_eq!(field, "action");
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_parse_recommendation_invalid_confidence_high() {
        let json = json!({
            "recommendation": {
                "action": "continue",
                "confidence": 1.5,
                "expected_benefit": "Test"
            }
        });
        let result = parse_recommendation(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::InvalidValue { field, reason }) => {
                assert_eq!(field, "confidence");
                assert!(reason.contains("between 0.0 and 1.0"));
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_parse_recommendation_invalid_confidence_low() {
        let json = json!({
            "recommendation": {
                "action": "continue",
                "confidence": -0.5,
                "expected_benefit": "Test"
            }
        });
        let result = parse_recommendation(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_recommendation_missing() {
        let json = json!({"other": {}});
        let result = parse_recommendation(&json);
        assert!(result.is_err());
    }

    // Edge Cases
    #[test]
    fn test_parse_quality_assessment_case_insensitive() {
        let json = json!({
            "quality_assessment": {
                "recent_values": [0.5],
                "trend": "DECLINING",
                "decline_magnitude": 0.1
            }
        });
        let result = parse_quality_assessment(&json);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap().trend, QualityTrend::Declining));
    }

    #[test]
    fn test_parse_alternatives_case_insensitive() {
        let json = json!({
            "alternative_actions": [
                {"action": "PRUNE", "rationale": "Test"}
            ]
        });
        let result = parse_alternatives(&json);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap()[0].action,
            AlternativeAction::Prune
        ));
    }

    #[test]
    fn test_parse_recommendation_case_insensitive() {
        let json = json!({
            "recommendation": {
                "action": "BACKTRACK",
                "confidence": 0.5,
                "expected_benefit": "Test"
            }
        });
        let result = parse_recommendation(&json);
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap().action,
            RecommendedAction::Backtrack
        ));
    }
}
