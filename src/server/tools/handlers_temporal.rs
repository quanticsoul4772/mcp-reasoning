use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::TimelineBranch as ModeTimelineBranch;
use crate::modes::{
    CausalAnalysis, CausalEdge, CausalModel, CausalQuestion, CommonPattern, FrontierNode, MctsMode,
    QualityAssessment, QualityTrend, RobustStrategy, SelectedNode, TemporalStructure,
    TimelineEvent, TimelineMode,
};
use crate::server::requests::{CounterfactualRequest, MctsRequest, TimelineRequest};
use crate::server::responses::{
    AssociationInfo, BacktrackSuggestion, BranchComparison, BranchDifferenceInfo, BranchEventInfo,
    BranchInfo, CausalEdgeInfo, CausalModelInfo, CausalStep, CommonPatternInfo,
    CompareRecommendationInfo, CounterfactualResponse, CounterfactualValidationInfo,
    DecisionPointInfo, FragileStrategyInfo, InterventionInfo, MctsAlternative, MctsBackpropagation,
    MctsExpandedNode, MctsFrontierNode, MctsNode, MctsRecommendation, MctsResponse,
    MctsSelectedNode, MctsValidationInfo, OpportunityAssessmentInfo, RiskAssessmentInfo,
    RobustStrategyInfo, TemporalStructureInfo, TimelineBranch, TimelineEventInfo, TimelineResponse,
    TimelineValidationInfo,
};

/// Validate a created timeline: event causes/effects and the temporal structure
/// must reference declared events, and the event causal graph must be acyclic.
fn verify_create(events: &[TimelineEvent], ts: &TemporalStructure) -> TimelineValidationInfo {
    let mut warnings = Vec::new();
    let ids: HashSet<&str> = events.iter().map(|e| e.id.as_str()).collect();

    // Only `causes` (backward references) must resolve to declared events.
    // `effects` are forward-looking and routinely name downstream or terminal
    // outcomes the model didn't elaborate into full event nodes, so requiring
    // them to be declared produces noise rather than catching real errors.
    for e in events {
        for c in &e.causes {
            if !ids.contains(c.as_str()) {
                warnings.push(format!(
                    "Event '{}' lists cause '{c}' which is not a declared event",
                    e.id
                ));
            }
        }
    }
    if !ts.start.is_empty() && !ids.contains(ts.start.as_str()) {
        warnings.push(format!(
            "temporal_structure.start '{}' is not a declared event",
            ts.start
        ));
    }
    if !ts.current.is_empty() && !ids.contains(ts.current.as_str()) {
        warnings.push(format!(
            "temporal_structure.current '{}' is not a declared event",
            ts.current
        ));
    }

    // Build the causal graph (cause → event) and check it is a DAG.
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in events {
        for c in &e.causes {
            adj.entry(c.as_str()).or_default().push(e.id.as_str());
        }
    }
    let mut color: HashMap<&str, u8> = HashMap::new();
    let nodes: Vec<&str> = events.iter().map(|e| e.id.as_str()).collect();
    for &n in &nodes {
        if color.get(n).copied().unwrap_or(0) == 0 && cycle_visit(n, &adj, &mut color) {
            warnings.push(
                "Timeline events contain a causal cycle; the graph must be acyclic".to_string(),
            );
            break;
        }
    }

    TimelineValidationInfo {
        consistent: warnings.is_empty(),
        warnings,
    }
}

/// Validate branch value ranges (plausibility, outcome quality, event probabilities).
fn verify_branch(branches: &[ModeTimelineBranch]) -> TimelineValidationInfo {
    let mut warnings = Vec::new();
    for b in branches {
        if !(0.0..=1.0).contains(&b.plausibility) {
            warnings.push(format!(
                "Branch '{}' plausibility {:.3} is outside [0, 1]",
                b.id, b.plausibility
            ));
        }
        if !(0.0..=1.0).contains(&b.outcome_quality) {
            warnings.push(format!(
                "Branch '{}' outcome_quality {:.3} is outside [0, 1]",
                b.id, b.outcome_quality
            ));
        }
        for ev in &b.events {
            if !(0.0..=1.0).contains(&ev.probability) {
                warnings.push(format!(
                    "Branch '{}' event '{}' probability {:.3} is outside [0, 1]",
                    b.id, ev.id, ev.probability
                ));
            }
        }
    }
    TimelineValidationInfo {
        consistent: warnings.is_empty(),
        warnings,
    }
}

/// Validate that a comparison's preferred branch names one of the compared branches.
fn verify_compare(branches_compared: &[String], preferred_branch: &str) -> TimelineValidationInfo {
    let mut warnings = Vec::new();
    // Allow non-branch verdicts like "depends"/"neither"/"both".
    let is_verdict = matches!(
        preferred_branch.to_lowercase().as_str(),
        "depends" | "neither" | "both" | "either" | ""
    );
    if !is_verdict && !branches_compared.iter().any(|b| b == preferred_branch) {
        warnings.push(format!(
            "Recommended branch '{preferred_branch}' is not among the compared branches"
        ));
    }
    TimelineValidationInfo {
        consistent: warnings.is_empty(),
        warnings,
    }
}

/// Validate merge value ranges (pattern frequency, strategy effectiveness).
fn verify_merge(
    patterns: &[CommonPattern],
    strategies: &[RobustStrategy],
) -> TimelineValidationInfo {
    let mut warnings = Vec::new();
    for p in patterns {
        if !(0.0..=1.0).contains(&p.frequency) {
            warnings.push(format!(
                "Pattern frequency {:.3} is outside [0, 1]",
                p.frequency
            ));
        }
    }
    for s in strategies {
        if !(0.0..=1.0).contains(&s.effectiveness) {
            warnings.push(format!(
                "Strategy effectiveness {:.3} is outside [0, 1]",
                s.effectiveness
            ));
        }
    }
    TimelineValidationInfo {
        consistent: warnings.is_empty(),
        warnings,
    }
}

/// Three-color DFS helper for cycle detection: white (absent), gray (1, on
/// stack), black (2, done). Returns true when a back-edge (cycle) is found.
fn cycle_visit<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    color: &mut HashMap<&'a str, u8>,
) -> bool {
    color.insert(node, 1);
    if let Some(neighbors) = adj.get(node) {
        for &m in neighbors {
            match color.get(m).copied().unwrap_or(0) {
                1 => return true,
                0 if cycle_visit(m, adj, color) => return true,
                _ => {}
            }
        }
    }
    color.insert(node, 2);
    false
}

/// Detect a directed cycle in the causal edges (a causal model must be a DAG).
fn causal_model_has_cycle(edges: &[CausalEdge]) -> bool {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut all: HashSet<&str> = HashSet::new();
    for e in edges {
        adj.entry(e.from.as_str()).or_default().push(e.to.as_str());
        all.insert(e.from.as_str());
        all.insert(e.to.as_str());
    }

    let mut color: HashMap<&str, u8> = HashMap::new();
    for &n in &all {
        if color.get(n).copied().unwrap_or(0) == 0 && cycle_visit(n, &adj, &mut color) {
            return true;
        }
    }
    false
}

/// Validate the causal model for structural consistency (declared nodes, the
/// cause/effect present, confounders connecting both, acyclicity) and value ranges.
fn verify_causal_model(
    model: &CausalModel,
    question: &CausalQuestion,
    analysis: &CausalAnalysis,
) -> CounterfactualValidationInfo {
    let mut warnings = Vec::new();
    // Names are matched case/whitespace-insensitively: models routinely vary
    // capitalization between the question variables and the DAG node labels
    // ("Average order value" vs "Average Order Value"), which is not a real
    // structural inconsistency.
    let norm = |s: &str| s.trim().to_lowercase();
    let nodes: HashSet<String> = model.nodes.iter().map(|n| norm(n)).collect();

    for e in &model.edges {
        if !nodes.contains(&norm(&e.from)) {
            warnings.push(format!("Edge source '{}' is not a declared node", e.from));
        }
        if !nodes.contains(&norm(&e.to)) {
            warnings.push(format!("Edge target '{}' is not a declared node", e.to));
        }
    }
    for c in &model.confounders {
        if !nodes.contains(&norm(c)) {
            warnings.push(format!("Confounder '{c}' is not a declared node"));
        }
    }

    let cause = &question.variables.cause;
    let effect = &question.variables.effect;
    let cause_n = norm(cause);
    let effect_n = norm(effect);
    if !nodes.contains(&cause_n) {
        warnings.push(format!("Cause '{cause}' is absent from the causal model"));
    }
    if !nodes.contains(&effect_n) {
        warnings.push(format!("Effect '{effect}' is absent from the causal model"));
    }

    // A confounder, by definition, influences both the cause and the effect.
    for c in &model.confounders {
        let to_cause = model
            .edges
            .iter()
            .any(|e| norm(&e.from) == norm(c) && norm(&e.to) == cause_n);
        let to_effect = model
            .edges
            .iter()
            .any(|e| norm(&e.from) == norm(c) && norm(&e.to) == effect_n);
        if !(to_cause && to_effect) {
            warnings.push(format!(
                "Confounder '{c}' should have edges to both the cause and the effect"
            ));
        }
    }

    if causal_model_has_cycle(&model.edges) {
        warnings.push("Causal model contains a cycle; a causal DAG must be acyclic".to_string());
    }

    let corr = analysis.association_level.observed_correlation;
    if !(-1.0..=1.0).contains(&corr) {
        warnings.push(format!("observed_correlation {corr:.3} is outside [-1, 1]"));
    }
    let conf = analysis.counterfactual_level.confidence;
    if !(0.0..=1.0).contains(&conf) {
        warnings.push(format!(
            "counterfactual confidence {conf:.3} is outside [0, 1]"
        ));
    }

    CounterfactualValidationInfo {
        consistent: warnings.is_empty(),
        warnings,
    }
}

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
                Ok(resp) => {
                    let validation = verify_create(&resp.events, &resp.temporal_structure);
                    let events = resp
                        .events
                        .iter()
                        .map(|e| TimelineEventInfo {
                            id: e.id.clone(),
                            description: e.description.clone(),
                            time: e.time.clone(),
                            event_type: enum_to_string(&e.event_type),
                            causes: e.causes.clone(),
                            effects: e.effects.clone(),
                        })
                        .collect();
                    let decision_points = resp
                        .decision_points
                        .iter()
                        .map(|d| DecisionPointInfo {
                            id: d.id.clone(),
                            description: d.description.clone(),
                            options: d.options.clone(),
                            deadline: d.deadline.clone(),
                        })
                        .collect();
                    let temporal_structure = TemporalStructureInfo {
                        start: resp.temporal_structure.start.clone(),
                        current: resp.temporal_structure.current.clone(),
                        horizon: resp.temporal_structure.horizon.clone(),
                    };
                    (
                        TimelineResponse {
                            timeline_id: resp.timeline_id,
                            events: Some(events),
                            decision_points: Some(decision_points),
                            temporal_structure: Some(temporal_structure),
                            validation: Some(validation),
                            ..Default::default()
                        },
                        true,
                    )
                }
                Err(e) => (
                    TimelineResponse {
                        timeline_id: format!(
                            "timeline create failed: {e}. \
                             Provide non-empty content describing the scenario. \
                             Use operation='branch' once a timeline_id exists."
                        ),
                        ..Default::default()
                    },
                    false,
                ),
            },
            "branch" => match mode.branch(content, req.session_id).await {
                Ok(resp) => {
                    let validation = verify_branch(&resp.branches);
                    let branch_details = resp
                        .branches
                        .iter()
                        .map(|b| BranchInfo {
                            id: b.id.clone(),
                            choice: b.choice.clone(),
                            plausibility: b.plausibility,
                            outcome_quality: b.outcome_quality,
                            events: b
                                .events
                                .iter()
                                .map(|e| BranchEventInfo {
                                    id: e.id.clone(),
                                    description: e.description.clone(),
                                    probability: e.probability,
                                    time_offset: e.time_offset.clone(),
                                })
                                .collect(),
                        })
                        .collect();
                    let branch_ids = resp.branches.iter().map(|b| b.id.clone()).collect();
                    // Legacy `branches` kept for compatibility (events joined to a string).
                    let branches = resp
                        .branches
                        .iter()
                        .map(|b| TimelineBranch {
                            id: b.id.clone(),
                            label: Some(b.choice.clone()),
                            content: b
                                .events
                                .iter()
                                .map(|e| e.description.clone())
                                .collect::<Vec<_>>()
                                .join("; "),
                            created_at: String::new(),
                        })
                        .collect();
                    (
                        TimelineResponse {
                            branch_id: Some(resp.branch_point.event_id.clone()),
                            branches: Some(branches),
                            comparison: Some(BranchComparison {
                                divergence_points: vec![resp.branch_point.description],
                                quality_differences: serde_json::json!({
                                    "most_likely_good_outcome": resp.comparison.most_likely_good_outcome,
                                    "highest_risk": resp.comparison.highest_risk
                                }),
                                convergence_opportunities: resp.comparison.key_differences,
                            }),
                            branch_details: Some(branch_details),
                            branch_ids: Some(branch_ids),
                            validation: Some(validation),
                            ..Default::default()
                        },
                        true,
                    )
                }
                Err(e) => (
                    TimelineResponse {
                        timeline_id: format!(
                            "timeline branch failed: {e}. \
                             Provide a session_id from a previous create call. \
                             Use operation='create' first if no timeline exists yet."
                        ),
                        ..Default::default()
                    },
                    false,
                ),
            },
            "compare" => match mode.compare(content, req.session_id).await {
                Ok(resp) => {
                    let validation = verify_compare(
                        &resp.branches_compared,
                        &resp.recommendation.preferred_branch,
                    );
                    let differences = resp
                        .key_differences
                        .iter()
                        .map(|d| BranchDifferenceInfo {
                            dimension: d.dimension.clone(),
                            branch_1_value: d.branch_1_value.clone(),
                            branch_2_value: d.branch_2_value.clone(),
                            significance: d.significance.clone(),
                        })
                        .collect();
                    (
                        TimelineResponse {
                            divergence_point: Some(resp.divergence_point),
                            branch_ids: Some(resp.branches_compared),
                            differences: Some(differences),
                            risk_assessment: Some(RiskAssessmentInfo {
                                branch_1_risks: resp.risk_assessment.branch_1_risks,
                                branch_2_risks: resp.risk_assessment.branch_2_risks,
                            }),
                            opportunity_assessment: Some(OpportunityAssessmentInfo {
                                branch_1_opportunities: resp
                                    .opportunity_assessment
                                    .branch_1_opportunities,
                                branch_2_opportunities: resp
                                    .opportunity_assessment
                                    .branch_2_opportunities,
                            }),
                            recommendation: Some(CompareRecommendationInfo {
                                preferred_branch: resp.recommendation.preferred_branch,
                                conditions: resp.recommendation.conditions,
                                key_factors: resp.recommendation.key_factors,
                            }),
                            validation: Some(validation),
                            ..Default::default()
                        },
                        true,
                    )
                }
                Err(e) => (
                    TimelineResponse {
                        timeline_id: format!(
                            "timeline compare failed: {e}. \
                             Provide a session_id with at least 2 branches to compare. \
                             Use operation='branch' first to create divergent paths."
                        ),
                        ..Default::default()
                    },
                    false,
                ),
            },
            "merge" => match mode.merge(content, req.session_id).await {
                Ok(resp) => {
                    let validation = verify_merge(&resp.common_patterns, &resp.robust_strategies);
                    let common_patterns = resp
                        .common_patterns
                        .iter()
                        .map(|p| CommonPatternInfo {
                            pattern: p.pattern.clone(),
                            frequency: p.frequency,
                            implications: p.implications.clone(),
                        })
                        .collect();
                    let robust_strategies = resp
                        .robust_strategies
                        .iter()
                        .map(|s| RobustStrategyInfo {
                            strategy: s.strategy.clone(),
                            effectiveness: s.effectiveness,
                            conditions: s.conditions.clone(),
                        })
                        .collect();
                    let fragile_strategies = resp
                        .fragile_strategies
                        .iter()
                        .map(|s| FragileStrategyInfo {
                            strategy: s.strategy.clone(),
                            failure_modes: s.failure_modes.clone(),
                        })
                        .collect();
                    (
                        TimelineResponse {
                            merged_content: Some(format!(
                                "Synthesis: {}. Recommendations: {}",
                                resp.synthesis,
                                resp.recommendations.join("; ")
                            )),
                            common_patterns: Some(common_patterns),
                            robust_strategies: Some(robust_strategies),
                            fragile_strategies: Some(fragile_strategies),
                            synthesis: Some(resp.synthesis),
                            recommendations: Some(resp.recommendations),
                            branch_ids: Some(resp.branches_merged),
                            validation: Some(validation),
                            ..Default::default()
                        },
                        true,
                    )
                }
                Err(e) => (
                    TimelineResponse {
                        timeline_id: format!(
                            "timeline merge failed: {e}. \
                             Provide a session_id with branches to synthesize. \
                             Use operation='compare' first to identify divergence points."
                        ),
                        ..Default::default()
                    },
                    false,
                ),
            },
            _ => (
                TimelineResponse {
                    timeline_id: format!("Unknown operation: {}", op_for_timeout),
                    ..Default::default()
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
                        ..Default::default()
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
                let validation = verify_causal_model(
                    &resp.causal_model,
                    &resp.causal_question,
                    &resp.analysis,
                );
                // Legacy causal_chain (kept for compatibility); the typed edges
                // live in `causal_model` below.
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
                let causal_model = CausalModelInfo {
                    nodes: resp.causal_model.nodes.clone(),
                    edges: resp
                        .causal_model
                        .edges
                        .iter()
                        .map(|e| CausalEdgeInfo {
                            from: e.from.clone(),
                            to: e.to.clone(),
                            edge_type: enum_to_string(&e.edge_type),
                        })
                        .collect(),
                    confounders: resp.causal_model.confounders.clone(),
                };
                let association = AssociationInfo {
                    observed_correlation: resp.analysis.association_level.observed_correlation,
                    interpretation: resp.analysis.association_level.interpretation.clone(),
                };
                let intervention = InterventionInfo {
                    causal_effect: resp.analysis.intervention_level.causal_effect,
                    mechanism: resp.analysis.intervention_level.mechanism.clone(),
                };

                CounterfactualResponse {
                    counterfactual_outcome: resp.analysis.counterfactual_level.outcome.clone(),
                    causal_chain,
                    session_id: Some(resp.session_id),
                    original_scenario: req.scenario,
                    intervention_applied: req.intervention,
                    analysis_depth: depth.to_string(),
                    key_differences: resp.conclusions.caveats.clone(),
                    confidence: resp.analysis.counterfactual_level.confidence,
                    assumptions: resp.causal_model.confounders.clone(),
                    ladder_rung: Some(enum_to_string(&resp.causal_question.ladder_rung)),
                    association: Some(association),
                    intervention: Some(intervention),
                    counterfactual_scenario: Some(
                        resp.analysis.counterfactual_level.scenario.clone(),
                    ),
                    causal_model: Some(causal_model),
                    causal_claim: Some(resp.conclusions.causal_claim.clone()),
                    causal_strength: Some(enum_to_string(&resp.conclusions.strength)),
                    actionable_insight: Some(resp.conclusions.actionable_insight.clone()),
                    validation: Some(validation),
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
                ladder_rung: None,
                association: None,
                intervention: None,
                counterfactual_scenario: None,
                causal_model: None,
                causal_claim: None,
                causal_strength: None,
                actionable_insight: None,
                validation: None,
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod counterfactual_verify_tests {
    use super::{causal_model_has_cycle, verify_causal_model};
    use crate::modes::{
        AssociationLevel, CausalAnalysis, CausalEdge, CausalModel, CausalQuestion, CausalVariables,
        CounterfactualLevel, EdgeType, InterventionLevel, LadderRung,
    };

    fn edge(from: &str, to: &str, t: EdgeType) -> CausalEdge {
        CausalEdge {
            from: from.to_string(),
            to: to.to_string(),
            edge_type: t,
        }
    }

    fn question(cause: &str, effect: &str) -> CausalQuestion {
        CausalQuestion {
            statement: "q".to_string(),
            ladder_rung: LadderRung::Counterfactual,
            variables: CausalVariables {
                cause: cause.to_string(),
                effect: effect.to_string(),
                intervention: "do".to_string(),
            },
        }
    }

    fn analysis(corr: f64, conf: f64) -> CausalAnalysis {
        CausalAnalysis {
            association_level: AssociationLevel {
                observed_correlation: corr,
                interpretation: "i".to_string(),
            },
            intervention_level: InterventionLevel {
                causal_effect: 0.3,
                mechanism: "m".to_string(),
            },
            counterfactual_level: CounterfactualLevel {
                scenario: "s".to_string(),
                outcome: "o".to_string(),
                confidence: conf,
            },
        }
    }

    #[test]
    fn test_consistent_model() {
        let model = CausalModel {
            nodes: vec!["X".to_string(), "Y".to_string(), "Z".to_string()],
            edges: vec![
                edge("X", "Y", EdgeType::Direct),
                edge("Z", "X", EdgeType::Confounded),
                edge("Z", "Y", EdgeType::Confounded),
            ],
            confounders: vec!["Z".to_string()],
        };
        let v = verify_causal_model(&model, &question("X", "Y"), &analysis(0.6, 0.7));
        assert!(v.consistent, "warnings: {:?}", v.warnings);
    }

    #[test]
    fn test_flags_undeclared_edge_node() {
        let model = CausalModel {
            nodes: vec!["X".to_string(), "Y".to_string()],
            edges: vec![edge("X", "GHOST", EdgeType::Direct)],
            confounders: vec![],
        };
        let v = verify_causal_model(&model, &question("X", "Y"), &analysis(0.5, 0.7));
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("not a declared node")));
    }

    #[test]
    fn test_flags_missing_cause() {
        let model = CausalModel {
            nodes: vec!["Y".to_string()],
            edges: vec![],
            confounders: vec![],
        };
        let v = verify_causal_model(&model, &question("X", "Y"), &analysis(0.5, 0.7));
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("Cause 'X' is absent")));
    }

    #[test]
    fn test_case_only_name_differences_are_not_flagged() {
        // The question's cause/effect differ from the DAG node labels only in
        // capitalization/whitespace — not a real structural inconsistency.
        let model = CausalModel {
            nodes: vec!["Average Order Value".to_string(), "Widget".to_string()],
            edges: vec![edge("Widget", "Average Order Value", EdgeType::Direct)],
            confounders: vec![],
        };
        let q = question("widget", "average order value");
        let v = verify_causal_model(&model, &q, &analysis(0.5, 0.7));
        assert!(v.consistent, "warnings: {:?}", v.warnings);
    }

    #[test]
    fn test_flags_confounder_not_connecting_both() {
        let model = CausalModel {
            nodes: vec!["X".to_string(), "Y".to_string(), "Z".to_string()],
            // Z points only to X, not to Y → not a true confounder.
            edges: vec![
                edge("X", "Y", EdgeType::Direct),
                edge("Z", "X", EdgeType::Direct),
            ],
            confounders: vec!["Z".to_string()],
        };
        let v = verify_causal_model(&model, &question("X", "Y"), &analysis(0.5, 0.7));
        assert!(!v.consistent);
        assert!(v
            .warnings
            .iter()
            .any(|w| w.contains("both the cause and the effect")));
    }

    #[test]
    fn test_flags_out_of_range_correlation() {
        let model = CausalModel {
            nodes: vec!["X".to_string(), "Y".to_string()],
            edges: vec![edge("X", "Y", EdgeType::Direct)],
            confounders: vec![],
        };
        let v = verify_causal_model(&model, &question("X", "Y"), &analysis(1.8, 0.7));
        assert!(!v.consistent);
        assert!(v
            .warnings
            .iter()
            .any(|w| w.contains("observed_correlation")));
    }

    #[test]
    fn test_cycle_detection() {
        let acyclic = vec![
            edge("A", "B", EdgeType::Direct),
            edge("B", "C", EdgeType::Direct),
        ];
        assert!(!causal_model_has_cycle(&acyclic));
        let cyclic = vec![
            edge("A", "B", EdgeType::Direct),
            edge("B", "C", EdgeType::Direct),
            edge("C", "A", EdgeType::Direct),
        ];
        assert!(causal_model_has_cycle(&cyclic));
    }

    #[test]
    fn test_verify_flags_cycle() {
        let model = CausalModel {
            nodes: vec!["X".to_string(), "Y".to_string()],
            edges: vec![
                edge("X", "Y", EdgeType::Direct),
                edge("Y", "X", EdgeType::Direct),
            ],
            confounders: vec![],
        };
        let v = verify_causal_model(&model, &question("X", "Y"), &analysis(0.5, 0.7));
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("acyclic")));
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod timeline_verify_tests {
    use super::{verify_branch, verify_compare, verify_create, verify_merge};
    use crate::modes::{
        BranchEvent, CommonPattern, EventType, RobustStrategy, TemporalStructure, TimelineBranch,
        TimelineEvent,
    };

    fn event(id: &str, causes: &[&str], effects: &[&str]) -> TimelineEvent {
        TimelineEvent {
            id: id.to_string(),
            description: "d".to_string(),
            time: "t".to_string(),
            event_type: EventType::Event,
            causes: causes.iter().map(|s| s.to_string()).collect(),
            effects: effects.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn ts(start: &str, current: &str) -> TemporalStructure {
        TemporalStructure {
            start: start.to_string(),
            current: current.to_string(),
            horizon: "h".to_string(),
        }
    }

    #[test]
    fn test_create_consistent() {
        let events = vec![event("a", &[], &["b"]), event("b", &["a"], &[])];
        let v = verify_create(&events, &ts("a", "b"));
        assert!(v.consistent, "warnings: {:?}", v.warnings);
    }

    #[test]
    fn test_create_flags_undeclared_reference() {
        // An undeclared *cause* (backward reference) is a real inconsistency.
        let events = vec![event("a", &["ghost"], &[])];
        let v = verify_create(&events, &ts("a", "a"));
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("cause 'ghost'")));
    }

    #[test]
    fn test_create_tolerates_undeclared_effect() {
        // An effect naming a downstream/terminal outcome that isn't a declared
        // event is normal modeling, not an error.
        let events = vec![event("a", &[], &["business_closure", "market_exit"])];
        let v = verify_create(&events, &ts("a", "a"));
        assert!(v.consistent, "warnings: {:?}", v.warnings);
    }

    #[test]
    fn test_create_flags_bad_temporal_ref() {
        let events = vec![event("a", &[], &[])];
        let v = verify_create(&events, &ts("a", "nope"));
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("current 'nope'")));
    }

    #[test]
    fn test_create_flags_cycle() {
        let events = vec![event("a", &["b"], &["b"]), event("b", &["a"], &["a"])];
        let v = verify_create(&events, &ts("a", "b"));
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("causal cycle")));
    }

    fn branch(id: &str, plaus: f64, quality: f64, prob: f64) -> TimelineBranch {
        TimelineBranch {
            id: id.to_string(),
            choice: "c".to_string(),
            events: vec![BranchEvent {
                id: "ev".to_string(),
                description: "d".to_string(),
                probability: prob,
                time_offset: "+1".to_string(),
            }],
            plausibility: plaus,
            outcome_quality: quality,
        }
    }

    #[test]
    fn test_branch_ranges_ok_and_flagged() {
        let ok = verify_branch(&[branch("b1", 0.8, 0.7, 0.9)]);
        assert!(ok.consistent);
        let bad = verify_branch(&[branch("b1", 1.5, 0.7, 0.9)]);
        assert!(!bad.consistent);
        assert!(bad.warnings.iter().any(|w| w.contains("plausibility")));
        let bad_prob = verify_branch(&[branch("b1", 0.8, 0.7, 1.4)]);
        assert!(bad_prob.warnings.iter().any(|w| w.contains("probability")));
    }

    #[test]
    fn test_compare_membership() {
        let ok = verify_compare(&["b1".to_string(), "b2".to_string()], "b1");
        assert!(ok.consistent);
        let verdict = verify_compare(&["b1".to_string()], "depends");
        assert!(verdict.consistent);
        let bad = verify_compare(&["b1".to_string(), "b2".to_string()], "b9");
        assert!(!bad.consistent);
        assert!(bad
            .warnings
            .iter()
            .any(|w| w.contains("not among the compared")));
    }

    #[test]
    fn test_merge_ranges() {
        let pattern = |f: f64| CommonPattern {
            pattern: "p".to_string(),
            frequency: f,
            implications: "i".to_string(),
        };
        let strat = |e: f64| RobustStrategy {
            strategy: "s".to_string(),
            effectiveness: e,
            conditions: "c".to_string(),
        };
        assert!(verify_merge(&[pattern(0.8)], &[strat(0.9)]).consistent);
        let bad = verify_merge(&[pattern(1.9)], &[strat(0.9)]);
        assert!(!bad.consistent);
        assert!(bad.warnings.iter().any(|w| w.contains("frequency")));
        assert!(!verify_merge(&[pattern(0.8)], &[strat(2.0)]).consistent);
    }
}
