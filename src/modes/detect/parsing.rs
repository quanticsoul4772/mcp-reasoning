//! JSON parsing helpers for detect mode.
//!
//! These functions extract structured data from LLM JSON responses.
//! Each parse_* function handles a specific detection component (biases,
//! fallacies, assessments). Returns ModeError::MissingField for absent fields.

use crate::error::ModeError;

use super::types::{
    ArgumentStructure, ArgumentValidity, BiasAssessment, BiasSeverity, DetectedBias,
    DetectedFallacy, FallacyAssessment, FallacyCategory,
};

// ============================================================================
// Bias Parsing
// ============================================================================

pub fn parse_biases(json: &serde_json::Value) -> Result<Vec<DetectedBias>, ModeError> {
    let biases_array = json
        .get("biases_detected")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "biases_detected".to_string(),
        })?;

    biases_array
        .iter()
        .map(|b| {
            let bias = b
                .get("bias")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "bias".to_string(),
                })?
                .to_string();

            let evidence = b
                .get("evidence")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "evidence".to_string(),
                })?
                .to_string();

            let severity_str = b
                .get("severity")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "severity".to_string(),
                })?;

            let severity = match severity_str.to_lowercase().as_str() {
                "low" => BiasSeverity::Low,
                "medium" => BiasSeverity::Medium,
                "high" => BiasSeverity::High,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "severity".to_string(),
                        reason: format!("must be low, medium, or high, got {severity_str}"),
                    })
                }
            };

            let impact = b
                .get("impact")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "impact".to_string(),
                })?
                .to_string();

            let debiasing = b
                .get("debiasing")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "debiasing".to_string(),
                })?
                .to_string();

            Ok(DetectedBias {
                bias,
                evidence,
                severity,
                impact,
                debiasing,
            })
        })
        .collect()
}

pub fn parse_bias_assessment(json: &serde_json::Value) -> Result<BiasAssessment, ModeError> {
    let assessment = json
        .get("overall_assessment")
        .ok_or_else(|| ModeError::MissingField {
            field: "overall_assessment".to_string(),
        })?;

    // Bias counts from analysis are small integers (typically < 20)
    #[allow(clippy::cast_possible_truncation)]
    let bias_count = assessment
        .get("bias_count")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ModeError::MissingField {
            field: "bias_count".to_string(),
        })? as u32;

    let most_severe = assessment
        .get("most_severe")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModeError::MissingField {
            field: "most_severe".to_string(),
        })?
        .to_string();

    let reasoning_quality = assessment
        .get("reasoning_quality")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: "reasoning_quality".to_string(),
        })?;

    if !(0.0..=1.0).contains(&reasoning_quality) {
        return Err(ModeError::InvalidValue {
            field: "reasoning_quality".to_string(),
            reason: format!("must be between 0.0 and 1.0, got {reasoning_quality}"),
        });
    }

    Ok(BiasAssessment {
        bias_count,
        most_severe,
        reasoning_quality,
    })
}

// ============================================================================
// Fallacy Parsing
// ============================================================================

pub fn parse_fallacies(json: &serde_json::Value) -> Result<Vec<DetectedFallacy>, ModeError> {
    let fallacies_array = json
        .get("fallacies_detected")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "fallacies_detected".to_string(),
        })?;

    fallacies_array
        .iter()
        .map(|f| {
            let fallacy = f
                .get("fallacy")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "fallacy".to_string(),
                })?
                .to_string();

            let category_str = f
                .get("category")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "category".to_string(),
                })?;

            let category = match category_str.to_lowercase().as_str() {
                "formal" => FallacyCategory::Formal,
                "informal" => FallacyCategory::Informal,
                "relevance" => FallacyCategory::Relevance,
                "presumption" => FallacyCategory::Presumption,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "category".to_string(),
                        reason: format!(
                            "must be formal, informal, relevance, or presumption, got {category_str}"
                        ),
                    })
                }
            };

            let passage = f
                .get("passage")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "passage".to_string(),
                })?
                .to_string();

            let explanation = f
                .get("explanation")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "explanation".to_string(),
                })?
                .to_string();

            let correction = f
                .get("correction")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "correction".to_string(),
                })?
                .to_string();

            Ok(DetectedFallacy {
                fallacy,
                category,
                passage,
                explanation,
                correction,
            })
        })
        .collect()
}

pub fn parse_argument_structure(json: &serde_json::Value) -> Result<ArgumentStructure, ModeError> {
    let structure = json
        .get("argument_structure")
        .ok_or_else(|| ModeError::MissingField {
            field: "argument_structure".to_string(),
        })?;

    let premises = structure
        .get("premises")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "premises".to_string(),
        })?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    let conclusion = structure
        .get("conclusion")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModeError::MissingField {
            field: "conclusion".to_string(),
        })?
        .to_string();

    let validity_str = structure
        .get("validity")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModeError::MissingField {
            field: "validity".to_string(),
        })?;

    let validity = match validity_str.to_lowercase().as_str() {
        "valid" => ArgumentValidity::Valid,
        "invalid" => ArgumentValidity::Invalid,
        "partially_valid" => ArgumentValidity::PartiallyValid,
        _ => {
            return Err(ModeError::InvalidValue {
                field: "validity".to_string(),
                reason: format!(
                    "must be valid, invalid, or partially_valid, got {validity_str}"
                ),
            })
        }
    };

    Ok(ArgumentStructure {
        premises,
        conclusion,
        validity,
    })
}

pub fn parse_fallacy_assessment(json: &serde_json::Value) -> Result<FallacyAssessment, ModeError> {
    let assessment = json
        .get("overall_assessment")
        .ok_or_else(|| ModeError::MissingField {
            field: "overall_assessment".to_string(),
        })?;

    // Fallacy counts from analysis are small integers (typically < 20)
    #[allow(clippy::cast_possible_truncation)]
    let fallacy_count = assessment
        .get("fallacy_count")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ModeError::MissingField {
            field: "fallacy_count".to_string(),
        })? as u32;

    let argument_strength = assessment
        .get("argument_strength")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: "argument_strength".to_string(),
        })?;

    if !(0.0..=1.0).contains(&argument_strength) {
        return Err(ModeError::InvalidValue {
            field: "argument_strength".to_string(),
            reason: format!("must be between 0.0 and 1.0, got {argument_strength}"),
        });
    }

    let most_critical = assessment
        .get("most_critical")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModeError::MissingField {
            field: "most_critical".to_string(),
        })?
        .to_string();

    Ok(FallacyAssessment {
        fallacy_count,
        argument_strength,
        most_critical,
    })
}
