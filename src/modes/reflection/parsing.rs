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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // parse_analysis tests
    #[test]
    fn test_parse_analysis_success() {
        let json = json!({
            "analysis": {
                "strengths": ["clear logic", "good structure"],
                "weaknesses": ["missing examples"],
                "gaps": ["needs more detail"]
            }
        });

        let result = parse_analysis(&json);
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.strengths.len(), 2);
        assert_eq!(analysis.weaknesses.len(), 1);
        assert!(analysis.gaps.is_some());
        assert_eq!(analysis.gaps.unwrap().len(), 1);
    }

    #[test]
    fn test_parse_analysis_without_gaps() {
        let json = json!({
            "analysis": {
                "strengths": ["strong argument"],
                "weaknesses": ["weak conclusion"]
            }
        });

        let result = parse_analysis(&json);
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.gaps.is_none());
    }

    #[test]
    fn test_parse_analysis_missing_analysis() {
        let json = json!({});

        let result = parse_analysis(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::MissingField { field }) => {
                assert_eq!(field, "analysis");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_analysis_missing_strengths() {
        let json = json!({
            "analysis": {
                "weaknesses": ["missing examples"]
            }
        });

        let result = parse_analysis(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::MissingField { field }) => {
                assert_eq!(field, "analysis.strengths");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_analysis_missing_weaknesses() {
        let json = json!({
            "analysis": {
                "strengths": ["good point"]
            }
        });

        let result = parse_analysis(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::MissingField { field }) => {
                assert_eq!(field, "analysis.weaknesses");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    // parse_improvements tests
    #[test]
    fn test_parse_improvements_success() {
        let json = json!({
            "improvements": [
                {"issue": "lack of detail", "suggestion": "add examples", "priority": "high"},
                {"issue": "unclear", "suggestion": "rephrase", "priority": "low"}
            ]
        });

        let result = parse_improvements(&json);
        assert!(result.is_ok());
        let improvements = result.unwrap();
        assert_eq!(improvements.len(), 2);
        assert_eq!(improvements[0].issue, "lack of detail");
        assert_eq!(improvements[0].priority, Priority::High);
        assert_eq!(improvements[1].priority, Priority::Low);
    }

    #[test]
    fn test_parse_improvements_default_priority() {
        let json = json!({
            "improvements": [
                {"issue": "unclear", "suggestion": "fix it"}
            ]
        });

        let result = parse_improvements(&json);
        assert!(result.is_ok());
        let improvements = result.unwrap();
        assert_eq!(improvements.len(), 1);
        assert_eq!(improvements[0].priority, Priority::Medium);
    }

    #[test]
    fn test_parse_improvements_skip_empty() {
        let json = json!({
            "improvements": [
                {"issue": "", "suggestion": "fix it"},
                {"issue": "real issue", "suggestion": ""}
            ]
        });

        let result = parse_improvements(&json);
        assert!(result.is_ok());
        let improvements = result.unwrap();
        assert_eq!(improvements.len(), 0);
    }

    #[test]
    fn test_parse_improvements_missing() {
        let json = json!({});

        let result = parse_improvements(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::MissingField { field }) => {
                assert_eq!(field, "improvements");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_improvements_not_array() {
        let json = json!({
            "improvements": "not an array"
        });

        let result = parse_improvements(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::InvalidValue { field, reason }) => {
                assert_eq!(field, "improvements");
                assert_eq!(reason, "expected array");
            }
            _ => panic!("Expected InvalidValue error"),
        }
    }

    // parse_session_assessment tests
    #[test]
    fn test_parse_session_assessment_success() {
        let json = json!({
            "session_assessment": {
                "overall_quality": 0.85,
                "coherence": 0.9,
                "completeness": 0.75,
                "depth": 0.8
            }
        });

        let result = parse_session_assessment(&json);
        assert!(result.is_ok());
        let assessment = result.unwrap();
        assert!((assessment.overall_quality - 0.85).abs() < 0.01);
        assert!((assessment.coherence - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_parse_session_assessment_missing() {
        let json = json!({});

        let result = parse_session_assessment(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::MissingField { field }) => {
                assert_eq!(field, "session_assessment");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_session_assessment_missing_quality() {
        let json = json!({
            "session_assessment": {
                "coherence": 0.9,
                "completeness": 0.75,
                "depth": 0.8
            }
        });

        let result = parse_session_assessment(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::MissingField { field }) => {
                assert_eq!(field, "session_assessment.overall_quality");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_session_assessment_missing_coherence() {
        let json = json!({
            "session_assessment": {
                "overall_quality": 0.85,
                "completeness": 0.75,
                "depth": 0.8
            }
        });

        let result = parse_session_assessment(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::MissingField { field }) => {
                assert_eq!(field, "session_assessment.coherence");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_session_assessment_missing_completeness() {
        let json = json!({
            "session_assessment": {
                "overall_quality": 0.85,
                "coherence": 0.9,
                "depth": 0.8
            }
        });

        let result = parse_session_assessment(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::MissingField { field }) => {
                assert_eq!(field, "session_assessment.completeness");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_session_assessment_missing_depth() {
        let json = json!({
            "session_assessment": {
                "overall_quality": 0.85,
                "coherence": 0.9,
                "completeness": 0.75
            }
        });

        let result = parse_session_assessment(&json);
        assert!(result.is_err());
        match result {
            Err(ModeError::MissingField { field }) => {
                assert_eq!(field, "session_assessment.depth");
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    // parse_string_array tests
    #[test]
    fn test_parse_string_array_success() {
        let json = json!({
            "items": ["one", "two", "three"]
        });

        let result = parse_string_array(&json, "items");
        assert!(result.is_some());
        let items = result.unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_string_array_missing() {
        let json = json!({});

        let result = parse_string_array(&json, "items");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_string_array_not_array() {
        let json = json!({
            "items": "not an array"
        });

        let result = parse_string_array(&json, "items");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_string_array_filters_non_strings() {
        let json = json!({
            "items": ["one", 2, "three", null]
        });

        let result = parse_string_array(&json, "items");
        assert!(result.is_some());
        let items = result.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "one");
        assert_eq!(items[1], "three");
    }
}
