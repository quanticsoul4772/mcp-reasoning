use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::{
    FrontierNode, MctsMode, QualityAssessment, QualityTrend, SelectedNode, TimelineMode,
};
use crate::server::requests::{CounterfactualRequest, MctsRequest, TimelineRequest};
use crate::server::responses::{
    BacktrackSuggestion, BranchComparison, CausalStep, CounterfactualResponse, MctsAlternative,
    MctsBackpropagation, MctsExpandedNode, MctsFrontierNode, MctsNode, MctsRecommendation,
    MctsResponse, MctsSelectedNode, MctsValidationInfo, TimelineBranch, TimelineResponse,
};

use super::{DEEP_THINKING, MAXIMUM_THINKING};

/// Serialize a `#[serde(rename_all)]` enum to its string form (e.g. `prune`).
fn enum_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Verify the UCB1 decomposition of each frontier node and that the selected
/// node is the argmax of `ucb1_score`. Returns the validation and the
/// recomputed best node id.
fn verify_explore(
    frontier: &[FrontierNode],
    selected: &SelectedNode,
) -> (MctsValidationInfo, Option<String>) {
    let mut warnings = Vec::new();

    // UCB1 = exploitation (average_value) + exploration_bonus.
    for n in frontier {
        let expected = n.average_value + n.exploration_bonus;
        if (n.ucb1_score - expected).abs() > 0.01 {
            warnings.push(format!(
                "Node '{}' UCB1 stated {:.3} but average_value + exploration_bonus = {:.3}",
                n.node_id, n.ucb1_score, expected
            ));
        }
    }

    // Selection should pick the highest-UCB1 node.
    let best = frontier.iter().max_by(|a, b| {
        a.ucb1_score
            .partial_cmp(&b.ucb1_score)
            .unwrap_or(Ordering::Equal)
    });
    let best_id = best.map(|n| n.node_id.clone());
    if let Some(b) = best {
        match frontier.iter().find(|n| n.node_id == selected.node_id) {
            Some(sel) if sel.ucb1_score + 0.01 < b.ucb1_score => warnings.push(format!(
                "Selected node '{}' (UCB1 {:.3}) is not the highest-UCB1 frontier node '{}' (UCB1 {:.3})",
                selected.node_id, sel.ucb1_score, b.node_id, b.ucb1_score
            )),
            None if !frontier.is_empty() => warnings.push(format!(
                "Selected node '{}' is not present in the frontier",
                selected.node_id
            )),
            _ => {}
        }
    }

    (
        MctsValidationInfo {
            consistent: warnings.is_empty(),
            warnings,
        },
        best_id,
    )
}

/// Verify that the stated quality trend and decline magnitude are consistent
/// with the recent value samples.
fn verify_backtrack(qa: &QualityAssessment) -> MctsValidationInfo {
    let mut warnings = Vec::new();
    let vals = &qa.recent_values;

    if vals.len() >= 2 {
        let first = vals[0];
        let last = vals[vals.len() - 1];
        let expected_trend = if last + 0.02 < first {
            QualityTrend::Declining
        } else if last > first + 0.02 {
            QualityTrend::Improving
        } else {
            QualityTrend::Stable
        };
        if qa.trend != expected_trend {
            warnings.push(format!(
                "Trend stated '{}' but recent values go {first:.2} → {last:.2} (implies '{}')",
                enum_to_string(&qa.trend),
                enum_to_string(&expected_trend)
            ));
        }

        let max = vals.iter().copied().fold(f64::MIN, f64::max);
        let min = vals.iter().copied().fold(f64::MAX, f64::min);
        let range = (max - min).max(0.0);
        if qa.decline_magnitude < -0.01 || qa.decline_magnitude > range + 0.15 {
            warnings.push(format!(
                "decline_magnitude stated {:.2} but the peak-to-trough range is only {range:.2}",
                qa.decline_magnitude
            ));
        }
    }

    MctsValidationInfo {
        consistent: warnings.is_empty(),
        warnings,
    }
}

impl super::ReasoningServer {
    pub(super) async fn handle_timeline(&self, req: TimelineRequest) -> TimelineResponse {
        let timer = Timer::start();
        let operation = req.operation.clone();
        let mode = TimelineMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let content = req.content.as_deref().unwrap_or("");

        // Apply tool-level timeout to prevent indefinite hangs
        let timeout_ms = self.state.config.timeout_for_thinking_budget(DEEP_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);
        let op_for_timeout = operation.clone();

        let (response, success) = match tokio::time::timeout(timeout_duration, async {
            match op_for_timeout.as_str() {
            "create" => match mode.create(content, req.session_id).await {
                Ok(resp) => (
                    TimelineResponse {
                        timeline_id: resp.timeline_id,
                        branch_id: None,
                        branches: None,
                        comparison: None,
                        merged_content: None,
                        metadata: None,
                    },
                    true,
                ),
                Err(e) => (
                    TimelineResponse {
                        timeline_id: format!(
                            "timeline create failed: {e}. \
                             Provide non-empty content describing the scenario. \
                             Use operation='branch' once a timeline_id exists."
                        ),
                        branch_id: None,
                        branches: None,
                        comparison: None,
                        merged_content: None,
                        metadata: None,
                    },
                    false,
                ),
            },
            "branch" => match mode.branch(content, req.session_id).await {
                Ok(resp) => (
                    TimelineResponse {
                        timeline_id: String::new(),
                        branch_id: Some(resp.branch_point.event_id.clone()),
                        branches: Some(
                            resp.branches
                                .into_iter()
                                .map(|b| TimelineBranch {
                                    id: b.id,
                                    label: Some(b.choice),
                                    content: b
                                        .events
                                        .into_iter()
                                        .map(|e| e.description)
                                        .collect::<Vec<_>>()
                                        .join("; "),
                                    created_at: String::new(),
                                })
                                .collect(),
                        ),
                        comparison: Some(BranchComparison {
                            divergence_points: vec![resp.branch_point.description],
                            quality_differences: serde_json::json!({
                                "most_likely_good_outcome": resp.comparison.most_likely_good_outcome,
                                "highest_risk": resp.comparison.highest_risk
                            }),
                            convergence_opportunities: resp.comparison.key_differences,
                        }),
                        merged_content: None,
                        metadata: None,
                    },
                    true,
                ),
                Err(e) => (
                    TimelineResponse {
                        timeline_id: format!(
                            "timeline branch failed: {e}. \
                             Provide a session_id from a previous create call. \
                             Use operation='create' first if no timeline exists yet."
                        ),
                        branch_id: None,
                        branches: None,
                        comparison: None,
                        merged_content: None,
                        metadata: None,
                    },
                    false,
                ),
            },
            "compare" => match mode.compare(content, req.session_id).await {
                Ok(resp) => (
                    TimelineResponse {
                        timeline_id: String::new(),
                        branch_id: None,
                        branches: None,
                        comparison: Some(BranchComparison {
                            divergence_points: vec![resp.divergence_point],
                            quality_differences: serde_json::json!({
                                "key_differences": resp.key_differences.iter().map(|d| {
                                    serde_json::json!({
                                        "dimension": d.dimension,
                                        "branch_1_value": d.branch_1_value,
                                        "branch_2_value": d.branch_2_value,
                                        "significance": d.significance
                                    })
                                }).collect::<Vec<_>>(),
                                "recommendation": {
                                    "preferred_branch": resp.recommendation.preferred_branch,
                                    "conditions": resp.recommendation.conditions,
                                    "key_factors": resp.recommendation.key_factors
                                }
                            }),
                            convergence_opportunities: resp.branches_compared,
                        }),
                        merged_content: None,
                        metadata: None,
                    },
                    true,
                ),
                Err(e) => (
                    TimelineResponse {
                        timeline_id: format!(
                            "timeline compare failed: {e}. \
                             Provide a session_id with at least 2 branches to compare. \
                             Use operation='branch' first to create divergent paths."
                        ),
                        branch_id: None,
                        branches: None,
                        comparison: None,
                        merged_content: None,
                        metadata: None,
                    },
                    false,
                ),
            },
            "merge" => match mode.merge(content, req.session_id).await {
                Ok(resp) => (
                    TimelineResponse {
                        timeline_id: String::new(),
                        branch_id: None,
                        branches: None,
                        comparison: None,
                        merged_content: Some(format!(
                            "Synthesis: {}. Recommendations: {}",
                            resp.synthesis,
                            resp.recommendations.join("; ")
                        )),
                        metadata: None,
                    },
                    true,
                ),
                Err(e) => (
                    TimelineResponse {
                        timeline_id: format!(
                            "timeline merge failed: {e}. \
                             Provide a session_id with branches to synthesize. \
                             Use operation='compare' first to identify divergence points."
                        ),
                        branch_id: None,
                        branches: None,
                        comparison: None,
                        merged_content: None,
                        metadata: None,
                    },
                    false,
                ),
            },
            _ => (
                TimelineResponse {
                    timeline_id: format!("Unknown operation: {}", op_for_timeout),
                    branch_id: None,
                    branches: None,
                    comparison: None,
                    merged_content: None,
                    metadata: None,
                },
                false,
            ),
            }
        })
        .await
        {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                tracing::error!(
                    tool = "reasoning_timeline",
                    timeout_ms = timeout_ms,
                    operation = %operation,
                    "Tool execution timed out"
                );
                (
                    TimelineResponse {
                        timeline_id: format!(
                            "timeline timed out after {timeout_ms}ms. \
                             Retry with shorter content or a simpler scenario."
                        ),
                        branch_id: None,
                        branches: None,
                        comparison: None,
                        merged_content: None,
                        metadata: None,
                    },
                    false,
                )
            }
        };

        self.state.metrics.record(
            MetricEvent::new("timeline", timer.elapsed_ms(), success).with_operation(&operation),
        );

        response
    }

    pub(super) async fn handle_mcts(&self, req: MctsRequest) -> MctsResponse {
        let timer = Timer::start();
        let mode = MctsMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let operation = req.operation.as_deref().unwrap_or("explore");
        let content = req.content.as_deref().unwrap_or("");
        let input_session_id = req.session_id.clone().unwrap_or_default();

        // Create progress reporter (use progress_token or generate one)
        let progress_token = req
            .progress_token
            .unwrap_or_else(|| format!("mcts-{}", uuid::Uuid::new_v4()));
        let progress = self.state.create_progress_reporter(&progress_token);

        tracing::info!(
            tool = "reasoning_mcts",
            operation = operation,
            progress_token = %progress_token,
            "Tool invocation started (streaming)"
        );

        // Apply tool-level timeout (MAXIMUM_THINKING = 16384 tokens)
        let timeout_ms = self
            .state
            .config
            .timeout_for_thinking_budget(MAXIMUM_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);

        let (response, success) = match operation {
            "explore" => {
                let explore_result = match tokio::time::timeout(
                    timeout_duration,
                    mode.explore_streaming(content, req.session_id, Some(&progress)),
                )
                .await
                {
                    Ok(inner) => inner,
                    Err(_elapsed) => {
                        tracing::error!(
                            tool = "reasoning_mcts",
                            operation = "explore",
                            timeout_ms = timeout_ms,
                            "Tool execution timed out"
                        );
                        Err(ModeError::Timeout {
                            elapsed_ms: timeout_ms,
                        })
                    }
                };
                match explore_result {
                    Ok(resp) => {
                        let (validation, _best_id) =
                            verify_explore(&resp.frontier_evaluation, &resp.selected_node);
                        let frontier: Vec<MctsFrontierNode> = resp
                            .frontier_evaluation
                            .iter()
                            .map(|n| MctsFrontierNode {
                                node_id: n.node_id.clone(),
                                visits: n.visits,
                                average_value: n.average_value,
                                exploration_bonus: n.exploration_bonus,
                                ucb1_score: n.ucb1_score,
                            })
                            .collect();
                        // `best_path` kept for backward compatibility, with honest
                        // (un-fabricated) content.
                        let best_path = resp
                            .frontier_evaluation
                            .iter()
                            .map(|n| MctsNode {
                                node_id: n.node_id.clone(),
                                content: String::new(),
                                ucb_score: n.ucb1_score,
                                visits: n.visits,
                            })
                            .collect();
                        let expanded_nodes = resp
                            .expansion
                            .new_nodes
                            .into_iter()
                            .map(|n| MctsExpandedNode {
                                id: n.id,
                                content: n.content,
                                simulated_value: n.simulated_value,
                            })
                            .collect();
                        (
                            MctsResponse {
                                session_id: resp.session_id,
                                best_path: Some(best_path),
                                iterations_completed: Some(resp.search_status.total_simulations),
                                backtrack_suggestion: None,
                                executed: None,
                                frontier: Some(frontier),
                                selected_node: Some(MctsSelectedNode {
                                    node_id: resp.selected_node.node_id,
                                    selection_reason: resp.selected_node.selection_reason,
                                }),
                                expanded_nodes: Some(expanded_nodes),
                                backpropagation: Some(MctsBackpropagation {
                                    updated_nodes: resp.backpropagation.updated_nodes,
                                    value_changes: resp.backpropagation.value_changes,
                                }),
                                best_path_value: Some(resp.search_status.best_path_value),
                                backtrack_to: None,
                                recent_values: None,
                                quality_trend: None,
                                alternatives: None,
                                recommendation: None,
                                validation: Some(validation),
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(_) => (
                        MctsResponse {
                            session_id: input_session_id.clone(),
                            best_path: None,
                            iterations_completed: None,
                            backtrack_suggestion: None,
                            executed: None,
                            frontier: None,
                            selected_node: None,
                            expanded_nodes: None,
                            backpropagation: None,
                            best_path_value: None,
                            backtrack_to: None,
                            recent_values: None,
                            quality_trend: None,
                            alternatives: None,
                            recommendation: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                }
            }
            "auto_backtrack" => {
                let backtrack_result = match tokio::time::timeout(
                    timeout_duration,
                    mode.auto_backtrack_streaming(
                        content,
                        Some(input_session_id.clone()),
                        Some(&progress),
                    ),
                )
                .await
                {
                    Ok(inner) => inner,
                    Err(_elapsed) => {
                        tracing::error!(
                            tool = "reasoning_mcts",
                            operation = "auto_backtrack",
                            timeout_ms = timeout_ms,
                            "Tool execution timed out"
                        );
                        Err(ModeError::Timeout {
                            elapsed_ms: timeout_ms,
                        })
                    }
                };
                match backtrack_result {
                    Ok(resp) => {
                        let validation = verify_backtrack(&resp.quality_assessment);
                        let alternatives = resp
                            .alternative_actions
                            .iter()
                            .map(|a| MctsAlternative {
                                action: enum_to_string(&a.action),
                                rationale: a.rationale.clone(),
                            })
                            .collect();
                        let recommendation = MctsRecommendation {
                            action: enum_to_string(&resp.recommendation.action),
                            confidence: resp.recommendation.confidence,
                            expected_benefit: resp.recommendation.expected_benefit.clone(),
                        };
                        (
                            MctsResponse {
                                session_id: resp.session_id,
                                best_path: None,
                                iterations_completed: None,
                                backtrack_suggestion: Some(BacktrackSuggestion {
                                    should_backtrack: resp.backtrack_decision.should_backtrack,
                                    target_step: resp.backtrack_decision.depth_reduction,
                                    reason: Some(resp.backtrack_decision.reason),
                                    quality_drop: Some(resp.quality_assessment.decline_magnitude),
                                }),
                                executed: req.auto_execute,
                                frontier: None,
                                selected_node: None,
                                expanded_nodes: None,
                                backpropagation: None,
                                best_path_value: None,
                                backtrack_to: resp.backtrack_decision.backtrack_to,
                                recent_values: Some(resp.quality_assessment.recent_values),
                                quality_trend: Some(enum_to_string(&resp.quality_assessment.trend)),
                                alternatives: Some(alternatives),
                                recommendation: Some(recommendation),
                                validation: Some(validation),
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(_) => (
                        MctsResponse {
                            session_id: input_session_id.clone(),
                            best_path: None,
                            iterations_completed: None,
                            backtrack_suggestion: None,
                            executed: None,
                            frontier: None,
                            selected_node: None,
                            expanded_nodes: None,
                            backpropagation: None,
                            best_path_value: None,
                            backtrack_to: None,
                            recent_values: None,
                            quality_trend: None,
                            alternatives: None,
                            recommendation: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                }
            }
            _ => (
                MctsResponse {
                    session_id: input_session_id,
                    best_path: None,
                    iterations_completed: None,
                    backtrack_suggestion: None,
                    executed: None,
                    frontier: None,
                    selected_node: None,
                    expanded_nodes: None,
                    backpropagation: None,
                    best_path_value: None,
                    backtrack_to: None,
                    recent_values: None,
                    quality_trend: None,
                    alternatives: None,
                    recommendation: None,
                    validation: None,
                    metadata: None,
                },
                false,
            ),
        };

        self.state.metrics.record(
            MetricEvent::new("mcts", timer.elapsed_ms(), success).with_operation(operation),
        );

        response
    }

    pub(super) async fn handle_counterfactual(
        &self,
        req: CounterfactualRequest,
    ) -> CounterfactualResponse {
        use crate::modes::CounterfactualMode;

        let timer = Timer::start();
        let mode = CounterfactualMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        // Build content from scenario and intervention
        let content = format!(
            "Scenario: {}\nIntervention: {}",
            req.scenario, req.intervention
        );

        // Map analysis_depth to ladder rung
        let depth = req.analysis_depth.as_deref().unwrap_or("counterfactual");

        // Create progress reporter (use progress_token or generate one)
        let progress_token = req
            .progress_token
            .unwrap_or_else(|| format!("counterfactual-{}", uuid::Uuid::new_v4()));
        let progress = self.state.create_progress_reporter(&progress_token);

        tracing::info!(
            tool = "reasoning_counterfactual",
            progress_token = %progress_token,
            "Tool invocation started (streaming)"
        );

        // Apply tool-level timeout (MAXIMUM_THINKING = 16384 tokens)
        let timeout_ms = self
            .state
            .config
            .timeout_for_thinking_budget(MAXIMUM_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);

        let result = match tokio::time::timeout(
            timeout_duration,
            mode.analyze_streaming(&content, req.session_id.clone(), Some(&progress)),
        )
        .await
        {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                tracing::error!(
                    tool = "reasoning_counterfactual",
                    timeout_ms = timeout_ms,
                    "Tool execution timed out"
                );
                Err(ModeError::Timeout {
                    elapsed_ms: timeout_ms,
                })
            }
        };
        let success = result.is_ok();

        let response = match result {
            Ok(resp) => {
                // Build causal chain from edges
                let causal_chain: Vec<CausalStep> = resp
                    .causal_model
                    .edges
                    .iter()
                    .enumerate()
                    .map(|(i, e)| CausalStep {
                        step: i as u32 + 1,
                        cause: e.from.clone(),
                        effect: e.to.clone(),
                        probability: resp.analysis.counterfactual_level.confidence,
                    })
                    .collect();

                CounterfactualResponse {
                    counterfactual_outcome: resp.analysis.counterfactual_level.outcome,
                    causal_chain,
                    session_id: Some(resp.session_id),
                    original_scenario: req.scenario,
                    intervention_applied: req.intervention,
                    analysis_depth: depth.to_string(),
                    key_differences: resp.conclusions.caveats,
                    confidence: resp.analysis.counterfactual_level.confidence,
                    assumptions: resp.causal_model.confounders,
                    metadata: None,
                }
            }
            Err(e) => CounterfactualResponse {
                counterfactual_outcome: format!(
                    "counterfactual failed: {e}. \
                     Provide a scenario and intervention to analyze. \
                     Use depth='counterfactual' for basic what-if, or 'interventional'/'causal' for deeper analysis."
                ),
                causal_chain: vec![],
                session_id: req.session_id,
                original_scenario: req.scenario,
                intervention_applied: req.intervention,
                analysis_depth: depth.to_string(),
                key_differences: vec![],
                confidence: 0.0,
                assumptions: vec![],
                metadata: None,
            },
        };

        self.state.metrics.record(MetricEvent::new(
            "counterfactual",
            timer.elapsed_ms(),
            success,
        ));

        response
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod mcts_verify_tests {
    use super::{verify_backtrack, verify_explore};
    use crate::modes::{FrontierNode, QualityAssessment, QualityTrend, SelectedNode};

    fn node(id: &str, avg: f64, bonus: f64, ucb: f64, visits: u32) -> FrontierNode {
        FrontierNode {
            node_id: id.to_string(),
            visits,
            average_value: avg,
            ucb1_score: ucb,
            exploration_bonus: bonus,
        }
    }

    fn selected(id: &str) -> SelectedNode {
        SelectedNode {
            node_id: id.to_string(),
            selection_reason: "r".to_string(),
        }
    }

    #[test]
    fn test_explore_consistent_and_argmax() {
        let frontier = vec![node("a", 0.6, 0.2, 0.8, 8), node("b", 0.4, 0.3, 0.7, 3)];
        let (v, best) = verify_explore(&frontier, &selected("a"));
        assert!(v.consistent, "warnings: {:?}", v.warnings);
        assert_eq!(best.as_deref(), Some("a"));
    }

    #[test]
    fn test_explore_flags_bad_ucb1_decomposition() {
        // 0.6 + 0.2 = 0.8, not the stated 0.95.
        let frontier = vec![node("a", 0.6, 0.2, 0.95, 8)];
        let (v, _) = verify_explore(&frontier, &selected("a"));
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("UCB1 stated")));
    }

    #[test]
    fn test_explore_flags_non_argmax_selection() {
        let frontier = vec![node("a", 0.6, 0.2, 0.8, 8), node("b", 0.5, 0.4, 0.9, 2)];
        // 'b' has the higher UCB1 (0.9) but 'a' was selected.
        let (v, best) = verify_explore(&frontier, &selected("a"));
        assert!(!v.consistent);
        assert_eq!(best.as_deref(), Some("b"));
        assert!(v.warnings.iter().any(|w| w.contains("highest-UCB1")));
    }

    #[test]
    fn test_explore_flags_selected_not_in_frontier() {
        let frontier = vec![node("a", 0.6, 0.2, 0.8, 8)];
        let (v, _) = verify_explore(&frontier, &selected("ghost"));
        assert!(!v.consistent);
        assert!(v
            .warnings
            .iter()
            .any(|w| w.contains("not present in the frontier")));
    }

    #[test]
    fn test_backtrack_consistent_declining() {
        let qa = QualityAssessment {
            recent_values: vec![0.7, 0.65, 0.5, 0.4],
            trend: QualityTrend::Declining,
            decline_magnitude: 0.3,
        };
        let v = verify_backtrack(&qa);
        assert!(v.consistent, "warnings: {:?}", v.warnings);
    }

    #[test]
    fn test_backtrack_flags_trend_mismatch() {
        // Values clearly decline but trend claims improving.
        let qa = QualityAssessment {
            recent_values: vec![0.8, 0.6, 0.4],
            trend: QualityTrend::Improving,
            decline_magnitude: 0.4,
        };
        let v = verify_backtrack(&qa);
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("Trend stated")));
    }

    #[test]
    fn test_backtrack_flags_impossible_decline() {
        let qa = QualityAssessment {
            recent_values: vec![0.6, 0.55, 0.5],
            trend: QualityTrend::Declining,
            decline_magnitude: 0.9, // range is only 0.1
        };
        let v = verify_backtrack(&qa);
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("peak-to-trough")));
    }
}
