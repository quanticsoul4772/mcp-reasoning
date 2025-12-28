//! JSON parsing helpers for evidence mode.
//!
//! These functions extract structured data from LLM JSON responses.
//! Each parse_* function handles a specific evidence component (pieces,
//! assessments, Bayesian updates). Returns ModeError::MissingField for absent fields.

use crate::error::ModeError;

use super::types::{
    BeliefDirection, BeliefMagnitude, BeliefUpdate, Credibility, EvidenceAnalysis, EvidencePiece,
    EvidenceQuality, OverallEvidenceAssessment, Posterior, Prior, SourceType,
};

// ============================================================================
// Assess Parsing
// ============================================================================

pub fn parse_evidence_pieces(json: &serde_json::Value) -> Result<Vec<EvidencePiece>, ModeError> {
    let pieces = json
        .get("evidence_pieces")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "evidence_pieces".to_string(),
        })?;

    pieces
        .iter()
        .map(|p| {
            let summary = p
                .get("summary")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "summary".to_string(),
                })?
                .to_string();

            let source_type_str = p
                .get("source_type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "source_type".to_string(),
                })?;

            let source_type = match source_type_str.to_lowercase().as_str() {
                "primary" => SourceType::Primary,
                "secondary" => SourceType::Secondary,
                "tertiary" => SourceType::Tertiary,
                "anecdotal" => SourceType::Anecdotal,
                _ => {
                    return Err(ModeError::InvalidValue {
                        field: "source_type".to_string(),
                        reason: format!(
                        "must be primary, secondary, tertiary, or anecdotal, got {source_type_str}"
                    ),
                    })
                }
            };

            let credibility = parse_credibility(p)?;
            let quality = parse_quality(p)?;

            Ok(EvidencePiece {
                summary,
                source_type,
                credibility,
                quality,
            })
        })
        .collect()
}

pub fn parse_credibility(piece: &serde_json::Value) -> Result<Credibility, ModeError> {
    let cred = piece
        .get("credibility")
        .ok_or_else(|| ModeError::MissingField {
            field: "credibility".to_string(),
        })?;

    let expertise = get_f64(cred, "expertise")?;
    let objectivity = get_f64(cred, "objectivity")?;
    let corroboration = get_f64(cred, "corroboration")?;
    let recency = get_f64(cred, "recency")?;
    let overall = get_f64(cred, "overall")?;

    Ok(Credibility {
        expertise,
        objectivity,
        corroboration,
        recency,
        overall,
    })
}

pub fn parse_quality(piece: &serde_json::Value) -> Result<EvidenceQuality, ModeError> {
    let qual = piece
        .get("quality")
        .ok_or_else(|| ModeError::MissingField {
            field: "quality".to_string(),
        })?;

    let relevance = get_f64(qual, "relevance")?;
    let strength = get_f64(qual, "strength")?;
    let representativeness = get_f64(qual, "representativeness")?;
    let overall = get_f64(qual, "overall")?;

    Ok(EvidenceQuality {
        relevance,
        strength,
        representativeness,
        overall,
    })
}

pub fn parse_overall_assessment(
    json: &serde_json::Value,
) -> Result<OverallEvidenceAssessment, ModeError> {
    let assessment = json
        .get("overall_assessment")
        .ok_or_else(|| ModeError::MissingField {
            field: "overall_assessment".to_string(),
        })?;

    let evidential_support = get_f64(assessment, "evidential_support")?;

    let key_strengths = get_string_array(assessment, "key_strengths")?;
    let key_weaknesses = get_string_array(assessment, "key_weaknesses")?;
    let gaps = get_string_array(assessment, "gaps")?;

    Ok(OverallEvidenceAssessment {
        evidential_support,
        key_strengths,
        key_weaknesses,
        gaps,
    })
}

pub fn parse_confidence(json: &serde_json::Value) -> Result<f64, ModeError> {
    let confidence = json
        .get("confidence_in_conclusion")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: "confidence_in_conclusion".to_string(),
        })?;

    if !(0.0..=1.0).contains(&confidence) {
        return Err(ModeError::InvalidValue {
            field: "confidence_in_conclusion".to_string(),
            reason: format!("must be between 0.0 and 1.0, got {confidence}"),
        });
    }

    Ok(confidence)
}

// ============================================================================
// Probabilistic Parsing
// ============================================================================

pub fn parse_prior(json: &serde_json::Value) -> Result<Prior, ModeError> {
    let prior = json.get("prior").ok_or_else(|| ModeError::MissingField {
        field: "prior".to_string(),
    })?;

    let probability = get_f64(prior, "probability")?;
    let basis = prior
        .get("basis")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModeError::MissingField {
            field: "basis".to_string(),
        })?
        .to_string();

    Ok(Prior { probability, basis })
}

pub fn parse_evidence_analysis(
    json: &serde_json::Value,
) -> Result<Vec<EvidenceAnalysis>, ModeError> {
    let analyses = json
        .get("evidence_analysis")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "evidence_analysis".to_string(),
        })?;

    analyses
        .iter()
        .map(|a| {
            let evidence = a
                .get("evidence")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| ModeError::MissingField {
                    field: "evidence".to_string(),
                })?
                .to_string();

            let likelihood_if_true = get_f64(a, "likelihood_if_true")?;
            let likelihood_if_false = get_f64(a, "likelihood_if_false")?;
            let bayes_factor = get_f64(a, "bayes_factor")?;

            Ok(EvidenceAnalysis {
                evidence,
                likelihood_if_true,
                likelihood_if_false,
                bayes_factor,
            })
        })
        .collect()
}

pub fn parse_posterior(json: &serde_json::Value) -> Result<Posterior, ModeError> {
    let post = json
        .get("posterior")
        .ok_or_else(|| ModeError::MissingField {
            field: "posterior".to_string(),
        })?;

    let probability = get_f64(post, "probability")?;
    let calculation = post
        .get("calculation")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModeError::MissingField {
            field: "calculation".to_string(),
        })?
        .to_string();

    Ok(Posterior {
        probability,
        calculation,
    })
}

pub fn parse_belief_update(json: &serde_json::Value) -> Result<BeliefUpdate, ModeError> {
    let update = json
        .get("belief_update")
        .ok_or_else(|| ModeError::MissingField {
            field: "belief_update".to_string(),
        })?;

    let direction_str = update
        .get("direction")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModeError::MissingField {
            field: "direction".to_string(),
        })?;

    let direction = match direction_str.to_lowercase().as_str() {
        "increase" => BeliefDirection::Increase,
        "decrease" => BeliefDirection::Decrease,
        "unchanged" => BeliefDirection::Unchanged,
        _ => {
            return Err(ModeError::InvalidValue {
                field: "direction".to_string(),
                reason: format!("must be increase, decrease, or unchanged, got {direction_str}"),
            })
        }
    };

    let magnitude_str = update
        .get("magnitude")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModeError::MissingField {
            field: "magnitude".to_string(),
        })?;

    let magnitude = match magnitude_str.to_lowercase().as_str() {
        "strong" => BeliefMagnitude::Strong,
        "moderate" => BeliefMagnitude::Moderate,
        "slight" => BeliefMagnitude::Slight,
        _ => {
            return Err(ModeError::InvalidValue {
                field: "magnitude".to_string(),
                reason: format!("must be strong, moderate, or slight, got {magnitude_str}"),
            })
        }
    };

    let interpretation = update
        .get("interpretation")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ModeError::MissingField {
            field: "interpretation".to_string(),
        })?
        .to_string();

    Ok(BeliefUpdate {
        direction,
        magnitude,
        interpretation,
    })
}

// ============================================================================
// Utility Helpers
// ============================================================================

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========================================================================
    // Helper function tests
    // ========================================================================

    #[test]
    fn test_get_f64_success() {
        let json = json!({"score": 0.75});
        let result = get_f64(&json, "score").unwrap();
        assert!((result - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_get_f64_missing() {
        let json = json!({});
        let result = get_f64(&json, "score");
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "score"));
    }

    #[test]
    fn test_get_string_array_success() {
        let json = json!({"items": ["a", "b", "c"]});
        let result = get_string_array(&json, "items").unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_get_string_array_missing() {
        let json = json!({});
        let result = get_string_array(&json, "items");
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "items"));
    }

    #[test]
    fn test_get_string_array_filters_non_strings() {
        let json = json!({"items": ["a", 123, "b", null]});
        let result = get_string_array(&json, "items").unwrap();
        assert_eq!(result, vec!["a", "b"]);
    }

    // ========================================================================
    // parse_evidence_pieces tests
    // ========================================================================

    #[test]
    fn test_parse_evidence_pieces_success_all_source_types() {
        let json = json!({
            "evidence_pieces": [
                {
                    "summary": "Direct observation",
                    "source_type": "primary",
                    "credibility": {
                        "expertise": 0.9,
                        "objectivity": 0.8,
                        "corroboration": 0.7,
                        "recency": 0.95,
                        "overall": 0.85
                    },
                    "quality": {
                        "relevance": 0.9,
                        "strength": 0.85,
                        "representativeness": 0.8,
                        "overall": 0.85
                    }
                },
                {
                    "summary": "Meta-analysis",
                    "source_type": "secondary",
                    "credibility": {
                        "expertise": 0.8,
                        "objectivity": 0.9,
                        "corroboration": 0.85,
                        "recency": 0.7,
                        "overall": 0.81
                    },
                    "quality": {
                        "relevance": 0.85,
                        "strength": 0.9,
                        "representativeness": 0.75,
                        "overall": 0.83
                    }
                },
                {
                    "summary": "Textbook reference",
                    "source_type": "tertiary",
                    "credibility": {
                        "expertise": 0.7,
                        "objectivity": 0.85,
                        "corroboration": 0.9,
                        "recency": 0.5,
                        "overall": 0.74
                    },
                    "quality": {
                        "relevance": 0.8,
                        "strength": 0.7,
                        "representativeness": 0.85,
                        "overall": 0.78
                    }
                },
                {
                    "summary": "Personal account",
                    "source_type": "anecdotal",
                    "credibility": {
                        "expertise": 0.5,
                        "objectivity": 0.4,
                        "corroboration": 0.3,
                        "recency": 0.9,
                        "overall": 0.52
                    },
                    "quality": {
                        "relevance": 0.6,
                        "strength": 0.4,
                        "representativeness": 0.3,
                        "overall": 0.43
                    }
                }
            ]
        });

        let result = parse_evidence_pieces(&json).unwrap();
        assert_eq!(result.len(), 4);
        assert!(matches!(result[0].source_type, SourceType::Primary));
        assert!(matches!(result[1].source_type, SourceType::Secondary));
        assert!(matches!(result[2].source_type, SourceType::Tertiary));
        assert!(matches!(result[3].source_type, SourceType::Anecdotal));
    }

    #[test]
    fn test_parse_evidence_pieces_missing_array() {
        let json = json!({});
        let result = parse_evidence_pieces(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "evidence_pieces")
        );
    }

    #[test]
    fn test_parse_evidence_pieces_missing_summary() {
        let json = json!({
            "evidence_pieces": [{
                "source_type": "primary",
                "credibility": {
                    "expertise": 0.9, "objectivity": 0.8, "corroboration": 0.7, "recency": 0.9, "overall": 0.85
                },
                "quality": {
                    "relevance": 0.9, "strength": 0.85, "representativeness": 0.8, "overall": 0.85
                }
            }]
        });
        let result = parse_evidence_pieces(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "summary"));
    }

    #[test]
    fn test_parse_evidence_pieces_missing_source_type() {
        let json = json!({
            "evidence_pieces": [{
                "summary": "test",
                "credibility": {
                    "expertise": 0.9, "objectivity": 0.8, "corroboration": 0.7, "recency": 0.9, "overall": 0.85
                },
                "quality": {
                    "relevance": 0.9, "strength": 0.85, "representativeness": 0.8, "overall": 0.85
                }
            }]
        });
        let result = parse_evidence_pieces(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "source_type"));
    }

    #[test]
    fn test_parse_evidence_pieces_invalid_source_type() {
        let json = json!({
            "evidence_pieces": [{
                "summary": "test",
                "source_type": "unknown_type",
                "credibility": {
                    "expertise": 0.9, "objectivity": 0.8, "corroboration": 0.7, "recency": 0.9, "overall": 0.85
                },
                "quality": {
                    "relevance": 0.9, "strength": 0.85, "representativeness": 0.8, "overall": 0.85
                }
            }]
        });
        let result = parse_evidence_pieces(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "source_type")
        );
    }

    // ========================================================================
    // parse_credibility tests
    // ========================================================================

    #[test]
    fn test_parse_credibility_success() {
        let json = json!({
            "credibility": {
                "expertise": 0.9,
                "objectivity": 0.85,
                "corroboration": 0.8,
                "recency": 0.75,
                "overall": 0.82
            }
        });

        let result = parse_credibility(&json).unwrap();
        assert!((result.expertise - 0.9).abs() < f64::EPSILON);
        assert!((result.overall - 0.82).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_credibility_missing_credibility() {
        let json = json!({});
        let result = parse_credibility(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "credibility"));
    }

    #[test]
    fn test_parse_credibility_missing_expertise() {
        let json = json!({
            "credibility": {
                "objectivity": 0.85,
                "corroboration": 0.8,
                "recency": 0.75,
                "overall": 0.82
            }
        });
        let result = parse_credibility(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "expertise"));
    }

    #[test]
    fn test_parse_credibility_missing_objectivity() {
        let json = json!({
            "credibility": {
                "expertise": 0.9,
                "corroboration": 0.8,
                "recency": 0.75,
                "overall": 0.82
            }
        });
        let result = parse_credibility(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "objectivity"));
    }

    #[test]
    fn test_parse_credibility_missing_corroboration() {
        let json = json!({
            "credibility": {
                "expertise": 0.9,
                "objectivity": 0.85,
                "recency": 0.75,
                "overall": 0.82
            }
        });
        let result = parse_credibility(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "corroboration")
        );
    }

    #[test]
    fn test_parse_credibility_missing_recency() {
        let json = json!({
            "credibility": {
                "expertise": 0.9,
                "objectivity": 0.85,
                "corroboration": 0.8,
                "overall": 0.82
            }
        });
        let result = parse_credibility(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "recency"));
    }

    #[test]
    fn test_parse_credibility_missing_overall() {
        let json = json!({
            "credibility": {
                "expertise": 0.9,
                "objectivity": 0.85,
                "corroboration": 0.8,
                "recency": 0.75
            }
        });
        let result = parse_credibility(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "overall"));
    }

    // ========================================================================
    // parse_quality tests
    // ========================================================================

    #[test]
    fn test_parse_quality_success() {
        let json = json!({
            "quality": {
                "relevance": 0.9,
                "strength": 0.85,
                "representativeness": 0.8,
                "overall": 0.85
            }
        });

        let result = parse_quality(&json).unwrap();
        assert!((result.relevance - 0.9).abs() < f64::EPSILON);
        assert!((result.overall - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_quality_missing_quality() {
        let json = json!({});
        let result = parse_quality(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "quality"));
    }

    #[test]
    fn test_parse_quality_missing_relevance() {
        let json = json!({
            "quality": {
                "strength": 0.85,
                "representativeness": 0.8,
                "overall": 0.85
            }
        });
        let result = parse_quality(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "relevance"));
    }

    #[test]
    fn test_parse_quality_missing_strength() {
        let json = json!({
            "quality": {
                "relevance": 0.9,
                "representativeness": 0.8,
                "overall": 0.85
            }
        });
        let result = parse_quality(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "strength"));
    }

    #[test]
    fn test_parse_quality_missing_representativeness() {
        let json = json!({
            "quality": {
                "relevance": 0.9,
                "strength": 0.85,
                "overall": 0.85
            }
        });
        let result = parse_quality(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "representativeness")
        );
    }

    #[test]
    fn test_parse_quality_missing_overall() {
        let json = json!({
            "quality": {
                "relevance": 0.9,
                "strength": 0.85,
                "representativeness": 0.8
            }
        });
        let result = parse_quality(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "overall"));
    }

    // ========================================================================
    // parse_overall_assessment tests
    // ========================================================================

    #[test]
    fn test_parse_overall_assessment_success() {
        let json = json!({
            "overall_assessment": {
                "evidential_support": 0.75,
                "key_strengths": ["Strong methodology", "Large sample size"],
                "key_weaknesses": ["Limited scope"],
                "gaps": ["Missing longitudinal data"]
            }
        });

        let result = parse_overall_assessment(&json).unwrap();
        assert!((result.evidential_support - 0.75).abs() < f64::EPSILON);
        assert_eq!(result.key_strengths.len(), 2);
        assert_eq!(result.key_weaknesses.len(), 1);
        assert_eq!(result.gaps.len(), 1);
    }

    #[test]
    fn test_parse_overall_assessment_missing() {
        let json = json!({});
        let result = parse_overall_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "overall_assessment")
        );
    }

    #[test]
    fn test_parse_overall_assessment_missing_evidential_support() {
        let json = json!({
            "overall_assessment": {
                "key_strengths": [],
                "key_weaknesses": [],
                "gaps": []
            }
        });
        let result = parse_overall_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "evidential_support")
        );
    }

    #[test]
    fn test_parse_overall_assessment_missing_key_strengths() {
        let json = json!({
            "overall_assessment": {
                "evidential_support": 0.75,
                "key_weaknesses": [],
                "gaps": []
            }
        });
        let result = parse_overall_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "key_strengths")
        );
    }

    #[test]
    fn test_parse_overall_assessment_missing_key_weaknesses() {
        let json = json!({
            "overall_assessment": {
                "evidential_support": 0.75,
                "key_strengths": [],
                "gaps": []
            }
        });
        let result = parse_overall_assessment(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "key_weaknesses")
        );
    }

    #[test]
    fn test_parse_overall_assessment_missing_gaps() {
        let json = json!({
            "overall_assessment": {
                "evidential_support": 0.75,
                "key_strengths": [],
                "key_weaknesses": []
            }
        });
        let result = parse_overall_assessment(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "gaps"));
    }

    // ========================================================================
    // parse_confidence tests
    // ========================================================================

    #[test]
    fn test_parse_confidence_success() {
        let json = json!({"confidence_in_conclusion": 0.85});
        let result = parse_confidence(&json).unwrap();
        assert!((result - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_confidence_missing() {
        let json = json!({});
        let result = parse_confidence(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "confidence_in_conclusion")
        );
    }

    #[test]
    fn test_parse_confidence_invalid_too_high() {
        let json = json!({"confidence_in_conclusion": 1.5});
        let result = parse_confidence(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "confidence_in_conclusion")
        );
    }

    #[test]
    fn test_parse_confidence_invalid_negative() {
        let json = json!({"confidence_in_conclusion": -0.1});
        let result = parse_confidence(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "confidence_in_conclusion")
        );
    }

    // ========================================================================
    // parse_prior tests
    // ========================================================================

    #[test]
    fn test_parse_prior_success() {
        let json = json!({
            "prior": {
                "probability": 0.5,
                "basis": "Base rate from population studies"
            }
        });

        let result = parse_prior(&json).unwrap();
        assert!((result.probability - 0.5).abs() < f64::EPSILON);
        assert_eq!(result.basis, "Base rate from population studies");
    }

    #[test]
    fn test_parse_prior_missing() {
        let json = json!({});
        let result = parse_prior(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "prior"));
    }

    #[test]
    fn test_parse_prior_missing_probability() {
        let json = json!({
            "prior": {
                "basis": "test"
            }
        });
        let result = parse_prior(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "probability"));
    }

    #[test]
    fn test_parse_prior_missing_basis() {
        let json = json!({
            "prior": {
                "probability": 0.5
            }
        });
        let result = parse_prior(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "basis"));
    }

    // ========================================================================
    // parse_evidence_analysis tests
    // ========================================================================

    #[test]
    fn test_parse_evidence_analysis_success() {
        let json = json!({
            "evidence_analysis": [
                {
                    "evidence": "Positive test result",
                    "likelihood_if_true": 0.95,
                    "likelihood_if_false": 0.05,
                    "bayes_factor": 19.0
                }
            ]
        });

        let result = parse_evidence_analysis(&json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].evidence, "Positive test result");
        assert!((result[0].bayes_factor - 19.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_evidence_analysis_missing() {
        let json = json!({});
        let result = parse_evidence_analysis(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "evidence_analysis")
        );
    }

    #[test]
    fn test_parse_evidence_analysis_missing_evidence() {
        let json = json!({
            "evidence_analysis": [{
                "likelihood_if_true": 0.95,
                "likelihood_if_false": 0.05,
                "bayes_factor": 19.0
            }]
        });
        let result = parse_evidence_analysis(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "evidence"));
    }

    #[test]
    fn test_parse_evidence_analysis_missing_likelihood_if_true() {
        let json = json!({
            "evidence_analysis": [{
                "evidence": "test",
                "likelihood_if_false": 0.05,
                "bayes_factor": 19.0
            }]
        });
        let result = parse_evidence_analysis(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "likelihood_if_true")
        );
    }

    #[test]
    fn test_parse_evidence_analysis_missing_likelihood_if_false() {
        let json = json!({
            "evidence_analysis": [{
                "evidence": "test",
                "likelihood_if_true": 0.95,
                "bayes_factor": 19.0
            }]
        });
        let result = parse_evidence_analysis(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "likelihood_if_false")
        );
    }

    #[test]
    fn test_parse_evidence_analysis_missing_bayes_factor() {
        let json = json!({
            "evidence_analysis": [{
                "evidence": "test",
                "likelihood_if_true": 0.95,
                "likelihood_if_false": 0.05
            }]
        });
        let result = parse_evidence_analysis(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "bayes_factor")
        );
    }

    // ========================================================================
    // parse_posterior tests
    // ========================================================================

    #[test]
    fn test_parse_posterior_success() {
        let json = json!({
            "posterior": {
                "probability": 0.9,
                "calculation": "P(H|E) = P(E|H) * P(H) / P(E)"
            }
        });

        let result = parse_posterior(&json).unwrap();
        assert!((result.probability - 0.9).abs() < f64::EPSILON);
        assert!(result.calculation.contains("P(H|E)"));
    }

    #[test]
    fn test_parse_posterior_missing() {
        let json = json!({});
        let result = parse_posterior(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "posterior"));
    }

    #[test]
    fn test_parse_posterior_missing_probability() {
        let json = json!({
            "posterior": {
                "calculation": "test"
            }
        });
        let result = parse_posterior(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "probability"));
    }

    #[test]
    fn test_parse_posterior_missing_calculation() {
        let json = json!({
            "posterior": {
                "probability": 0.9
            }
        });
        let result = parse_posterior(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "calculation"));
    }

    // ========================================================================
    // parse_belief_update tests
    // ========================================================================

    #[test]
    fn test_parse_belief_update_success_all_directions() {
        let test_cases = [
            ("increase", BeliefDirection::Increase),
            ("decrease", BeliefDirection::Decrease),
            ("unchanged", BeliefDirection::Unchanged),
        ];

        for (direction_str, expected) in test_cases {
            let json = json!({
                "belief_update": {
                    "direction": direction_str,
                    "magnitude": "moderate",
                    "interpretation": "test"
                }
            });

            let result = parse_belief_update(&json).unwrap();
            assert!(
                matches!(result.direction, expected2 if std::mem::discriminant(&expected2) == std::mem::discriminant(&expected))
            );
        }
    }

    #[test]
    fn test_parse_belief_update_success_all_magnitudes() {
        let test_cases = [
            ("strong", BeliefMagnitude::Strong),
            ("moderate", BeliefMagnitude::Moderate),
            ("slight", BeliefMagnitude::Slight),
        ];

        for (magnitude_str, expected) in test_cases {
            let json = json!({
                "belief_update": {
                    "direction": "increase",
                    "magnitude": magnitude_str,
                    "interpretation": "test"
                }
            });

            let result = parse_belief_update(&json).unwrap();
            assert!(
                matches!(result.magnitude, expected2 if std::mem::discriminant(&expected2) == std::mem::discriminant(&expected))
            );
        }
    }

    #[test]
    fn test_parse_belief_update_missing() {
        let json = json!({});
        let result = parse_belief_update(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "belief_update")
        );
    }

    #[test]
    fn test_parse_belief_update_missing_direction() {
        let json = json!({
            "belief_update": {
                "magnitude": "moderate",
                "interpretation": "test"
            }
        });
        let result = parse_belief_update(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "direction"));
    }

    #[test]
    fn test_parse_belief_update_invalid_direction() {
        let json = json!({
            "belief_update": {
                "direction": "sideways",
                "magnitude": "moderate",
                "interpretation": "test"
            }
        });
        let result = parse_belief_update(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "direction")
        );
    }

    #[test]
    fn test_parse_belief_update_missing_magnitude() {
        let json = json!({
            "belief_update": {
                "direction": "increase",
                "interpretation": "test"
            }
        });
        let result = parse_belief_update(&json);
        assert!(matches!(result, Err(ModeError::MissingField { field }) if field == "magnitude"));
    }

    #[test]
    fn test_parse_belief_update_invalid_magnitude() {
        let json = json!({
            "belief_update": {
                "direction": "increase",
                "magnitude": "huge",
                "interpretation": "test"
            }
        });
        let result = parse_belief_update(&json);
        assert!(
            matches!(result, Err(ModeError::InvalidValue { field, .. }) if field == "magnitude")
        );
    }

    #[test]
    fn test_parse_belief_update_missing_interpretation() {
        let json = json!({
            "belief_update": {
                "direction": "increase",
                "magnitude": "moderate"
            }
        });
        let result = parse_belief_update(&json);
        assert!(
            matches!(result, Err(ModeError::MissingField { field }) if field == "interpretation")
        );
    }
}
