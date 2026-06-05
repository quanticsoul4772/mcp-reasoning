//! Rule-based suggestion engine for tool composition, blended with empirical
//! tool-chain transitions observed at runtime.

use super::{get_default_timing, ComplexityMetrics, PresetIndex, PresetSuggestion};
use crate::metadata::ToolSuggestion;
use crate::metrics::TransitionStats;
use std::sync::Arc;

/// Minimum observations of a transition before it is trusted enough to alter
/// suggestions (promote or suppress). Below this we defer to the static rules.
const MIN_TRANSITION_SAMPLES: u32 = 3;

/// Success rate at or above which an observed transition is promoted to the
/// front of the suggestions (it reliably works in practice).
const HIGH_SUCCESS_RATE: f64 = 0.6;

/// Success rate below which a sufficiently-observed transition is treated as an
/// anti-pattern and suppressed from suggestions (it tends to fail).
const LOW_SUCCESS_RATE: f64 = 0.4;

/// Upper bound on returned suggestions so the agent is not overwhelmed.
const MAX_SUGGESTIONS: usize = 5;

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
            // Existing handlers (8 tools)
            "reasoning_divergent" => self.suggest_after_divergent(result_context),
            "reasoning_tree" => self.suggest_after_tree(result_context),
            "reasoning_linear" => self.suggest_after_linear(result_context),
            "reasoning_decision" => self.suggest_after_decision(result_context),
            "reasoning_graph" => self.suggest_after_graph(result_context),
            "reasoning_reflection" => self.suggest_after_reflection(result_context),
            "reasoning_mcts" => self.suggest_after_mcts(result_context),
            "reasoning_evidence" => self.suggest_after_evidence(result_context),
            // New handlers (7 tools)
            "reasoning_auto" => self.suggest_after_auto(result_context),
            "reasoning_detect" => self.suggest_after_detect(result_context),
            "reasoning_timeline" => self.suggest_after_timeline(result_context),
            "reasoning_counterfactual" => self.suggest_after_counterfactual(result_context),
            "reasoning_checkpoint" => self.suggest_after_checkpoint(result_context),
            "reasoning_preset" => self.suggest_after_preset(result_context),
            "reasoning_metrics" => vec![], // Terminal tool, no suggestions
            _ => vec![],
        }
    }

    /// Generate next-tool suggestions, blending the static rules with the
    /// empirically observed outgoing transitions from `current_tool`.
    ///
    /// `empirical` is the transition data from
    /// [`crate::metrics::MetricsCollector::transition_stats_from`] (keyed in the
    /// same tool-name namespace as the static rules). With no observations this
    /// degrades exactly to [`Self::suggest_next_tools`].
    ///
    /// Blending (only transitions with ≥ [`MIN_TRANSITION_SAMPLES`] count):
    /// - **Suppress** destinations whose success rate is below
    ///   [`LOW_SUCCESS_RATE`] — don't recommend a next step that tends to fail.
    /// - **Promote** destinations at or above [`HIGH_SUCCESS_RATE`] to the front
    ///   (most-observed first), annotating the reason with the evidence and
    ///   adding ones the static rules missed.
    #[must_use]
    pub fn suggest_next_tools_blended(
        &self,
        current_tool: &str,
        result_context: &ResultContext,
        empirical: &[(String, TransitionStats)],
    ) -> Vec<ToolSuggestion> {
        let mut suggestions = self.suggest_next_tools(current_tool, result_context);
        Self::apply_empirical(&mut suggestions, empirical);
        suggestions
    }

    /// Blend observed transitions into a static suggestion list in place.
    fn apply_empirical(
        suggestions: &mut Vec<ToolSuggestion>,
        empirical: &[(String, TransitionStats)],
    ) {
        // 1. Suppress anti-patterns: well-observed but low-success destinations.
        for (to_tool, stats) in empirical {
            if stats.count >= MIN_TRANSITION_SAMPLES && stats.success_rate < LOW_SUCCESS_RATE {
                suggestions.retain(|s| &s.tool != to_tool);
            }
        }

        // 2. Promote high-success destinations to the front, in count-desc order
        //    (empirical is pre-sorted), pulling matching static entries up and
        //    annotating them, or inserting new ones.
        let mut promoted: Vec<ToolSuggestion> = Vec::new();
        for (to_tool, stats) in empirical {
            if stats.count < MIN_TRANSITION_SAMPLES || stats.success_rate < HIGH_SUCCESS_RATE {
                continue;
            }
            let evidence = format!(
                "observed {}× next, {:.0}% success",
                stats.count,
                stats.success_rate * 100.0
            );
            if let Some(pos) = suggestions.iter().position(|s| &s.tool == to_tool) {
                let mut existing = suggestions.remove(pos);
                existing.reason = format!("{} — {evidence}", existing.reason);
                promoted.push(existing);
            } else {
                promoted.push(ToolSuggestion {
                    tool: to_tool.clone(),
                    reason: format!("Empirically follows this tool ({evidence})"),
                    estimated_duration_ms: get_default_timing(
                        to_tool,
                        &ComplexityMetrics::default(),
                    ),
                });
            }
        }

        // Empirically-backed suggestions first, then the remaining static ones.
        promoted.append(suggestions);
        *suggestions = promoted;
        suggestions.truncate(MAX_SUGGESTIONS);
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

    // ========================================================================
    // New handlers for missing 7 tools
    // ========================================================================

    fn suggest_after_auto(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        vec![
            ToolSuggestion {
                tool: "reasoning_checkpoint".into(),
                reason: "Save auto-selected analysis results".into(),
                estimated_duration_ms: 100,
            },
            ToolSuggestion {
                tool: "reasoning_reflection".into(),
                reason: "Review the auto-selected reasoning approach".into(),
                estimated_duration_ms: 25_000,
            },
        ]
    }

    fn suggest_after_detect(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
        let mut suggestions = vec![];

        if ctx.num_outputs > 0 {
            suggestions.push(ToolSuggestion {
                tool: "reasoning_reflection".into(),
                reason: "Reflect on detected biases/fallacies".into(),
                estimated_duration_ms: 25_000,
            });
        }

        suggestions.push(ToolSuggestion {
            tool: "reasoning_linear".into(),
            reason: "Re-analyze with detected issues in mind".into(),
            estimated_duration_ms: 12_000,
        });

        suggestions
    }

    fn suggest_after_timeline(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        vec![
            ToolSuggestion {
                tool: "reasoning_decision".into(),
                reason: "Compare timeline branches for decision".into(),
                estimated_duration_ms: 18_000,
            },
            ToolSuggestion {
                tool: "reasoning_checkpoint".into(),
                reason: "Save timeline state".into(),
                estimated_duration_ms: 100,
            },
        ]
    }

    fn suggest_after_counterfactual(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        vec![
            ToolSuggestion {
                tool: "reasoning_decision".into(),
                reason: "Use causal insights for decision-making".into(),
                estimated_duration_ms: 18_000,
            },
            ToolSuggestion {
                tool: "reasoning_evidence".into(),
                reason: "Evaluate evidence for causal claims".into(),
                estimated_duration_ms: 20_000,
            },
        ]
    }

    fn suggest_after_checkpoint(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        // Checkpoint is typically terminal, but can continue
        vec![ToolSuggestion {
            tool: "reasoning_linear".into(),
            reason: "Continue analysis from saved state".into(),
            estimated_duration_ms: 12_000,
        }]
    }

    fn suggest_after_preset(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
        // Preset execution may suggest continuation based on complexity
        if ctx.complexity == "complex" {
            vec![ToolSuggestion {
                tool: "reasoning_reflection".into(),
                reason: "Review preset workflow results".into(),
                estimated_duration_ms: 25_000,
            }]
        } else {
            vec![ToolSuggestion {
                tool: "reasoning_checkpoint".into(),
                reason: "Save preset execution results".into(),
                estimated_duration_ms: 100,
            }]
        }
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

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_checkpoint"));
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
        assert!(suggestions.iter().any(|s| s.tool == "reasoning_reflection"));
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

    // ========================================================================
    // Tests for new tool handlers
    // ========================================================================

    #[test]
    fn test_suggest_after_auto() {
        let engine = create_test_engine();
        let ctx = ResultContext::default();

        let suggestions = engine.suggest_next_tools("reasoning_auto", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_checkpoint"));
        assert!(suggestions.iter().any(|s| s.tool == "reasoning_reflection"));
    }

    #[test]
    fn test_suggest_after_detect_with_outputs() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            num_outputs: 3,
            ..Default::default()
        };

        let suggestions = engine.suggest_next_tools("reasoning_detect", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_reflection"));
        assert!(suggestions.iter().any(|s| s.tool == "reasoning_linear"));
    }

    #[test]
    fn test_suggest_after_detect_no_outputs() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            num_outputs: 0,
            ..Default::default()
        };

        let suggestions = engine.suggest_next_tools("reasoning_detect", &ctx);

        assert!(!suggestions.iter().any(|s| s.tool == "reasoning_reflection"));
        assert!(suggestions.iter().any(|s| s.tool == "reasoning_linear"));
    }

    #[test]
    fn test_suggest_after_timeline() {
        let engine = create_test_engine();
        let ctx = ResultContext::default();

        let suggestions = engine.suggest_next_tools("reasoning_timeline", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_decision"));
        assert!(suggestions.iter().any(|s| s.tool == "reasoning_checkpoint"));
    }

    #[test]
    fn test_suggest_after_counterfactual() {
        let engine = create_test_engine();
        let ctx = ResultContext::default();

        let suggestions = engine.suggest_next_tools("reasoning_counterfactual", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_decision"));
        assert!(suggestions.iter().any(|s| s.tool == "reasoning_evidence"));
    }

    #[test]
    fn test_suggest_after_checkpoint() {
        let engine = create_test_engine();
        let ctx = ResultContext::default();

        let suggestions = engine.suggest_next_tools("reasoning_checkpoint", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_linear"));
        assert_eq!(suggestions.len(), 1);
    }

    #[test]
    fn test_suggest_after_preset_complex() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            complexity: "complex".into(),
            ..Default::default()
        };

        let suggestions = engine.suggest_next_tools("reasoning_preset", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_reflection"));
    }

    #[test]
    fn test_suggest_after_preset_simple() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            complexity: "simple".into(),
            ..Default::default()
        };

        let suggestions = engine.suggest_next_tools("reasoning_preset", &ctx);

        assert!(suggestions.iter().any(|s| s.tool == "reasoning_checkpoint"));
    }

    #[test]
    fn test_suggest_after_metrics() {
        let engine = create_test_engine();
        let ctx = ResultContext::default();

        let suggestions = engine.suggest_next_tools("reasoning_metrics", &ctx);

        assert!(suggestions.is_empty()); // Terminal tool
    }

    // ========================================================================
    // Empirical blending
    // ========================================================================

    fn stats(count: u32, success_rate: f64) -> TransitionStats {
        TransitionStats {
            count,
            success_rate,
            avg_time_between_ms: 0,
        }
    }

    #[test]
    fn test_blended_no_empirical_matches_static() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            has_branches: true,
            ..Default::default()
        };

        let static_only = engine.suggest_next_tools("reasoning_tree", &ctx);
        let blended = engine.suggest_next_tools_blended("reasoning_tree", &ctx, &[]);

        assert_eq!(static_only, blended);
    }

    #[test]
    fn test_blended_ignores_low_sample_transitions() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            has_branches: true,
            ..Default::default()
        };

        // Only 2 observations (< MIN_TRANSITION_SAMPLES): no effect.
        let empirical = vec![("reasoning_mcts".to_string(), stats(2, 1.0))];
        let blended = engine.suggest_next_tools_blended("reasoning_tree", &ctx, &empirical);

        assert!(!blended.iter().any(|s| s.tool == "reasoning_mcts"));
    }

    #[test]
    fn test_blended_promotes_high_success_transition_to_front() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            has_branches: true,
            ..Default::default()
        };

        // A tool the static rules don't suggest, strongly observed to follow.
        let empirical = vec![("reasoning_mcts".to_string(), stats(10, 0.9))];
        let blended = engine.suggest_next_tools_blended("reasoning_tree", &ctx, &empirical);

        assert_eq!(
            blended.first().map(|s| s.tool.as_str()),
            Some("reasoning_mcts")
        );
        assert!(blended[0].reason.contains("observed 10×"));
        assert!(blended[0].estimated_duration_ms > 0);
    }

    #[test]
    fn test_blended_annotates_existing_static_suggestion() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            has_branches: true,
            ..Default::default()
        };

        // reasoning_decision is a static suggestion after tree; back it with data.
        let empirical = vec![("reasoning_decision".to_string(), stats(8, 0.75))];
        let blended = engine.suggest_next_tools_blended("reasoning_tree", &ctx, &empirical);

        let decision = blended
            .iter()
            .find(|s| s.tool == "reasoning_decision")
            .expect("decision still suggested");
        assert!(decision.reason.contains("75% success"));
        // Promoted to the front.
        assert_eq!(
            blended.first().map(|s| s.tool.as_str()),
            Some("reasoning_decision")
        );
    }

    #[test]
    fn test_blended_suppresses_low_success_anti_pattern() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            has_branches: true,
            ..Default::default()
        };

        // reasoning_decision is normally suggested after tree, but it fails a lot.
        let empirical = vec![("reasoning_decision".to_string(), stats(10, 0.2))];
        let blended = engine.suggest_next_tools_blended("reasoning_tree", &ctx, &empirical);

        assert!(!blended.iter().any(|s| s.tool == "reasoning_decision"));
    }

    #[test]
    fn test_blended_caps_suggestion_count() {
        let engine = create_test_engine();
        let ctx = ResultContext {
            num_outputs: 4,
            complexity: "complex".into(),
            ..Default::default()
        };

        let empirical = vec![
            ("reasoning_mcts".to_string(), stats(9, 0.9)),
            ("reasoning_graph".to_string(), stats(8, 0.9)),
            ("reasoning_timeline".to_string(), stats(7, 0.9)),
            ("reasoning_evidence".to_string(), stats(6, 0.9)),
            ("reasoning_counterfactual".to_string(), stats(5, 0.9)),
            ("reasoning_linear".to_string(), stats(4, 0.9)),
        ];
        let blended = engine.suggest_next_tools_blended("reasoning_divergent", &ctx, &empirical);

        assert!(blended.len() <= MAX_SUGGESTIONS);
    }
}
