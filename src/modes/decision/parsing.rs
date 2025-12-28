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
        let scores_obj = scores_val.as_object().ok_or_else(|| ModeError::InvalidValue {
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

pub fn parse_relative_closeness(json: &serde_json::Value) -> Result<HashMap<String, f64>, ModeError> {
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
