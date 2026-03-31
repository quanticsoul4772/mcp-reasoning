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

impl BiasSeverity {
    /// Returns the lowercase string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
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

impl FallacyCategory {
    /// Returns the lowercase string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Formal => "formal",
            Self::Informal => "informal",
            Self::Relevance => "relevance",
            Self::Presumption => "presumption",
        }
    }
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

// ============================================================================
// Response Types - Knowledge Gaps
// ============================================================================

/// Category of knowledge gap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GapCategory {
    /// Required facts, measurements, or evidence not present.
    MissingData,
    /// Premises accepted without verification.
    UncheckedAssumption,
    /// Entire perspective or field not considered.
    UnexploredDomain,
    /// Important question the reasoning never poses.
    UnaskedQuestion,
}

impl GapCategory {
    /// Returns the snake_case string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MissingData => "missing_data",
            Self::UncheckedAssumption => "unchecked_assumption",
            Self::UnexploredDomain => "unexplored_domain",
            Self::UnaskedQuestion => "unasked_question",
        }
    }
}

/// A single knowledge gap — absent information that could change the conclusion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeGap {
    /// Name of the gap (e.g., "Competitor response to pricing change").
    pub gap: String,
    /// Category of gap.
    pub category: GapCategory,
    /// How discovering this would affect the conclusion.
    pub impact: String,
    /// Whether closing this gap would change the conclusion.
    pub would_change_conclusion: String,
    /// Specific step to close this gap.
    pub investigation: String,
}

/// Overall assessment of knowledge gaps in content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeGapAssessment {
    /// Number of gaps detected.
    pub gap_count: u32,
    /// The gap most likely to change the conclusion.
    pub most_critical: String,
    /// Completeness score (0.0 = critically incomplete, 1.0 = comprehensive).
    pub completeness_score: f64,
}

/// Response from knowledge gap detection operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeGapsResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// List of detected knowledge gaps.
    pub gaps: Vec<KnowledgeGap>,
    /// Assumptions the reasoning takes as given without verification.
    pub unchallenged_assumptions: Vec<String>,
    /// Overall assessment.
    pub overall_assessment: KnowledgeGapAssessment,
}

impl KnowledgeGapsResponse {
    /// Create a new knowledge gaps response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        gaps: Vec<KnowledgeGap>,
        unchallenged_assumptions: Vec<String>,
        overall_assessment: KnowledgeGapAssessment,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            gaps,
            unchallenged_assumptions,
            overall_assessment,
        }
    }
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

    #[test]
    fn test_gap_category_serialize() {
        assert_eq!(
            serde_json::to_string(&GapCategory::MissingData).unwrap(),
            "\"missing_data\""
        );
        assert_eq!(
            serde_json::to_string(&GapCategory::UncheckedAssumption).unwrap(),
            "\"unchecked_assumption\""
        );
        assert_eq!(
            serde_json::to_string(&GapCategory::UnexploredDomain).unwrap(),
            "\"unexplored_domain\""
        );
        assert_eq!(
            serde_json::to_string(&GapCategory::UnaskedQuestion).unwrap(),
            "\"unasked_question\""
        );
    }

    #[test]
    fn test_gap_category_as_str() {
        assert_eq!(GapCategory::MissingData.as_str(), "missing_data");
        assert_eq!(
            GapCategory::UncheckedAssumption.as_str(),
            "unchecked_assumption"
        );
        assert_eq!(GapCategory::UnexploredDomain.as_str(), "unexplored_domain");
        assert_eq!(GapCategory::UnaskedQuestion.as_str(), "unasked_question");
    }

    #[test]
    fn test_knowledge_gap_clone() {
        let gap = KnowledgeGap {
            gap: "Missing data".to_string(),
            category: GapCategory::MissingData,
            impact: "Changes conclusion".to_string(),
            would_change_conclusion: "yes".to_string(),
            investigation: "Research it".to_string(),
        };
        let cloned = gap.clone();
        assert_eq!(gap, cloned);
    }

    #[test]
    fn test_knowledge_gaps_response_new() {
        let gaps = vec![KnowledgeGap {
            gap: "Test gap".to_string(),
            category: GapCategory::UnaskedQuestion,
            impact: "Could flip decision".to_string(),
            would_change_conclusion: "yes".to_string(),
            investigation: "Ask stakeholders".to_string(),
        }];
        let assessment = KnowledgeGapAssessment {
            gap_count: 1,
            most_critical: "Test gap".to_string(),
            completeness_score: 0.5,
        };
        let resp = KnowledgeGapsResponse::new(
            "t1",
            "s1",
            gaps,
            vec!["Assumption A".to_string()],
            assessment,
        );
        assert_eq!(resp.thought_id, "t1");
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.gaps.len(), 1);
        assert_eq!(resp.unchallenged_assumptions.len(), 1);
    }
}
