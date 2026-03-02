//! Skill system integration tests.
//!
//! Tests the skill layer end-to-end:
//! 1. Skill registry with built-in skills
//! 2. Skill step definitions and conditions
//! 3. Skill context passing
//! 4. Skill discovery from patterns
//! 5. Preset-to-skill import
//! 6. Serialization round-trips
//!
//! Note: Tests requiring LLM mocks (SkillExecutor) are in the unit tests
//! within src/skills/executor.rs since mockall types are only available
//! inside the crate.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect,
    clippy::float_cmp
)]

use mcp_reasoning::presets::PresetRegistry;
use mcp_reasoning::skills::discovery::{DiscoveredPattern, SkillDiscovery};
use mcp_reasoning::skills::registry::SkillRegistry;
use mcp_reasoning::skills::types::{
    ErrorStrategy, Skill, SkillCategory, SkillContext, SkillInfo, SkillStep, StepCondition,
};

// ============================================================================
// Skill Registry Integration Tests
// ============================================================================

#[test]
fn test_registry_has_five_builtin_skills() {
    let registry = SkillRegistry::new();
    let skills = registry.list();
    assert_eq!(skills.len(), 5, "Should have 5 built-in skills");

    let skill_ids: Vec<&str> = skills.iter().map(|s| s.id.as_str()).collect();
    assert!(skill_ids.contains(&"deep-code-review"));
    assert!(skill_ids.contains(&"hypothesis-testing"));
    assert!(skill_ids.contains(&"systems-analysis"));
    assert!(skill_ids.contains(&"risk-assessment"));
    assert!(skill_ids.contains(&"creative-solution"));
}

#[test]
fn test_registry_with_presets_imports_all() {
    let preset_registry = PresetRegistry::new();
    let registry = SkillRegistry::with_presets(&preset_registry);

    // Should have built-in skills (5) + imported presets (5)
    assert!(
        registry.list().len() >= 10,
        "Should have at least 10 skills (5 builtin + 5 from presets)"
    );

    // Imported presets should be accessible
    assert!(registry.get("code-review").is_some());
    assert!(registry.get("debug-analysis").is_some());
    assert!(registry.get("architecture-decision").is_some());
    assert!(registry.get("strategic-decision").is_some());
    assert!(registry.get("evidence-conclusion").is_some());
}

#[test]
fn test_skill_category_filtering() {
    let registry = SkillRegistry::new();

    let code_skills = registry.list_by_category(&SkillCategory::CodeQuality);
    assert!(!code_skills.is_empty());
    assert!(code_skills
        .iter()
        .all(|s| s.category == SkillCategory::CodeQuality));

    let analysis_skills = registry.list_by_category(&SkillCategory::Analysis);
    assert!(!analysis_skills.is_empty());

    let decision_skills = registry.list_by_category(&SkillCategory::Decision);
    assert!(!decision_skills.is_empty());

    let research_skills = registry.list_by_category(&SkillCategory::Research);
    assert!(!research_skills.is_empty());
}

// ============================================================================
// Skill Step Definitions Integration Tests
// ============================================================================

#[test]
fn test_deep_code_review_workflow() {
    let registry = SkillRegistry::new();
    let skill = registry.get("deep-code-review").unwrap();

    assert_eq!(skill.steps.len(), 5);
    assert_eq!(skill.category, SkillCategory::CodeQuality);

    // Verify step progression
    assert_eq!(skill.steps[0].mode, "linear");
    assert_eq!(skill.steps[1].mode, "detect");
    assert_eq!(skill.steps[1].operation, Some("biases".to_string()));
    assert_eq!(skill.steps[2].mode, "detect");
    assert_eq!(skill.steps[2].operation, Some("fallacies".to_string()));
    assert_eq!(skill.steps[3].mode, "divergent");
    assert_eq!(skill.steps[4].mode, "reflection");

    // Detect steps should have Skip error strategy for resilience
    assert_eq!(skill.steps[1].on_error, ErrorStrategy::Skip);
    assert_eq!(skill.steps[2].on_error, ErrorStrategy::Skip);

    // Steps should produce output
    assert!(skill.steps[0].output_key.is_some());
    assert!(skill.steps[3].output_key.is_some());
}

#[test]
fn test_hypothesis_testing_has_conditional_steps() {
    let registry = SkillRegistry::new();
    let skill = registry.get("hypothesis-testing").unwrap();

    assert_eq!(skill.steps.len(), 5);

    // Step 4 (probabilistic) should be conditional on evidence existing
    let probabilistic_step = &skill.steps[3];
    assert_eq!(probabilistic_step.mode, "evidence");
    assert_eq!(
        probabilistic_step.operation,
        Some("probabilistic".to_string())
    );
    assert!(matches!(
        probabilistic_step.condition,
        StepCondition::IfKeyExists(ref key) if key == "evidence"
    ));
}

#[test]
fn test_systems_analysis_uses_graph_pipeline() {
    let registry = SkillRegistry::new();
    let skill = registry.get("systems-analysis").unwrap();

    let graph_steps: Vec<&SkillStep> = skill.steps.iter().filter(|s| s.mode == "graph").collect();
    assert_eq!(
        graph_steps.len(),
        4,
        "Systems analysis should use 4 graph steps"
    );

    // Graph operations should be: init -> generate -> score -> finalize
    assert_eq!(graph_steps[0].operation, Some("init".to_string()));
    assert_eq!(graph_steps[1].operation, Some("generate".to_string()));
    assert_eq!(graph_steps[2].operation, Some("score".to_string()));
    assert_eq!(graph_steps[3].operation, Some("finalize".to_string()));
}

#[test]
fn test_risk_assessment_has_skip_strategy() {
    let registry = SkillRegistry::new();
    let skill = registry.get("risk-assessment").unwrap();

    // Timeline step (last) should have Skip error strategy
    let last_step = skill.steps.last().unwrap();
    assert_eq!(last_step.mode, "timeline");
    assert_eq!(last_step.on_error, ErrorStrategy::Skip);
}

#[test]
fn test_creative_solution_uses_force_rebellion() {
    let registry = SkillRegistry::new();
    let skill = registry.get("creative-solution").unwrap();

    let divergent_step = &skill.steps[0];
    assert_eq!(divergent_step.mode, "divergent");

    let config = divergent_step.config.as_ref().unwrap();
    assert_eq!(config["force_rebellion"], true);
    assert_eq!(config["num_perspectives"], 5);
}

#[test]
fn test_all_builtin_skills_have_descriptions_and_output_keys() {
    let registry = SkillRegistry::new();

    for skill in registry.list() {
        assert!(
            !skill.description.is_empty(),
            "Skill '{}' should have description",
            skill.id
        );

        for (i, step) in skill.steps.iter().enumerate() {
            assert!(
                step.description.is_some(),
                "Skill '{}' step {} should have description",
                skill.id,
                i
            );
        }

        let has_output = skill.steps.iter().any(|s| s.output_key.is_some());
        assert!(
            has_output,
            "Skill '{}' should have at least one output key",
            skill.id
        );
    }
}

// ============================================================================
// Skill Context Integration Tests
// ============================================================================

#[test]
fn test_context_flows_between_steps() {
    let mut context = SkillContext::new("Analyze this code");

    // Simulate step 1 output
    context.set(
        "analysis",
        serde_json::json!({"quality": "good", "issues": 2}),
    );
    assert!(context.has_key("analysis"));

    // Simulate step 2 reading step 1's output
    let analysis = context.get("analysis").unwrap();
    assert_eq!(analysis["quality"], "good");

    context.set(
        "recommendations",
        serde_json::json!(["fix issue 1", "fix issue 2"]),
    );
    assert!(context.has_key("recommendations"));

    // Original input should still be available
    assert_eq!(context.input_str(), "Analyze this code");
}

#[test]
fn test_context_failure_tracking_across_steps() {
    let mut context = SkillContext::new("test");

    // Steps 0 and 2 fail, step 1 succeeds
    context.record_failure(0);
    context.record_failure(2);

    assert!(context.step_failed(0));
    assert!(!context.step_failed(1));
    assert!(context.step_failed(2));
    assert!(!context.step_failed(3));
}

#[test]
fn test_context_serialization() {
    let mut context = SkillContext::new("input data").with_session("session-123");
    context.set("result", serde_json::json!({"score": 0.95}));
    context.record_failure(1);

    let json = serde_json::to_string(&context).unwrap();
    let deserialized: SkillContext = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.input_str(), "input data");
    assert_eq!(deserialized.session_id, Some("session-123".to_string()));
    assert!(deserialized.has_key("result"));
    assert!(deserialized.step_failed(1));
}

// ============================================================================
// Skill Discovery Integration Tests
// ============================================================================

#[test]
fn test_discover_patterns_from_usage() {
    let mut discovery = SkillDiscovery::new();

    // Simulate repeated use of linear -> tree -> reflection
    for _ in 0..15 {
        discovery.record_chain(
            vec![
                "linear".to_string(),
                "tree".to_string(),
                "reflection".to_string(),
            ],
            true,
        );
    }

    // Record a less common pattern
    for _ in 0..3 {
        discovery.record_chain(vec!["divergent".to_string(), "decision".to_string()], true);
    }

    // Only the first pattern should be significant (min 10 occurrences, 80% success)
    let significant = discovery.significant_patterns(10, 0.8);
    assert_eq!(significant.len(), 1);
    assert_eq!(
        significant[0].tool_chain,
        vec!["linear", "tree", "reflection"]
    );
    assert_eq!(significant[0].occurrences, 15);
}

#[test]
fn test_materialize_discovered_pattern() {
    let mut discovery = SkillDiscovery::new();

    for _ in 0..10 {
        discovery.record_chain(
            vec!["divergent".to_string(), "reflection".to_string()],
            true,
        );
    }

    let skill = discovery.materialize(0).unwrap();
    assert!(skill.id.starts_with("discovered-"));
    assert_eq!(skill.category, SkillCategory::Discovered);
    assert_eq!(skill.steps.len(), 2);
    assert_eq!(skill.steps[0].mode, "divergent");
    assert_eq!(skill.steps[1].mode, "reflection");

    // All steps should have output keys and descriptions
    for step in &skill.steps {
        assert!(step.output_key.is_some());
        assert!(step.description.is_some());
    }

    // Cannot materialize again
    assert!(discovery.materialize(0).is_none());
}

#[test]
fn test_discovery_tracks_success_rate() {
    let mut discovery = SkillDiscovery::new();

    // 7 successes, 3 failures = 70% success rate
    for _ in 0..7 {
        discovery.record_chain(vec!["linear".to_string(), "evidence".to_string()], true);
    }
    for _ in 0..3 {
        discovery.record_chain(vec!["linear".to_string(), "evidence".to_string()], false);
    }

    let patterns = discovery.all_patterns();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].occurrences, 10);
    assert!((patterns[0].avg_success_rate - 0.7).abs() < 0.01);

    // Should NOT be significant at 80% threshold
    assert!(discovery.significant_patterns(5, 0.8).is_empty());

    // SHOULD be significant at 60% threshold
    assert_eq!(discovery.significant_patterns(5, 0.6).len(), 1);
}

#[test]
fn test_discovered_skill_can_be_registered() {
    let mut discovery = SkillDiscovery::new();
    for _ in 0..10 {
        discovery.record_chain(
            vec!["graph".to_string(), "counterfactual".to_string()],
            true,
        );
    }

    let skill = discovery.materialize(0).unwrap();

    let mut registry = SkillRegistry::new();
    let initial_count = registry.list().len();
    registry.register(skill);
    assert_eq!(registry.list().len(), initial_count + 1);

    let discovered = registry.list_by_category(&SkillCategory::Discovered);
    assert_eq!(discovered.len(), 1);
}

#[test]
fn test_discovery_ignores_single_tool_chains() {
    let mut discovery = SkillDiscovery::new();
    discovery.record_chain(vec!["linear".to_string()], true);
    assert!(discovery.all_patterns().is_empty());
}

// ============================================================================
// Skill Info and Serialization Integration Tests
// ============================================================================

#[test]
fn test_skill_info_conversion() {
    let registry = SkillRegistry::new();

    for skill in registry.list() {
        let info = SkillInfo::from(skill);
        assert_eq!(info.id, skill.id);
        assert_eq!(info.name, skill.name);
        assert_eq!(info.category, skill.category.to_string());
        assert_eq!(info.step_count, skill.steps.len());
    }
}

#[test]
fn test_skill_round_trip_serialization() {
    let skill = Skill::new(
        "test-skill",
        "Test",
        "A test skill",
        SkillCategory::Custom,
        vec![
            SkillStep::new("linear")
                .with_description("Step 1")
                .with_output_key("result")
                .with_error_strategy(ErrorStrategy::Retry(3)),
            SkillStep::new("tree")
                .with_operation("create")
                .with_description("Step 2")
                .with_condition(StepCondition::IfKeyExists("result".to_string()))
                .with_input_map("result", "content"),
        ],
    );

    let json = serde_json::to_string(&skill).unwrap();
    let deserialized: Skill = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "test-skill");
    assert_eq!(deserialized.steps.len(), 2);
    assert_eq!(deserialized.steps[0].on_error, ErrorStrategy::Retry(3));
    assert!(matches!(
        deserialized.steps[1].condition,
        StepCondition::IfKeyExists(_)
    ));
    assert_eq!(deserialized.steps[1].input_mapping["result"], "content");
}

#[test]
fn test_error_strategy_all_variants_serialize() {
    let strategies = vec![
        ErrorStrategy::Fail,
        ErrorStrategy::Skip,
        ErrorStrategy::Retry(5),
        ErrorStrategy::Fallback("linear".to_string()),
    ];

    for strategy in &strategies {
        let json = serde_json::to_string(strategy).unwrap();
        let back: ErrorStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, strategy);
    }
}

#[test]
fn test_custom_skill_registration_and_retrieval() {
    let mut registry = SkillRegistry::new();

    registry.register(Skill::new(
        "my-workflow",
        "My Workflow",
        "A custom reasoning workflow",
        SkillCategory::Custom,
        vec![
            SkillStep::new("divergent")
                .with_config(serde_json::json!({"force_rebellion": true}))
                .with_description("Brainstorm")
                .with_output_key("ideas"),
            SkillStep::new("decision")
                .with_operation("topsis")
                .with_description("Rank solutions")
                .with_condition(StepCondition::IfKeyExists("ideas".to_string())),
        ],
    ));

    let custom_skills = registry.list_by_category(&SkillCategory::Custom);
    assert_eq!(custom_skills.len(), 1);
    assert_eq!(custom_skills[0].id, "my-workflow");
    assert_eq!(custom_skills[0].steps.len(), 2);
}

// ============================================================================
// Preset Import Integration Tests
// ============================================================================

#[test]
fn test_preset_import_preserves_structure() {
    let preset_registry = PresetRegistry::new();
    let skill_registry = SkillRegistry::with_presets(&preset_registry);

    // code-review preset imported as skill should match
    let skill = skill_registry.get("code-review").unwrap();
    let preset = preset_registry.get("code-review").unwrap();

    assert_eq!(skill.steps.len(), preset.steps.len());
    for (skill_step, preset_step) in skill.steps.iter().zip(preset.steps.iter()) {
        assert_eq!(skill_step.mode, preset_step.mode);
        assert_eq!(skill_step.operation, preset_step.operation);
    }
}

#[test]
fn test_import_does_not_overwrite_builtin_skills() {
    // If a builtin skill has the same ID as a preset, import should not overwrite
    let preset_registry = PresetRegistry::new();
    let registry = SkillRegistry::with_presets(&preset_registry);

    // All 5 built-in skills should still be present
    assert!(registry.get("deep-code-review").is_some());
    assert!(registry.get("hypothesis-testing").is_some());
    assert!(registry.get("systems-analysis").is_some());
    assert!(registry.get("risk-assessment").is_some());
    assert!(registry.get("creative-solution").is_some());
}

#[test]
fn test_skill_category_matches_preset_category() {
    let preset_registry = PresetRegistry::new();
    let skill_registry = SkillRegistry::with_presets(&preset_registry);

    // code-review preset is CodeQuality category
    let skill = skill_registry.get("code-review").unwrap();
    assert_eq!(skill.category, SkillCategory::CodeQuality);

    // debug-analysis preset is Analysis category
    let skill = skill_registry.get("debug-analysis").unwrap();
    assert_eq!(skill.category, SkillCategory::Analysis);

    // strategic-decision is Decision category
    let skill = skill_registry.get("strategic-decision").unwrap();
    assert_eq!(skill.category, SkillCategory::Decision);
}
