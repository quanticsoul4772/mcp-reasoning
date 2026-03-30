use std::sync::Arc;
use std::time::Duration;

use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::{MctsMode, TimelineMode};
use crate::server::requests::{CounterfactualRequest, MctsRequest, TimelineRequest};
use crate::server::responses::{
    BacktrackSuggestion, BranchComparison, CausalStep, CounterfactualResponse, MctsNode,
    MctsResponse, TimelineBranch, TimelineResponse,
};

use super::{DEEP_THINKING, MAXIMUM_THINKING};

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
                    Ok(resp) => (
                        MctsResponse {
                            session_id: resp.session_id,
                            best_path: Some(
                                resp.frontier_evaluation
                                    .into_iter()
                                    .map(|n| MctsNode {
                                        node_id: n.node_id,
                                        content: format!("UCB1: {:.3}", n.ucb1_score),
                                        ucb_score: n.ucb1_score,
                                        visits: n.visits,
                                    })
                                    .collect(),
                            ),
                            iterations_completed: Some(resp.search_status.total_simulations),
                            backtrack_suggestion: None,
                            executed: None,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(_) => (
                        MctsResponse {
                            session_id: input_session_id.clone(),
                            best_path: None,
                            iterations_completed: None,
                            backtrack_suggestion: None,
                            executed: None,
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
                    Ok(resp) => (
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
                            metadata: None,
                        },
                        true,
                    ),
                    Err(_) => (
                        MctsResponse {
                            session_id: input_session_id.clone(),
                            best_path: None,
                            iterations_completed: None,
                            backtrack_suggestion: None,
                            executed: None,
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
