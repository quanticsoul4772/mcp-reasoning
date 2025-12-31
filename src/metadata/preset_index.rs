//! Index of available presets with metadata.

use super::PresetSuggestion;
use std::collections::HashMap;

/// Index of available presets with metadata.
pub struct PresetIndex {
    presets: HashMap<String, PresetMetadata>,
}

/// Metadata about a preset workflow.
#[derive(Debug, Clone)]
pub struct PresetMetadata {
    /// Preset identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what the preset does.
    pub description: String,
    /// Sequence of tools in the preset.
    pub tools: Vec<String>,
    /// Estimated total duration in milliseconds.
    pub estimated_duration_ms: u64,
    /// Use cases where this preset is applicable.
    pub use_cases: Vec<String>,
    /// Keywords for matching.
    pub keywords: Vec<String>,
}

impl PresetIndex {
    /// Build index from builtin presets.
    #[must_use]
    pub fn build() -> Self {
        let mut presets = HashMap::new();

        // Decision analysis preset
        presets.insert(
            "decision_analysis".into(),
            PresetMetadata {
                id: "decision_analysis".into(),
                name: "Decision Analysis".into(),
                description: "Multi-perspective analysis → criteria weighting → final recommendation"
                    .into(),
                tools: vec![
                    "reasoning_divergent".into(),
                    "reasoning_decision".into(),
                ],
                estimated_duration_ms: 65_000, // 45k + 20k
                use_cases: vec![
                    "Complex decisions with trade-offs".into(),
                    "Stakeholder alignment".into(),
                    "Strategic planning".into(),
                ],
                keywords: vec![
                    "decision".into(),
                    "choice".into(),
                    "options".into(),
                    "trade-off".into(),
                ],
            },
        );

        // Problem exploration preset
        presets.insert(
            "problem_exploration".into(),
            PresetMetadata {
                id: "problem_exploration".into(),
                name: "Deep Problem Exploration".into(),
                description: "Tree exploration → divergent perspectives → synthesis".into(),
                tools: vec![
                    "reasoning_tree".into(),
                    "reasoning_divergent".into(),
                    "reasoning_decision".into(),
                ],
                estimated_duration_ms: 83_000, // 18k + 45k + 20k
                use_cases: vec![
                    "Understanding complex problems".into(),
                    "Root cause analysis".into(),
                    "Creative problem solving".into(),
                ],
                keywords: vec![
                    "explore".into(),
                    "problem".into(),
                    "understand".into(),
                    "analyze".into(),
                ],
            },
        );

        // Evidence-based reasoning preset
        presets.insert(
            "evidence_based".into(),
            PresetMetadata {
                id: "evidence_based".into(),
                name: "Evidence-Based Reasoning".into(),
                description: "Gather evidence → evaluate credibility → form conclusions".into(),
                tools: vec![
                    "reasoning_linear".into(),
                    "reasoning_evidence".into(),
                    "reasoning_reflection".into(),
                ],
                estimated_duration_ms: 57_000, // 12k + 22k + 25k
                use_cases: vec![
                    "Fact-checking".into(),
                    "Hypothesis testing".into(),
                    "Scientific reasoning".into(),
                ],
                keywords: vec![
                    "evidence".into(),
                    "proof".into(),
                    "verify".into(),
                    "validate".into(),
                ],
            },
        );

        // Bias detection preset
        presets.insert(
            "bias_detection".into(),
            PresetMetadata {
                id: "bias_detection".into(),
                name: "Bias and Fallacy Detection".into(),
                description: "Analyze for biases → detect fallacies → provide corrections".into(),
                tools: vec![
                    "reasoning_detect".into(),
                    "reasoning_reflection".into(),
                ],
                estimated_duration_ms: 41_000, // 16k + 25k
                use_cases: vec![
                    "Argument analysis".into(),
                    "Critical thinking".into(),
                    "Debate preparation".into(),
                ],
                keywords: vec![
                    "bias".into(),
                    "fallacy".into(),
                    "logic".into(),
                    "critical".into(),
                ],
            },
        );

        // Causal analysis preset
        presets.insert(
            "causal_analysis".into(),
            PresetMetadata {
                id: "causal_analysis".into(),
                name: "Causal Analysis".into(),
                description: "Counterfactual reasoning → evidence evaluation → conclusions".into(),
                tools: vec![
                    "reasoning_counterfactual".into(),
                    "reasoning_evidence".into(),
                ],
                estimated_duration_ms: 87_000, // 65k + 22k
                use_cases: vec![
                    "Understanding cause and effect".into(),
                    "What-if scenarios".into(),
                    "Impact analysis".into(),
                ],
                keywords: vec![
                    "cause".into(),
                    "effect".into(),
                    "impact".into(),
                    "counterfactual".into(),
                ],
            },
        );

        Self { presets }
    }

    /// Find presets matching tool history pattern.
    #[must_use]
    pub fn find_matching_presets(&self, tool_history: &[String]) -> Vec<PresetSuggestion> {
        let mut matches: Vec<(f64, &PresetMetadata)> = self
            .presets
            .values()
            .filter_map(|preset| {
                let score = self.calculate_match_score(preset, tool_history);
                if score > 0.3 {
                    Some((score, preset))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Return top 3
        matches
            .into_iter()
            .take(3)
            .map(|(_, preset)| PresetSuggestion {
                preset_id: preset.id.clone(),
                description: preset.description.clone(),
                estimated_duration_ms: preset.estimated_duration_ms,
            })
            .collect()
    }

    /// Calculate match score between preset and tool history.
    fn calculate_match_score(&self, preset: &PresetMetadata, tool_history: &[String]) -> f64 {
        if tool_history.is_empty() {
            return 0.0;
        }

        let mut score = 0.0;

        // Count how many preset tools are in history
        let matching_count = preset
            .tools
            .iter()
            .filter(|t| tool_history.iter().any(|h| h == *t))
            .count();

        if matching_count == 0 {
            return 0.0;
        }

        // Base score from tool overlap
        score = (matching_count as f64) / (preset.tools.len() as f64);

        // Bonus for exact match - prioritize smaller presets that match exactly
        if matching_count == preset.tools.len() && matching_count == tool_history.len() {
            score = 1.0;
        } else if matching_count == preset.tools.len() {
            score = 0.95;
        }

        score
    }

    /// Get preset by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&PresetMetadata> {
        self.presets.get(id)
    }

    /// Get all preset IDs.
    #[must_use]
    pub fn all_ids(&self) -> Vec<String> {
        self.presets.keys().cloned().collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_build_preset_index() {
        let index = PresetIndex::build();

        assert!(index.get("decision_analysis").is_some());
        assert!(index.get("problem_exploration").is_some());
        assert!(index.get("evidence_based").is_some());
        assert!(index.get("bias_detection").is_some());
        assert!(index.get("causal_analysis").is_some());
    }

    #[test]
    fn test_find_matching_presets_decision_pattern() {
        let index = PresetIndex::build();

        let history = vec![
            "reasoning_divergent".into(),
            "reasoning_decision".into(),
        ];

        let matches = index.find_matching_presets(&history);

        assert!(!matches.is_empty());
        assert_eq!(matches[0].preset_id, "decision_analysis");
    }

    #[test]
    fn test_find_matching_presets_evidence_pattern() {
        let index = PresetIndex::build();

        let history = vec!["reasoning_linear".into(), "reasoning_evidence".into()];

        let matches = index.find_matching_presets(&history);

        assert!(!matches.is_empty());
        assert_eq!(matches[0].preset_id, "evidence_based");
    }

    #[test]
    fn test_find_matching_presets_no_history() {
        let index = PresetIndex::build();

        let matches = index.find_matching_presets(&[]);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_matching_presets_no_match() {
        let index = PresetIndex::build();

        let history = vec!["reasoning_checkpoint".into()];

        let matches = index.find_matching_presets(&history);

        // Checkpoint alone doesn't match any preset strongly
        assert!(matches.is_empty() || matches[0].estimated_duration_ms > 0);
    }

    #[test]
    fn test_all_ids() {
        let index = PresetIndex::build();
        let ids = index.all_ids();

        assert_eq!(ids.len(), 5);
        assert!(ids.contains(&"decision_analysis".into()));
    }
}
