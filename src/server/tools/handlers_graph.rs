use std::sync::Arc;
use std::time::Duration;

use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::{DetectMode, GraphMode};
use crate::server::metadata_builders;
use crate::server::requests::{DetectRequest, GraphRequest};
use crate::server::responses::{
    ArgumentStructureInfo, DetectResponse, DetectValidationInfo, Detection, GraphNode,
    GraphResponse, GraphState,
};

use super::{DEEP_THINKING, STANDARD_THINKING};

/// Normalize text for quote-grounding comparison: collapse whitespace, lowercase.
fn normalize_for_grounding(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// True when `quote` appears (whitespace/case-insensitively) within `content`.
/// An empty quote is treated as grounded (nothing to verify).
fn quote_is_grounded(content_normalized: &str, quote: &str) -> bool {
    let q = normalize_for_grounding(quote);
    q.is_empty() || content_normalized.contains(&q)
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
            "Evidence for '{name}' was not found verbatim in the content (may be paraphrased)"
        ));
    }
    DetectValidationInfo {
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
                            metadata: None,
                        })
                }
                "generate" => {
                    let sid = session_id.clone();
                    let node_id = req.node_id.as_deref();
                    mode.generate(req.content.as_deref(), node_id, Some(session_id.clone()))
                        .await
                        .map(move |r| GraphResponse {
                            session_id: sid,
                            node_id: None,
                            nodes: Some(
                                r.children
                                    .into_iter()
                                    .map(|n| GraphNode {
                                        id: n.id,
                                        content: n.content,
                                        score: Some(n.score),
                                        depth: None,
                                        parent_id: None,
                                    })
                                    .collect(),
                            ),
                            aggregated_insight: None,
                            conclusions: None,
                            state: None,
                            metadata: None,
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
                            metadata: None,
                        })
                }
                "prune" => {
                    let sid = session_id.clone();
                    mode.prune(content, Some(session_id.clone()))
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
                            metadata: None,
                        })
                }
                "state" => {
                    let sid = session_id.clone();
                    mode.state(req.content.as_deref(), &session_id)
                        .await
                        .map(move |r| GraphResponse {
                            session_id: sid,
                            node_id: None,
                            nodes: None,
                            aggregated_insight: None,
                            conclusions: None,
                            state: Some(GraphState {
                                total_nodes: r.structure.total_nodes,
                                active_nodes: r.structure.total_nodes - r.structure.pruned_count,
                                max_depth: r.structure.depth,
                                pruned_count: r.structure.pruned_count,
                            }),
                            metadata: None,
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
        self.state
            .metrics
            .record(MetricEvent::new("graph", elapsed_ms, success).with_operation(&operation));

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
            MetricEvent::new("detect", timer.elapsed_ms(), success).with_operation(detect_type),
        );

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
        assert!(v.warnings.iter().any(|w| w.contains("not found verbatim")));
    }
}
