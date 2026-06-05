use std::sync::Arc;
use std::time::Duration;

use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::{DetectMode, GraphMode};
use crate::server::metadata_builders;
use crate::server::requests::{DetectRequest, GraphRequest};
use crate::server::responses::{
    ArgumentStructureInfo, DetectResponse, DetectValidationInfo, Detection, GraphNode,
    GraphResponse, GraphState, GraphValidationInfo,
};

use super::{DEEP_THINKING, STANDARD_THINKING};

/// Normalize text for quote-grounding comparison: collapse whitespace, lowercase.
fn normalize_for_grounding(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Length of the longest run of words present contiguously in both slices.
fn longest_common_word_run(a: &[&str], b: &[&str]) -> usize {
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    let mut prev = vec![0usize; b.len() + 1];
    let mut best = 0;
    for ai in a {
        let mut cur = vec![0usize; b.len() + 1];
        for (j, bj) in b.iter().enumerate() {
            if ai == bj {
                cur[j + 1] = prev[j] + 1;
                best = best.max(cur[j + 1]);
            }
        }
        prev = cur;
    }
    best
}

/// True when the cited `quote` is grounded in `content`.
///
/// Models rarely quote verbatim — they embed the actual passage inside longer
/// commentary (e.g. `"…never let me down - showing a preference for…"`). So
/// rather than demand an exact substring, this accepts the quote when a
/// substantial contiguous run of its words appears in the content: a run of ≥5
/// words, or ≥half the quote (with a 3-word floor). An empty quote is grounded.
fn quote_is_grounded(content_normalized: &str, quote: &str) -> bool {
    let q = normalize_for_grounding(quote);
    if q.is_empty() || content_normalized.contains(&q) {
        return true;
    }
    let q_words: Vec<&str> = q.split(' ').collect();
    let c_words: Vec<&str> = content_normalized.split(' ').collect();
    let run = longest_common_word_run(&q_words, &c_words);
    run >= 5 || (run >= 3 && run * 2 >= q_words.len())
}

/// Build a detect validation from a reported count vs. the actual detections
/// and the names of any detections whose quote was not found in the content.
fn build_detect_validation(
    reported_count: u32,
    actual_count: usize,
    ungrounded: &[String],
) -> DetectValidationInfo {
    let mut warnings = Vec::new();
    if reported_count as usize != actual_count {
        warnings.push(format!(
            "Reported count ({reported_count}) differs from the {actual_count} detection(s) returned"
        ));
    }
    for name in ungrounded {
        warnings.push(format!(
            "Evidence for '{name}' could not be located in the content (may be paraphrased)"
        ));
    }
    DetectValidationInfo {
        consistent: warnings.is_empty(),
        warnings,
    }
}

/// Verify that every generated node's quality score sits in the [0, 1] range
/// the graph prompts specify. Nodes without a score are skipped.
fn verify_graph_generate(nodes: &[GraphNode]) -> GraphValidationInfo {
    let mut warnings = Vec::new();
    for n in nodes {
        if let Some(s) = n.score {
            if !(0.0..=1.0).contains(&s) {
                warnings.push(format!(
                    "Node '{}' score {s:.3} is outside the [0, 1] range",
                    n.id
                ));
            }
        }
    }
    GraphValidationInfo {
        consistent: warnings.is_empty(),
        warnings,
    }
}

/// Verify the reported graph state counts reconcile: neither the pruned nor the
/// active count may exceed the total.
fn verify_graph_state(state: &GraphState) -> GraphValidationInfo {
    let mut warnings = Vec::new();
    if state.pruned_count > state.total_nodes {
        warnings.push(format!(
            "pruned_count {} exceeds total_nodes {}",
            state.pruned_count, state.total_nodes
        ));
    }
    if state.active_nodes > state.total_nodes {
        warnings.push(format!(
            "active_nodes {} exceeds total_nodes {}",
            state.active_nodes, state.total_nodes
        ));
    }
    GraphValidationInfo {
        consistent: warnings.is_empty(),
        warnings,
    }
}

impl super::ReasoningServer {
    pub(super) async fn handle_graph(&self, req: GraphRequest) -> GraphResponse {
        let timer = Timer::start();
        let operation = req.operation.clone();
        let mode = GraphMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let session_id = req.session_id;
        let content = req.content.as_deref().unwrap_or("");

        // Apply tool-level timeout to prevent indefinite hangs
        let timeout_ms = self
            .state
            .config
            .timeout_for_thinking_budget(STANDARD_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);
        let op_for_timeout = operation.clone();

        let result = match tokio::time::timeout(timeout_duration, async {
            match op_for_timeout.as_str() {
                "init" => {
                    let sid = session_id.clone();
                    mode.init(content, Some(session_id.clone()))
                        .await
                        .map(move |r| GraphResponse {
                            session_id: sid,
                            node_id: Some(r.root.id),
                            nodes: None,
                            aggregated_insight: None,
                            conclusions: None,
                            state: None,
                            validation: None,
                            persistence_warning: None,
                            metadata: None,
                        })
                }
                "generate" => {
                    let sid = session_id.clone();
                    let node_id = req.node_id.as_deref();
                    mode.generate(req.content.as_deref(), node_id, Some(session_id.clone()))
                        .await
                        .map(move |r| {
                            let persistence_warning = (r.persistence_failures > 0).then(|| {
                                format!(
                                    "{} graph write(s) did not persist — typically an edge whose \
                                     parent node was never saved. The generated nodes are returned, \
                                     but the stored graph is incomplete. Run init first (or generate \
                                     from a node that exists) so edges can reference a persisted parent.",
                                    r.persistence_failures
                                )
                            });
                            let nodes: Vec<GraphNode> = r
                                .children
                                .into_iter()
                                .map(|n| GraphNode {
                                    id: n.id,
                                    content: n.content,
                                    score: Some(n.score),
                                    depth: None,
                                    parent_id: None,
                                })
                                .collect();
                            let validation = Some(verify_graph_generate(&nodes));
                            GraphResponse {
                                session_id: sid,
                                node_id: None,
                                nodes: Some(nodes),
                                aggregated_insight: None,
                                conclusions: None,
                                state: None,
                                validation,
                                persistence_warning,
                                metadata: None,
                            }
                        })
                }
                "score" => {
                    let sid = session_id.clone();
                    let node_id = req.node_id.as_deref();
                    mode.score(req.content.as_deref(), node_id, Some(session_id.clone()))
                        .await
                        .map(move |r| GraphResponse {
                            session_id: sid,
                            node_id: Some(r.node_id),
                            nodes: None,
                            aggregated_insight: None,
                            conclusions: None,
                            state: None,
                            validation: None,
                            persistence_warning: None,
                            metadata: None,
                        })
                }
                "aggregate" => {
                    let sid = session_id.clone();
                    mode.aggregate(content, Some(session_id.clone()))
                        .await
                        .map(move |r| GraphResponse {
                            session_id: sid,
                            node_id: None,
                            nodes: None,
                            aggregated_insight: Some(r.synthesis.content),
                            conclusions: None,
                            state: None,
                            validation: None,
                            persistence_warning: None,
                            metadata: None,
                        })
                }
                "refine" => {
                    let sid = session_id.clone();
                    mode.refine(content, Some(session_id.clone()))
                        .await
                        .map(move |r| GraphResponse {
                            session_id: sid,
                            node_id: Some(r.refined_node.id),
                            nodes: None,
                            aggregated_insight: None,
                            conclusions: None,
                            state: None,
                            validation: None,
                            persistence_warning: None,
                            metadata: None,
                        })
                }
                "prune" => {
                    let sid = session_id.clone();
                    // Caller value wins; otherwise the tunable Config default.
                    let quality_floor = req
                        .threshold
                        .unwrap_or(self.state.config.graph_prune_threshold);
                    mode.prune(content, Some(session_id.clone()), quality_floor)
                        .await
                        .map(move |r| GraphResponse {
                            session_id: sid,
                            node_id: None,
                            nodes: None,
                            aggregated_insight: None,
                            conclusions: None,
                            state: Some(GraphState {
                                total_nodes: 0,
                                active_nodes: 0,
                                max_depth: 0,
                                pruned_count: r.prune_candidates.len() as u32,
                            }),
                            validation: None,
                            persistence_warning: None,
                            metadata: None,
                        })
                }
                "finalize" => {
                    let sid = session_id.clone();
                    mode.finalize(content, Some(session_id.clone()))
                        .await
                        .map(move |r| GraphResponse {
                            session_id: sid,
                            node_id: None,
                            nodes: None,
                            aggregated_insight: None,
                            conclusions: Some(
                                r.conclusions.into_iter().map(|c| c.conclusion).collect(),
                            ),
                            state: None,
                            validation: None,
                            persistence_warning: None,
                            metadata: None,
                        })
                }
                "state" => {
                    let sid = session_id.clone();
                    mode.state(req.content.as_deref(), &session_id)
                        .await
                        .map(move |r| {
                            // saturating_sub guards the count subtraction: a
                            // structure reporting pruned_count > total_nodes
                            // would otherwise underflow-panic in debug. The
                            // inconsistency is surfaced by verify_graph_state.
                            let state = GraphState {
                                total_nodes: r.structure.total_nodes,
                                active_nodes: r
                                    .structure
                                    .total_nodes
                                    .saturating_sub(r.structure.pruned_count),
                                max_depth: r.structure.depth,
                                pruned_count: r.structure.pruned_count,
                            };
                            let validation = Some(verify_graph_state(&state));
                            GraphResponse {
                                session_id: sid,
                                node_id: None,
                                nodes: None,
                                aggregated_insight: None,
                                conclusions: None,
                                state: Some(state),
                                validation,
                                persistence_warning: None,
                                metadata: None,
                            }
                        })
                }
                _ => Err(ModeError::InvalidOperation {
                    mode: "graph".to_string(),
                    operation: op_for_timeout.clone(),
                }),
            }
        })
        .await
        {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                tracing::error!(
                    tool = "reasoning_graph",
                    timeout_ms = timeout_ms,
                    operation = %operation,
                    "Tool execution timed out"
                );
                Err(ModeError::Timeout {
                    elapsed_ms: timeout_ms,
                })
            }
        };

        let elapsed_ms = timer.elapsed_ms();
        let success = result.is_ok();
        let graph_consistent = result
            .as_ref()
            .ok()
            .and_then(|r| r.validation.as_ref())
            .map(|v| v.consistent);
        self.state.metrics.record(
            MetricEvent::new("graph", elapsed_ms, success)
                .with_operation(&operation)
                .with_validation(graph_consistent),
        );
        self.state
            .metrics
            .record_tool_use(&session_id, "graph", success);

        let mut response = result.unwrap_or_else(|e| GraphResponse {
            session_id: session_id.clone(),
            node_id: None,
            nodes: None,
            aggregated_insight: Some(format!(
                "graph {operation} failed: {e}. \
                 Valid operations: init, generate, score, aggregate, refine, prune, finalize, state. \
                 Use operation='init' first if no session_id exists, then 'generate' to add nodes."
            )),
            conclusions: None,
            state: None,
            validation: None,
            persistence_warning: None,
            metadata: None,
        });

        // Add metadata on success
        if success {
            let num_nodes = response
                .nodes
                .as_ref()
                .map(std::vec::Vec::len)
                .or_else(|| response.conclusions.as_ref().map(std::vec::Vec::len))
                .unwrap_or(1);

            match metadata_builders::build_metadata_for_graph(
                &self.state.metadata_builder,
                content.len(),
                &operation,
                num_nodes,
                Some(session_id),
                elapsed_ms,
            )
            .await
            {
                Ok(metadata) => {
                    response.metadata = Some(metadata);
                }
                Err(e) => {
                    tracing::warn!(
                        tool = "reasoning_graph",
                        operation = %operation,
                        error = %e,
                        "Metadata enrichment failed, returning response without metadata"
                    );
                }
            }
        }

        response
    }

    pub(super) async fn handle_detect(&self, req: DetectRequest) -> DetectResponse {
        let timer = Timer::start();
        // Effective session id for tool-chain linking (the response carries none).
        let input_session_id = req.session_id.clone().unwrap_or_default();
        let mode = DetectMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let content = req.content.as_deref().unwrap_or("");
        let detect_type = req.detect_type.as_str();

        // Apply tool-level timeout to prevent indefinite hangs
        let timeout_ms = self.state.config.timeout_for_thinking_budget(DEEP_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);
        let detect_type_for_timeout = detect_type.to_string();

        let (response, success) = match tokio::time::timeout(timeout_duration, async {
            match detect_type_for_timeout.as_str() {
                "biases" => match mode.biases(content, req.session_id).await {
                    Ok(resp) => {
                        let content_norm = normalize_for_grounding(content);
                        let mut ungrounded = Vec::new();
                        let detections: Vec<Detection> = resp
                            .biases_detected
                            .into_iter()
                            .map(|b| {
                                let grounded = quote_is_grounded(&content_norm, &b.evidence);
                                if !grounded {
                                    ungrounded.push(b.bias.clone());
                                }
                                Detection {
                                    detection_type: b.bias,
                                    category: None, // Biases don't have categories
                                    severity: b.severity.as_str().to_string(),
                                    confidence: b.confidence,
                                    evidence: b.evidence,
                                    explanation: b.impact,
                                    remediation: Some(b.debiasing),
                                    changes_conclusion: Some(b.changes_conclusion),
                                    grounded: Some(grounded),
                                }
                            })
                            .collect();
                        let validation = build_detect_validation(
                            resp.overall_assessment.bias_count,
                            detections.len(),
                            &ungrounded,
                        );
                        let altering = resp.overall_assessment.conclusion_altering_biases.clone();
                        (
                            DetectResponse {
                                summary: Some(format!(
                                    "{} biases detected. Most severe: {}.",
                                    resp.overall_assessment.bias_count,
                                    resp.overall_assessment.most_severe
                                )),
                                overall_quality: Some(resp.overall_assessment.reasoning_quality),
                                debiased_version: Some(resp.debiased_version),
                                argument_structure: None,
                                unchallenged_assumptions: None,
                                conclusion_altering_biases: (!altering.is_empty()).then_some(altering),
                                validation: Some(validation),
                                detections,
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(e) => (
                        DetectResponse {
                            detections: vec![],
                            summary: Some(format!(
                                "bias detection failed: {e}. \
                                 Provide non-empty content or a valid thought_id from a prior reasoning session."
                            )),
                            overall_quality: None,
                            debiased_version: None,
                            argument_structure: None,
                            unchallenged_assumptions: None,
                            conclusion_altering_biases: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "fallacies" => match mode.fallacies(content, req.session_id).await {
                    Ok(resp) => {
                        let content_norm = normalize_for_grounding(content);
                        let mut ungrounded = Vec::new();
                        let detections: Vec<Detection> = resp
                            .fallacies_detected
                            .into_iter()
                            .map(|f| {
                                let grounded = quote_is_grounded(&content_norm, &f.passage);
                                if !grounded {
                                    ungrounded.push(f.fallacy.clone());
                                }
                                Detection {
                                    detection_type: f.fallacy,
                                    category: Some(f.category.as_str().to_string()),
                                    severity: f.severity.as_str().to_string(),
                                    confidence: f.confidence,
                                    evidence: f.passage,
                                    explanation: f.explanation,
                                    remediation: Some(f.correction),
                                    changes_conclusion: None,
                                    grounded: Some(grounded),
                                }
                            })
                            .collect();
                        let validation = build_detect_validation(
                            resp.overall_assessment.fallacy_count,
                            detections.len(),
                            &ungrounded,
                        );
                        let argument_structure = ArgumentStructureInfo {
                            premises: resp.argument_structure.premises,
                            conclusion: resp.argument_structure.conclusion,
                            validity: resp.argument_structure.validity.as_str().to_string(),
                        };
                        (
                            DetectResponse {
                                summary: Some(format!(
                                    "{} fallacies detected. Most critical: {}.",
                                    resp.overall_assessment.fallacy_count,
                                    resp.overall_assessment.most_critical,
                                )),
                                overall_quality: Some(resp.overall_assessment.argument_strength),
                                debiased_version: None,
                                argument_structure: Some(argument_structure),
                                unchallenged_assumptions: None,
                                conclusion_altering_biases: None,
                                validation: Some(validation),
                                detections,
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(e) => (
                        DetectResponse {
                            detections: vec![],
                            summary: Some(format!(
                                "fallacy detection failed: {e}. \
                                 Provide non-empty content or a valid thought_id. \
                                 Use detect_type='biases' to check cognitive biases instead."
                            )),
                            overall_quality: None,
                            debiased_version: None,
                            argument_structure: None,
                            unchallenged_assumptions: None,
                            conclusion_altering_biases: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "knowledge_gaps" => match mode.knowledge_gaps(content, req.session_id).await {
                    Ok(resp) => {
                        let detections: Vec<Detection> = resp
                            .gaps
                            .into_iter()
                            .map(|g| {
                                // Map would_change_conclusion → severity, tolerant of
                                // case/whitespace (a bare "Yes" must not fall to "low").
                                let severity = match g.would_change_conclusion.trim().to_lowercase().as_str() {
                                    "yes" => "high",
                                    "maybe" => "medium",
                                    _ => "low",
                                }
                                .to_string();
                                Detection {
                                    detection_type: g.gap,
                                    category: Some(g.category.as_str().to_string()),
                                    severity,
                                    confidence: g.confidence,
                                    evidence: g.investigation,
                                    explanation: g.impact,
                                    remediation: None,
                                    changes_conclusion: Some(g.would_change_conclusion),
                                    grounded: None, // gaps describe absent info; nothing to ground
                                }
                            })
                            .collect();
                        let validation = build_detect_validation(
                            resp.overall_assessment.gap_count,
                            detections.len(),
                            &[],
                        );
                        (
                            DetectResponse {
                                summary: Some(format!(
                                    "{} knowledge gaps detected. Most critical: {}. \
                                     Completeness: {:.0}%. {} unchallenged assumptions.",
                                    resp.overall_assessment.gap_count,
                                    resp.overall_assessment.most_critical,
                                    resp.overall_assessment.completeness_score * 100.0,
                                    resp.unchallenged_assumptions.len(),
                                )),
                                overall_quality: Some(resp.overall_assessment.completeness_score),
                                debiased_version: None,
                                argument_structure: None,
                                unchallenged_assumptions: Some(resp.unchallenged_assumptions),
                                conclusion_altering_biases: None,
                                validation: Some(validation),
                                detections,
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(e) => (
                        DetectResponse {
                            detections: vec![],
                            summary: Some(format!(
                                "knowledge gap detection failed: {e}. \
                                 Provide non-empty content or a valid thought_id."
                            )),
                            overall_quality: None,
                            debiased_version: None,
                            argument_structure: None,
                            unchallenged_assumptions: None,
                            conclusion_altering_biases: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                _ => (
                    DetectResponse {
                        detections: vec![],
                        summary: Some(format!(
                            "Unknown detect type '{}'. Use 'biases', 'fallacies', or 'knowledge_gaps'.",
                            detect_type_for_timeout
                        )),
                        overall_quality: None,
                        debiased_version: None,
                        argument_structure: None,
                        unchallenged_assumptions: None,
                        conclusion_altering_biases: None,
                        validation: None,
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
                    tool = "reasoning_detect",
                    timeout_ms = timeout_ms,
                    detect_type = %detect_type,
                    "Tool execution timed out"
                );
                (
                    DetectResponse {
                        detections: vec![],
                        summary: Some(format!("Tool execution timed out after {}ms", timeout_ms)),
                        overall_quality: None,
                        debiased_version: None,
                        argument_structure: None,
                        unchallenged_assumptions: None,
                        conclusion_altering_biases: None,
                        validation: None,
                        metadata: None,
                    },
                    false,
                )
            }
        };

        self.state.metrics.record(
            MetricEvent::new("detect", timer.elapsed_ms(), success)
                .with_operation(detect_type)
                .with_validation(response.validation.as_ref().map(|v| v.consistent)),
        );
        self.state
            .metrics
            .record_tool_use(&input_session_id, "detect", success);

        response
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod detect_helper_tests {
    use super::{build_detect_validation, normalize_for_grounding, quote_is_grounded};

    #[test]
    fn test_normalize_collapses_whitespace_and_case() {
        assert_eq!(
            normalize_for_grounding("  The   QUICK\nBrown "),
            "the quick brown"
        );
    }

    #[test]
    fn test_quote_grounded_when_present() {
        let content = normalize_for_grounding("Our product is superior because customers say so");
        assert!(quote_is_grounded(&content, "customers say so"));
        // Whitespace/case differences still match.
        assert!(quote_is_grounded(&content, "Customers   Say So"));
    }

    #[test]
    fn test_quote_not_grounded_when_absent() {
        let content = normalize_for_grounding("Our product is superior");
        assert!(!quote_is_grounded(
            &content,
            "a paraphrased claim not present"
        ));
    }

    #[test]
    fn test_quote_grounded_when_embedded_in_commentary() {
        // The real failure mode: the model quotes a passage then appends its own
        // commentary, so the full evidence string is not a verbatim substring.
        let content =
            normalize_for_grounding("I've used them for years and they've never let me down");
        let evidence =
            "I've used them for years and they've never let me down - showing a preference \
             for maintaining the current vendor relationship without objective evaluation";
        assert!(quote_is_grounded(&content, evidence));
    }

    #[test]
    fn test_single_shared_word_does_not_ground_long_quote() {
        // A lone common word must not count as grounding.
        let content = normalize_for_grounding("The deployment pipeline is fast and reliable");
        assert!(!quote_is_grounded(
            &content,
            "the moon orbits a distant planet very quietly"
        ));
    }

    #[test]
    fn test_empty_quote_is_grounded() {
        let content = normalize_for_grounding("anything");
        assert!(quote_is_grounded(&content, "   "));
    }

    #[test]
    fn test_validation_consistent_when_count_matches_and_grounded() {
        let v = build_detect_validation(2, 2, &[]);
        assert!(v.consistent);
        assert!(v.warnings.is_empty());
    }

    #[test]
    fn test_validation_flags_count_mismatch() {
        let v = build_detect_validation(3, 2, &[]);
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("Reported count")));
    }

    #[test]
    fn test_validation_flags_ungrounded_quote() {
        let v = build_detect_validation(1, 1, &["Confirmation Bias".to_string()]);
        assert!(!v.consistent);
        assert!(v
            .warnings
            .iter()
            .any(|w| w.contains("could not be located")));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod graph_verify_tests {
    use super::{verify_graph_generate, verify_graph_state};
    use crate::server::responses::{GraphNode, GraphState};

    fn node(id: &str, score: Option<f64>) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            content: "c".to_string(),
            score,
            depth: None,
            parent_id: None,
        }
    }

    #[test]
    fn test_generate_scores_in_range_are_consistent() {
        let nodes = vec![node("a", Some(0.0)), node("b", Some(0.8)), node("c", None)];
        let v = verify_graph_generate(&nodes);
        assert!(v.consistent, "warnings: {:?}", v.warnings);
    }

    #[test]
    fn test_generate_flags_out_of_range_score() {
        let nodes = vec![node("a", Some(0.8)), node("b", Some(1.4))];
        let v = verify_graph_generate(&nodes);
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("outside the [0, 1]")));
    }

    #[test]
    fn test_generate_flags_negative_score() {
        let v = verify_graph_generate(&[node("a", Some(-0.1))]);
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("'a'")));
    }

    #[test]
    fn test_state_counts_reconcile() {
        let state = GraphState {
            total_nodes: 10,
            active_nodes: 8,
            max_depth: 3,
            pruned_count: 2,
        };
        let v = verify_graph_state(&state);
        assert!(v.consistent, "warnings: {:?}", v.warnings);
    }

    #[test]
    fn test_state_flags_pruned_exceeding_total() {
        let state = GraphState {
            total_nodes: 3,
            active_nodes: 0,
            max_depth: 1,
            pruned_count: 5,
        };
        let v = verify_graph_state(&state);
        assert!(!v.consistent);
        assert!(v
            .warnings
            .iter()
            .any(|w| w.contains("pruned_count 5 exceeds total_nodes 3")));
    }

    #[test]
    fn test_state_flags_active_exceeding_total() {
        let state = GraphState {
            total_nodes: 4,
            active_nodes: 9,
            max_depth: 2,
            pruned_count: 0,
        };
        let v = verify_graph_state(&state);
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("active_nodes 9")));
    }
}
