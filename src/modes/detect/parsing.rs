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
                reason: format!("must be valid, invalid, or partially_valid, got {validity_str}"),
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

    // ========================================================================
    // parse_biases tests
    // ========================================================================

    #[test]
    fn test_parse_biases_success_all_severities() {
        let json = json!({
            "biases_detected": [
                {
                    "bias": "confirmation bias",
                    "evidence": "Only supporting evidence cited",
                    "severity": "high",
                    "impact": "Skews conclusions",
                    "debiasing": "Seek disconfirming evidence"
                },
                {
                    "bias": "anchoring bias",
                    "evidence": "First number dominates",
                    "severity": "medium",
                    "impact": "Affects estimates",
                    "debiasing": "Consider multiple anchors"
                },
                {
                    "bias": "availability heuristic",
                    "evidence": "Recent events overweighted",
                    "severity": "low",
                    "impact": "Minor distortion",
                    "debiasing": "Use base rates"
                }
            ]
        });

        let result = parse_biases(&json).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].bias, "confirmation bias");
        assert!(matches!(result[0].severity, BiasSeverity::High));
        assert!(matches!(result[1].severity, BiasSeverity::Medium));
        assert!(matches!(result[2].severity, BiasSeverity::Low));
    }

    #[test]
    fn test_parse_biases_missing_biases_detected() {
        let json = json!({});
        let result = parse_biases(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "biases_detected")
        );
    }

    #[test]
    fn test_parse_biases_missing_bias_field() {
        let json = json!({
            "biases_detected": [{
                "evidence": "test",
                "severity": "low",
                "impact": "test",
                "debiasing": "test"
            }]
        });
        let result = parse_biases(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "bias"));
    }

    #[test]
    fn test_parse_biases_missing_evidence() {
        let json = json!({
            "biases_detected": [{
                "bias": "test",
                "severity": "low",
                "impact": "test",
                "debiasing": "test"
            }]
        });
        let result = parse_biases(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "evidence"));
    }

    #[test]
    fn test_parse_biases_missing_severity() {
        let json = json!({
            "biases_detected": [{
                "bias": "test",
                "evidence": "test",
                "impact": "test",
                "debiasing": "test"
            }]
        });
        let result = parse_biases(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "severity"));
    }

    #[test]
    fn test_parse_biases_invalid_severity() {
        let json = json!({
            "biases_detected": [{
                "bias": "test",
                "evidence": "test",
                "severity": "extreme",
                "impact": "test",
                "debiasing": "test"
            }]
        });
        let result = parse_biases(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "severity")
        );
    }

    #[test]
    fn test_parse_biases_missing_impact() {
        let json = json!({
            "biases_detected": [{
                "bias": "test",
                "evidence": "test",
                "severity": "low",
                "debiasing": "test"
            }]
        });
        let result = parse_biases(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "impact"));
    }

    #[test]
    fn test_parse_biases_missing_debiasing() {
        let json = json!({
            "biases_detected": [{
                "bias": "test",
                "evidence": "test",
                "severity": "low",
                "impact": "test"
            }]
        });
        let result = parse_biases(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "debiasing"));
    }

    // ========================================================================
    // parse_bias_assessment tests
    // ========================================================================

    #[test]
    fn test_parse_bias_assessment_success() {
        let json = json!({
            "overall_assessment": {
                "bias_count": 3,
                "most_severe": "confirmation bias",
                "reasoning_quality": 0.75
            }
        });

        let result = parse_bias_assessment(&json).unwrap();
        assert_eq!(result.bias_count, 3);
        assert_eq!(result.most_severe, "confirmation bias");
        assert!((result.reasoning_quality - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_bias_assessment_missing_overall() {
        let json = json!({});
        let result = parse_bias_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "overall_assessment")
        );
    }

    #[test]
    fn test_parse_bias_assessment_missing_bias_count() {
        let json = json!({
            "overall_assessment": {
                "most_severe": "test",
                "reasoning_quality": 0.5
            }
        });
        let result = parse_bias_assessment(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "bias_count"));
    }

    #[test]
    fn test_parse_bias_assessment_missing_most_severe() {
        let json = json!({
            "overall_assessment": {
                "bias_count": 1,
                "reasoning_quality": 0.5
            }
        });
        let result = parse_bias_assessment(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "most_severe"));
    }

    #[test]
    fn test_parse_bias_assessment_missing_reasoning_quality() {
        let json = json!({
            "overall_assessment": {
                "bias_count": 1,
                "most_severe": "test"
            }
        });
        let result = parse_bias_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "reasoning_quality")
        );
    }

    #[test]
    fn test_parse_bias_assessment_invalid_reasoning_quality_too_high() {
        let json = json!({
            "overall_assessment": {
                "bias_count": 1,
                "most_severe": "test",
                "reasoning_quality": 1.5
            }
        });
        let result = parse_bias_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "reasoning_quality")
        );
    }

    #[test]
    fn test_parse_bias_assessment_invalid_reasoning_quality_negative() {
        let json = json!({
            "overall_assessment": {
                "bias_count": 1,
                "most_severe": "test",
                "reasoning_quality": -0.1
            }
        });
        let result = parse_bias_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "reasoning_quality")
        );
    }

    // ========================================================================
    // parse_fallacies tests
    // ========================================================================

    #[test]
    fn test_parse_fallacies_success_all_categories() {
        let json = json!({
            "fallacies_detected": [
                {
                    "fallacy": "affirming the consequent",
                    "category": "formal",
                    "passage": "If A then B. B. Therefore A.",
                    "explanation": "Invalid logical form",
                    "correction": "Cannot conclude A from B"
                },
                {
                    "fallacy": "ad hominem",
                    "category": "informal",
                    "passage": "He's wrong because he's biased",
                    "explanation": "Attacks person not argument",
                    "correction": "Address the argument directly"
                },
                {
                    "fallacy": "red herring",
                    "category": "relevance",
                    "passage": "But what about...",
                    "explanation": "Changes the topic",
                    "correction": "Stay on topic"
                },
                {
                    "fallacy": "begging the question",
                    "category": "presumption",
                    "passage": "It's true because it's true",
                    "explanation": "Circular reasoning",
                    "correction": "Provide independent evidence"
                }
            ]
        });

        let result = parse_fallacies(&json).unwrap();
        assert_eq!(result.len(), 4);
        assert!(matches!(result[0].category, FallacyCategory::Formal));
        assert!(matches!(result[1].category, FallacyCategory::Informal));
        assert!(matches!(result[2].category, FallacyCategory::Relevance));
        assert!(matches!(result[3].category, FallacyCategory::Presumption));
    }

    #[test]
    fn test_parse_fallacies_missing_fallacies_detected() {
        let json = json!({});
        let result = parse_fallacies(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "fallacies_detected")
        );
    }

    #[test]
    fn test_parse_fallacies_missing_fallacy() {
        let json = json!({
            "fallacies_detected": [{
                "category": "formal",
                "passage": "test",
                "explanation": "test",
                "correction": "test"
            }]
        });
        let result = parse_fallacies(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "fallacy"));
    }

    #[test]
    fn test_parse_fallacies_missing_category() {
        let json = json!({
            "fallacies_detected": [{
                "fallacy": "test",
                "passage": "test",
                "explanation": "test",
                "correction": "test"
            }]
        });
        let result = parse_fallacies(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "category"));
    }

    #[test]
    fn test_parse_fallacies_invalid_category() {
        let json = json!({
            "fallacies_detected": [{
                "fallacy": "test",
                "category": "unknown_category",
                "passage": "test",
                "explanation": "test",
                "correction": "test"
            }]
        });
        let result = parse_fallacies(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "category")
        );
    }

    #[test]
    fn test_parse_fallacies_missing_passage() {
        let json = json!({
            "fallacies_detected": [{
                "fallacy": "test",
                "category": "formal",
                "explanation": "test",
                "correction": "test"
            }]
        });
        let result = parse_fallacies(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "passage"));
    }

    #[test]
    fn test_parse_fallacies_missing_explanation() {
        let json = json!({
            "fallacies_detected": [{
                "fallacy": "test",
                "category": "formal",
                "passage": "test",
                "correction": "test"
            }]
        });
        let result = parse_fallacies(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "explanation"));
    }

    #[test]
    fn test_parse_fallacies_missing_correction() {
        let json = json!({
            "fallacies_detected": [{
                "fallacy": "test",
                "category": "formal",
                "passage": "test",
                "explanation": "test"
            }]
        });
        let result = parse_fallacies(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "correction"));
    }

    // ========================================================================
    // parse_argument_structure tests
    // ========================================================================

    #[test]
    fn test_parse_argument_structure_success_valid() {
        let json = json!({
            "argument_structure": {
                "premises": ["All men are mortal", "Socrates is a man"],
                "conclusion": "Socrates is mortal",
                "validity": "valid"
            }
        });

        let result = parse_argument_structure(&json).unwrap();
        assert_eq!(result.premises.len(), 2);
        assert_eq!(result.conclusion, "Socrates is mortal");
        assert!(matches!(result.validity, ArgumentValidity::Valid));
    }

    #[test]
    fn test_parse_argument_structure_success_invalid() {
        let json = json!({
            "argument_structure": {
                "premises": ["If A then B", "B"],
                "conclusion": "A",
                "validity": "invalid"
            }
        });

        let result = parse_argument_structure(&json).unwrap();
        assert!(matches!(result.validity, ArgumentValidity::Invalid));
    }

    #[test]
    fn test_parse_argument_structure_success_partially_valid() {
        let json = json!({
            "argument_structure": {
                "premises": ["Most X are Y", "Z is X"],
                "conclusion": "Z is likely Y",
                "validity": "partially_valid"
            }
        });

        let result = parse_argument_structure(&json).unwrap();
        assert!(matches!(result.validity, ArgumentValidity::PartiallyValid));
    }

    #[test]
    fn test_parse_argument_structure_missing_structure() {
        let json = json!({});
        let result = parse_argument_structure(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "argument_structure")
        );
    }

    #[test]
    fn test_parse_argument_structure_missing_premises() {
        let json = json!({
            "argument_structure": {
                "conclusion": "test",
                "validity": "valid"
            }
        });
        let result = parse_argument_structure(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "premises"));
    }

    #[test]
    fn test_parse_argument_structure_missing_conclusion() {
        let json = json!({
            "argument_structure": {
                "premises": ["test"],
                "validity": "valid"
            }
        });
        let result = parse_argument_structure(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "conclusion"));
    }

    #[test]
    fn test_parse_argument_structure_missing_validity() {
        let json = json!({
            "argument_structure": {
                "premises": ["test"],
                "conclusion": "test"
            }
        });
        let result = parse_argument_structure(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "validity"));
    }

    #[test]
    fn test_parse_argument_structure_invalid_validity() {
        let json = json!({
            "argument_structure": {
                "premises": ["test"],
                "conclusion": "test",
                "validity": "somewhat_valid"
            }
        });
        let result = parse_argument_structure(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "validity")
        );
    }

    // ========================================================================
    // parse_fallacy_assessment tests
    // ========================================================================

    #[test]
    fn test_parse_fallacy_assessment_success() {
        let json = json!({
            "overall_assessment": {
                "fallacy_count": 2,
                "argument_strength": 0.6,
                "most_critical": "ad hominem"
            }
        });

        let result = parse_fallacy_assessment(&json).unwrap();
        assert_eq!(result.fallacy_count, 2);
        assert!((result.argument_strength - 0.6).abs() < f64::EPSILON);
        assert_eq!(result.most_critical, "ad hominem");
    }

    #[test]
    fn test_parse_fallacy_assessment_missing_overall() {
        let json = json!({});
        let result = parse_fallacy_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "overall_assessment")
        );
    }

    #[test]
    fn test_parse_fallacy_assessment_missing_fallacy_count() {
        let json = json!({
            "overall_assessment": {
                "argument_strength": 0.5,
                "most_critical": "test"
            }
        });
        let result = parse_fallacy_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "fallacy_count")
        );
    }

    #[test]
    fn test_parse_fallacy_assessment_missing_argument_strength() {
        let json = json!({
            "overall_assessment": {
                "fallacy_count": 1,
                "most_critical": "test"
            }
        });
        let result = parse_fallacy_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "argument_strength")
        );
    }

    #[test]
    fn test_parse_fallacy_assessment_invalid_argument_strength_too_high() {
        let json = json!({
            "overall_assessment": {
                "fallacy_count": 1,
                "argument_strength": 1.1,
                "most_critical": "test"
            }
        });
        let result = parse_fallacy_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "argument_strength")
        );
    }

    #[test]
    fn test_parse_fallacy_assessment_invalid_argument_strength_negative() {
        let json = json!({
            "overall_assessment": {
                "fallacy_count": 1,
                "argument_strength": -0.5,
                "most_critical": "test"
            }
        });
        let result = parse_fallacy_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "argument_strength")
        );
    }

    #[test]
    fn test_parse_fallacy_assessment_missing_most_critical() {
        let json = json!({
            "overall_assessment": {
                "fallacy_count": 1,
                "argument_strength": 0.5
            }
        });
        let result = parse_fallacy_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "most_critical")
        );
    }
}
