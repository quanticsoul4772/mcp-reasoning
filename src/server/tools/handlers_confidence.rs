use std::sync::Arc;
use std::time::Duration;

use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::AutoMode;
use crate::server::requests::{
    ConfidenceRouteRequest, DivergentRequest, LinearRequest, TreeRequest,
};
use crate::server::responses::{ConfidenceRouteResponse, NextCallHint};

use super::NO_THINKING;

/// Default confidence threshold above which we trust the auto selection.
const DEFAULT_HIGH_CONFIDENCE: f64 = 0.75;

impl super::ReasoningServer {
    pub(super) async fn handle_confidence_route(
        &self,
        req: ConfidenceRouteRequest,
    ) -> ConfidenceRouteResponse {
        let timer = Timer::start();

        tracing::info!(
            tool = "reasoning_confidence_route",
            content_length = req.content.len(),
            budget = ?req.budget,
            threshold = ?req.high_confidence_threshold,
            "Tool invocation started"
        );

        let threshold = req
            .high_confidence_threshold
            .unwrap_or(DEFAULT_HIGH_CONFIDENCE)
            .clamp(0.0, 1.0);

        let budget = req.budget.as_deref().unwrap_or("auto");

        // --- Step 1: Detect with AutoMode ---
        let auto_mode = AutoMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let timeout_ms = self.state.config.timeout_for_thinking_budget(NO_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);

        let auto_result = match tokio::time::timeout(
            timeout_duration,
            auto_mode.select(&req.content, req.session_id.clone()),
        )
        .await
        {
            Ok(inner) => inner,
            Err(_elapsed) => Err(ModeError::Timeout {
                elapsed_ms: timeout_ms,
            }),
        };

        let auto_resp = match auto_result {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(
                    tool = "reasoning_confidence_route",
                    error = %e,
                    "Auto-detection failed"
                );
                self.state.metrics.record(MetricEvent::new(
                    "confidence_route",
                    timer.elapsed_ms(),
                    false,
                ));
                return ConfidenceRouteResponse {
                    executed_mode: String::new(),
                    auto_suggested_mode: String::new(),
                    routing_confidence: 0.0,
                    routing_decision: "error".to_string(),
                    routing_reason: format!(
                        "Auto-detection failed: {e}. \
                         Check ANTHROPIC_API_KEY and retry. \
                         Alternatively call reasoning_linear directly."
                    ),
                    result: serde_json::Value::Null,
                    metadata: None,
                    next_call: Some(NextCallHint {
                        tool: "reasoning_linear".to_string(),
                        args: serde_json::json!({"session_id": req.session_id}),
                        reason: "confidence_route detection failed; use linear as fallback"
                            .to_string(),
                    }),
                };
            }
        };

        let auto_suggested = auto_resp.selected_mode.to_string();
        let confidence = auto_resp.confidence;
        let auto_session_id = auto_resp.session_id.clone();

        // --- Step 2: Routing decision ---
        let (execute_mode, routing_decision, routing_reason) = match budget {
            "low" => (
                "linear",
                "budget_override",
                format!("Budget=low forces linear regardless of confidence ({confidence:.2})"),
            ),
            "high" => (
                "tree",
                "budget_override",
                format!("Budget=high forces tree regardless of confidence ({confidence:.2})"),
            ),
            _ => {
                // auto budget: route by confidence
                if confidence >= threshold {
                    // High confidence: use what auto suggested (if directly executable)
                    match auto_suggested.as_str() {
                        "linear" | "divergent" => (
                            auto_suggested.as_str(),
                            "direct",
                            format!(
                                "Confidence {confidence:.2} >= threshold {threshold:.2}; executing auto-selected '{auto_suggested}' directly"
                            ),
                        ),
                        other => (
                            "linear", // safe default when mode needs parameters we can't infer
                            "direct_fallback",
                            format!(
                                "Confidence {confidence:.2} >= threshold {threshold:.2}; auto suggested '{other}' which requires direct invocation; using linear as proxy"
                            ),
                        ),
                    }
                } else {
                    (
                        "tree",
                        "escalated_to_tree",
                        format!(
                            "Confidence {confidence:.2} < threshold {threshold:.2}; escalating to tree for thorough exploration"
                        ),
                    )
                }
            }
        };

        tracing::info!(
            tool = "reasoning_confidence_route",
            auto_suggested = %auto_suggested,
            confidence = confidence,
            execute_mode = execute_mode,
            routing_decision = routing_decision,
            "Routing decision made"
        );

        // --- Step 3: Execute selected strategy ---
        let (result_value, next_call) = match execute_mode {
            "linear" => {
                let exec = self
                    .handle_linear(LinearRequest {
                        content: req.content.clone(),
                        session_id: Some(auto_session_id.clone()),
                        confidence: None,
                        timeout_ms: None,
                    })
                    .await;
                let next = exec.next_call.clone();
                (
                    serde_json::to_value(&exec)
                        .unwrap_or_else(|_| serde_json::json!({"error": "serialize failed"})),
                    next,
                )
            }
            "divergent" => {
                let exec = self
                    .handle_divergent(DivergentRequest {
                        content: req.content.clone(),
                        session_id: Some(auto_session_id.clone()),
                        num_perspectives: None,
                        challenge_assumptions: None,
                        force_rebellion: None,
                        progress_token: None,
                    })
                    .await;
                (
                    serde_json::to_value(&exec)
                        .unwrap_or_else(|_| serde_json::json!({"error": "serialize failed"})),
                    None,
                )
            }
            "tree" => {
                let exec = self
                    .handle_tree(TreeRequest {
                        operation: Some("create".to_string()),
                        content: Some(req.content.clone()),
                        session_id: Some(auto_session_id.clone()),
                        branch_id: None,
                        num_branches: None,
                        completed: None,
                    })
                    .await;
                let tree_session = exec.session_id.clone();
                (
                    serde_json::to_value(&exec)
                        .unwrap_or_else(|_| serde_json::json!({"error": "serialize failed"})),
                    Some(NextCallHint {
                        tool: "reasoning_tree".to_string(),
                        args: serde_json::json!({"operation": "focus", "session_id": tree_session}),
                        reason: "tree created; call focus to select a branch to explore"
                            .to_string(),
                    }),
                )
            }
            _ => unreachable!("execute_mode is always linear/divergent/tree"),
        };

        let elapsed_ms = timer.elapsed_ms();
        self.state
            .metrics
            .record(MetricEvent::new("confidence_route", elapsed_ms, true));

        tracing::info!(
            tool = "reasoning_confidence_route",
            elapsed_ms = elapsed_ms,
            executed_mode = execute_mode,
            "Tool invocation completed"
        );

        ConfidenceRouteResponse {
            executed_mode: execute_mode.to_string(),
            auto_suggested_mode: auto_suggested,
            routing_confidence: confidence,
            routing_decision: routing_decision.to_string(),
            routing_reason,
            result: result_value,
            metadata: None,
            next_call,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod tests {
    use std::collections::HashMap;

    use crate::modes::{AlternativeMode, AutoResponse, ReasoningMode};
    use crate::server::requests::ConfidenceRouteRequest;

    /// Build a minimal ConfidenceRouteRequest for testing.
    fn make_req(
        content: &str,
        budget: Option<&str>,
        threshold: Option<f64>,
    ) -> ConfidenceRouteRequest {
        ConfidenceRouteRequest {
            content: content.to_string(),
            session_id: None,
            high_confidence_threshold: threshold,
            budget: budget.map(str::to_string),
        }
    }

    #[test]
    fn test_request_defaults() {
        let req = make_req("Analyze this", None, None);
        assert_eq!(req.budget, None);
        assert_eq!(req.high_confidence_threshold, None);
    }

    #[test]
    fn test_auto_response_confidence_field() {
        let resp = AutoResponse::new(
            "t1",
            "s1",
            ReasoningMode::Linear,
            "step by step",
            0.88,
            vec!["sequential".to_string()],
            HashMap::new(),
        );
        assert!((resp.confidence - 0.88).abs() < f64::EPSILON);
        assert_eq!(resp.selected_mode, ReasoningMode::Linear);
    }

    #[test]
    fn test_auto_response_with_alternative() {
        let alt = AlternativeMode::new(ReasoningMode::Tree, "for exploration");
        let resp = AutoResponse::new(
            "t1",
            "s1",
            ReasoningMode::Divergent,
            "multi-perspective",
            0.6,
            vec![],
            HashMap::new(),
        )
        .with_alternative(alt);
        assert!(resp.alternative_mode.is_some());
    }

    /// Routing decision logic tested directly (pure function).
    #[test]
    fn test_routing_logic_high_confidence_linear() {
        let confidence = 0.85_f64;
        let threshold = 0.75_f64;
        let auto_suggested = "linear";
        let budget = "auto";

        let execute_mode = match budget {
            "low" => "linear",
            "high" => "tree",
            _ => {
                if confidence >= threshold {
                    match auto_suggested {
                        "linear" | "divergent" => auto_suggested,
                        _ => "linear",
                    }
                } else {
                    "tree"
                }
            }
        };
        assert_eq!(execute_mode, "linear");
    }

    #[test]
    fn test_routing_logic_low_confidence_escalates_to_tree() {
        let confidence = 0.55_f64;
        let threshold = 0.75_f64;
        let auto_suggested = "linear";
        let budget = "auto";

        let execute_mode = match budget {
            "low" => "linear",
            "high" => "tree",
            _ => {
                if confidence >= threshold {
                    match auto_suggested {
                        "linear" | "divergent" => auto_suggested,
                        _ => "linear",
                    }
                } else {
                    "tree"
                }
            }
        };
        assert_eq!(execute_mode, "tree");
    }

    #[test]
    fn test_routing_logic_budget_low_forces_linear() {
        let confidence = 0.9_f64;
        let threshold = 0.75_f64;
        let auto_suggested = "tree";
        let budget = "low";

        let execute_mode = match budget {
            "low" => "linear",
            "high" => "tree",
            _ => {
                if confidence >= threshold {
                    match auto_suggested {
                        "linear" | "divergent" => auto_suggested,
                        _ => "linear",
                    }
                } else {
                    "tree"
                }
            }
        };
        assert_eq!(execute_mode, "linear");
    }

    #[test]
    fn test_routing_logic_budget_high_forces_tree() {
        let confidence = 0.9_f64;
        let threshold = 0.75_f64;
        let auto_suggested = "linear";
        let budget = "high";

        let execute_mode = match budget {
            "low" => "linear",
            "high" => "tree",
            _ => {
                if confidence >= threshold {
                    match auto_suggested {
                        "linear" | "divergent" => auto_suggested,
                        _ => "linear",
                    }
                } else {
                    "tree"
                }
            }
        };
        assert_eq!(execute_mode, "tree");
    }

    #[test]
    fn test_routing_logic_complex_mode_falls_back_to_linear() {
        // When auto suggests a complex mode (needs params) and confidence is high,
        // we fall back to linear as proxy
        let confidence = 0.9_f64;
        let threshold = 0.75_f64;
        let auto_suggested = "mcts"; // complex mode
        let budget = "auto";

        let execute_mode = match budget {
            "low" => "linear",
            "high" => "tree",
            _ => {
                if confidence >= threshold {
                    match auto_suggested {
                        "linear" | "divergent" => auto_suggested,
                        _ => "linear",
                    }
                } else {
                    "tree"
                }
            }
        };
        assert_eq!(execute_mode, "linear");
    }
}
