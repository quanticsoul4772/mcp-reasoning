//! Bias and fallacy detection response types.
//!
//! This module contains all the structured response types for the detect mode:
//! - Bias detection types (DetectedBias, BiasSeverity, BiasAssessment, BiasesResponse)
//! - Fallacy detection types (DetectedFallacy, FallacyCategory, ArgumentStructure, etc.)

use serde::{Deserialize, Serialize};

// ============================================================================
// Response Types - Biases
// ============================================================================

/// A single detected cognitive bias.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedBias {
    /// Name of the bias (e.g., "Confirmation Bias").
    pub bias: String,
    /// Evidence showing this bias.
    pub evidence: String,
    /// Severity level.
    pub severity: BiasSeverity,
    /// Impact on reasoning.
    pub impact: String,
    /// Strategy to counteract.
    pub debiasing: String,
}

/// Severity level for a detected bias.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BiasSeverity {
    /// Low severity - minor impact on reasoning.
    Low,
    /// Medium severity - noticeable impact.
    Medium,
    /// High severity - significant impact on conclusions.
    High,
}

/// Overall assessment of biases in content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BiasAssessment {
    /// Number of biases detected.
    pub bias_count: u32,
    /// The most severe bias found.
    pub most_severe: String,
    /// Overall reasoning quality (0.0-1.0).
    pub reasoning_quality: f64,
}

/// Response from bias detection operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BiasesResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// List of detected biases.
    pub biases_detected: Vec<DetectedBias>,
    /// Overall assessment.
    pub overall_assessment: BiasAssessment,
    /// Debiased version of the argument.
    pub debiased_version: String,
}

impl BiasesResponse {
    /// Create a new biases response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        biases_detected: Vec<DetectedBias>,
        overall_assessment: BiasAssessment,
        debiased_version: impl Into<String>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            biases_detected,
            overall_assessment,
            debiased_version: debiased_version.into(),
        }
    }
}

// ============================================================================
// Response Types - Fallacies
// ============================================================================

/// A single detected logical fallacy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedFallacy {
    /// Name of the fallacy (e.g., "Ad Hominem").
    pub fallacy: String,
    /// Category of fallacy.
    pub category: FallacyCategory,
    /// The passage containing the fallacy.
    pub passage: String,
    /// Explanation of why it's a fallacy.
    pub explanation: String,
    /// How to fix the argument.
    pub correction: String,
}

/// Category of logical fallacy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FallacyCategory {
    /// Formal fallacy - error in logical form.
    Formal,
    /// Informal fallacy - error in reasoning content.
    Informal,
    /// Relevance fallacy - premises don't support conclusion.
    Relevance,
    /// Presumption fallacy - unwarranted assumption.
    Presumption,
}

/// Structure of the analyzed argument.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArgumentStructure {
    /// Identified premises.
    pub premises: Vec<String>,
    /// The main conclusion.
    pub conclusion: String,
    /// Validity of the argument.
    pub validity: ArgumentValidity,
}

/// Validity assessment of an argument.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArgumentValidity {
    /// Argument is logically valid.
    Valid,
    /// Argument is logically invalid.
    Invalid,
    /// Argument has some valid and some invalid parts.
    PartiallyValid,
}

/// Overall assessment of fallacies in content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FallacyAssessment {
    /// Number of fallacies detected.
    pub fallacy_count: u32,
    /// Overall argument strength (0.0-1.0).
    pub argument_strength: f64,
    /// The most critical fallacy.
    pub most_critical: String,
}

/// Response from fallacy detection operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FallaciesResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// List of detected fallacies.
    pub fallacies_detected: Vec<DetectedFallacy>,
    /// Structure of the analyzed argument.
    pub argument_structure: ArgumentStructure,
    /// Overall assessment.
    pub overall_assessment: FallacyAssessment,
}

impl FallaciesResponse {
    /// Create a new fallacies response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        fallacies_detected: Vec<DetectedFallacy>,
        argument_structure: ArgumentStructure,
        overall_assessment: FallacyAssessment,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            fallacies_detected,
            argument_structure,
            overall_assessment,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bias_severity_serialize() {
        assert_eq!(
            serde_json::to_string(&BiasSeverity::Low).unwrap(),
            "\"low\""
        );
        assert_eq!(
            serde_json::to_string(&BiasSeverity::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&BiasSeverity::High).unwrap(),
            "\"high\""
        );
    }

    #[test]
    fn test_fallacy_category_serialize() {
        assert_eq!(
            serde_json::to_string(&FallacyCategory::Formal).unwrap(),
            "\"formal\""
        );
        assert_eq!(
            serde_json::to_string(&FallacyCategory::Informal).unwrap(),
            "\"informal\""
        );
    }

    #[test]
    fn test_argument_validity_serialize() {
        assert_eq!(
            serde_json::to_string(&ArgumentValidity::Valid).unwrap(),
            "\"valid\""
        );
        assert_eq!(
            serde_json::to_string(&ArgumentValidity::PartiallyValid).unwrap(),
            "\"partially_valid\""
        );
    }

    #[test]
    fn test_detected_bias_clone() {
        let bias = DetectedBias {
            bias: "Test".to_string(),
            evidence: "E".to_string(),
            severity: BiasSeverity::High,
            impact: "I".to_string(),
            debiasing: "D".to_string(),
        };
        let cloned = bias.clone();
        assert_eq!(bias, cloned);
    }

    #[test]
    fn test_detected_fallacy_clone() {
        let fallacy = DetectedFallacy {
            fallacy: "Test".to_string(),
            category: FallacyCategory::Formal,
            passage: "P".to_string(),
            explanation: "E".to_string(),
            correction: "C".to_string(),
        };
        let cloned = fallacy.clone();
        assert_eq!(fallacy, cloned);
    }
}
