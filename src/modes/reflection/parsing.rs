//! JSON parsing helpers for reflection mode.
//!
//! These functions extract structured data from LLM JSON responses.
//! Each parse_* function handles a specific reflection component (analysis,
//! improvements, assessments). Returns ModeError::MissingField for absent fields.

use crate::error::ModeError;

use super::types::{Improvement, Priority, ReasoningAnalysis, SessionAssessment};

// ============================================================================
// Process Parsing
// ============================================================================

/// Parse reasoning analysis from JSON.
pub fn parse_analysis(json: &serde_json::Value) -> Result<ReasoningAnalysis, ModeError> {
    let analysis_json = json
        .get("analysis")
        .ok_or_else(|| ModeError::MissingField {
            field: "analysis".to_string(),
        })?;

    let strengths =
        parse_string_array(analysis_json, "strengths").ok_or_else(|| ModeError::MissingField {
            field: "analysis.strengths".to_string(),
        })?;

    let weaknesses =
        parse_string_array(analysis_json, "weaknesses").ok_or_else(|| ModeError::MissingField {
            field: "analysis.weaknesses".to_string(),
        })?;

    let gaps = parse_string_array(analysis_json, "gaps");

    let mut analysis = ReasoningAnalysis::new(strengths, weaknesses);
    if let Some(g) = gaps {
        analysis = analysis.with_gaps(g);
    }

    Ok(analysis)
}

/// Parse improvements from JSON.
pub fn parse_improvements(json: &serde_json::Value) -> Result<Vec<Improvement>, ModeError> {
    let improvements_json = json
        .get("improvements")
        .ok_or_else(|| ModeError::MissingField {
            field: "improvements".to_string(),
        })?;

    let improvements_arr = improvements_json
        .as_array()
        .ok_or_else(|| ModeError::InvalidValue {
            field: "improvements".to_string(),
            reason: "expected array".to_string(),
        })?;

    let mut improvements = Vec::new();
    for item in improvements_arr {
        let issue = item
            .get("issue")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let suggestion = item
            .get("suggestion")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let priority = item
            .get("priority")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Priority>().ok())
            .unwrap_or(Priority::Medium);

        if !issue.is_empty() && !suggestion.is_empty() {
            improvements.push(Improvement::new(issue, suggestion, priority));
        }
    }

    Ok(improvements)
}

// ============================================================================
// Evaluate Parsing
// ============================================================================

/// Parse session assessment from JSON.
pub fn parse_session_assessment(json: &serde_json::Value) -> Result<SessionAssessment, ModeError> {
    let assessment_json =
        json.get("session_assessment")
            .ok_or_else(|| ModeError::MissingField {
                field: "session_assessment".to_string(),
            })?;

    let overall_quality = assessment_json
        .get("overall_quality")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: "session_assessment.overall_quality".to_string(),
        })?;

    let coherence = assessment_json
        .get("coherence")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: "session_assessment.coherence".to_string(),
        })?;

    let completeness = assessment_json
        .get("completeness")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: "session_assessment.completeness".to_string(),
        })?;

    let depth = assessment_json
        .get("depth")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: "session_assessment.depth".to_string(),
        })?;

    Ok(SessionAssessment::new(
        overall_quality,
        coherence,
        completeness,
        depth,
    ))
}

// ============================================================================
// Utility Helpers
// ============================================================================

/// Parse an array of strings from JSON.
pub fn parse_string_array(json: &serde_json::Value, key: &str) -> Option<Vec<String>> {
    json.get(key).and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect()
        })
    })
}
