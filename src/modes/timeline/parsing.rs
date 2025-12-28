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
