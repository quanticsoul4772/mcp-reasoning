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
