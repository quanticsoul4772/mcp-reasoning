//! Reflection mode response types.
//!
//! This module contains all the structured response types for the reflection mode:
//! - Process operation types (ReasoningAnalysis, Improvement, ProcessResponse)
//! - Evaluate operation types (SessionAssessment, EvaluateResponse)

use serde::{Deserialize, Serialize};

// ============================================================================
// Response Types - Process
// ============================================================================

/// Priority level for an improvement suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// High priority - address immediately.
    High,
    /// Medium priority - address soon.
    #[default]
    Medium,
    /// Low priority - address when convenient.
    Low,
}

impl std::str::FromStr for Priority {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => Err(()),
        }
    }
}

/// Analysis of reasoning strengths and weaknesses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReasoningAnalysis {
    /// Strengths in the reasoning.
    pub strengths: Vec<String>,
    /// Weaknesses in the reasoning.
    pub weaknesses: Vec<String>,
    /// Gaps in the analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gaps: Option<Vec<String>>,
}

impl ReasoningAnalysis {
    /// Create a new reasoning analysis.
    #[must_use]
    pub fn new(strengths: Vec<String>, weaknesses: Vec<String>) -> Self {
        Self {
            strengths,
            weaknesses,
            gaps: None,
        }
    }

    /// Add gaps.
    #[must_use]
    pub fn with_gaps(mut self, gaps: Vec<String>) -> Self {
        self.gaps = Some(gaps);
        self
    }
}

/// A suggested improvement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Improvement {
    /// The specific issue.
    pub issue: String,
    /// How to address it.
    pub suggestion: String,
    /// Priority level.
    pub priority: Priority,
}

impl Improvement {
    /// Create a new improvement suggestion.
    #[must_use]
    pub fn new(
        issue: impl Into<String>,
        suggestion: impl Into<String>,
        priority: Priority,
    ) -> Self {
        Self {
            issue: issue.into(),
            suggestion: suggestion.into(),
            priority,
        }
    }
}

/// Response from the `process` operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessResponse {
    /// Unique thought identifier.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Analysis of the reasoning.
    pub analysis: ReasoningAnalysis,
    /// Suggested improvements.
    pub improvements: Vec<Improvement>,
    /// Refined version of the reasoning.
    pub refined_reasoning: String,
    /// Expected confidence improvement (0.0-1.0).
    pub confidence_improvement: f64,
}

impl ProcessResponse {
    /// Create a new process response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        analysis: ReasoningAnalysis,
        improvements: Vec<Improvement>,
        refined_reasoning: impl Into<String>,
        confidence_improvement: f64,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            analysis,
            improvements,
            refined_reasoning: refined_reasoning.into(),
            confidence_improvement,
        }
    }
}

// ============================================================================
// Response Types - Evaluate
// ============================================================================

/// Session quality assessment metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionAssessment {
    /// Overall quality (0.0-1.0).
    pub overall_quality: f64,
    /// Coherence of reasoning (0.0-1.0).
    pub coherence: f64,
    /// Completeness of analysis (0.0-1.0).
    pub completeness: f64,
    /// Depth of exploration (0.0-1.0).
    pub depth: f64,
}

impl SessionAssessment {
    /// Create a new session assessment.
    #[must_use]
    pub fn new(overall_quality: f64, coherence: f64, completeness: f64, depth: f64) -> Self {
        Self {
            overall_quality: overall_quality.clamp(0.0, 1.0),
            coherence: coherence.clamp(0.0, 1.0),
            completeness: completeness.clamp(0.0, 1.0),
            depth: depth.clamp(0.0, 1.0),
        }
    }

    /// Calculate average score.
    #[must_use]
    pub fn average(&self) -> f64 {
        (self.overall_quality + self.coherence + self.completeness + self.depth) / 4.0
    }
}

/// Response from the `evaluate` operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvaluateResponse {
    /// Unique thought identifier.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Session quality metrics.
    pub session_assessment: SessionAssessment,
    /// Strongest elements of the session.
    pub strongest_elements: Vec<String>,
    /// Areas for improvement.
    pub areas_for_improvement: Vec<String>,
    /// Key insights from the session.
    pub key_insights: Vec<String>,
    /// Recommendations for next steps.
    pub recommendations: Vec<String>,
    /// Higher-level observations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_observations: Option<String>,
}

impl EvaluateResponse {
    /// Create a new evaluate response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        session_assessment: SessionAssessment,
        strongest_elements: Vec<String>,
        areas_for_improvement: Vec<String>,
        key_insights: Vec<String>,
        recommendations: Vec<String>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            session_assessment,
            strongest_elements,
            areas_for_improvement,
            key_insights,
            recommendations,
            meta_observations: None,
        }
    }

    /// Add meta observations.
    #[must_use]
    pub fn with_meta_observations(mut self, observations: impl Into<String>) -> Self {
        self.meta_observations = Some(observations.into());
        self
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
    fn test_priority_from_str() {
        assert_eq!("high".parse::<Priority>(), Ok(Priority::High));
        assert_eq!("medium".parse::<Priority>(), Ok(Priority::Medium));
        assert_eq!("low".parse::<Priority>(), Ok(Priority::Low));
        assert_eq!("HIGH".parse::<Priority>(), Ok(Priority::High));
        assert!("invalid".parse::<Priority>().is_err());
    }

    #[test]
    fn test_priority_default() {
        assert_eq!(Priority::default(), Priority::Medium);
    }

    #[test]
    fn test_priority_serialize() {
        let json = serde_json::to_string(&Priority::High).unwrap();
        assert_eq!(json, "\"high\"");
    }

    #[test]
    fn test_reasoning_analysis_new() {
        let analysis = ReasoningAnalysis::new(
            vec!["Strength 1".to_string()],
            vec!["Weakness 1".to_string()],
        );
        assert_eq!(analysis.strengths.len(), 1);
        assert_eq!(analysis.weaknesses.len(), 1);
        assert!(analysis.gaps.is_none());
    }

    #[test]
    fn test_reasoning_analysis_with_gaps() {
        let analysis = ReasoningAnalysis::new(vec![], vec![]).with_gaps(vec!["Gap 1".to_string()]);
        assert!(analysis.gaps.is_some());
        assert_eq!(analysis.gaps.unwrap().len(), 1);
    }

    #[test]
    fn test_improvement_new() {
        let improvement = Improvement::new("Issue", "Suggestion", Priority::High);
        assert_eq!(improvement.issue, "Issue");
        assert_eq!(improvement.suggestion, "Suggestion");
        assert_eq!(improvement.priority, Priority::High);
    }

    #[test]
    fn test_session_assessment_new() {
        let assessment = SessionAssessment::new(0.85, 0.9, 0.75, 0.8);
        assert!((assessment.overall_quality - 0.85).abs() < f64::EPSILON);
        assert!((assessment.coherence - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_session_assessment_clamps_values() {
        let assessment = SessionAssessment::new(1.5, -0.1, 0.5, 0.5);
        assert!((assessment.overall_quality - 1.0).abs() < f64::EPSILON);
        assert!((assessment.coherence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_session_assessment_average() {
        let assessment = SessionAssessment::new(0.8, 0.8, 0.8, 0.8);
        assert!((assessment.average() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_process_response_new() {
        let analysis = ReasoningAnalysis::new(vec![], vec![]);
        let response = ProcessResponse::new("t-1", "s-1", analysis, vec![], "refined", 0.1);
        assert_eq!(response.thought_id, "t-1");
        assert_eq!(response.session_id, "s-1");
        assert_eq!(response.refined_reasoning, "refined");
    }

    #[test]
    fn test_evaluate_response_new() {
        let assessment = SessionAssessment::new(0.8, 0.8, 0.8, 0.8);
        let response = EvaluateResponse::new(
            "t-1",
            "s-1",
            assessment,
            vec!["Strong".to_string()],
            vec!["Weak".to_string()],
            vec!["Insight".to_string()],
            vec!["Recommendation".to_string()],
        );
        assert_eq!(response.strongest_elements.len(), 1);
        assert!(response.meta_observations.is_none());
    }

    #[test]
    fn test_evaluate_response_with_meta_observations() {
        let assessment = SessionAssessment::new(0.8, 0.8, 0.8, 0.8);
        let response =
            EvaluateResponse::new("t-1", "s-1", assessment, vec![], vec![], vec![], vec![])
                .with_meta_observations("Meta insight");
        assert_eq!(response.meta_observations, Some("Meta insight".to_string()));
    }
}
