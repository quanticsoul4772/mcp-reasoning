//! Default timing estimates for tools when no historical data is available.

// Allow intentional numeric casts for timing calculations
#![allow(clippy::cast_sign_loss)]

use super::timing::ComplexityMetrics;

/// Get default timing estimate for a tool.
///
/// Returns estimated duration in milliseconds based on:
/// - Tool type (fast/standard/heavy)
/// - Complexity metrics (perspectives, branches, thinking budget)
///
/// # Example
///
/// ```
/// use mcp_reasoning::metadata::{get_default_timing, ComplexityMetrics};
///
/// let complexity = ComplexityMetrics {
///     content_length: 1000,
///     thinking_budget: None,
///     num_perspectives: Some(4),
///     num_branches: None,
/// };
///
/// let estimate = get_default_timing("reasoning_divergent", &complexity);
/// assert!(estimate > 30_000); // Heavy tool with 4 perspectives
/// ```
#[must_use]
pub fn get_default_timing(tool: &str, complexity: &ComplexityMetrics) -> u64 {
    let base_time = get_base_time(tool);
    let complexity_factor = calculate_complexity_factor(complexity);

    (base_time as f64 * complexity_factor) as u64
}

/// Get base execution time for a tool in milliseconds.
fn get_base_time(tool: &str) -> u64 {
    match tool {
        // Instant tools (<1s): local registry / DB reads, no LLM call.
        "reasoning_checkpoint"
        | "reasoning_si_status"
        | "reasoning_si_diagnoses"
        | "reasoning_si_overrides"
        | "reasoning_si_approve"
        | "reasoning_si_reject"
        | "reasoning_si_rollback"
        | "reasoning_si_trigger"
        | "reasoning_agent_list"
        | "reasoning_team_list" => 100,

        // Fast tools (~0.5s): in-memory metric/DB queries, no LLM call.
        "reasoning_metrics" | "reasoning_agent_metrics" | "reasoning_list_sessions" => 500,

        // Session resume: DB read, plus an optional summarization call when
        // compress=true (default off), so allow a little headroom.
        "reasoning_resume" => 2_000,

        // Semantic memory: Voyage embedding + recall (+ rerank for search).
        "reasoning_search" => 4_000,
        "reasoning_relate" => 5_000,

        // Standard tools (8-15s)
        "reasoning_linear" => 12_000,
        "reasoning_auto" => 10_000,
        // Meta: a single problem-classification call, then returns a tool pick.
        "reasoning_meta" => 8_000,

        // Medium tools (15-30s)
        "reasoning_tree" => 18_000,
        "reasoning_decision" => 20_000,
        "reasoning_evidence" => 22_000,
        "reasoning_detect" => 16_000,
        // Confidence routing detects a mode and then executes it (often
        // linear/tree, sometimes escalates), so it carries an execution cost.
        "reasoning_confidence_route" => 25_000,

        // Heavy tools (30-60s)
        "reasoning_divergent" => 45_000,
        "reasoning_reflection" => 35_000,
        "reasoning_timeline" => 40_000,
        // A single agent / skill run is an LLM-backed reasoning step.
        "reasoning_agent_invoke" | "reasoning_skill_run" => 30_000,

        // Very heavy tools (60-120s)
        "reasoning_graph" => 75_000,
        "reasoning_mcts" => 90_000,
        "reasoning_counterfactual" => 65_000,
        // Teams / crews coordinate multiple agents (many LLM calls).
        "reasoning_team_run" | "reasoning_crew_invoke" => 90_000,

        // Preset execution (variable)
        "reasoning_preset" => 30_000,

        // Unknown tool - conservative estimate
        _ => 15_000,
    }
}

/// Calculate complexity multiplier based on request characteristics.
fn calculate_complexity_factor(complexity: &ComplexityMetrics) -> f64 {
    let mut factor = 1.0;

    // Multiple perspectives increase time significantly
    if let Some(perspectives) = complexity.num_perspectives {
        factor *= match perspectives {
            4.. => 1.5,
            3 => 1.3,
            2 => 1.2,
            _ => 1.0,
        };
    }

    // Multiple branches increase time
    if let Some(branches) = complexity.num_branches {
        factor *= match branches {
            4.. => 1.4,
            3 => 1.3,
            2 => 1.2,
            _ => 1.0,
        };
    }

    // Deep/maximum thinking budgets increase time
    if let Some(budget) = complexity.thinking_budget {
        factor *= match budget {
            16384.. => 1.4, // Maximum
            8192.. => 1.3,  // Deep
            4096.. => 1.2,  // Standard
            _ => 1.0,
        };
    }

    // Very long content increases processing time
    if complexity.content_length > 10_000 {
        factor *= 1.5;
    } else if complexity.content_length > 5000 {
        factor *= 1.3;
    }

    factor
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_complexity() -> ComplexityMetrics {
        ComplexityMetrics {
            content_length: 500,
            thinking_budget: None,
            num_perspectives: None,
            num_branches: None,
        }
    }

    #[test]
    fn test_instant_tools() {
        let complexity = simple_complexity();
        assert_eq!(get_default_timing("reasoning_checkpoint", &complexity), 100);
        assert_eq!(get_default_timing("reasoning_si_status", &complexity), 100);
    }

    #[test]
    fn test_standard_tools() {
        let complexity = simple_complexity();
        assert_eq!(get_default_timing("reasoning_linear", &complexity), 12_000);
        assert_eq!(get_default_timing("reasoning_auto", &complexity), 10_000);
    }

    #[test]
    fn test_previously_unmapped_tools_have_explicit_estimates() {
        let c = simple_complexity();
        // Each of these used to fall through to the generic 15_000 fallback.
        let expected: &[(&str, u64)] = &[
            ("reasoning_si_overrides", 100),
            ("reasoning_agent_list", 100),
            ("reasoning_team_list", 100),
            ("reasoning_agent_metrics", 500),
            ("reasoning_list_sessions", 500),
            ("reasoning_resume", 2_000),
            ("reasoning_search", 4_000),
            ("reasoning_relate", 5_000),
            ("reasoning_meta", 8_000),
            ("reasoning_confidence_route", 25_000),
            ("reasoning_agent_invoke", 30_000),
            ("reasoning_skill_run", 30_000),
            ("reasoning_team_run", 90_000),
            ("reasoning_crew_invoke", 90_000),
        ];
        for (tool, want) in expected {
            assert_eq!(
                get_default_timing(tool, &c),
                *want,
                "unexpected base estimate for {tool}"
            );
            // None of them should land on the unknown-tool fallback.
            assert_ne!(
                get_default_timing(tool, &c),
                15_000,
                "{tool} still falls through to the fallback"
            );
        }
    }

    #[test]
    fn test_heavy_tools() {
        let complexity = simple_complexity();
        assert_eq!(
            get_default_timing("reasoning_divergent", &complexity),
            45_000
        );
        assert!(get_default_timing("reasoning_mcts", &complexity) > 60_000);
    }

    #[test]
    fn test_complexity_factor_perspectives() {
        let complexity = ComplexityMetrics {
            num_perspectives: Some(4),
            ..simple_complexity()
        };

        let estimate = get_default_timing("reasoning_divergent", &complexity);
        assert!(estimate > 60_000); // 45000 * 1.5
    }

    #[test]
    fn test_complexity_factor_thinking_budget() {
        let complexity = ComplexityMetrics {
            thinking_budget: Some(16384),
            ..simple_complexity()
        };

        let estimate = get_default_timing("reasoning_reflection", &complexity);
        assert!(estimate > 45_000); // 35000 * 1.4
    }

    #[test]
    fn test_complexity_factor_content_length() {
        let complexity = ComplexityMetrics {
            content_length: 12_000,
            ..simple_complexity()
        };

        let estimate = get_default_timing("reasoning_linear", &complexity);
        assert!(estimate > 15_000); // 12000 * 1.4
    }

    #[test]
    fn test_combined_complexity_factors() {
        let complexity = ComplexityMetrics {
            num_perspectives: Some(4),
            thinking_budget: Some(16384),
            content_length: 8000,
            num_branches: None,
        };

        // Base: 45000, perspectives: *1.5, budget: *1.4, content: *1.2
        // Total: 45000 * 1.5 * 1.4 * 1.2 ≈ 113400
        let estimate = get_default_timing("reasoning_divergent", &complexity);
        assert!(estimate > 100_000);
    }
}
