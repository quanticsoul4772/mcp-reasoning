//! Preset registry workflow integration tests.
//!
//! Tests the preset system functionality:
//! 1. List available presets
//! 2. Get preset by ID
//! 3. Filter presets by category
//! 4. Verify preset step definitions
//!
//! Note: Preset execution is handled by the server/tools.rs handlers.
//! These tests focus on the preset registry and definitions.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use mcp_reasoning::presets::{Preset, PresetCategory, PresetInfo, PresetRegistry, PresetStep};

// ============================================================================
// Registry Tests
// ============================================================================

#[test]
fn test_preset_registry_has_builtin_presets() {
    let registry = PresetRegistry::new();
    let presets = registry.list();

    // Should have 5 built-in presets
    assert_eq!(presets.len(), 5, "Should have 5 built-in presets");

    // Check all expected presets exist
    let preset_ids: Vec<&str> = presets.iter().map(|p| p.id.as_str()).collect();
    assert!(preset_ids.contains(&"code-review"));
    assert!(preset_ids.contains(&"debug-analysis"));
    assert!(preset_ids.contains(&"architecture-decision"));
    assert!(preset_ids.contains(&"strategic-decision"));
    assert!(preset_ids.contains(&"evidence-conclusion"));
}

#[test]
fn test_preset_registry_get_by_id() {
    let registry = PresetRegistry::new();

    // Get existing preset
    let code_review = registry.get("code-review");
    assert!(code_review.is_some(), "Should find code-review preset");
    let preset = code_review.unwrap();
    assert_eq!(preset.name, "Code Review");
    assert_eq!(preset.category, PresetCategory::CodeQuality);

    // Get non-existent preset
    let missing = registry.get("nonexistent-preset");
    assert!(missing.is_none(), "Should not find nonexistent preset");
}

#[test]
fn test_preset_registry_list_by_category() {
    let registry = PresetRegistry::new();

    // Decision category should have 2 presets
    let decision_presets = registry.list_by_category(&PresetCategory::Decision);
    assert_eq!(decision_presets.len(), 2);

    let decision_ids: Vec<&str> = decision_presets.iter().map(|p| p.id.as_str()).collect();
    assert!(decision_ids.contains(&"architecture-decision"));
    assert!(decision_ids.contains(&"strategic-decision"));

    // Research category should have 1 preset
    let research_presets = registry.list_by_category(&PresetCategory::Research);
    assert_eq!(research_presets.len(), 1);
    assert_eq!(research_presets[0].id, "evidence-conclusion");

    // CodeQuality category should have 1 preset
    let code_presets = registry.list_by_category(&PresetCategory::CodeQuality);
    assert_eq!(code_presets.len(), 1);
    assert_eq!(code_presets[0].id, "code-review");

    // Analysis category should have 1 preset
    let analysis_presets = registry.list_by_category(&PresetCategory::Analysis);
    assert_eq!(analysis_presets.len(), 1);
    assert_eq!(analysis_presets[0].id, "debug-analysis");
}

// ============================================================================
// Preset Step Definition Tests
// ============================================================================

#[test]
fn test_code_review_preset_steps() {
    let registry = PresetRegistry::new();
    let preset = registry.get("code-review").unwrap();

    assert_eq!(preset.steps.len(), 4, "Code review should have 4 steps");

    // Step 1: Linear analysis
    assert_eq!(preset.steps[0].mode, "linear");
    assert!(preset.steps[0].description.is_some());

    // Step 2: Detect biases
    assert_eq!(preset.steps[1].mode, "detect");
    assert_eq!(preset.steps[1].operation, Some("biases".to_string()));

    // Step 3: Divergent perspectives
    assert_eq!(preset.steps[2].mode, "divergent");
    assert!(preset.steps[2].config.is_some());

    // Step 4: Reflection
    assert_eq!(preset.steps[3].mode, "reflection");
    assert_eq!(preset.steps[3].operation, Some("evaluate".to_string()));
}

#[test]
fn test_debug_analysis_preset_steps() {
    let registry = PresetRegistry::new();
    let preset = registry.get("debug-analysis").unwrap();

    assert_eq!(preset.steps.len(), 4, "Debug analysis should have 4 steps");

    // Should use tree for hypothesis generation
    let tree_step = preset.steps.iter().find(|s| s.mode == "tree");
    assert!(tree_step.is_some(), "Should have tree step");
    assert_eq!(tree_step.unwrap().operation, Some("create".to_string()));

    // Should use evidence assessment
    let evidence_step = preset.steps.iter().find(|s| s.mode == "evidence");
    assert!(evidence_step.is_some(), "Should have evidence step");

    // Should use counterfactual
    let cf_step = preset.steps.iter().find(|s| s.mode == "counterfactual");
    assert!(cf_step.is_some(), "Should have counterfactual step");
}

#[test]
fn test_architecture_decision_preset_steps() {
    let registry = PresetRegistry::new();
    let preset = registry.get("architecture-decision").unwrap();

    assert_eq!(
        preset.steps.len(),
        5,
        "Architecture decision should have 5 steps"
    );

    // Should use divergent with challenge_assumptions
    let divergent_step = preset.steps.iter().find(|s| s.mode == "divergent");
    assert!(divergent_step.is_some());
    let config = divergent_step.unwrap().config.as_ref().unwrap();
    assert_eq!(config["challenge_assumptions"], true);

    // Should use weighted decision
    let decision_step = preset
        .steps
        .iter()
        .find(|s| s.mode == "decision" && s.operation == Some("weighted".to_string()));
    assert!(
        decision_step.is_some(),
        "Should have weighted decision step"
    );

    // Should use graph and mcts
    assert!(preset.steps.iter().any(|s| s.mode == "graph"));
    assert!(preset.steps.iter().any(|s| s.mode == "mcts"));
}

#[test]
fn test_strategic_decision_preset_steps() {
    let registry = PresetRegistry::new();
    let preset = registry.get("strategic-decision").unwrap();

    assert_eq!(
        preset.steps.len(),
        5,
        "Strategic decision should have 5 steps"
    );

    // Should start with perspectives
    assert_eq!(preset.steps[0].mode, "decision");
    assert_eq!(preset.steps[0].operation, Some("perspectives".to_string()));

    // Should use timeline for scenarios
    let timeline_steps: Vec<&PresetStep> = preset
        .steps
        .iter()
        .filter(|s| s.mode == "timeline")
        .collect();
    assert_eq!(timeline_steps.len(), 2, "Should have 2 timeline steps");

    // Should end with TOPSIS
    let last_step = preset.steps.last().unwrap();
    assert_eq!(last_step.mode, "decision");
    assert_eq!(last_step.operation, Some("topsis".to_string()));
}

#[test]
fn test_evidence_conclusion_preset_steps() {
    let registry = PresetRegistry::new();
    let preset = registry.get("evidence-conclusion").unwrap();

    assert_eq!(
        preset.steps.len(),
        5,
        "Evidence conclusion should have 5 steps"
    );

    // Should start with evidence assessment
    assert_eq!(preset.steps[0].mode, "evidence");
    assert_eq!(preset.steps[0].operation, Some("assess".to_string()));

    // Should detect fallacies
    let detect_step = preset.steps.iter().find(|s| s.mode == "detect");
    assert!(detect_step.is_some());
    assert_eq!(
        detect_step.unwrap().operation,
        Some("fallacies".to_string())
    );

    // Should use graph for argument mapping
    let graph_steps: Vec<&PresetStep> = preset.steps.iter().filter(|s| s.mode == "graph").collect();
    assert_eq!(graph_steps.len(), 2, "Should have 2 graph steps");
}

// ============================================================================
// Custom Preset Registration Tests
// ============================================================================

#[test]
fn test_custom_preset_registration() {
    let mut registry = PresetRegistry::new();
    let initial_count = registry.list().len();

    // Register custom preset
    let custom_preset = Preset::new(
        "custom-workflow",
        "Custom Workflow",
        "A custom reasoning workflow",
        PresetCategory::Custom,
        vec![
            PresetStep::new("linear").with_description("Initial analysis"),
            PresetStep::new("tree")
                .with_operation("create")
                .with_config(serde_json::json!({"num_branches": 4})),
            PresetStep::new("reflection").with_operation("evaluate"),
        ],
    );

    registry.register(custom_preset);

    // Verify registration
    assert_eq!(registry.list().len(), initial_count + 1);

    let retrieved = registry.get("custom-workflow");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().steps.len(), 3);

    // Should appear in custom category
    let custom_presets = registry.list_by_category(&PresetCategory::Custom);
    assert_eq!(custom_presets.len(), 1);
    assert_eq!(custom_presets[0].id, "custom-workflow");
}

// ============================================================================
// Preset Info Conversion Tests
// ============================================================================

#[test]
fn test_preset_info_conversion() {
    let registry = PresetRegistry::new();
    let preset = registry.get("code-review").unwrap();

    let info = PresetInfo::from(preset);

    assert_eq!(info.id, "code-review");
    assert_eq!(info.name, "Code Review");
    assert!(!info.description.is_empty());
    assert_eq!(info.category, "code_quality");
    assert_eq!(info.step_count, 4);
}

#[test]
fn test_all_presets_have_descriptions() {
    let registry = PresetRegistry::new();

    for preset in registry.list() {
        assert!(
            !preset.description.is_empty(),
            "Preset {} should have description",
            preset.id
        );

        // Each step should have a description
        for (i, step) in preset.steps.iter().enumerate() {
            assert!(
                step.description.is_some(),
                "Preset {} step {} should have description",
                preset.id,
                i
            );
        }
    }
}

#[test]
fn test_preset_step_builder_pattern() {
    let step = PresetStep::new("graph")
        .with_operation("init")
        .with_config(serde_json::json!({"problem": "Test problem"}))
        .with_description("Initialize the graph structure");

    assert_eq!(step.mode, "graph");
    assert_eq!(step.operation, Some("init".to_string()));
    assert!(step.config.is_some());
    assert_eq!(
        step.description,
        Some("Initialize the graph structure".to_string())
    );
}

#[test]
fn test_preset_category_display() {
    assert_eq!(PresetCategory::CodeQuality.to_string(), "code_quality");
    assert_eq!(PresetCategory::Analysis.to_string(), "analysis");
    assert_eq!(PresetCategory::Decision.to_string(), "decision");
    assert_eq!(PresetCategory::Research.to_string(), "research");
    assert_eq!(PresetCategory::Custom.to_string(), "custom");
}

#[test]
fn test_preset_serialization() {
    let registry = PresetRegistry::new();
    let preset = registry.get("code-review").unwrap();

    // Should serialize without errors
    let json = serde_json::to_string(preset).expect("Should serialize");
    assert!(json.contains("\"id\":\"code-review\""));
    assert!(json.contains("\"category\":\"code_quality\""));
    assert!(json.contains("\"mode\":\"linear\""));
}
