//! JSON parsing helpers for decision mode.
//!
//! These functions extract structured data from LLM JSON responses.
//! Each parse_* function handles a specific response component, while get_*
//! functions provide common field extraction with consistent error handling.
//! All parsers return ModeError::MissingField for absent or malformed fields.

use std::collections::HashMap;

use crate::error::ModeError;

use super::types::{
    Alignment, BalancedRecommendation, Conflict, ConflictSeverity, Criterion, CriterionType,
    InfluenceLevel, PairwiseComparison, PairwiseRank, PreferenceResult, PreferenceStrength,
    RankedOption, Stakeholder, TopsisCreterion, TopsisDistances, TopsisRank,
};

// ============================================================================
// Weighted Parsing
// ============================================================================

pub fn parse_criteria(json: &serde_json::Value) -> Result<Vec<Criterion>, ModeError> {
    let arr = json
        .get("criteria")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "criteria".to_string(),
        })?;

    arr.iter()
        .map(|c| {
            Ok(Criterion {
                name: get_str(c, "name")?,
                weight: get_f64(c, "weight")?,
                description: get_str(c, "description")?,
            })
        })
        .collect()
}

pub fn parse_scores(
    json: &serde_json::Value,
) -> Result<HashMap<String, HashMap<String, f64>>, ModeError> {
    let obj = json
        .get("scores")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ModeError::MissingField {
            field: "scores".to_string(),
        })?;

    let mut result = HashMap::new();
    for (option, scores_val) in obj {
        let scores_obj = scores_val
            .as_object()
            .ok_or_else(|| ModeError::InvalidValue {
                field: "scores".to_string(),
                reason: format!("Expected object for {option}"),
            })?;

        let mut option_scores = HashMap::new();
        for (criterion, score) in scores_obj {
            let s = score.as_f64().ok_or_else(|| ModeError::InvalidValue {
                field: "scores".to_string(),
                reason: format!("Expected f64 for {criterion}"),
            })?;
            option_scores.insert(criterion.clone(), s);
        }
        result.insert(option.clone(), option_scores);
    }
    Ok(result)
}

pub fn parse_weighted_totals(json: &serde_json::Value) -> Result<HashMap<String, f64>, ModeError> {
    let obj = json
        .get("weighted_totals")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ModeError::MissingField {
            field: "weighted_totals".to_string(),
        })?;

    Ok(obj
        .iter()
        .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
        .collect())
}

pub fn parse_weighted_ranking(json: &serde_json::Value) -> Result<Vec<RankedOption>, ModeError> {
    let arr = json
        .get("ranking")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "ranking".to_string(),
        })?;

    arr.iter()
        .map(|r| {
            // Rank is a small ordinal (1-N where N is typically < 10)
            #[allow(clippy::cast_possible_truncation)]
            let rank = r
                .get("rank")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| ModeError::MissingField {
                    field: "rank".to_string(),
                })? as u32;

            Ok(RankedOption {
                option: get_str(r, "option")?,
                score: get_f64(r, "score")?,
                rank,
            })
        })
        .collect()
}

// ============================================================================
// Pairwise Parsing
// ============================================================================

pub fn parse_comparisons(json: &serde_json::Value) -> Result<Vec<PairwiseComparison>, ModeError> {
    let arr = json
        .get("comparisons")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "comparisons".to_string(),
        })?;

    arr.iter()
        .map(|c| {
            let preferred_str = get_str(c, "preferred")?;
            let preferred = match preferred_str.to_lowercase().as_str() {
                "option_a" => PreferenceResult::OptionA,
                "option_b" => PreferenceResult::OptionB,
                "tie" => PreferenceResult::Tie,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "preferred".to_string(),
                        reason: format!("must be option_a, option_b, or tie, got {preferred_str}"),
                    })
                }
            };

            let strength_str = get_str(c, "strength")?;
            let strength = match strength_str.to_lowercase().as_str() {
                "strong" => PreferenceStrength::Strong,
                "moderate" => PreferenceStrength::Moderate,
                "slight" => PreferenceStrength::Slight,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "strength".to_string(),
                        reason: format!("must be strong, moderate, or slight, got {strength_str}"),
                    })
                }
            };

            Ok(PairwiseComparison {
                option_a: get_str(c, "option_a")?,
                option_b: get_str(c, "option_b")?,
                preferred,
                strength,
                reasoning: get_str(c, "reasoning")?,
            })
        })
        .collect()
}

pub fn parse_pairwise_matrix(json: &serde_json::Value) -> Result<HashMap<String, i32>, ModeError> {
    let obj = json
        .get("pairwise_matrix")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ModeError::MissingField {
            field: "pairwise_matrix".to_string(),
        })?;

    // Pairwise scores are small integers (typically -N to N where N < 10)
    #[allow(clippy::cast_possible_truncation)]
    Ok(obj
        .iter()
        .filter_map(|(k, v)| v.as_i64().map(|i| (k.clone(), i as i32)))
        .collect())
}

pub fn parse_pairwise_ranking(json: &serde_json::Value) -> Result<Vec<PairwiseRank>, ModeError> {
    let arr = json
        .get("ranking")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "ranking".to_string(),
        })?;

    arr.iter()
        .map(|r| {
            // Win counts and ranks are small ordinals (typically < 20)
            #[allow(clippy::cast_possible_truncation)]
            let wins = r
                .get("wins")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| ModeError::MissingField {
                    field: "wins".to_string(),
                })? as u32;

            #[allow(clippy::cast_possible_truncation)]
            let rank = r
                .get("rank")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| ModeError::MissingField {
                    field: "rank".to_string(),
                })? as u32;

            Ok(PairwiseRank {
                option: get_str(r, "option")?,
                wins,
                rank,
            })
        })
        .collect()
}

// ============================================================================
// TOPSIS Parsing
// ============================================================================

pub fn parse_topsis_criteria(json: &serde_json::Value) -> Result<Vec<TopsisCreterion>, ModeError> {
    let arr = json
        .get("criteria")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "criteria".to_string(),
        })?;

    arr.iter()
        .map(|c| {
            let type_str = get_str(c, "type")?;
            let criterion_type = match type_str.to_lowercase().as_str() {
                "benefit" => CriterionType::Benefit,
                "cost" => CriterionType::Cost,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "type".to_string(),
                        reason: format!("must be benefit or cost, got {type_str}"),
                    })
                }
            };

            Ok(TopsisCreterion {
                name: get_str(c, "name")?,
                criterion_type,
                weight: get_f64(c, "weight")?,
            })
        })
        .collect()
}

pub fn parse_decision_matrix(
    json: &serde_json::Value,
) -> Result<HashMap<String, Vec<f64>>, ModeError> {
    let obj = json
        .get("decision_matrix")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ModeError::MissingField {
            field: "decision_matrix".to_string(),
        })?;

    let mut result = HashMap::new();
    for (option, values) in obj {
        let arr = values.as_array().ok_or_else(|| ModeError::InvalidValue {
            field: "decision_matrix".to_string(),
            reason: format!("Expected array for {option}"),
        })?;

        let vals: Vec<f64> = arr.iter().filter_map(serde_json::Value::as_f64).collect();
        result.insert(option.clone(), vals);
    }
    Ok(result)
}

pub fn parse_f64_array(json: &serde_json::Value, field: &str) -> Result<Vec<f64>, ModeError> {
    let arr = json
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })?;

    Ok(arr.iter().filter_map(serde_json::Value::as_f64).collect())
}

pub fn parse_distances(
    json: &serde_json::Value,
) -> Result<HashMap<String, TopsisDistances>, ModeError> {
    let obj = json
        .get("distances")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ModeError::MissingField {
            field: "distances".to_string(),
        })?;

    let mut result = HashMap::new();
    for (option, dist) in obj {
        let to_ideal = get_f64(dist, "to_ideal")?;
        let to_anti_ideal = get_f64(dist, "to_anti_ideal")?;
        result.insert(
            option.clone(),
            TopsisDistances {
                to_ideal,
                to_anti_ideal,
            },
        );
    }
    Ok(result)
}

pub fn parse_relative_closeness(
    json: &serde_json::Value,
) -> Result<HashMap<String, f64>, ModeError> {
    let obj = json
        .get("relative_closeness")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| ModeError::MissingField {
            field: "relative_closeness".to_string(),
        })?;

    Ok(obj
        .iter()
        .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
        .collect())
}

pub fn parse_topsis_ranking(json: &serde_json::Value) -> Result<Vec<TopsisRank>, ModeError> {
    let arr = json
        .get("ranking")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "ranking".to_string(),
        })?;

    arr.iter()
        .map(|r| {
            // Rank is a small ordinal (1-N where N is typically < 10)
            #[allow(clippy::cast_possible_truncation)]
            let rank = r
                .get("rank")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| ModeError::MissingField {
                    field: "rank".to_string(),
                })? as u32;

            Ok(TopsisRank {
                option: get_str(r, "option")?,
                closeness: get_f64(r, "closeness")?,
                rank,
            })
        })
        .collect()
}

// ============================================================================
// Perspectives Parsing
// ============================================================================

pub fn parse_stakeholders(json: &serde_json::Value) -> Result<Vec<Stakeholder>, ModeError> {
    let arr = json
        .get("stakeholders")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "stakeholders".to_string(),
        })?;

    arr.iter()
        .map(|s| {
            let influence_str = get_str(s, "influence_level")?;
            let influence_level = match influence_str.to_lowercase().as_str() {
                "high" => InfluenceLevel::High,
                "medium" => InfluenceLevel::Medium,
                "low" => InfluenceLevel::Low,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "influence_level".to_string(),
                        reason: format!("must be high, medium, or low, got {influence_str}"),
                    })
                }
            };

            Ok(Stakeholder {
                name: get_str(s, "name")?,
                interests: get_string_array(s, "interests")?,
                preferred_option: get_str(s, "preferred_option")?,
                concerns: get_string_array(s, "concerns")?,
                influence_level,
            })
        })
        .collect()
}

pub fn parse_conflicts(json: &serde_json::Value) -> Result<Vec<Conflict>, ModeError> {
    let arr = json
        .get("conflicts")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "conflicts".to_string(),
        })?;

    arr.iter()
        .map(|c| {
            let severity_str = get_str(c, "severity")?;
            let severity = match severity_str.to_lowercase().as_str() {
                "high" => ConflictSeverity::High,
                "medium" => ConflictSeverity::Medium,
                "low" => ConflictSeverity::Low,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "severity".to_string(),
                        reason: format!("must be high, medium, or low, got {severity_str}"),
                    })
                }
            };

            Ok(Conflict {
                between: get_string_array(c, "between")?,
                issue: get_str(c, "issue")?,
                severity,
            })
        })
        .collect()
}

pub fn parse_alignments(json: &serde_json::Value) -> Result<Vec<Alignment>, ModeError> {
    let arr = json
        .get("alignments")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "alignments".to_string(),
        })?;

    arr.iter()
        .map(|a| {
            Ok(Alignment {
                stakeholders: get_string_array(a, "stakeholders")?,
                common_ground: get_str(a, "common_ground")?,
            })
        })
        .collect()
}

pub fn parse_balanced_recommendation(
    json: &serde_json::Value,
) -> Result<BalancedRecommendation, ModeError> {
    let b = json
        .get("balanced_recommendation")
        .ok_or_else(|| ModeError::MissingField {
            field: "balanced_recommendation".to_string(),
        })?;

    Ok(BalancedRecommendation {
        option: get_str(b, "option")?,
        rationale: get_str(b, "rationale")?,
        mitigation: get_str(b, "mitigation")?,
    })
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;

    // Weighted parsing tests
    #[test]
    fn test_parse_criteria_success() {
        let json = json!({
            "criteria": [
                {"name": "cost", "weight": 0.5, "description": "Cost efficiency"},
                {"name": "quality", "weight": 0.5, "description": "Quality level"}
            ]
        });
        let result = parse_criteria(&json);
        assert!(result.is_ok());
        let criteria = result.unwrap();
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0].name, "cost");
    }

    #[test]
    fn test_parse_criteria_missing() {
        let result = parse_criteria(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_scores_success() {
        let json = json!({
            "scores": {
                "option_a": {"cost": 0.8, "quality": 0.9},
                "option_b": {"cost": 0.6, "quality": 0.7}
            }
        });
        let result = parse_scores(&json);
        assert!(result.is_ok());
        let scores = result.unwrap();
        assert_eq!(scores.len(), 2);
    }

    #[test]
    fn test_parse_scores_invalid_option() {
        let json = json!({"scores": {"option_a": "not_an_object"}});
        let result = parse_scores(&json);
        assert!(matches!(result, Err(ModeError::InvalidValue { .. })));
    }

    #[test]
    fn test_parse_scores_invalid_value() {
        let json = json!({"scores": {"option_a": {"cost": "not_a_number"}}});
        let result = parse_scores(&json);
        assert!(matches!(result, Err(ModeError::InvalidValue { .. })));
    }

    #[test]
    fn test_parse_weighted_totals_success() {
        let json = json!({"weighted_totals": {"a": 0.8, "b": 0.6}});
        let result = parse_weighted_totals(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_weighted_ranking_success() {
        let json = json!({
            "ranking": [
                {"option": "a", "score": 0.9, "rank": 1},
                {"option": "b", "score": 0.7, "rank": 2}
            ]
        });
        let result = parse_weighted_ranking(&json);
        assert!(result.is_ok());
        let ranking = result.unwrap();
        assert_eq!(ranking.len(), 2);
        assert_eq!(ranking[0].rank, 1);
    }

    // Pairwise parsing tests
    #[test]
    fn test_parse_comparisons_success() {
        let json = json!({
            "comparisons": [{
                "option_a": "first",
                "option_b": "second",
                "preferred": "option_a",
                "strength": "strong",
                "reasoning": "Better overall"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(result.is_ok());
        let comps = result.unwrap();
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].preferred, PreferenceResult::OptionA);
    }

    #[test]
    fn test_parse_comparisons_option_b() {
        let json = json!({
            "comparisons": [{
                "option_a": "first",
                "option_b": "second",
                "preferred": "option_b",
                "strength": "moderate",
                "reasoning": "Better value"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(result.is_ok());
        let comps = result.unwrap();
        assert_eq!(comps[0].preferred, PreferenceResult::OptionB);
        assert_eq!(comps[0].strength, PreferenceStrength::Moderate);
    }

    #[test]
    fn test_parse_comparisons_tie() {
        let json = json!({
            "comparisons": [{
                "option_a": "first",
                "option_b": "second",
                "preferred": "tie",
                "strength": "slight",
                "reasoning": "Too close"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(result.is_ok());
        let comps = result.unwrap();
        assert_eq!(comps[0].preferred, PreferenceResult::Tie);
        assert_eq!(comps[0].strength, PreferenceStrength::Slight);
    }

    #[test]
    fn test_parse_comparisons_invalid_preferred() {
        let json = json!({
            "comparisons": [{
                "option_a": "a", "option_b": "b",
                "preferred": "invalid",
                "strength": "strong", "reasoning": "x"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(matches!(result, Err(ModeError::InvalidValue { .. })));
    }

    #[test]
    fn test_parse_comparisons_invalid_strength() {
        let json = json!({
            "comparisons": [{
                "option_a": "a", "option_b": "b",
                "preferred": "option_a",
                "strength": "invalid", "reasoning": "x"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(matches!(result, Err(ModeError::InvalidValue { .. })));
    }

    #[test]
    fn test_parse_pairwise_matrix_success() {
        let json = json!({"pairwise_matrix": {"a_vs_b": 2, "a_vs_c": -1}});
        let result = parse_pairwise_matrix(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pairwise_ranking_success() {
        let json = json!({
            "ranking": [
                {"option": "a", "wins": 3, "rank": 1},
                {"option": "b", "wins": 1, "rank": 2}
            ]
        });
        let result = parse_pairwise_ranking(&json);
        assert!(result.is_ok());
    }

    // TOPSIS parsing tests
    #[test]
    fn test_parse_topsis_criteria_success() {
        let json = json!({
            "criteria": [
                {"name": "cost", "type": "cost", "weight": 0.4},
                {"name": "quality", "type": "benefit", "weight": 0.6}
            ]
        });
        let result = parse_topsis_criteria(&json);
        assert!(result.is_ok());
        let criteria = result.unwrap();
        assert_eq!(criteria[0].criterion_type, CriterionType::Cost);
        assert_eq!(criteria[1].criterion_type, CriterionType::Benefit);
    }

    #[test]
    fn test_parse_topsis_criteria_invalid_type() {
        let json = json!({
            "criteria": [{"name": "x", "type": "invalid", "weight": 0.5}]
        });
        let result = parse_topsis_criteria(&json);
        assert!(matches!(result, Err(ModeError::InvalidValue { .. })));
    }

    #[test]
    fn test_parse_decision_matrix_success() {
        let json = json!({
            "decision_matrix": {
                "a": [0.8, 0.9],
                "b": [0.6, 0.7]
            }
        });
        let result = parse_decision_matrix(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_decision_matrix_invalid() {
        let json = json!({"decision_matrix": {"a": "not_array"}});
        let result = parse_decision_matrix(&json);
        assert!(matches!(result, Err(ModeError::InvalidValue { .. })));
    }

    #[test]
    fn test_parse_f64_array_success() {
        let json = json!({"values": [0.1, 0.2, 0.3]});
        let result = parse_f64_array(&json, "values");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_parse_distances_success() {
        let json = json!({
            "distances": {
                "a": {"to_ideal": 0.2, "to_anti_ideal": 0.8},
                "b": {"to_ideal": 0.5, "to_anti_ideal": 0.5}
            }
        });
        let result = parse_distances(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_relative_closeness_success() {
        let json = json!({"relative_closeness": {"a": 0.8, "b": 0.5}});
        let result = parse_relative_closeness(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_topsis_ranking_success() {
        let json = json!({
            "ranking": [
                {"option": "a", "closeness": 0.8, "rank": 1}
            ]
        });
        let result = parse_topsis_ranking(&json);
        assert!(result.is_ok());
    }

    // Perspectives parsing tests
    #[test]
    fn test_parse_stakeholders_success() {
        let json = json!({
            "stakeholders": [{
                "name": "Customer",
                "interests": ["quality", "price"],
                "preferred_option": "option_a",
                "concerns": ["delivery"],
                "influence_level": "high"
            }]
        });
        let result = parse_stakeholders(&json);
        assert!(result.is_ok());
        let stakeholders = result.unwrap();
        assert_eq!(stakeholders[0].influence_level, InfluenceLevel::High);
    }

    #[test]
    fn test_parse_stakeholders_medium_influence() {
        let json = json!({
            "stakeholders": [{
                "name": "Partner", "interests": [], "preferred_option": "a",
                "concerns": [], "influence_level": "medium"
            }]
        });
        let result = parse_stakeholders(&json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].influence_level, InfluenceLevel::Medium);
    }

    #[test]
    fn test_parse_stakeholders_low_influence() {
        let json = json!({
            "stakeholders": [{
                "name": "Vendor", "interests": [], "preferred_option": "b",
                "concerns": [], "influence_level": "low"
            }]
        });
        let result = parse_stakeholders(&json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].influence_level, InfluenceLevel::Low);
    }

    #[test]
    fn test_parse_stakeholders_invalid_influence() {
        let json = json!({
            "stakeholders": [{
                "name": "X", "interests": [], "preferred_option": "a",
                "concerns": [], "influence_level": "invalid"
            }]
        });
        let result = parse_stakeholders(&json);
        assert!(matches!(result, Err(ModeError::InvalidValue { .. })));
    }

    #[test]
    fn test_parse_conflicts_success() {
        let json = json!({
            "conflicts": [{
                "between": ["A", "B"],
                "issue": "resource allocation",
                "severity": "high"
            }]
        });
        let result = parse_conflicts(&json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].severity, ConflictSeverity::High);
    }

    #[test]
    fn test_parse_conflicts_medium_severity() {
        let json = json!({
            "conflicts": [{
                "between": ["A", "B"], "issue": "timing", "severity": "medium"
            }]
        });
        let result = parse_conflicts(&json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].severity, ConflictSeverity::Medium);
    }

    #[test]
    fn test_parse_conflicts_low_severity() {
        let json = json!({
            "conflicts": [{
                "between": ["A", "B"], "issue": "minor", "severity": "low"
            }]
        });
        let result = parse_conflicts(&json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].severity, ConflictSeverity::Low);
    }

    #[test]
    fn test_parse_conflicts_invalid_severity() {
        let json = json!({
            "conflicts": [{
                "between": ["A", "B"], "issue": "x", "severity": "invalid"
            }]
        });
        let result = parse_conflicts(&json);
        assert!(matches!(result, Err(ModeError::InvalidValue { .. })));
    }

    #[test]
    fn test_parse_alignments_success() {
        let json = json!({
            "alignments": [{
                "stakeholders": ["A", "B"],
                "common_ground": "shared goal"
            }]
        });
        let result = parse_alignments(&json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_balanced_recommendation_success() {
        let json = json!({
            "balanced_recommendation": {
                "option": "option_a",
                "rationale": "Best overall",
                "mitigation": "Address concerns"
            }
        });
        let result = parse_balanced_recommendation(&json);
        assert!(result.is_ok());
    }

    // Utility helper tests
    #[test]
    fn test_get_str_success() {
        let json = json!({"name": "test"});
        let result = get_str(&json, "name");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_get_str_missing() {
        let result = get_str(&json!({}), "name");
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_get_f64_success() {
        let json = json!({"value": 3.14});
        let result = get_f64(&json, "value");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_f64_missing() {
        let result = get_f64(&json!({}), "value");
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
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
        let result = get_string_array(&json!({}), "items");
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    // Additional coverage tests for missing field error paths

    #[test]
    fn test_parse_criteria_missing_name() {
        let json = json!({
            "criteria": [{"weight": 0.5, "description": "No name"}]
        });
        let result = parse_criteria(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "name"));
    }

    #[test]
    fn test_parse_criteria_missing_weight() {
        let json = json!({
            "criteria": [{"name": "test", "description": "No weight"}]
        });
        let result = parse_criteria(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "weight"));
    }

    #[test]
    fn test_parse_criteria_missing_description() {
        let json = json!({
            "criteria": [{"name": "test", "weight": 0.5}]
        });
        let result = parse_criteria(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "description"));
    }

    #[test]
    fn test_parse_scores_missing() {
        let result = parse_scores(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_weighted_totals_missing() {
        let result = parse_weighted_totals(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_weighted_ranking_missing() {
        let result = parse_weighted_ranking(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_weighted_ranking_missing_rank() {
        let json = json!({
            "ranking": [{"option": "a", "score": 0.9}]
        });
        let result = parse_weighted_ranking(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "rank"));
    }

    #[test]
    fn test_parse_weighted_ranking_missing_option() {
        let json = json!({
            "ranking": [{"score": 0.9, "rank": 1}]
        });
        let result = parse_weighted_ranking(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "option"));
    }

    #[test]
    fn test_parse_weighted_ranking_missing_score() {
        let json = json!({
            "ranking": [{"option": "a", "rank": 1}]
        });
        let result = parse_weighted_ranking(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "score"));
    }

    #[test]
    fn test_parse_comparisons_missing() {
        let result = parse_comparisons(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_comparisons_missing_option_a() {
        let json = json!({
            "comparisons": [{
                "option_b": "b", "preferred": "option_a",
                "strength": "strong", "reasoning": "x"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "option_a"));
    }

    #[test]
    fn test_parse_comparisons_missing_option_b() {
        let json = json!({
            "comparisons": [{
                "option_a": "a", "preferred": "option_a",
                "strength": "strong", "reasoning": "x"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "option_b"));
    }

    #[test]
    fn test_parse_comparisons_missing_reasoning() {
        let json = json!({
            "comparisons": [{
                "option_a": "a", "option_b": "b",
                "preferred": "option_a", "strength": "strong"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "reasoning"));
    }

    #[test]
    fn test_parse_pairwise_matrix_missing() {
        let result = parse_pairwise_matrix(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_pairwise_ranking_missing() {
        let result = parse_pairwise_ranking(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_pairwise_ranking_missing_wins() {
        let json = json!({
            "ranking": [{"option": "a", "rank": 1}]
        });
        let result = parse_pairwise_ranking(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "wins"));
    }

    #[test]
    fn test_parse_pairwise_ranking_missing_rank() {
        let json = json!({
            "ranking": [{"option": "a", "wins": 3}]
        });
        let result = parse_pairwise_ranking(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "rank"));
    }

    #[test]
    fn test_parse_pairwise_ranking_missing_option() {
        let json = json!({
            "ranking": [{"wins": 3, "rank": 1}]
        });
        let result = parse_pairwise_ranking(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "option"));
    }

    #[test]
    fn test_parse_topsis_criteria_missing() {
        let result = parse_topsis_criteria(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_topsis_criteria_missing_name() {
        let json = json!({
            "criteria": [{"type": "benefit", "weight": 0.5}]
        });
        let result = parse_topsis_criteria(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "name"));
    }

    #[test]
    fn test_parse_topsis_criteria_missing_weight() {
        let json = json!({
            "criteria": [{"name": "test", "type": "benefit"}]
        });
        let result = parse_topsis_criteria(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "weight"));
    }

    #[test]
    fn test_parse_decision_matrix_missing() {
        let result = parse_decision_matrix(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_f64_array_missing() {
        let result = parse_f64_array(&json!({}), "values");
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_distances_missing() {
        let result = parse_distances(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_distances_missing_to_ideal() {
        let json = json!({
            "distances": {"a": {"to_anti_ideal": 0.8}}
        });
        let result = parse_distances(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "to_ideal"));
    }

    #[test]
    fn test_parse_distances_missing_to_anti_ideal() {
        let json = json!({
            "distances": {"a": {"to_ideal": 0.2}}
        });
        let result = parse_distances(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "to_anti_ideal"));
    }

    #[test]
    fn test_parse_relative_closeness_missing() {
        let result = parse_relative_closeness(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_topsis_ranking_missing() {
        let result = parse_topsis_ranking(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_topsis_ranking_missing_rank() {
        let json = json!({
            "ranking": [{"option": "a", "closeness": 0.8}]
        });
        let result = parse_topsis_ranking(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "rank"));
    }

    #[test]
    fn test_parse_topsis_ranking_missing_option() {
        let json = json!({
            "ranking": [{"closeness": 0.8, "rank": 1}]
        });
        let result = parse_topsis_ranking(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "option"));
    }

    #[test]
    fn test_parse_topsis_ranking_missing_closeness() {
        let json = json!({
            "ranking": [{"option": "a", "rank": 1}]
        });
        let result = parse_topsis_ranking(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "closeness"));
    }

    #[test]
    fn test_parse_stakeholders_missing() {
        let result = parse_stakeholders(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_stakeholders_missing_name() {
        let json = json!({
            "stakeholders": [{
                "interests": [], "preferred_option": "a",
                "concerns": [], "influence_level": "high"
            }]
        });
        let result = parse_stakeholders(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "name"));
    }

    #[test]
    fn test_parse_stakeholders_missing_interests() {
        let json = json!({
            "stakeholders": [{
                "name": "Test", "preferred_option": "a",
                "concerns": [], "influence_level": "high"
            }]
        });
        let result = parse_stakeholders(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "interests"));
    }

    #[test]
    fn test_parse_stakeholders_missing_preferred_option() {
        let json = json!({
            "stakeholders": [{
                "name": "Test", "interests": [],
                "concerns": [], "influence_level": "high"
            }]
        });
        let result = parse_stakeholders(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "preferred_option"));
    }

    #[test]
    fn test_parse_stakeholders_missing_concerns() {
        let json = json!({
            "stakeholders": [{
                "name": "Test", "interests": [],
                "preferred_option": "a", "influence_level": "high"
            }]
        });
        let result = parse_stakeholders(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "concerns"));
    }

    #[test]
    fn test_parse_conflicts_missing() {
        let result = parse_conflicts(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_conflicts_missing_between() {
        let json = json!({
            "conflicts": [{"issue": "test", "severity": "high"}]
        });
        let result = parse_conflicts(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "between"));
    }

    #[test]
    fn test_parse_conflicts_missing_issue() {
        let json = json!({
            "conflicts": [{"between": ["A", "B"], "severity": "high"}]
        });
        let result = parse_conflicts(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "issue"));
    }

    #[test]
    fn test_parse_alignments_missing() {
        let result = parse_alignments(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_alignments_missing_stakeholders() {
        let json = json!({
            "alignments": [{"common_ground": "shared"}]
        });
        let result = parse_alignments(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "stakeholders"));
    }

    #[test]
    fn test_parse_alignments_missing_common_ground() {
        let json = json!({
            "alignments": [{"stakeholders": ["A", "B"]}]
        });
        let result = parse_alignments(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "common_ground"));
    }

    #[test]
    fn test_parse_balanced_recommendation_missing() {
        let result = parse_balanced_recommendation(&json!({}));
        assert!(matches!(result, Err(ModeError::MissingField { .. })));
    }

    #[test]
    fn test_parse_balanced_recommendation_missing_option() {
        let json = json!({
            "balanced_recommendation": {
                "rationale": "test", "mitigation": "test"
            }
        });
        let result = parse_balanced_recommendation(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "option"));
    }

    #[test]
    fn test_parse_balanced_recommendation_missing_rationale() {
        let json = json!({
            "balanced_recommendation": {
                "option": "a", "mitigation": "test"
            }
        });
        let result = parse_balanced_recommendation(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "rationale"));
    }

    #[test]
    fn test_parse_balanced_recommendation_missing_mitigation() {
        let json = json!({
            "balanced_recommendation": {
                "option": "a", "rationale": "test"
            }
        });
        let result = parse_balanced_recommendation(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "mitigation"));
    }

    #[test]
    fn test_parse_comparisons_missing_preferred() {
        let json = json!({
            "comparisons": [{
                "option_a": "a", "option_b": "b",
                "strength": "strong", "reasoning": "x"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "preferred"));
    }

    #[test]
    fn test_parse_comparisons_missing_strength() {
        let json = json!({
            "comparisons": [{
                "option_a": "a", "option_b": "b",
                "preferred": "option_a", "reasoning": "x"
            }]
        });
        let result = parse_comparisons(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "strength"));
    }

    #[test]
    fn test_parse_stakeholders_missing_influence_level() {
        let json = json!({
            "stakeholders": [{
                "name": "Test", "interests": [],
                "preferred_option": "a", "concerns": []
            }]
        });
        let result = parse_stakeholders(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "influence_level"));
    }

    #[test]
    fn test_parse_conflicts_missing_severity() {
        let json = json!({
            "conflicts": [{"between": ["A", "B"], "issue": "test"}]
        });
        let result = parse_conflicts(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "severity"));
    }

    #[test]
    fn test_parse_topsis_criteria_missing_type() {
        let json = json!({
            "criteria": [{"name": "test", "weight": 0.5}]
        });
        let result = parse_topsis_criteria(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "type"));
    }
}
