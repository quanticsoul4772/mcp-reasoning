//! Evidence evaluation response types.
//!
//! This module contains all the structured response types for the evidence mode:
//! - Assess operation types (Credibility, EvidenceQuality, EvidencePiece, etc.)
//! - Probabilistic operation types (Prior, Posterior, BeliefUpdate, etc.)

use serde::{Deserialize, Serialize};

// ============================================================================
// Response Types - Assess
// ============================================================================

/// Credibility scores for a piece of evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Credibility {
    /// Expertise of the source (0.0-1.0).
    pub expertise: f64,
    /// Objectivity of the source (0.0-1.0).
    pub objectivity: f64,
    /// Level of corroboration (0.0-1.0).
    pub corroboration: f64,
    /// Recency of the evidence (0.0-1.0).
    pub recency: f64,
    /// Overall credibility score (0.0-1.0).
    pub overall: f64,
}

/// Quality scores for a piece of evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceQuality {
    /// Relevance to the claim (0.0-1.0).
    pub relevance: f64,
    /// Strength of support (0.0-1.0).
    pub strength: f64,
    /// Representativeness of the evidence (0.0-1.0).
    pub representativeness: f64,
    /// Overall quality score (0.0-1.0).
    pub overall: f64,
}

/// Type of evidence source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    /// Direct, first-hand evidence.
    Primary,
    /// Second-hand interpretation.
    Secondary,
    /// Third-hand compilation.
    Tertiary,
    /// Personal account or story.
    Anecdotal,
}

/// A piece of evidence with credibility and quality assessment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidencePiece {
    /// Brief description of the evidence.
    pub summary: String,
    /// Type of source.
    pub source_type: SourceType,
    /// Credibility assessment.
    pub credibility: Credibility,
    /// Quality assessment.
    pub quality: EvidenceQuality,
}

/// Overall assessment of evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverallEvidenceAssessment {
    /// Overall evidential support (0.0-1.0).
    pub evidential_support: f64,
    /// Key strengths of the evidence.
    pub key_strengths: Vec<String>,
    /// Key weaknesses.
    pub key_weaknesses: Vec<String>,
    /// What evidence is missing.
    pub gaps: Vec<String>,
}

/// Response from assess operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssessResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// List of evidence pieces with assessments.
    pub evidence_pieces: Vec<EvidencePiece>,
    /// Overall evidence assessment.
    pub overall_assessment: OverallEvidenceAssessment,
    /// Confidence in the conclusion.
    pub confidence_in_conclusion: f64,
}

impl AssessResponse {
    /// Create a new assess response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        evidence_pieces: Vec<EvidencePiece>,
        overall_assessment: OverallEvidenceAssessment,
        confidence_in_conclusion: f64,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            evidence_pieces,
            overall_assessment,
            confidence_in_conclusion,
        }
    }
}

// ============================================================================
// Response Types - Probabilistic
// ============================================================================

/// Prior probability with basis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Prior {
    /// Prior probability (0.0-1.0).
    pub probability: f64,
    /// Why this prior was chosen.
    pub basis: String,
}

/// Analysis of a single piece of evidence for Bayesian updating.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceAnalysis {
    /// Description of the evidence.
    pub evidence: String,
    /// P(E|H) - likelihood of evidence if hypothesis is true.
    pub likelihood_if_true: f64,
    /// P(E|¬H) - likelihood of evidence if hypothesis is false.
    pub likelihood_if_false: f64,
    /// Bayes factor = P(E|H) / P(E|¬H).
    pub bayes_factor: f64,
}

/// Posterior probability with calculation explanation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Posterior {
    /// Posterior probability (0.0-1.0).
    pub probability: f64,
    /// How the posterior was derived.
    pub calculation: String,
}

/// Direction of belief change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BeliefDirection {
    /// Belief increased.
    Increase,
    /// Belief decreased.
    Decrease,
    /// Belief unchanged.
    Unchanged,
}

/// Magnitude of belief change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BeliefMagnitude {
    /// Strong change.
    Strong,
    /// Moderate change.
    Moderate,
    /// Slight change.
    Slight,
}

/// Summary of belief update.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BeliefUpdate {
    /// Direction of change.
    pub direction: BeliefDirection,
    /// Magnitude of change.
    pub magnitude: BeliefMagnitude,
    /// Plain language interpretation.
    pub interpretation: String,
}

/// Response from probabilistic operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProbabilisticResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// The hypothesis being evaluated.
    pub hypothesis: String,
    /// Prior probability.
    pub prior: Prior,
    /// Evidence analysis with Bayes factors.
    pub evidence_analysis: Vec<EvidenceAnalysis>,
    /// Posterior probability.
    pub posterior: Posterior,
    /// Summary of belief update.
    pub belief_update: BeliefUpdate,
    /// Sensitivity to prior assumptions.
    pub sensitivity: String,
}

impl ProbabilisticResponse {
    /// Create a new probabilistic response.
    #[must_use]
    // Bayesian analysis requires all probability components for valid inference
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        hypothesis: impl Into<String>,
        prior: Prior,
        evidence_analysis: Vec<EvidenceAnalysis>,
        posterior: Posterior,
        belief_update: BeliefUpdate,
        sensitivity: impl Into<String>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            hypothesis: hypothesis.into(),
            prior,
            evidence_analysis,
            posterior,
            belief_update,
            sensitivity: sensitivity.into(),
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
    fn test_source_type_serialize() {
        assert_eq!(
            serde_json::to_string(&SourceType::Primary).unwrap(),
            "\"primary\""
        );
        assert_eq!(
            serde_json::to_string(&SourceType::Anecdotal).unwrap(),
            "\"anecdotal\""
        );
    }

    #[test]
    fn test_belief_direction_serialize() {
        assert_eq!(
            serde_json::to_string(&BeliefDirection::Increase).unwrap(),
            "\"increase\""
        );
    }

    #[test]
    fn test_belief_magnitude_serialize() {
        assert_eq!(
            serde_json::to_string(&BeliefMagnitude::Strong).unwrap(),
            "\"strong\""
        );
    }
}
