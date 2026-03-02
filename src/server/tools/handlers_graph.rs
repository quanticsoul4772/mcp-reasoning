use std::sync::Arc;
use std::time::Duration;

use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::{DetectMode, GraphMode};
use crate::server::metadata_builders;
use crate::server::requests::{DetectRequest, GraphRequest};
use crate::server::responses::{DetectResponse, Detection, GraphNode, GraphResponse, GraphState};

use super::{DEEP_THINKING, STANDARD_THINKING};

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
            aggregated_insight: Some(format!("ERROR: {e}")),
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

            if let Ok(metadata) = metadata_builders::build_metadata_for_graph(
                &self.state.metadata_builder,
                content.len(),
                &operation,
                num_nodes,
                Some(session_id),
                elapsed_ms,
            )
            .await
            {
                response.metadata = Some(metadata);
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
                    Ok(resp) => (
                        DetectResponse {
                            detections: resp
                                .biases_detected
                                .into_iter()
                                .map(|b| Detection {
                                    detection_type: b.bias,
                                    category: None, // Biases don't have categories
                                    severity: b.severity.as_str().to_string(),
                                    confidence: resp.overall_assessment.reasoning_quality,
                                    evidence: b.evidence,
                                    explanation: b.impact,
                                    remediation: Some(b.debiasing),
                                })
                                .collect(),
                            summary: Some(format!(
                                "{} biases detected. Most severe: {}. Debiased version available.",
                                resp.overall_assessment.bias_count,
                                resp.overall_assessment.most_severe
                            )),
                            overall_quality: Some(resp.overall_assessment.reasoning_quality),
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DetectResponse {
                            detections: vec![],
                            summary: Some(format!("Error detecting biases: {e}")),
                            overall_quality: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "fallacies" => match mode.fallacies(content, req.session_id).await {
                    Ok(resp) => (
                        DetectResponse {
                            detections: resp
                                .fallacies_detected
                                .into_iter()
                                .map(|f| Detection {
                                    detection_type: f.fallacy,
                                    category: Some(f.category.as_str().to_string()),
                                    severity: if resp.overall_assessment.argument_strength < 0.4 {
                                        "high".to_string()
                                    } else if resp.overall_assessment.argument_strength < 0.7 {
                                        "medium".to_string()
                                    } else {
                                        "low".to_string()
                                    },
                                    confidence: resp.overall_assessment.argument_strength,
                                    evidence: f.passage,
                                    explanation: f.explanation,
                                    remediation: Some(f.correction),
                                })
                                .collect(),
                            summary: Some(format!(
                                "{} fallacies detected. Most critical: {}. Argument validity: {:?}",
                                resp.overall_assessment.fallacy_count,
                                resp.overall_assessment.most_critical,
                                resp.argument_structure.validity
                            )),
                            overall_quality: Some(resp.overall_assessment.argument_strength),
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DetectResponse {
                            detections: vec![],
                            summary: Some(format!("Error detecting fallacies: {e}")),
                            overall_quality: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                _ => (
                    DetectResponse {
                        detections: vec![],
                        summary: Some(format!(
                            "Unknown detect type '{}'. Use 'biases' or 'fallacies'.",
                            detect_type_for_timeout
                        )),
                        overall_quality: None,
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
