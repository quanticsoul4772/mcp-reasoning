use std::sync::Arc;
use std::time::Duration;

use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::meta::MetaMode;
use crate::modes::{AutoMode, LinearMode, TreeMode};
use crate::server::metadata_builders;
use crate::server::requests::{
    AutoRequest, DivergentRequest, LinearRequest, MetaRequest, TreeRequest,
};
use crate::server::responses::{
    AutoResponse, Branch, LinearResponse, MetaResponse, NextCallHint, TreeResponse,
};

use super::NO_THINKING;

impl super::ReasoningServer {
    pub(super) async fn handle_linear(&self, req: LinearRequest) -> LinearResponse {
        let timer = Timer::start();
        let content_length = req.content.len();

        tracing::info!(
            tool = "reasoning_linear",
            content_length = content_length,
            session_id = ?req.session_id,
            confidence_threshold = ?req.confidence,
            "Tool invocation started"
        );

        let mode = LinearMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let input_session_id = req.session_id.clone().unwrap_or_default();
        let session_id_for_metadata = req.session_id.clone();

        // Apply tool-level timeout to prevent indefinite hangs.
        // Per-request override (req.timeout_ms) takes precedence over server default.
        let timeout_ms = req
            .timeout_ms
            .unwrap_or_else(|| self.state.config.timeout_for_thinking_budget(NO_THINKING));
        let timeout_duration = Duration::from_millis(timeout_ms);

        let result = match tokio::time::timeout(
            timeout_duration,
            mode.process(
                &req.content,
                req.session_id,
                req.confidence
                    .map(super::super::requests::ConfidenceThreshold::value),
            ),
        )
        .await
        {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                tracing::error!(
                    tool = "reasoning_linear",
                    timeout_ms = timeout_ms,
                    "Tool execution timed out"
                );
                Err(ModeError::Timeout {
                    elapsed_ms: timeout_ms,
                })
            }
        };

        let elapsed_ms = timer.elapsed_ms();
        let success = result.is_ok();

        tracing::info!(
            tool = "reasoning_linear",
            elapsed_ms = elapsed_ms,
            success = success,
            "Tool invocation completed"
        );

        self.state
            .metrics
            .record(MetricEvent::new("linear", elapsed_ms, success));

        // Build metadata for response enrichment
        let metadata = if success {
            match self
                .build_metadata_for_linear(content_length, session_id_for_metadata, elapsed_ms)
                .await
            {
                Ok(m) => Some(m),
                Err(e) => {
                    tracing::warn!(
                        tool = "reasoning_linear",
                        error = %e,
                        "Metadata enrichment failed, returning response without metadata"
                    );
                    None
                }
            }
        } else {
            None
        };

        match result {
            Ok(resp) => {
                let session_id_for_hint = resp.session_id.clone();
                LinearResponse {
                    thought_id: resp.thought_id,
                    session_id: resp.session_id,
                    content: resp.content,
                    confidence: resp.confidence,
                    next_step: resp.next_step,
                    metadata,
                    next_call: Some(NextCallHint {
                        tool: "reasoning_checkpoint".to_string(),
                        args: serde_json::json!({"operation": "create", "session_id": session_id_for_hint}),
                        reason: "save progress after linear reasoning step".to_string(),
                    }),
                }
            }
            Err(e) => LinearResponse {
                thought_id: String::new(),
                session_id: input_session_id,
                content: format!(
                    "linear failed: {e}. \
                     Ensure content is non-empty. \
                     If this is a timeout, retry or increase timeout_ms. \
                     If the API is unavailable, check ANTHROPIC_API_KEY."
                ),
                confidence: 0.0,
                next_step: None,
                metadata: None,
                next_call: None,
            },
        }
    }
    pub(super) async fn handle_tree(&self, req: TreeRequest) -> TreeResponse {
        let timer = Timer::start();
        let operation = req.operation.as_deref().unwrap_or("create");

        tracing::info!(
            tool = "reasoning_tree",
            operation = operation,
            session_id = ?req.session_id,
            num_branches = ?req.num_branches,
            "Tool invocation started"
        );

        let mut mode = TreeMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let session_id = req.session_id.clone().unwrap_or_default();

        // Apply tool-level timeout for API-calling operations
        let timeout_ms = self.state.config.timeout_for_thinking_budget(NO_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);

        let (mut response, success) = match operation {
            "create" => {
                let content = req.content.as_deref().unwrap_or("");
                let create_result = match tokio::time::timeout(
                    timeout_duration,
                    mode.create(content, req.session_id, req.num_branches),
                )
                .await
                {
                    Ok(inner) => inner,
                    Err(_elapsed) => {
                        tracing::error!(
                            tool = "reasoning_tree",
                            operation = "create",
                            timeout_ms = timeout_ms,
                            "Tool execution timed out"
                        );
                        Err(ModeError::Timeout {
                            elapsed_ms: timeout_ms,
                        })
                    }
                };
                match create_result {
                    Ok(resp) => (
                        TreeResponse {
                            session_id: resp.session_id,
                            branch_id: resp.branch_id,
                            branches: resp.branches.map(|bs| {
                                bs.into_iter()
                                    .map(|b| Branch {
                                        id: b.id,
                                        content: b.content,
                                        score: b.score,
                                        status: b.status.as_str().to_string(),
                                    })
                                    .collect()
                            }),
                            recommendation: resp.recommendation,
                            synthesis: None,
                            key_findings: None,
                            best_insights: None,
                            metadata: None,
                            next_call: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        TreeResponse {
                            session_id,
                            branch_id: None,
                            branches: None,
                            recommendation: Some(format!(
                                "create failed: {e}. \
                                 Ensure content is non-empty. \
                                 Retry with different content, or reduce num_branches (2-4)."
                            )),
                            synthesis: None,
                            key_findings: None,
                            best_insights: None,
                            metadata: None,
                            next_call: None,
                        },
                        false,
                    ),
                }
            }
            "focus" => {
                let branch_id = req.branch_id.as_deref().unwrap_or("");
                let focus_result = match tokio::time::timeout(
                    timeout_duration,
                    mode.focus(&session_id, branch_id),
                )
                .await
                {
                    Ok(inner) => inner,
                    Err(_elapsed) => {
                        tracing::error!(
                            tool = "reasoning_tree",
                            operation = "focus",
                            timeout_ms = timeout_ms,
                            "Tool execution timed out"
                        );
                        Err(ModeError::Timeout {
                            elapsed_ms: timeout_ms,
                        })
                    }
                };
                match focus_result {
                    Ok(resp) => (
                        TreeResponse {
                            session_id: resp.session_id,
                            branch_id: resp.branch_id,
                            branches: resp.branches.map(|bs| {
                                bs.into_iter()
                                    .map(|b| Branch {
                                        id: b.id,
                                        content: b.content,
                                        score: b.score,
                                        status: b.status.as_str().to_string(),
                                    })
                                    .collect()
                            }),
                            recommendation: resp.recommendation,
                            synthesis: None,
                            key_findings: None,
                            best_insights: None,
                            metadata: None,
                            next_call: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        TreeResponse {
                            session_id,
                            branch_id: None,
                            branches: None,
                            recommendation: Some(format!(
                                "focus failed: {e}. \
                                 Use operation='list' to see valid branch_id values \
                                 for this session_id."
                            )),
                            synthesis: None,
                            key_findings: None,
                            best_insights: None,
                            metadata: None,
                            next_call: None,
                        },
                        false,
                    ),
                }
            }
            "list" => match mode.list(&session_id).await {
                Ok(resp) => (
                    TreeResponse {
                        session_id: resp.session_id,
                        branch_id: resp.branch_id,
                        branches: resp.branches.map(|bs| {
                            bs.into_iter()
                                .map(|b| Branch {
                                    id: b.id,
                                    content: b.content,
                                    score: b.score,
                                    status: b.status.as_str().to_string(),
                                })
                                .collect()
                        }),
                        recommendation: resp.recommendation,
                        synthesis: None,
                        key_findings: None,
                        best_insights: None,
                        metadata: None,
                        next_call: None,
                    },
                    true,
                ),
                Err(e) => (
                    TreeResponse {
                        session_id,
                        branch_id: None,
                        branches: None,
                        recommendation: Some(format!(
                            "list failed: {e}. \
                             Verify session_id is from a previous create call. \
                             An empty list (not an error) means no branches were created yet."
                        )),
                        synthesis: None,
                        key_findings: None,
                        best_insights: None,
                        metadata: None,
                        next_call: None,
                    },
                    false,
                ),
            },
            "complete" => {
                let branch_id = req.branch_id.as_deref().unwrap_or("");
                let completed = req.completed.unwrap_or(true);
                match mode.complete(&session_id, branch_id, completed).await {
                    Ok(resp) => (
                        TreeResponse {
                            session_id: resp.session_id,
                            branch_id: resp.branch_id,
                            branches: resp.branches.map(|bs| {
                                bs.into_iter()
                                    .map(|b| Branch {
                                        id: b.id,
                                        content: b.content,
                                        score: b.score,
                                        status: b.status.as_str().to_string(),
                                    })
                                    .collect()
                            }),
                            recommendation: resp.recommendation,
                            synthesis: None,
                            key_findings: None,
                            best_insights: None,
                            metadata: None,
                            next_call: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        TreeResponse {
                            session_id,
                            branch_id: None,
                            branches: None,
                            recommendation: Some(format!(
                                "complete failed: {e}. \
                                 Use operation='list' to verify branch_id belongs to \
                                 this session_id."
                            )),
                            synthesis: None,
                            key_findings: None,
                            best_insights: None,
                            metadata: None,
                            next_call: None,
                        },
                        false,
                    ),
                }
            }
            "summarize" => {
                let summarize_result =
                    match tokio::time::timeout(timeout_duration, mode.summarize(&session_id)).await
                    {
                        Ok(inner) => inner,
                        Err(_elapsed) => {
                            tracing::error!(
                                tool = "reasoning_tree",
                                operation = "summarize",
                                timeout_ms = timeout_ms,
                                "Tool execution timed out"
                            );
                            Err(ModeError::Timeout {
                                elapsed_ms: timeout_ms,
                            })
                        }
                    };
                match summarize_result {
                    Ok(resp) => (
                        TreeResponse {
                            session_id: resp.session_id,
                            branch_id: resp.branch_id,
                            branches: resp.branches.map(|bs| {
                                bs.into_iter()
                                    .map(|b| Branch {
                                        id: b.id,
                                        content: b.content,
                                        score: b.score,
                                        status: b.status.as_str().to_string(),
                                    })
                                    .collect()
                            }),
                            recommendation: resp.recommendation,
                            synthesis: resp.synthesis,
                            key_findings: resp.key_findings,
                            best_insights: resp.best_insights,
                            metadata: None,
                            next_call: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        TreeResponse {
                            session_id,
                            branch_id: None,
                            branches: None,
                            recommendation: Some(format!(
                                "summarize failed: {e}. \
                                 Run operation='create' first if no branches exist yet, \
                                 or use operation='list' to confirm branches are present."
                            )),
                            synthesis: None,
                            key_findings: None,
                            best_insights: None,
                            metadata: None,
                            next_call: None,
                        },
                        false,
                    ),
                }
            }
            _ => (
                TreeResponse {
                    session_id,
                    branch_id: None,
                    branches: None,
                    recommendation: Some(format!(
                        "Unknown operation: {operation}. Use create/focus/list/complete/summarize."
                    )),
                    synthesis: None,
                    key_findings: None,
                    best_insights: None,
                    metadata: None,
                    next_call: None,
                },
                false,
            ),
        };

        let elapsed_ms = timer.elapsed_ms();

        tracing::info!(
            tool = "reasoning_tree",
            operation = operation,
            elapsed_ms = elapsed_ms,
            success = success,
            "Tool invocation completed"
        );

        self.state
            .metrics
            .record(MetricEvent::new("tree", elapsed_ms, success).with_operation(operation));

        // Add metadata on success
        if success {
            let num_branches = response.branches.as_ref().map_or(0, Vec::len);

            match metadata_builders::build_metadata_for_tree(
                &self.state.metadata_builder,
                req.content.as_deref().unwrap_or("").len(),
                operation,
                num_branches,
                Some(response.session_id.clone()),
                elapsed_ms,
            )
            .await
            {
                Ok(metadata) => {
                    response.metadata = Some(metadata);
                }
                Err(e) => {
                    tracing::warn!(
                        tool = "reasoning_tree",
                        operation = operation,
                        error = %e,
                        "Metadata enrichment failed, returning response without metadata"
                    );
                }
            }
        }

        response
    }
    pub(super) async fn handle_auto(&self, req: AutoRequest) -> AutoResponse {
        let timer = Timer::start();
        let mode = AutoMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        // Apply tool-level timeout (NO_THINKING - fast mode)
        let timeout_ms = self.state.config.timeout_for_thinking_budget(NO_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);

        let result =
            match tokio::time::timeout(timeout_duration, mode.select(&req.content, req.session_id))
                .await
            {
                Ok(inner_result) => inner_result,
                Err(_elapsed) => {
                    tracing::error!(
                        tool = "reasoning_auto",
                        timeout_ms = timeout_ms,
                        "Tool execution timed out"
                    );
                    Err(ModeError::Timeout {
                        elapsed_ms: timeout_ms,
                    })
                }
            };
        let success = result.is_ok();
        self.state
            .metrics
            .record(MetricEvent::new("auto", timer.elapsed_ms(), success));

        match result {
            Ok(resp) => {
                // Build selection metadata
                let selection_meta = serde_json::json!({
                    "thought_id": resp.thought_id,
                    "session_id": resp.session_id,
                    "characteristics": resp.characteristics,
                    "suggested_parameters": resp.suggested_parameters,
                    "alternative": resp.alternative_mode.map(|a| serde_json::json!({
                        "mode": a.mode.to_string(),
                        "reason": a.reason
                    }))
                });

                let selected_mode_name = resp.selected_mode.to_string();
                let session_id = resp.session_id.clone();
                let content = req.content.clone();
                let should_execute = req.execute == Some(true);

                // When execute=true, immediately run the selected mode (linear/divergent only).
                // Other modes have required parameters that auto cannot infer — return next_call hint.
                if should_execute {
                    match selected_mode_name.as_str() {
                        "linear" => {
                            let exec_resp = self
                                .handle_linear(LinearRequest {
                                    content,
                                    session_id: Some(session_id),
                                    confidence: None,
                                    timeout_ms: None,
                                })
                                .await;
                            return AutoResponse {
                                selected_mode: selected_mode_name,
                                confidence: resp.confidence,
                                rationale: resp.reasoning,
                                result: serde_json::to_value(&exec_resp).unwrap_or(selection_meta),
                                metadata: None,
                                next_call: None,
                                executed: Some(true),
                            };
                        }
                        "divergent" => {
                            let exec_resp = self
                                .handle_divergent(DivergentRequest {
                                    content,
                                    session_id: Some(session_id),
                                    num_perspectives: None,
                                    challenge_assumptions: None,
                                    force_rebellion: None,
                                    progress_token: None,
                                })
                                .await;
                            return AutoResponse {
                                selected_mode: selected_mode_name,
                                confidence: resp.confidence,
                                rationale: resp.reasoning,
                                result: serde_json::to_value(&exec_resp).unwrap_or(selection_meta),
                                metadata: None,
                                next_call: None,
                                executed: Some(true),
                            };
                        }
                        other => {
                            tracing::debug!(
                                mode = other,
                                "execute=true requested but mode requires direct invocation; returning next_call hint"
                            );
                        }
                    }
                }

                AutoResponse {
                    selected_mode: selected_mode_name.clone(),
                    confidence: resp.confidence,
                    rationale: resp.reasoning,
                    result: selection_meta,
                    metadata: None,
                    next_call: Some(NextCallHint {
                        tool: format!("reasoning_{selected_mode_name}"),
                        args: serde_json::json!({"session_id": session_id}),
                        reason: format!(
                            "auto selected {selected_mode_name}; call it now to begin reasoning"
                        ),
                    }),
                    executed: None,
                }
            }
            Err(e) => AutoResponse {
                selected_mode: "linear".to_string(),
                confidence: 0.0,
                rationale: format!(
                    "auto failed: {e}. \
                     Ensure content is non-empty. \
                     If the API is unavailable, check ANTHROPIC_API_KEY. \
                     Fallback: use reasoning_linear directly."
                ),
                result: serde_json::Value::Null,
                metadata: None,
                next_call: Some(NextCallHint {
                    tool: "reasoning_linear".to_string(),
                    args: serde_json::json!({}),
                    reason: "auto failed; linear is the safe fallback".to_string(),
                }),
                executed: None,
            },
        }
    }

    pub(super) async fn handle_meta(&self, req: MetaRequest) -> MetaResponse {
        let timer = Timer::start();

        tracing::info!(
            tool = "reasoning_meta",
            content_length = req.content.len(),
            problem_type_hint = ?req.problem_type_hint,
            "Meta-reasoning invocation started"
        );

        let mode = MetaMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let timeout_ms = self.state.config.timeout_for_thinking_budget(NO_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);

        let result = match tokio::time::timeout(
            timeout_duration,
            mode.route(
                &req.content,
                req.problem_type_hint,
                req.min_confidence,
                &self.state.metrics,
            ),
        )
        .await
        {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                tracing::error!(
                    tool = "reasoning_meta",
                    timeout_ms = timeout_ms,
                    "Tool execution timed out"
                );
                Err(ModeError::Timeout {
                    elapsed_ms: timeout_ms,
                })
            }
        };

        let elapsed_ms = timer.elapsed_ms();
        let success = result.is_ok();

        // Record metric with problem type for meta-learning
        let mut metric = MetricEvent::new("meta", elapsed_ms, success);
        if let Ok(ref route) = result {
            metric = metric
                .with_problem_type(&route.problem_type)
                .with_quality_rating(route.confidence);
        }
        self.state.metrics.record(metric);

        match result {
            Ok(route) => MetaResponse {
                selected_tool: route.selected_tool,
                problem_type: route.problem_type,
                confidence: route.confidence,
                reasoning: route.reasoning,
                fallback_to_auto: route.fallback_to_auto,
                candidates_evaluated: route.candidates.len(),
                metadata: None,
            },
            Err(e) => MetaResponse {
                selected_tool: "auto".to_string(),
                problem_type: "unknown".to_string(),
                confidence: 0.0,
                reasoning: format!(
                    "meta routing failed: {e}. \
                     Fallback: use reasoning_auto which applies the same selection logic. \
                     If the API is unavailable, check ANTHROPIC_API_KEY."
                ),
                fallback_to_auto: true,
                candidates_evaluated: 0,
                metadata: None,
            },
        }
    }
}
