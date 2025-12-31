//! Rule-based suggestion engine for tool composition.

use super::{PresetIndex, PresetSuggestion};
use crate::metadata::ToolSuggestion;
use std::sync::Arc;

/// Rule-based suggestion engine for tool composition.
pub struct SuggestionEngine {
    preset_index: Arc<PresetIndex>,
}

impl SuggestionEngine {
    /// Create a new suggestion engine.
    #[must_use]
    pub fn new(preset_index: Arc<PresetIndex>) -> Self {
        Self { preset_index }
    }

    /// Generate next-tool suggestions based on current context.
    #[must_use]
    pub fn suggest_next_tools(
        &self,
        current_tool: &str,
        result_context: &ResultContext,
    ) -> Vec<ToolSuggestion> {
        match current_tool {
            "reasoning_divergent" => self.suggest_after_divergent(result_context),
            "reasoning_tree" => self.suggest_after_tree(result_context),
            "reasoning_linear" => self.suggest_after_linear(result_context),
            "reasoning_decision" => self.suggest_after_decision(result_context),
            "reasoning_graph" => self.suggest_after_graph(result_context),
            "reasoning_reflection" => self.suggest_after_reflection(result_context),
            "reasoning_mcts" => self.suggest_after_mcts(result_context),
            "reasoning_evidence" => self.suggest_after_evidence(result_context),
            _ => vec![],
        }
    }

    /// Find relevant presets for current workflow.
    #[must_use]
    pub fn suggest_presets(
        &self,
        tool_history: &[String],
        _current_goal: Option<&str>,
    ) -> Vec<PresetSuggestion> {
        self.preset_index.find_matching_presets(tool_history)
    }

    fn suggest_after_divergent(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
        let mut suggestions = vec![];

        // Always suggest checkpoint after complex analysis
        suggestions.push(ToolSuggestion {
            tool: "reasoning_checkpoint".into(),
            reason: "Save this multi-perspective analysis before continuing".into(),
            estimated_duration_ms: 100,
        });

        // Suggest decision analysis if multiple perspectives
        if ctx.num_outputs >= 3 {
            suggestions.push(ToolSuggestion {
                tool: "reasoning_decision".into(),
                reason: format!(
                    "Synthesize {} perspectives into decision options",
                    ctx.num_outputs
                ),
                estimated_duration_ms: 15_000,
            });
        }

        // Suggest reflection if complex
        if ctx.complexity == "complex" {
            suggestions.push(ToolSuggestion {
                tool: "reasoning_reflection".into(),
                reason: "Evaluate and refine this analysis".into(),
                estimated_duration_ms: 25_000,
            });
        }

        suggestions
    }

    fn suggest_after_tree(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
        let mut suggestions = vec![];

        if ctx.has_branches {
            suggestions.push(ToolSuggestion {
                tool: "reasoning_decision".into(),
                reason: "Compare and evaluate the different branches".into(),
                estimated_duration_ms: 18_000,
            });
        }

        suggestions.push(ToolSuggestion {
            tool: "reasoning_checkpoint".into(),
            reason: "Save branch state for later exploration".into(),
            estimated_duration_ms: 100,
        });

        suggestions
    }

    fn suggest_after_linear(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
        let mut suggestions = vec![];

        // Suggest divergent for more perspectives
        if ctx.complexity != "simple" {
            suggestions.push(ToolSuggestion {
                tool: "reasoning_divergent".into(),
                reason: "Explore alternative perspectives on this analysis".into(),
                estimated_duration_ms: 45_000,
            });
        }

        // Suggest evidence evaluation
        suggestions.push(ToolSuggestion {
            tool: "reasoning_evidence".into(),
            reason: "Evaluate the strength of evidence and claims".into(),
            estimated_duration_ms: 20_000,
        });

        suggestions
    }

    fn suggest_after_decision(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        vec![
            ToolSuggestion {
                tool: "reasoning_checkpoint".into(),
                reason: "Save decision analysis before proceeding".into(),
                estimated_duration_ms: 100,
            },
            ToolSuggestion {
                tool: "reasoning_reflection".into(),
                reason: "Review the decision-making process for improvements".into(),
                estimated_duration_ms: 25_000,
            },
        ]
    }

    fn suggest_after_graph(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
        let mut suggestions = vec![];

        if ctx.num_outputs > 5 {
            suggestions.push(ToolSuggestion {
                tool: "reasoning_decision".into(),
                reason: "Synthesize insights from graph exploration".into(),
                estimated_duration_ms: 20_000,
            });
        }

        suggestions.push(ToolSuggestion {
            tool: "reasoning_checkpoint".into(),
            reason: "Save graph state for later analysis".into(),
            estimated_duration_ms: 100,
        });

        suggestions
    }

    fn suggest_after_reflection(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        vec![ToolSuggestion {
            tool: "reasoning_checkpoint".into(),
            reason: "Save refined analysis".into(),
            estimated_duration_ms: 100,
        }]
    }

    fn suggest_after_mcts(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
        let mut suggestions = vec![];

        suggestions.push(ToolSuggestion {
            tool: "reasoning_decision".into(),
            reason: "Evaluate the explored search paths".into(),
            estimated_duration_ms: 18_000,
        });

        if ctx.session_id.is_some() {
            suggestions.push(ToolSuggestion {
                tool: "reasoning_checkpoint".into(),
                reason: "Save search tree state".into(),
                estimated_duration_ms: 100,
            });
        }

        suggestions
    }

    fn suggest_after_evidence(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        vec![
            ToolSuggestion {
                tool: "reasoning_decision".into(),
                reason: "Make decisions based on evaluated evidence".into(),
                estimated_duration_ms: 18_000,
            },
            ToolSuggestion {
                tool: "reasoning_reflection".into(),
                reason: "Reflect on evidence quality and reasoning".into(),
                estimated_duration_ms: 25_000,
            },
        ]
    }
}

/// Context from tool execution result.
#[derive(Debug, Clone, Default)]
pub struct ResultContext {
    /// Number of outputs generated.
    pub num_outputs: usize,
    /// Whether the result has branches.
    pub has_branches: bool,
    /// Session ID if applicable.
    pub session_id: Option<String>,
    /// Complexity level: "simple", "moderate", "complex".
    pub complexity: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::metadata::PresetIndex;

    fn create_test_engine() -> SuggestionEngine {
        let preset_index = PresetIndex::build();
        SuggestionEngine::new(Arc::new(preset_index))
    }

    #[test]
    fn test_suggest_after_divergent_simple() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            num_outputs: 2,
            complexity: "simple".into(),
            ..Default::default()
        };

        let suggestions = engine.suggest_next_tools("reasoning_divergent", &ctx);

        assert!(suggestions
            .iter()
            .any(|s| s.tool == "reasoning_checkpoint"));
        assert!(!suggestions.iter().any(|s| s.tool == "reasoning_decision")); // <3 outputs
    }

    #[test]
    fn test_suggest_after_divergent_complex() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            num_outputs: 4,
            complexity: "complex".into(),
            ..Default::default()
        };

        let suggestions = engine.suggest_next_tools("reasoning_divergent", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_checkpoint"));
        assert!(suggestions.iter().any(|s| s.tool == "reasoning_decision"));
        assert!(suggestions
            .iter()
            .any(|s| s.tool == "reasoning_reflection"));
    }

    #[test]
    fn test_suggest_after_tree() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            has_branches: true,
            ..Default::default()
        };

        let suggestions = engine.suggest_next_tools("reasoning_tree", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_decision"));
        assert!(suggestions.iter().any(|s| s.tool == "reasoning_checkpoint"));
    }

    #[test]
    fn test_suggest_after_linear() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            complexity: "moderate".into(),
            ..Default::default()
        };

        let suggestions = engine.suggest_next_tools("reasoning_linear", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_divergent"));
        assert!(suggestions.iter().any(|s| s.tool == "reasoning_evidence"));
    }

    #[test]
    fn test_suggest_unknown_tool() {
        let engine = create_test_engine();
        let ctx = ResultContext::default();

        let suggestions = engine.suggest_next_tools("unknown_tool", &ctx);

        assert!(suggestions.is_empty());
    }
}
