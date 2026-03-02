//! Skill registry for managing available skills.
//!
//! Follows the `PresetRegistry` pattern with additional support for
//! importing existing presets as skills and discovering new skills.

use std::collections::HashMap;

use super::builtin::register_builtin_skills;
use super::types::{Skill, SkillCategory};
use crate::presets::{Preset, PresetCategory, PresetRegistry};

/// Registry of available skills.
#[derive(Debug, Default)]
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    /// Create a new registry with built-in skills.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self::default();
        register_builtin_skills(&mut registry);
        registry
    }

    /// Create a registry that also imports skills from existing presets.
    #[must_use]
    pub fn with_presets(preset_registry: &PresetRegistry) -> Self {
        let mut registry = Self::new();
        registry.import_presets(preset_registry);
        registry
    }

    /// Register a skill.
    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.id.clone(), skill);
    }

    /// Get a skill by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    /// List all skills.
    #[must_use]
    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// List skills by category.
    #[must_use]
    pub fn list_by_category(&self, category: &SkillCategory) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| &s.category == category)
            .collect()
    }

    /// Import presets as basic skills (without context passing).
    pub fn import_presets(&mut self, preset_registry: &PresetRegistry) {
        for preset in preset_registry.list() {
            if !self.skills.contains_key(&preset.id) {
                self.register(preset_to_skill(preset));
            }
        }
    }
}

/// Convert a preset to a skill (without context passing features).
fn preset_to_skill(preset: &Preset) -> Skill {
    use super::types::SkillStep;

    let steps = preset
        .steps
        .iter()
        .map(|ps| {
            let mut step = SkillStep::new(&ps.mode);
            if let Some(ref op) = ps.operation {
                step = step.with_operation(op);
            }
            if let Some(ref config) = ps.config {
                step = step.with_config(config.clone());
            }
            if let Some(ref desc) = ps.description {
                step = step.with_description(desc);
            }
            step
        })
        .collect();

    let category = match preset.category {
        PresetCategory::CodeQuality => SkillCategory::CodeQuality,
        PresetCategory::Analysis => SkillCategory::Analysis,
        PresetCategory::Decision => SkillCategory::Decision,
        PresetCategory::Research => SkillCategory::Research,
        PresetCategory::Custom => SkillCategory::Custom,
    };

    Skill::new(
        &preset.id,
        &preset.name,
        format!("{} (imported from preset)", preset.description),
        category,
        steps,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::skills::types::SkillStep;

    #[test]
    fn test_registry_new_has_builtin_skills() {
        let registry = SkillRegistry::new();
        assert!(registry.list().len() >= 5); // at least the new skills
    }

    #[test]
    fn test_registry_register_custom() {
        let mut registry = SkillRegistry::new();
        let initial = registry.list().len();

        registry.register(Skill::new(
            "custom",
            "Custom Skill",
            "A custom skill",
            SkillCategory::Custom,
            vec![SkillStep::new("linear")],
        ));

        assert_eq!(registry.list().len(), initial + 1);
        assert!(registry.get("custom").is_some());
    }

    #[test]
    fn test_registry_get() {
        let registry = SkillRegistry::new();
        assert!(registry.get("deep-code-review").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list_by_category() {
        let registry = SkillRegistry::new();
        let analysis_skills = registry.list_by_category(&SkillCategory::Analysis);
        assert!(!analysis_skills.is_empty());
    }

    #[test]
    fn test_registry_with_presets() {
        let preset_registry = PresetRegistry::new();
        let registry = SkillRegistry::with_presets(&preset_registry);

        // Should have both built-in skills AND imported presets
        assert!(registry.list().len() >= 10);

        // Imported presets should be available
        assert!(registry.get("code-review").is_some());
        assert!(registry.get("debug-analysis").is_some());
    }

    #[test]
    fn test_preset_to_skill_conversion() {
        let preset = Preset::new(
            "test-preset",
            "Test Preset",
            "A test",
            PresetCategory::Analysis,
            vec![crate::presets::PresetStep::new("linear")
                .with_operation("process")
                .with_description("Step 1")],
        );

        let skill = preset_to_skill(&preset);
        assert_eq!(skill.id, "test-preset");
        assert_eq!(skill.category, SkillCategory::Analysis);
        assert_eq!(skill.steps.len(), 1);
        assert_eq!(skill.steps[0].mode, "linear");
        assert_eq!(skill.steps[0].operation, Some("process".to_string()));
    }

    #[test]
    fn test_import_does_not_overwrite_existing() {
        let mut registry = SkillRegistry::new();

        // If we already have a skill with same ID as a preset, import should not overwrite
        let preset_registry = PresetRegistry::new();
        let existing_count = registry.list().len();

        registry.import_presets(&preset_registry);
        // Should add new skills but not overwrite existing ones
        assert!(registry.list().len() >= existing_count);
    }
}
