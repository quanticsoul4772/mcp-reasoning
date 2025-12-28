//! Decision analysis response types.
//!
//! This module contains all the structured response types for the decision mode:
//! - Weighted multi-criteria analysis types
//! - Pairwise comparison types
//! - TOPSIS (ideal solution) types
//! - Multi-stakeholder perspective types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Response Types - Weighted
// ============================================================================

/// A criterion with its weight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Criterion {
    /// Criterion name.
    pub name: String,
    /// Weight (0.0-1.0, all weights should sum to 1.0).
    pub weight: f64,
    /// Description of what this criterion measures.
    pub description: String,
}

/// A ranked option with its score.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RankedOption {
    /// Option name.
    pub option: String,
    /// Weighted score.
    pub score: f64,
    /// Rank (1 = best).
    pub rank: u32,
}

/// Response from weighted operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeightedResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Options being compared.
    pub options: Vec<String>,
    /// Evaluation criteria with weights.
    pub criteria: Vec<Criterion>,
    /// Scores per option per criterion.
    pub scores: HashMap<String, HashMap<String, f64>>,
    /// Weighted totals per option.
    pub weighted_totals: HashMap<String, f64>,
    /// Final ranking.
    pub ranking: Vec<RankedOption>,
    /// Notes on sensitivity to weight changes.
    pub sensitivity_notes: String,
}

impl WeightedResponse {
    /// Create a new weighted response.
    #[must_use]
    // Weighted decision requires all scoring components for valid analysis
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        options: Vec<String>,
        criteria: Vec<Criterion>,
        scores: HashMap<String, HashMap<String, f64>>,
        weighted_totals: HashMap<String, f64>,
        ranking: Vec<RankedOption>,
        sensitivity_notes: impl Into<String>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            options,
            criteria,
            scores,
            weighted_totals,
            ranking,
            sensitivity_notes: sensitivity_notes.into(),
        }
    }
}

// ============================================================================
// Response Types - Pairwise
// ============================================================================

/// Preference strength.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PreferenceStrength {
    /// Strong preference.
    Strong,
    /// Moderate preference.
    Moderate,
    /// Slight preference.
    Slight,
}

/// Result of preferring one option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceResult {
    /// Option A is preferred.
    OptionA,
    /// Option B is preferred.
    OptionB,
    /// Options are tied.
    Tie,
}

/// A pairwise comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairwiseComparison {
    /// First option.
    pub option_a: String,
    /// Second option.
    pub option_b: String,
    /// Which is preferred.
    pub preferred: PreferenceResult,
    /// How strong is the preference.
    pub strength: PreferenceStrength,
    /// Reasoning for the preference.
    pub reasoning: String,
}

/// An option with its win count.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairwiseRank {
    /// Option name.
    pub option: String,
    /// Number of pairwise wins.
    pub wins: u32,
    /// Rank (1 = best).
    pub rank: u32,
}

/// Response from pairwise operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairwiseResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Individual comparisons.
    pub comparisons: Vec<PairwiseComparison>,
    /// Win/loss matrix as string keys.
    pub pairwise_matrix: HashMap<String, i32>,
    /// Final ranking.
    pub ranking: Vec<PairwiseRank>,
    /// Check for preference transitivity.
    pub consistency_check: String,
}

impl PairwiseResponse {
    /// Create a new pairwise response.
    #[must_use]
    // Pairwise comparison requires all matrix components for consistency checking
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        comparisons: Vec<PairwiseComparison>,
        pairwise_matrix: HashMap<String, i32>,
        ranking: Vec<PairwiseRank>,
        consistency_check: impl Into<String>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            comparisons,
            pairwise_matrix,
            ranking,
            consistency_check: consistency_check.into(),
        }
    }
}

// ============================================================================
// Response Types - TOPSIS
// ============================================================================

/// Criterion type for TOPSIS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CriterionType {
    /// Higher is better.
    Benefit,
    /// Lower is better.
    Cost,
}

/// A criterion for TOPSIS analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopsisCreterion {
    /// Criterion name.
    pub name: String,
    /// Benefit or cost.
    #[serde(rename = "type")]
    pub criterion_type: CriterionType,
    /// Weight.
    pub weight: f64,
}

/// Distance to ideal/anti-ideal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopsisDistances {
    /// Distance to ideal solution.
    pub to_ideal: f64,
    /// Distance to anti-ideal solution.
    pub to_anti_ideal: f64,
}

/// An option ranked by closeness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopsisRank {
    /// Option name.
    pub option: String,
    /// Relative closeness to ideal (0-1).
    pub closeness: f64,
    /// Rank (1 = best).
    pub rank: u32,
}

/// Response from TOPSIS operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopsisResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Criteria with types and weights.
    pub criteria: Vec<TopsisCreterion>,
    /// Raw decision matrix (option → scores).
    pub decision_matrix: HashMap<String, Vec<f64>>,
    /// Ideal solution values.
    pub ideal_solution: Vec<f64>,
    /// Anti-ideal solution values.
    pub anti_ideal_solution: Vec<f64>,
    /// Distances per option.
    pub distances: HashMap<String, TopsisDistances>,
    /// Relative closeness per option.
    pub relative_closeness: HashMap<String, f64>,
    /// Final ranking.
    pub ranking: Vec<TopsisRank>,
}

impl TopsisResponse {
    /// Create a new TOPSIS response.
    #[must_use]
    // TOPSIS algorithm requires all distance components for valid ranking
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        criteria: Vec<TopsisCreterion>,
        decision_matrix: HashMap<String, Vec<f64>>,
        ideal_solution: Vec<f64>,
        anti_ideal_solution: Vec<f64>,
        distances: HashMap<String, TopsisDistances>,
        relative_closeness: HashMap<String, f64>,
        ranking: Vec<TopsisRank>,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            criteria,
            decision_matrix,
            ideal_solution,
            anti_ideal_solution,
            distances,
            relative_closeness,
            ranking,
        }
    }
}

// ============================================================================
// Response Types - Perspectives
// ============================================================================

/// Influence level of a stakeholder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InfluenceLevel {
    /// High influence.
    High,
    /// Medium influence.
    Medium,
    /// Low influence.
    Low,
}

/// Conflict severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConflictSeverity {
    /// High severity.
    High,
    /// Medium severity.
    Medium,
    /// Low severity.
    Low,
}

/// A stakeholder's perspective.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Stakeholder {
    /// Stakeholder name.
    pub name: String,
    /// Their interests.
    pub interests: Vec<String>,
    /// Their preferred option.
    pub preferred_option: String,
    /// Their concerns.
    pub concerns: Vec<String>,
    /// Their level of influence.
    pub influence_level: InfluenceLevel,
}

/// A conflict between stakeholders.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Conflict {
    /// Stakeholders in conflict.
    pub between: Vec<String>,
    /// What they disagree about.
    pub issue: String,
    /// How severe the conflict is.
    pub severity: ConflictSeverity,
}

/// An alignment between stakeholders.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Alignment {
    /// Stakeholders who agree.
    pub stakeholders: Vec<String>,
    /// What they agree on.
    pub common_ground: String,
}

/// Balanced recommendation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BalancedRecommendation {
    /// Recommended option.
    pub option: String,
    /// How this balances interests.
    pub rationale: String,
    /// How to address concerns.
    pub mitigation: String,
}

/// Response from perspectives operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PerspectivesResponse {
    /// Unique identifier for this thought.
    pub thought_id: String,
    /// Session this thought belongs to.
    pub session_id: String,
    /// Stakeholder perspectives.
    pub stakeholders: Vec<Stakeholder>,
    /// Conflicts identified.
    pub conflicts: Vec<Conflict>,
    /// Alignments identified.
    pub alignments: Vec<Alignment>,
    /// Balanced recommendation.
    pub balanced_recommendation: BalancedRecommendation,
}

impl PerspectivesResponse {
    /// Create a new perspectives response.
    #[must_use]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        stakeholders: Vec<Stakeholder>,
        conflicts: Vec<Conflict>,
        alignments: Vec<Alignment>,
        balanced_recommendation: BalancedRecommendation,
    ) -> Self {
        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            stakeholders,
            conflicts,
            alignments,
            balanced_recommendation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preference_strength_serialize() {
        assert_eq!(
            serde_json::to_string(&PreferenceStrength::Strong).unwrap(),
            "\"strong\""
        );
    }

    #[test]
    fn test_criterion_type_serialize() {
        assert_eq!(
            serde_json::to_string(&CriterionType::Benefit).unwrap(),
            "\"benefit\""
        );
    }

    #[test]
    fn test_influence_level_serialize() {
        assert_eq!(
            serde_json::to_string(&InfluenceLevel::High).unwrap(),
            "\"high\""
        );
    }
}
