//! JSON parsing helpers for timeline mode.
//!
//! These functions extract structured data from LLM JSON responses.
//! Each parse_* function handles a specific response component, while get_*
//! functions provide common field extraction with consistent error handling.
//! All parsers return ModeError::MissingField for absent or malformed fields.

use crate::error::ModeError;

use super::types::{
    BranchComparison, BranchDifference, BranchEvent, BranchPoint, CommonPattern,
    CompareRecommendation, DecisionPoint, EventType, FragileStrategy, OpportunityAssessment,
    RiskAssessment, RobustStrategy, TemporalStructure, TimelineBranch, TimelineEvent,
};

// ============================================================================
// Create Operation Parsing
// ============================================================================

pub fn parse_events(json: &serde_json::Value) -> Result<Vec<TimelineEvent>, ModeError> {
    let arr = json
        .get("events")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "events".to_string(),
        })?;

    arr.iter()
        .map(|e| {
            let type_str = get_str(e, "type")?;
            let event_type = match type_str.to_lowercase().as_str() {
                "event" => EventType::Event,
                "state" => EventType::State,
                "decision_point" => EventType::DecisionPoint,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "type".to_string(),
                        reason: format!("must be event, state, or decision_point, got {type_str}"),
                    })
                }
            };

            Ok(TimelineEvent {
                id: get_str(e, "id")?,
                description: get_str(e, "description")?,
                time: get_str(e, "time")?,
                event_type,
                causes: get_string_array(e, "causes").unwrap_or_default(),
                effects: get_string_array(e, "effects").unwrap_or_default(),
            })
        })
        .collect()
}

pub fn parse_decision_points(json: &serde_json::Value) -> Result<Vec<DecisionPoint>, ModeError> {
    let arr = json
        .get("decision_points")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "decision_points".to_string(),
        })?;

    arr.iter()
        .map(|d| {
            Ok(DecisionPoint {
                id: get_str(d, "id")?,
                description: get_str(d, "description")?,
                options: get_string_array(d, "options")?,
                deadline: get_str(d, "deadline")?,
            })
        })
        .collect()
}

pub fn parse_temporal_structure(json: &serde_json::Value) -> Result<TemporalStructure, ModeError> {
    let t = json
        .get("temporal_structure")
        .ok_or_else(|| ModeError::MissingField {
            field: "temporal_structure".to_string(),
        })?;

    Ok(TemporalStructure {
        start: get_str(t, "start")?,
        current: get_str(t, "current")?,
        horizon: get_str(t, "horizon")?,
    })
}

// ============================================================================
// Branch Operation Parsing
// ============================================================================

pub fn parse_branch_point(json: &serde_json::Value) -> Result<BranchPoint, ModeError> {
    let b = json
        .get("branch_point")
        .ok_or_else(|| ModeError::MissingField {
            field: "branch_point".to_string(),
        })?;

    Ok(BranchPoint {
        event_id: get_str(b, "event_id")?,
        description: get_str(b, "description")?,
    })
}

pub fn parse_branches(json: &serde_json::Value) -> Result<Vec<TimelineBranch>, ModeError> {
    let arr = json
        .get("branches")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "branches".to_string(),
        })?;

    arr.iter()
        .map(|b| {
            let events = parse_branch_events(b)?;

            Ok(TimelineBranch {
                id: get_str(b, "id")?,
                choice: get_str(b, "choice")?,
                events,
                plausibility: get_f64(b, "plausibility")?,
                outcome_quality: get_f64(b, "outcome_quality")?,
            })
        })
        .collect()
}

pub fn parse_branch_events(branch: &serde_json::Value) -> Result<Vec<BranchEvent>, ModeError> {
    let arr = branch
        .get("events")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "events".to_string(),
        })?;

    arr.iter()
        .map(|e| {
            Ok(BranchEvent {
                id: get_str(e, "id")?,
                description: get_str(e, "description")?,
                probability: get_f64(e, "probability")?,
                time_offset: get_str(e, "time_offset")?,
            })
        })
        .collect()
}

pub fn parse_branch_comparison(json: &serde_json::Value) -> Result<BranchComparison, ModeError> {
    let c = json
        .get("comparison")
        .ok_or_else(|| ModeError::MissingField {
            field: "comparison".to_string(),
        })?;

    Ok(BranchComparison {
        most_likely_good_outcome: get_str(c, "most_likely_good_outcome")?,
        highest_risk: get_str(c, "highest_risk")?,
        key_differences: get_string_array(c, "key_differences")?,
    })
}

// ============================================================================
// Compare Operation Parsing
// ============================================================================

pub fn parse_key_differences(json: &serde_json::Value) -> Result<Vec<BranchDifference>, ModeError> {
    let arr = json
        .get("key_differences")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "key_differences".to_string(),
        })?;

    arr.iter()
        .map(|d| {
            Ok(BranchDifference {
                dimension: get_str(d, "dimension")?,
                branch_1_value: get_str(d, "branch_1_value")?,
                branch_2_value: get_str(d, "branch_2_value")?,
                significance: get_str(d, "significance")?,
            })
        })
        .collect()
}

pub fn parse_risk_assessment(json: &serde_json::Value) -> Result<RiskAssessment, ModeError> {
    let r = json
        .get("risk_assessment")
        .ok_or_else(|| ModeError::MissingField {
            field: "risk_assessment".to_string(),
        })?;

    Ok(RiskAssessment {
        branch_1_risks: get_string_array(r, "branch_1_risks")?,
        branch_2_risks: get_string_array(r, "branch_2_risks")?,
    })
}

pub fn parse_opportunity_assessment(
    json: &serde_json::Value,
) -> Result<OpportunityAssessment, ModeError> {
    let o = json
        .get("opportunity_assessment")
        .ok_or_else(|| ModeError::MissingField {
            field: "opportunity_assessment".to_string(),
        })?;

    Ok(OpportunityAssessment {
        branch_1_opportunities: get_string_array(o, "branch_1_opportunities")?,
        branch_2_opportunities: get_string_array(o, "branch_2_opportunities")?,
    })
}

pub fn parse_compare_recommendation(
    json: &serde_json::Value,
) -> Result<CompareRecommendation, ModeError> {
    let r = json
        .get("recommendation")
        .ok_or_else(|| ModeError::MissingField {
            field: "recommendation".to_string(),
        })?;

    Ok(CompareRecommendation {
        preferred_branch: get_str(r, "preferred_branch")?,
        conditions: get_str(r, "conditions")?,
        key_factors: get_str(r, "key_factors")?,
    })
}

// ============================================================================
// Merge Operation Parsing
// ============================================================================

pub fn parse_common_patterns(json: &serde_json::Value) -> Result<Vec<CommonPattern>, ModeError> {
    let arr = json
        .get("common_patterns")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "common_patterns".to_string(),
        })?;

    arr.iter()
        .map(|p| {
            Ok(CommonPattern {
                pattern: get_str(p, "pattern")?,
                frequency: get_f64(p, "frequency")?,
                implications: get_str(p, "implications")?,
            })
        })
        .collect()
}

pub fn parse_robust_strategies(json: &serde_json::Value) -> Result<Vec<RobustStrategy>, ModeError> {
    let arr = json
        .get("robust_strategies")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "robust_strategies".to_string(),
        })?;

    arr.iter()
        .map(|s| {
            Ok(RobustStrategy {
                strategy: get_str(s, "strategy")?,
                effectiveness: get_f64(s, "effectiveness")?,
                conditions: get_str(s, "conditions")?,
            })
        })
        .collect()
}

pub fn parse_fragile_strategies(
    json: &serde_json::Value,
) -> Result<Vec<FragileStrategy>, ModeError> {
    let arr = json
        .get("fragile_strategies")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "fragile_strategies".to_string(),
        })?;

    arr.iter()
        .map(|s| {
            Ok(FragileStrategy {
                strategy: get_str(s, "strategy")?,
                failure_modes: get_str(s, "failure_modes")?,
            })
        })
        .collect()
}

// ============================================================================
// Utility Helpers
// ============================================================================

pub fn get_str(json: &serde_json::Value, field: &str) -> Result<String, ModeError> {
    json.get(field)
        .and_then(serde_json::Value::as_str)
        .map(String::from)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })
}

pub fn get_f64(json: &serde_json::Value, field: &str) -> Result<f64, ModeError> {
    json.get(field)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })
}

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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
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
        match result {
            Err(ModeError::MissingField { field }) => assert_eq!(field, "name"),
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_get_str_not_string() {
        let json = json!({"name": 123});
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
    fn test_get_f64_integer() {
        let json = json!({"value": 42});
        let result = get_f64(&json, "value");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42.0);
    }

    #[test]
    fn test_get_f64_missing() {
        let json = json!({"other": 1.0});
        let result = get_f64(&json, "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_f64_not_number() {
        let json = json!({"value": "not a number"});
        let result = get_f64(&json, "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_string_array_success() {
        let json = json!({"items": ["a", "b", "c"]});
        let result = get_string_array(&json, "items");
        assert!(result.is_ok());
        let arr = result.unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], "a");
    }

    #[test]
    fn test_get_string_array_empty() {
        let json = json!({"items": []});
        let result = get_string_array(&json, "items");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_string_array_missing() {
        let json = json!({"other": []});
        let result = get_string_array(&json, "items");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_string_array_not_array() {
        let json = json!({"items": "not an array"});
        let result = get_string_array(&json, "items");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_string_array_mixed_types() {
        let json = json!({"items": ["a", 1, "b", null]});
        let result = get_string_array(&json, "items");
        assert!(result.is_ok());
        let arr = result.unwrap();
        assert_eq!(arr.len(), 2); // Only strings are kept
        assert_eq!(arr, vec!["a", "b"]);
    }

    // Parse Events Tests
    #[test]
    fn test_parse_events_success() {
        let json = json!({
            "events": [
                {
                    "id": "e1",
                    "description": "Event 1",
                    "time": "2024-01-01",
                    "type": "event",
                    "causes": ["c1"],
                    "effects": ["ef1"]
                }
            ]
        });
        let result = parse_events(&json);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "e1");
        assert_eq!(events[0].description, "Event 1");
        assert_eq!(events[0].event_type, EventType::Event);
    }

    #[test]
    fn test_parse_events_state_type() {
        let json = json!({
            "events": [{"id": "s1", "description": "State", "time": "now", "type": "state"}]
        });
        let result = parse_events(&json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].event_type, EventType::State);
    }

    #[test]
    fn test_parse_events_decision_point_type() {
        let json = json!({
            "events": [{"id": "d1", "description": "Decision", "time": "future", "type": "decision_point"}]
        });
        let result = parse_events(&json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].event_type, EventType::DecisionPoint);
    }

    #[test]
    fn test_parse_events_invalid_type() {
        let json = json!({
            "events": [{"id": "x1", "description": "Bad", "time": "now", "type": "unknown"}]
        });
        let result = parse_events(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::InvalidValue { field, reason }) => {
                assert_eq!(field, "type");
                assert!(reason.contains("unknown"));
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_parse_events_missing() {
        let json = json!({"other": []});
        let result = parse_events(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_events_optional_causes_effects() {
        let json = json!({
            "events": [{"id": "e1", "description": "Event", "time": "now", "type": "event"}]
        });
        let result = parse_events(&json);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert!(events[0].causes.is_empty());
        assert!(events[0].effects.is_empty());
    }

    // Parse Decision Points Tests
    #[test]
    fn test_parse_decision_points_success() {
        let json = json!({
            "decision_points": [
                {
                    "id": "dp1",
                    "description": "Choose path",
                    "options": ["A", "B", "C"],
                    "deadline": "2024-06-01"
                }
            ]
        });
        let result = parse_decision_points(&json);
        assert!(result.is_ok());
        let points = result.unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].options.len(), 3);
    }

    #[test]
    fn test_parse_decision_points_missing() {
        let json = json!({"other": []});
        let result = parse_decision_points(&json);
        assert!(result.is_err());
    }

    // Parse Temporal Structure Tests
    #[test]
    fn test_parse_temporal_structure_success() {
        let json = json!({
            "temporal_structure": {
                "start": "2024-01-01",
                "current": "2024-06-15",
                "horizon": "2025-01-01"
            }
        });
        let result = parse_temporal_structure(&json);
        assert!(result.is_ok());
        let ts = result.unwrap();
        assert_eq!(ts.start, "2024-01-01");
        assert_eq!(ts.current, "2024-06-15");
        assert_eq!(ts.horizon, "2025-01-01");
    }

    #[test]
    fn test_parse_temporal_structure_missing() {
        let json = json!({"other": {}});
        let result = parse_temporal_structure(&json);
        assert!(result.is_err());
    }

    // Parse Branch Point Tests
    #[test]
    fn test_parse_branch_point_success() {
        let json = json!({
            "branch_point": {
                "event_id": "e1",
                "description": "Decision point"
            }
        });
        let result = parse_branch_point(&json);
        assert!(result.is_ok());
        let bp = result.unwrap();
        assert_eq!(bp.event_id, "e1");
        assert_eq!(bp.description, "Decision point");
    }

    #[test]
    fn test_parse_branch_point_missing() {
        let json = json!({"other": {}});
        let result = parse_branch_point(&json);
        assert!(result.is_err());
    }

    // Parse Branches Tests
    #[test]
    fn test_parse_branches_success() {
        let json = json!({
            "branches": [
                {
                    "id": "b1",
                    "choice": "Option A",
                    "events": [
                        {"id": "be1", "description": "Branch event", "probability": 0.8, "time_offset": "+1d"}
                    ],
                    "plausibility": 0.7,
                    "outcome_quality": 0.85
                }
            ]
        });
        let result = parse_branches(&json);
        assert!(result.is_ok());
        let branches = result.unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].id, "b1");
        assert_eq!(branches[0].events.len(), 1);
        assert!((branches[0].plausibility - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_branches_missing() {
        let json = json!({"other": []});
        let result = parse_branches(&json);
        assert!(result.is_err());
    }

    // Parse Branch Events Tests
    #[test]
    fn test_parse_branch_events_success() {
        let json = json!({
            "events": [
                {"id": "be1", "description": "Event 1", "probability": 0.9, "time_offset": "+2h"},
                {"id": "be2", "description": "Event 2", "probability": 0.5, "time_offset": "+1d"}
            ]
        });
        let result = parse_branch_events(&json);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].probability, 0.9);
    }

    #[test]
    fn test_parse_branch_events_missing() {
        let json = json!({"other": []});
        let result = parse_branch_events(&json);
        assert!(result.is_err());
    }

    // Parse Branch Comparison Tests
    #[test]
    fn test_parse_branch_comparison_success() {
        let json = json!({
            "comparison": {
                "most_likely_good_outcome": "Branch A",
                "highest_risk": "Branch B",
                "key_differences": ["diff1", "diff2"]
            }
        });
        let result = parse_branch_comparison(&json);
        assert!(result.is_ok());
        let comp = result.unwrap();
        assert_eq!(comp.most_likely_good_outcome, "Branch A");
        assert_eq!(comp.highest_risk, "Branch B");
        assert_eq!(comp.key_differences.len(), 2);
    }

    #[test]
    fn test_parse_branch_comparison_missing() {
        let json = json!({"other": {}});
        let result = parse_branch_comparison(&json);
        assert!(result.is_err());
    }

    // Parse Key Differences Tests
    #[test]
    fn test_parse_key_differences_success() {
        let json = json!({
            "key_differences": [
                {
                    "dimension": "Risk",
                    "branch_1_value": "Low",
                    "branch_2_value": "High",
                    "significance": "Critical"
                }
            ]
        });
        let result = parse_key_differences(&json);
        assert!(result.is_ok());
        let diffs = result.unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].dimension, "Risk");
    }

    #[test]
    fn test_parse_key_differences_missing() {
        let json = json!({"other": []});
        let result = parse_key_differences(&json);
        assert!(result.is_err());
    }

    // Parse Risk Assessment Tests
    #[test]
    fn test_parse_risk_assessment_success() {
        let json = json!({
            "risk_assessment": {
                "branch_1_risks": ["risk1", "risk2"],
                "branch_2_risks": ["risk3"]
            }
        });
        let result = parse_risk_assessment(&json);
        assert!(result.is_ok());
        let ra = result.unwrap();
        assert_eq!(ra.branch_1_risks.len(), 2);
        assert_eq!(ra.branch_2_risks.len(), 1);
    }

    #[test]
    fn test_parse_risk_assessment_missing() {
        let json = json!({"other": {}});
        let result = parse_risk_assessment(&json);
        assert!(result.is_err());
    }

    // Parse Opportunity Assessment Tests
    #[test]
    fn test_parse_opportunity_assessment_success() {
        let json = json!({
            "opportunity_assessment": {
                "branch_1_opportunities": ["opp1"],
                "branch_2_opportunities": ["opp2", "opp3"]
            }
        });
        let result = parse_opportunity_assessment(&json);
        assert!(result.is_ok());
        let oa = result.unwrap();
        assert_eq!(oa.branch_1_opportunities.len(), 1);
        assert_eq!(oa.branch_2_opportunities.len(), 2);
    }

    #[test]
    fn test_parse_opportunity_assessment_missing() {
        let json = json!({"other": {}});
        let result = parse_opportunity_assessment(&json);
        assert!(result.is_err());
    }

    // Parse Compare Recommendation Tests
    #[test]
    fn test_parse_compare_recommendation_success() {
        let json = json!({
            "recommendation": {
                "preferred_branch": "Branch A",
                "conditions": "If risk tolerance is low",
                "key_factors": "Stability, cost"
            }
        });
        let result = parse_compare_recommendation(&json);
        assert!(result.is_ok());
        let rec = result.unwrap();
        assert_eq!(rec.preferred_branch, "Branch A");
        assert_eq!(rec.conditions, "If risk tolerance is low");
    }

    #[test]
    fn test_parse_compare_recommendation_missing() {
        let json = json!({"other": {}});
        let result = parse_compare_recommendation(&json);
        assert!(result.is_err());
    }

    // Parse Common Patterns Tests
    #[test]
    fn test_parse_common_patterns_success() {
        let json = json!({
            "common_patterns": [
                {
                    "pattern": "Growth trend",
                    "frequency": 0.75,
                    "implications": "Positive outlook"
                }
            ]
        });
        let result = parse_common_patterns(&json);
        assert!(result.is_ok());
        let patterns = result.unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern, "Growth trend");
        assert!((patterns[0].frequency - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_common_patterns_missing() {
        let json = json!({"other": []});
        let result = parse_common_patterns(&json);
        assert!(result.is_err());
    }

    // Parse Robust Strategies Tests
    #[test]
    fn test_parse_robust_strategies_success() {
        let json = json!({
            "robust_strategies": [
                {
                    "strategy": "Diversification",
                    "effectiveness": 0.9,
                    "conditions": "All market conditions"
                }
            ]
        });
        let result = parse_robust_strategies(&json);
        assert!(result.is_ok());
        let strategies = result.unwrap();
        assert_eq!(strategies.len(), 1);
        assert_eq!(strategies[0].strategy, "Diversification");
    }

    #[test]
    fn test_parse_robust_strategies_missing() {
        let json = json!({"other": []});
        let result = parse_robust_strategies(&json);
        assert!(result.is_err());
    }

    // Parse Fragile Strategies Tests
    #[test]
    fn test_parse_fragile_strategies_success() {
        let json = json!({
            "fragile_strategies": [
                {
                    "strategy": "Single bet",
                    "failure_modes": "Market crash"
                }
            ]
        });
        let result = parse_fragile_strategies(&json);
        assert!(result.is_ok());
        let strategies = result.unwrap();
        assert_eq!(strategies.len(), 1);
        assert_eq!(strategies[0].strategy, "Single bet");
        assert_eq!(strategies[0].failure_modes, "Market crash");
    }

    #[test]
    fn test_parse_fragile_strategies_missing() {
        let json = json!({"other": []});
        let result = parse_fragile_strategies(&json);
        assert!(result.is_err());
    }

    // Edge Cases
    #[test]
    fn test_parse_events_empty_array() {
        let json = json!({"events": []});
        let result = parse_events(&json);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_branches_empty_array() {
        let json = json!({"branches": []});
        let result = parse_branches(&json);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_events_case_insensitive_type() {
        let json = json!({
            "events": [{"id": "e1", "description": "E", "time": "now", "type": "EVENT"}]
        });
        let result = parse_events(&json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].event_type, EventType::Event);
    }
}
