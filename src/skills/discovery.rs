//! Skill discovery from tool chain patterns.
//!
//! Analyzes metrics data to find recurring tool usage patterns
//! and materialize them as registrable skills.

use serde::{Deserialize, Serialize};

use super::types::{Skill, SkillCategory, SkillStep};

/// A discovered tool chain pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPattern {
    /// Tool chain (ordered list of modes).
    pub tool_chain: Vec<String>,
    /// How many times this pattern has been observed.
    pub occurrences: u64,
    /// Average success rate when this pattern was used.
    pub avg_success_rate: f64,
    /// Whether this has been materialized into a skill.
    pub materialized: bool,
    /// The skill ID if materialized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
}

impl DiscoveredPattern {
    /// Create a new discovered pattern.
    #[must_use]
    pub fn new(tool_chain: Vec<String>, occurrences: u64, avg_success_rate: f64) -> Self {
        Self {
            tool_chain,
            occurrences,
            avg_success_rate,
            materialized: false,
            skill_id: None,
        }
    }

    /// Check if this pattern is worth materializing.
    #[must_use]
    pub fn is_significant(&self, min_occurrences: u64, min_success_rate: f64) -> bool {
        self.occurrences >= min_occurrences && self.avg_success_rate >= min_success_rate
    }
}

/// Discovers skills from usage patterns.
#[derive(Debug, Default)]
pub struct SkillDiscovery {
    patterns: Vec<DiscoveredPattern>,
}

impl SkillDiscovery {
    /// Create a new skill discovery system.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a tool chain observation.
    pub fn record_chain(&mut self, chain: Vec<String>, success: bool) {
        if chain.len() < 2 {
            return; // Need at least 2 tools for a pattern
        }

        let existing = self.patterns.iter_mut().find(|p| p.tool_chain == chain);

        if let Some(pattern) = existing {
            let n = pattern.occurrences as f64;
            let success_val = if success { 1.0 } else { 0.0 };
            pattern.avg_success_rate =
                pattern.avg_success_rate * n / (n + 1.0) + success_val / (n + 1.0);
            pattern.occurrences += 1;
        } else {
            self.patterns.push(DiscoveredPattern::new(
                chain,
                1,
                if success { 1.0 } else { 0.0 },
            ));
        }
    }

    /// Get all significant patterns.
    #[must_use]
    pub fn significant_patterns(
        &self,
        min_occurrences: u64,
        min_success_rate: f64,
    ) -> Vec<&DiscoveredPattern> {
        self.patterns
            .iter()
            .filter(|p| p.is_significant(min_occurrences, min_success_rate))
            .collect()
    }

    /// Materialize a pattern into a skill.
    pub fn materialize(&mut self, pattern_index: usize) -> Option<Skill> {
        let pattern = self.patterns.get_mut(pattern_index)?;
        if pattern.materialized {
            return None;
        }

        let skill_id = format!(
            "discovered-{}",
            pattern
                .tool_chain
                .iter()
                .map(|s| &s[..s.len().min(4)])
                .collect::<Vec<&str>>()
                .join("-")
        );

        let steps = pattern
            .tool_chain
            .iter()
            .enumerate()
            .map(|(i, mode)| {
                SkillStep::new(mode)
                    .with_description(format!("Step {}: {mode}", i + 1))
                    .with_output_key(format!("step_{i}_result"))
            })
            .collect();

        let skill = Skill::new(
            &skill_id,
            format!("Discovered: {}", pattern.tool_chain.join(" -> ")),
            format!(
                "Automatically discovered pattern with {:.0}% success rate over {} uses",
                pattern.avg_success_rate * 100.0,
                pattern.occurrences
            ),
            SkillCategory::Discovered,
            steps,
        );

        pattern.materialized = true;
        pattern.skill_id = Some(skill_id);

        Some(skill)
    }

    /// Get all patterns.
    #[must_use]
    pub fn all_patterns(&self) -> &[DiscoveredPattern] {
        &self.patterns
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use super::*;

    #[test]
    fn test_discovered_pattern_new() {
        let pattern =
            DiscoveredPattern::new(vec!["linear".to_string(), "tree".to_string()], 5, 0.8);
        assert_eq!(pattern.occurrences, 5);
        assert!(!pattern.materialized);
    }

    #[test]
    fn test_discovered_pattern_is_significant() {
        let pattern =
            DiscoveredPattern::new(vec!["linear".to_string(), "tree".to_string()], 10, 0.9);
        assert!(pattern.is_significant(5, 0.7));
        assert!(!pattern.is_significant(20, 0.7));
        assert!(!pattern.is_significant(5, 0.95));
    }

    #[test]
    fn test_discovery_record_chain() {
        let mut discovery = SkillDiscovery::new();
        discovery.record_chain(vec!["linear".to_string(), "tree".to_string()], true);
        discovery.record_chain(vec!["linear".to_string(), "tree".to_string()], true);
        discovery.record_chain(vec!["linear".to_string(), "tree".to_string()], false);

        assert_eq!(discovery.all_patterns().len(), 1);
        assert_eq!(discovery.all_patterns()[0].occurrences, 3);
        assert!((discovery.all_patterns()[0].avg_success_rate - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_discovery_ignores_short_chains() {
        let mut discovery = SkillDiscovery::new();
        discovery.record_chain(vec!["linear".to_string()], true);
        assert!(discovery.all_patterns().is_empty());
    }

    #[test]
    fn test_discovery_multiple_patterns() {
        let mut discovery = SkillDiscovery::new();
        discovery.record_chain(vec!["linear".to_string(), "tree".to_string()], true);
        discovery.record_chain(
            vec!["divergent".to_string(), "reflection".to_string()],
            true,
        );

        assert_eq!(discovery.all_patterns().len(), 2);
    }

    #[test]
    fn test_discovery_significant_patterns() {
        let mut discovery = SkillDiscovery::new();
        for _ in 0..10 {
            discovery.record_chain(vec!["linear".to_string(), "tree".to_string()], true);
        }
        discovery.record_chain(
            vec!["divergent".to_string(), "reflection".to_string()],
            true,
        );

        let significant = discovery.significant_patterns(5, 0.7);
        assert_eq!(significant.len(), 1);
        assert_eq!(significant[0].tool_chain, vec!["linear", "tree"]);
    }

    #[test]
    fn test_discovery_materialize() {
        let mut discovery = SkillDiscovery::new();
        for _ in 0..10 {
            discovery.record_chain(vec!["linear".to_string(), "tree".to_string()], true);
        }

        let skill = discovery.materialize(0).unwrap();
        assert!(skill.id.starts_with("discovered-"));
        assert_eq!(skill.category, SkillCategory::Discovered);
        assert_eq!(skill.steps.len(), 2);

        // Should not materialize again
        assert!(discovery.materialize(0).is_none());

        // Original pattern should be marked materialized
        assert!(discovery.all_patterns()[0].materialized);
    }

    #[test]
    fn test_discovery_materialize_invalid_index() {
        let mut discovery = SkillDiscovery::new();
        assert!(discovery.materialize(0).is_none());
    }

    #[test]
    fn test_pattern_serialize() {
        let pattern =
            DiscoveredPattern::new(vec!["linear".to_string(), "tree".to_string()], 5, 0.9);
        let json = serde_json::to_string(&pattern).unwrap();
        assert!(json.contains("\"occurrences\":5"));
    }
}
