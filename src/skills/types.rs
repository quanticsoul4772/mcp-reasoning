//! Skill type definitions.
//!
//! Enhanced versions of preset types with context passing,
//! conditional execution, and error strategies.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Error handling strategy for a skill step.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStrategy {
    /// Stop the skill on error.
    #[default]
    Fail,
    /// Skip this step and continue.
    Skip,
    /// Retry the step up to N times.
    Retry(u32),
    /// Use a fallback mode instead.
    Fallback(String),
}

impl std::fmt::Display for ErrorStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fail => write!(f, "fail"),
            Self::Skip => write!(f, "skip"),
            Self::Retry(n) => write!(f, "retry({n})"),
            Self::Fallback(mode) => write!(f, "fallback({mode})"),
        }
    }
}

/// Condition for executing a skill step.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepCondition {
    /// Always execute.
    #[default]
    Always,
    /// Execute if a context key exists.
    IfKeyExists(String),
    /// Execute if confidence from previous step exceeds threshold.
    IfConfidenceAbove(f64),
    /// Execute if a specific step index failed.
    IfStepFailed(usize),
}

/// A single step in a skill workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    /// The reasoning mode to use.
    pub mode: String,
    /// Specific operation within the mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Additional configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,
    /// Description of this step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Map context keys to step input parameters.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub input_mapping: HashMap<String, String>,
    /// Store step result under this context key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_key: Option<String>,
    /// Condition for executing this step.
    #[serde(default)]
    pub condition: StepCondition,
    /// Error handling strategy.
    #[serde(default)]
    pub on_error: ErrorStrategy,
}

impl SkillStep {
    /// Create a new skill step.
    #[must_use]
    pub fn new(mode: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            operation: None,
            config: None,
            description: None,
            input_mapping: HashMap::new(),
            output_key: None,
            condition: StepCondition::default(),
            on_error: ErrorStrategy::default(),
        }
    }

    /// Set the operation.
    #[must_use]
    pub fn with_operation(mut self, op: impl Into<String>) -> Self {
        self.operation = Some(op.into());
        self
    }

    /// Set configuration.
    #[must_use]
    pub fn with_config(mut self, config: Value) -> Self {
        self.config = Some(config);
        self
    }

    /// Set description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set output key for context storage.
    #[must_use]
    pub fn with_output_key(mut self, key: impl Into<String>) -> Self {
        self.output_key = Some(key.into());
        self
    }

    /// Add an input mapping from context key to parameter.
    #[must_use]
    pub fn with_input_map(
        mut self,
        context_key: impl Into<String>,
        param: impl Into<String>,
    ) -> Self {
        self.input_mapping.insert(context_key.into(), param.into());
        self
    }

    /// Set the execution condition.
    #[must_use]
    pub fn with_condition(mut self, condition: StepCondition) -> Self {
        self.condition = condition;
        self
    }

    /// Set error strategy.
    #[must_use]
    pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
        self.on_error = strategy;
        self
    }
}

/// Execution context that flows between skill steps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillContext {
    /// Named values stored by previous steps.
    pub values: HashMap<String, Value>,
    /// Session ID for this execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Step failure indices.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_steps: Vec<usize>,
}

impl SkillContext {
    /// Create a new context with initial input.
    #[must_use]
    pub fn new(input: impl Into<String>) -> Self {
        let mut values = HashMap::new();
        values.insert("input".to_string(), Value::String(input.into()));
        Self {
            values,
            session_id: None,
            failed_steps: Vec::new(),
        }
    }

    /// Set the session ID.
    #[must_use]
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Store a value in context.
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.values.insert(key.into(), value);
    }

    /// Get a value from context.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// Check if a key exists.
    #[must_use]
    pub fn has_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Get the input value as a string.
    #[must_use]
    pub fn input_str(&self) -> &str {
        self.values
            .get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Record a step failure.
    pub fn record_failure(&mut self, step_index: usize) {
        self.failed_steps.push(step_index);
    }

    /// Check if a step failed.
    #[must_use]
    pub fn step_failed(&self, step_index: usize) -> bool {
        self.failed_steps.contains(&step_index)
    }
}

/// Category of a skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    /// Code quality and review.
    CodeQuality,
    /// General analysis.
    Analysis,
    /// Decision making.
    Decision,
    /// Research and synthesis.
    Research,
    /// Discovered from patterns.
    Discovered,
    /// User-defined.
    Custom,
}

impl std::fmt::Display for SkillCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CodeQuality => write!(f, "code_quality"),
            Self::Analysis => write!(f, "analysis"),
            Self::Decision => write!(f, "decision"),
            Self::Research => write!(f, "research"),
            Self::Discovered => write!(f, "discovered"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

/// A composable skill definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Unique skill identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Category.
    pub category: SkillCategory,
    /// Steps in the skill workflow.
    pub steps: Vec<SkillStep>,
}

impl Skill {
    /// Create a new skill.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: SkillCategory,
        steps: Vec<SkillStep>,
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

/// Summary info for listing skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    /// Skill ID.
    pub id: String,
    /// Skill name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Category.
    pub category: String,
    /// Number of steps.
    pub step_count: usize,
}

impl From<&Skill> for SkillInfo {
    fn from(skill: &Skill) -> Self {
        Self {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            category: skill.category.to_string(),
            step_count: skill.steps.len(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_error_strategy_display() {
        assert_eq!(ErrorStrategy::Fail.to_string(), "fail");
        assert_eq!(ErrorStrategy::Skip.to_string(), "skip");
        assert_eq!(ErrorStrategy::Retry(3).to_string(), "retry(3)");
        assert_eq!(
            ErrorStrategy::Fallback("linear".to_string()).to_string(),
            "fallback(linear)"
        );
    }

    #[test]
    fn test_error_strategy_default() {
        assert_eq!(ErrorStrategy::default(), ErrorStrategy::Fail);
    }

    #[test]
    fn test_skill_step_new() {
        let step = SkillStep::new("linear");
        assert_eq!(step.mode, "linear");
        assert!(step.operation.is_none());
        assert!(matches!(step.condition, StepCondition::Always));
        assert_eq!(step.on_error, ErrorStrategy::Fail);
    }

    #[test]
    fn test_skill_step_builder() {
        let step = SkillStep::new("tree")
            .with_operation("create")
            .with_config(serde_json::json!({"num_branches": 3}))
            .with_description("Generate hypotheses")
            .with_output_key("hypotheses")
            .with_input_map("analysis", "content")
            .with_condition(StepCondition::IfKeyExists("analysis".to_string()))
            .with_error_strategy(ErrorStrategy::Skip);

        assert_eq!(step.mode, "tree");
        assert_eq!(step.operation, Some("create".to_string()));
        assert!(step.config.is_some());
        assert_eq!(step.output_key, Some("hypotheses".to_string()));
        assert_eq!(step.input_mapping["analysis"], "content");
        assert!(matches!(step.condition, StepCondition::IfKeyExists(_)));
        assert_eq!(step.on_error, ErrorStrategy::Skip);
    }

    #[test]
    fn test_skill_context_new() {
        let ctx = SkillContext::new("Review this code");
        assert_eq!(ctx.input_str(), "Review this code");
        assert!(ctx.has_key("input"));
    }

    #[test]
    fn test_skill_context_set_get() {
        let mut ctx = SkillContext::new("task");
        ctx.set("result", serde_json::json!({"score": 0.9}));
        assert!(ctx.has_key("result"));
        assert_eq!(ctx.get("result").unwrap()["score"], 0.9);
    }

    #[test]
    fn test_skill_context_failure_tracking() {
        let mut ctx = SkillContext::new("task");
        assert!(!ctx.step_failed(0));
        ctx.record_failure(0);
        assert!(ctx.step_failed(0));
        assert!(!ctx.step_failed(1));
    }

    #[test]
    fn test_skill_context_with_session() {
        let ctx = SkillContext::new("task").with_session("s1");
        assert_eq!(ctx.session_id, Some("s1".to_string()));
    }

    #[test]
    fn test_skill_category_display() {
        assert_eq!(SkillCategory::CodeQuality.to_string(), "code_quality");
        assert_eq!(SkillCategory::Discovered.to_string(), "discovered");
    }

    #[test]
    fn test_skill_new() {
        let skill = Skill::new(
            "test",
            "Test Skill",
            "A test skill",
            SkillCategory::Analysis,
            vec![SkillStep::new("linear")],
        );
        assert_eq!(skill.id, "test");
        assert_eq!(skill.steps.len(), 1);
    }

    #[test]
    fn test_skill_info_from() {
        let skill = Skill::new(
            "test",
            "Test",
            "desc",
            SkillCategory::Custom,
            vec![SkillStep::new("linear"), SkillStep::new("tree")],
        );
        let info = SkillInfo::from(&skill);
        assert_eq!(info.id, "test");
        assert_eq!(info.category, "custom");
        assert_eq!(info.step_count, 2);
    }

    #[test]
    fn test_skill_serialize() {
        let skill = Skill::new(
            "test",
            "Test",
            "desc",
            SkillCategory::Analysis,
            vec![SkillStep::new("linear").with_output_key("result")],
        );
        let json = serde_json::to_string(&skill).unwrap();
        assert!(json.contains("\"output_key\":\"result\""));
    }

    #[test]
    fn test_step_condition_default() {
        assert!(matches!(StepCondition::default(), StepCondition::Always));
    }
}
