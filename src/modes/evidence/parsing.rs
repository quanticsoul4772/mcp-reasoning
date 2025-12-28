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
    let qual = piece.get("quality").ok_or_else(|| ModeError::MissingField {
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
                reason: format!(
                    "must be increase, decrease, or unchanged, got {direction_str}"
                ),
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
                reason: format!(
                    "must be strong, moderate, or slight, got {magnitude_str}"
                ),
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
