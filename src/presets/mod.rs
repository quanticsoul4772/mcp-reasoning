//! Workflow presets.
//!
//! This module provides:
//! - Built-in preset definitions (5 presets)
//! - Preset execution logic
//! - Preset listing and management
//!
//! # Available Presets
//!
//! | Preset | Category | Description |
//! |--------|----------|-------------|
//! | code-review | CodeQuality | Analyze code with bias detection and alternatives |
//! | debug-analysis | Analysis | Hypothesis-driven debugging with evidence |
//! | architecture-decision | Decision | Multi-factor architectural decision making |
//! | strategic-decision | Decision | Stakeholder-aware strategic planning |
//! | evidence-conclusion | Research | Evidence-based research synthesis |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Category of a preset workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetCategory {
    /// Code quality analysis.
    CodeQuality,
    /// General analysis.
    Analysis,
    /// Decision making.
    Decision,
    /// Research synthesis.
    Research,
    /// User-defined.
    Custom,
}

impl std::fmt::Display for PresetCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CodeQuality => write!(f, "code_quality"),
            Self::Analysis => write!(f, "analysis"),
            Self::Decision => write!(f, "decision"),
            Self::Research => write!(f, "research"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

/// A single step in a preset workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetStep {
    /// The reasoning mode to use.
    pub mode: String,
    /// Specific operation within the mode (e.g., "create", "assess").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Additional configuration for this step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    /// Description of what this step accomplishes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PresetStep {
    /// Create a new preset step.
    #[must_use]
    pub fn new(mode: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            operation: None,
            config: None,
            description: None,
        }
    }

    /// Add an operation.
    #[must_use]
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    /// Add configuration.
    #[must_use]
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }

    /// Add a description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// A preset workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of the workflow.
    pub description: String,
    /// Category.
    pub category: PresetCategory,
    /// Steps in the workflow.
    pub steps: Vec<PresetStep>,
}

impl Preset {
    /// Create a new preset.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: PresetCategory,
        steps: Vec<PresetStep>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            category,
            steps,
        }
    }
}

/// Result from running a preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetResult {
    /// Preset that was run.
    pub preset_id: String,
    /// Session ID for context.
    pub session_id: String,
    /// Results from each step.
    pub step_results: Vec<StepResult>,
    /// Overall success.
    pub success: bool,
    /// Final synthesis (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthesis: Option<String>,
}

/// Result from a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step index (0-based).
    pub step_index: usize,
    /// Mode used.
    pub mode: String,
    /// Operation used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Whether step succeeded.
    pub success: bool,
    /// Step output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl StepResult {
    /// Create a successful step result.
    #[must_use]
    pub fn success(
        step_index: usize,
        mode: impl Into<String>,
        operation: Option<String>,
        output: serde_json::Value,
    ) -> Self {
        Self {
            step_index,
            mode: mode.into(),
            operation,
            success: true,
            output: Some(output),
            error: None,
        }
    }

    /// Create a failed step result.
    #[must_use]
    pub fn failure(
        step_index: usize,
        mode: impl Into<String>,
        operation: Option<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            step_index,
            mode: mode.into(),
            operation,
            success: false,
            output: None,
            error: Some(error.into()),
        }
    }
}

/// Registry of available presets.
#[derive(Debug, Default)]
pub struct PresetRegistry {
    presets: HashMap<String, Preset>,
}

impl PresetRegistry {
    /// Create a new registry with built-in presets.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_builtin_presets();
        registry
    }

    /// Register the 5 built-in presets.
    fn register_builtin_presets(&mut self) {
        // 1. Code Review
        self.register(Preset::new(
            "code-review",
            "Code Review",
            "Comprehensive code analysis with bias detection and alternative approaches",
            PresetCategory::CodeQuality,
            vec![
                PresetStep::new("linear").with_description("Understand the code structure"),
                PresetStep::new("detect")
                    .with_operation("biases")
                    .with_description("Check for cognitive biases in the implementation"),
                PresetStep::new("divergent")
                    .with_config(serde_json::json!({"num_perspectives": 3}))
                    .with_description("Generate alternative approaches"),
                PresetStep::new("reflection")
                    .with_operation("evaluate")
                    .with_description("Final assessment and recommendations"),
            ],
        ));

        // 2. Debug Analysis
        self.register(Preset::new(
            "debug-analysis",
            "Debug Analysis",
            "Hypothesis-driven debugging with evidence evaluation",
            PresetCategory::Analysis,
            vec![
                PresetStep::new("linear").with_description("Understand the problem"),
                PresetStep::new("tree")
                    .with_operation("create")
                    .with_config(serde_json::json!({"num_branches": 3}))
                    .with_description("Generate hypotheses"),
                PresetStep::new("evidence")
                    .with_operation("assess")
                    .with_description("Evaluate evidence for each hypothesis"),
                PresetStep::new("counterfactual").with_description("Analyze what-if scenarios"),
            ],
        ));

        // 3. Architecture Decision
        self.register(Preset::new(
            "architecture-decision",
            "Architecture Decision",
            "Multi-factor architectural decision making with impact analysis",
            PresetCategory::Decision,
            vec![
                PresetStep::new("divergent")
                    .with_config(serde_json::json!({"challenge_assumptions": true}))
                    .with_description("Generate architectural options"),
                PresetStep::new("decision")
                    .with_operation("weighted")
                    .with_description("Score options against criteria"),
                PresetStep::new("graph")
                    .with_operation("init")
                    .with_description("Map dependencies and impacts"),
                PresetStep::new("mcts")
                    .with_operation("explore")
                    .with_description("Explore implications"),
                PresetStep::new("reflection")
                    .with_operation("evaluate")
                    .with_description("Final decision and rationale"),
            ],
        ));

        // 4. Strategic Decision
        self.register(Preset::new(
            "strategic-decision",
            "Strategic Decision",
            "Stakeholder-aware strategic planning with risk assessment",
            PresetCategory::Decision,
            vec![
                PresetStep::new("decision")
                    .with_operation("perspectives")
                    .with_description("Map stakeholder perspectives"),
                PresetStep::new("timeline")
                    .with_operation("create")
                    .with_description("Create future scenarios"),
                PresetStep::new("timeline")
                    .with_operation("branch")
                    .with_description("Explore alternative paths"),
                PresetStep::new("evidence")
                    .with_operation("probabilistic")
                    .with_description("Assess risks probabilistically"),
                PresetStep::new("decision")
                    .with_operation("topsis")
                    .with_description("Final ranking with TOPSIS"),
            ],
        ));

        // 5. Evidence-Based Conclusion
        self.register(Preset::new(
            "evidence-conclusion",
            "Evidence-Based Conclusion",
            "Research synthesis with rigorous evidence evaluation",
            PresetCategory::Research,
            vec![
                PresetStep::new("evidence")
                    .with_operation("assess")
                    .with_description("Evaluate source credibility"),
                PresetStep::new("detect")
                    .with_operation("fallacies")
                    .with_description("Check for logical fallacies"),
                PresetStep::new("graph")
                    .with_operation("init")
                    .with_description("Build argument map"),
                PresetStep::new("graph")
                    .with_operation("aggregate")
                    .with_description("Synthesize arguments"),
                PresetStep::new("linear").with_description("Draw final conclusion"),
            ],
        ));
    }

    /// Register a preset.
    pub fn register(&mut self, preset: Preset) {
        self.presets.insert(preset.id.clone(), preset);
    }

    /// Get a preset by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Preset> {
        self.presets.get(id)
    }

    /// List all presets.
    #[must_use]
    pub fn list(&self) -> Vec<&Preset> {
        self.presets.values().collect()
    }

    /// List presets by category.
    #[must_use]
    pub fn list_by_category(&self, category: &PresetCategory) -> Vec<&Preset> {
        self.presets
            .values()
            .filter(|p| &p.category == category)
            .collect()
    }
}

/// Response from listing presets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPresetsResponse {
    /// Available presets.
    pub presets: Vec<PresetInfo>,
    /// Total count.
    pub total: usize,
}

/// Summary info for a preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetInfo {
    /// Preset ID.
    pub id: String,
    /// Preset name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Category.
    pub category: String,
    /// Number of steps.
    pub step_count: usize,
}

impl From<&Preset> for PresetInfo {
    fn from(preset: &Preset) -> Self {
        Self {
            id: preset.id.clone(),
            name: preset.name.clone(),
            description: preset.description.clone(),
            category: preset.category.to_string(),
            step_count: preset.steps.len(),
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
    fn test_preset_category_display() {
        assert_eq!(PresetCategory::CodeQuality.to_string(), "code_quality");
        assert_eq!(PresetCategory::Analysis.to_string(), "analysis");
        assert_eq!(PresetCategory::Decision.to_string(), "decision");
        assert_eq!(PresetCategory::Research.to_string(), "research");
        assert_eq!(PresetCategory::Custom.to_string(), "custom");
    }

    #[test]
    fn test_preset_step_new() {
        let step = PresetStep::new("linear");
        assert_eq!(step.mode, "linear");
        assert!(step.operation.is_none());
        assert!(step.config.is_none());
        assert!(step.description.is_none());
    }

    #[test]
    fn test_preset_step_with_operation() {
        let step = PresetStep::new("tree").with_operation("create");
        assert_eq!(step.mode, "tree");
        assert_eq!(step.operation, Some("create".to_string()));
    }

    #[test]
    fn test_preset_step_with_config() {
        let step =
            PresetStep::new("divergent").with_config(serde_json::json!({"num_perspectives": 3}));
        assert!(step.config.is_some());
        let config = step.config.unwrap();
        assert_eq!(config["num_perspectives"], 3);
    }

    #[test]
    fn test_preset_step_with_description() {
        let step = PresetStep::new("linear").with_description("Analyze the code");
        assert_eq!(step.description, Some("Analyze the code".to_string()));
    }

    #[test]
    fn test_preset_step_chained() {
        let step = PresetStep::new("tree")
            .with_operation("create")
            .with_config(serde_json::json!({"num_branches": 3}))
            .with_description("Generate hypotheses");

        assert_eq!(step.mode, "tree");
        assert_eq!(step.operation, Some("create".to_string()));
        assert!(step.config.is_some());
        assert!(step.description.is_some());
    }

    #[test]
    fn test_preset_new() {
        let preset = Preset::new(
            "test",
            "Test Preset",
            "A test preset",
            PresetCategory::Analysis,
            vec![PresetStep::new("linear")],
        );

        assert_eq!(preset.id, "test");
        assert_eq!(preset.name, "Test Preset");
        assert_eq!(preset.description, "A test preset");
        assert_eq!(preset.category, PresetCategory::Analysis);
        assert_eq!(preset.steps.len(), 1);
    }

    #[test]
    fn test_step_result_success() {
        let result = StepResult::success(0, "linear", None, serde_json::json!({"content": "test"}));

        assert!(result.success);
        assert_eq!(result.step_index, 0);
        assert_eq!(result.mode, "linear");
        assert!(result.output.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_step_result_failure() {
        let result = StepResult::failure(1, "tree", Some("create".to_string()), "API error");

        assert!(!result.success);
        assert_eq!(result.step_index, 1);
        assert_eq!(result.mode, "tree");
        assert_eq!(result.operation, Some("create".to_string()));
        assert!(result.output.is_none());
        assert_eq!(result.error, Some("API error".to_string()));
    }

    #[test]
    fn test_preset_registry_new() {
        let registry = PresetRegistry::new();

        // Should have 5 built-in presets
        assert_eq!(registry.list().len(), 5);
    }

    #[test]
    fn test_preset_registry_get() {
        let registry = PresetRegistry::new();

        let code_review = registry.get("code-review");
        assert!(code_review.is_some());
        assert_eq!(code_review.unwrap().name, "Code Review");

        let unknown = registry.get("unknown");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_preset_registry_list_by_category() {
        let registry = PresetRegistry::new();

        let decision_presets = registry.list_by_category(&PresetCategory::Decision);
        assert_eq!(decision_presets.len(), 2); // architecture-decision, strategic-decision

        let research_presets = registry.list_by_category(&PresetCategory::Research);
        assert_eq!(research_presets.len(), 1); // evidence-conclusion
    }

    #[test]
    fn test_code_review_preset() {
        let registry = PresetRegistry::new();
        let preset = registry.get("code-review").unwrap();

        assert_eq!(preset.category, PresetCategory::CodeQuality);
        assert_eq!(preset.steps.len(), 4);
        assert_eq!(preset.steps[0].mode, "linear");
        assert_eq!(preset.steps[1].mode, "detect");
        assert_eq!(preset.steps[1].operation, Some("biases".to_string()));
    }

    #[test]
    fn test_debug_analysis_preset() {
        let registry = PresetRegistry::new();
        let preset = registry.get("debug-analysis").unwrap();

        assert_eq!(preset.category, PresetCategory::Analysis);
        assert_eq!(preset.steps.len(), 4);
        assert_eq!(preset.steps[3].mode, "counterfactual");
    }

    #[test]
    fn test_architecture_decision_preset() {
        let registry = PresetRegistry::new();
        let preset = registry.get("architecture-decision").unwrap();

        assert_eq!(preset.category, PresetCategory::Decision);
        assert_eq!(preset.steps.len(), 5);
        assert_eq!(preset.steps[2].mode, "graph");
        assert_eq!(preset.steps[2].operation, Some("init".to_string()));
    }

    #[test]
    fn test_strategic_decision_preset() {
        let registry = PresetRegistry::new();
        let preset = registry.get("strategic-decision").unwrap();

        assert_eq!(preset.category, PresetCategory::Decision);
        assert_eq!(preset.steps.len(), 5);
        assert_eq!(preset.steps[4].mode, "decision");
        assert_eq!(preset.steps[4].operation, Some("topsis".to_string()));
    }

    #[test]
    fn test_evidence_conclusion_preset() {
        let registry = PresetRegistry::new();
        let preset = registry.get("evidence-conclusion").unwrap();

        assert_eq!(preset.category, PresetCategory::Research);
        assert_eq!(preset.steps.len(), 5);
        assert_eq!(preset.steps[0].mode, "evidence");
        assert_eq!(preset.steps[0].operation, Some("assess".to_string()));
    }

    #[test]
    fn test_preset_info_from() {
        let preset = Preset::new(
            "test",
            "Test",
            "Description",
            PresetCategory::Custom,
            vec![PresetStep::new("linear"), PresetStep::new("tree")],
        );

        let info = PresetInfo::from(&preset);
        assert_eq!(info.id, "test");
        assert_eq!(info.name, "Test");
        assert_eq!(info.category, "custom");
        assert_eq!(info.step_count, 2);
    }

    #[test]
    fn test_preset_category_serialize() {
        let category = PresetCategory::CodeQuality;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"code_quality\"");
    }

    #[test]
    fn test_preset_serialize() {
        let preset = Preset::new(
            "test",
            "Test",
            "A test",
            PresetCategory::Analysis,
            vec![PresetStep::new("linear").with_operation("process")],
        );

        let json = serde_json::to_string(&preset).unwrap();
        assert!(json.contains("\"id\":\"test\""));
        assert!(json.contains("\"mode\":\"linear\""));
        assert!(json.contains("\"operation\":\"process\""));
    }

    #[test]
    fn test_preset_registry_register() {
        let mut registry = PresetRegistry::new();
        let initial_count = registry.list().len();

        registry.register(Preset::new(
            "custom-preset",
            "Custom",
            "A custom preset",
            PresetCategory::Custom,
            vec![PresetStep::new("linear")],
        ));

        assert_eq!(registry.list().len(), initial_count + 1);
        assert!(registry.get("custom-preset").is_some());
    }

    #[test]
    fn test_list_presets_response() {
        let registry = PresetRegistry::new();
        let presets: Vec<PresetInfo> = registry
            .list()
            .iter()
            .map(|p| PresetInfo::from(*p))
            .collect();

        let response = ListPresetsResponse {
            total: presets.len(),
            presets,
        };

        assert_eq!(response.total, 5);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":5"));
    }
}
