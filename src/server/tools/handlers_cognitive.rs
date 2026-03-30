use std::sync::Arc;
use std::time::Duration;

use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::{CheckpointContext, CheckpointMode, DivergentMode, ReflectionMode};
use crate::server::metadata_builders;
use crate::server::requests::{CheckpointRequest, DivergentRequest, ReflectionRequest};
use crate::server::responses::{
    Checkpoint, CheckpointResponse, DivergentResponse, Perspective, ReflectionResponse,
};

use super::DEEP_THINKING;

impl super::ReasoningServer {
    pub(super) async fn handle_divergent(&self, req: DivergentRequest) -> DivergentResponse {
        let timer = Timer::start();
        let challenge = req.challenge_assumptions.unwrap_or(false);
        let rebellion = req.force_rebellion.unwrap_or(false);
        let content_length = req.content.len();

        tracing::info!(
            tool = "reasoning_divergent",
            content_length = content_length,
            num_perspectives = ?req.num_perspectives,
            challenge_assumptions = challenge,
            force_rebellion = rebellion,
            session_id = ?req.session_id,
            progress_token = ?req.progress_token,
            "Tool invocation started (streaming)"
        );

        let mode = DivergentMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let input_session_id = req.session_id.clone().unwrap_or_default();

        // Create progress reporter (use progress_token or generate one)
        let progress_token = req.progress_token.unwrap_or_else(|| {
            // Efficient string building with pre-allocated capacity
            let uuid = uuid::Uuid::new_v4();
            let mut token = String::with_capacity(46); // "divergent-" (10) + UUID (36)
            token.push_str("divergent-");
            token.push_str(&uuid.to_string());
            token
        });
        let progress = self.state.create_progress_reporter(&progress_token);

        // Apply tool-level timeout (DEEP_THINKING = 8192 tokens = 60s)
        let timeout_ms = self.state.config.timeout_for_thinking_budget(DEEP_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);

        let result = match tokio::time::timeout(
            timeout_duration,
            mode.process_streaming(
                &req.content,
                req.session_id,
                req.num_perspectives,
                challenge,
                rebellion,
                Some(&progress),
            ),
        )
        .await
        {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                tracing::error!(
                    tool = "reasoning_divergent",
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
            tool = "reasoning_divergent",
            elapsed_ms = elapsed_ms,
            success = success,
            "Tool invocation completed"
        );

        self.state
            .metrics
            .record(MetricEvent::new("divergent", elapsed_ms, success));

        match result {
            Ok(resp) => {
                let num_perspectives = resp.perspectives.len();
                let session_id_clone = resp.session_id.clone();

                let metadata = match metadata_builders::build_metadata_for_divergent(
                    &self.state.metadata_builder,
                    content_length,
                    num_perspectives,
                    rebellion,
                    Some(session_id_clone),
                    elapsed_ms,
                )
                .await
                {
                    Ok(m) => Some(m),
                    Err(e) => {
                        tracing::warn!(
                            tool = "reasoning_divergent",
                            error = %e,
                            "Metadata enrichment failed, returning response without metadata"
                        );
                        None
                    }
                };

                DivergentResponse {
                    thought_id: resp.thought_id,
                    session_id: resp.session_id,
                    perspectives: resp
                        .perspectives
                        .into_iter()
                        .map(|p| Perspective {
                            viewpoint: p.viewpoint,
                            content: p.content,
                            novelty_score: p.novelty_score,
                        })
                        .collect(),
                    challenged_assumptions: resp.challenged_assumptions,
                    synthesis: resp.synthesis,
                    metadata,
                }
            }
            Err(e) => DivergentResponse {
                thought_id: String::new(),
                session_id: input_session_id,
                perspectives: vec![],
                challenged_assumptions: None,
                synthesis: Some(format!(
                    "divergent failed: {e}. Ensure content is non-empty. \
                     Try reducing num_perspectives (2-4) or retry without force_rebellion."
                )),
                metadata: None,
            },
        }
    }

    pub(super) async fn handle_reflection(&self, req: ReflectionRequest) -> ReflectionResponse {
        let timer = Timer::start();
        let mode = ReflectionMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let operation = req.operation.as_deref().unwrap_or("process");

        // Create progress reporter (use progress_token or generate one)
        let progress_token = req.progress_token.unwrap_or_else(|| {
            // Efficient string building with pre-allocated capacity
            let uuid = uuid::Uuid::new_v4();
            let mut token = String::with_capacity(47); // "reflection-" (11) + UUID (36)
            token.push_str("reflection-");
            token.push_str(&uuid.to_string());
            token
        });
        let progress = self.state.create_progress_reporter(&progress_token);

        tracing::info!(
            tool = "reasoning_reflection",
            operation = operation,
            progress_token = %progress_token,
            "Tool invocation started (streaming)"
        );

        // Apply tool-level timeout (DEEP_THINKING = 8192 tokens)
        let timeout_ms = self.state.config.timeout_for_thinking_budget(DEEP_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);

        let (mut response, success) = match operation {
            "process" => {
                let content = req.content.as_deref().unwrap_or("");
                let process_result = match tokio::time::timeout(
                    timeout_duration,
                    mode.process_streaming(content, req.session_id, Some(&progress)),
                )
                .await
                {
                    Ok(inner) => inner,
                    Err(_elapsed) => {
                        tracing::error!(
                            tool = "reasoning_reflection",
                            operation = "process",
                            timeout_ms = timeout_ms,
                            "Tool execution timed out"
                        );
                        Err(ModeError::Timeout {
                            elapsed_ms: timeout_ms,
                        })
                    }
                };
                match process_result {
                    Ok(resp) => (
                        ReflectionResponse {
                            quality_score: resp.confidence_improvement,
                            thought_id: Some(resp.thought_id),
                            session_id: Some(resp.session_id),
                            iterations_used: Some(1),
                            strengths: Some(resp.analysis.strengths),
                            weaknesses: Some(resp.analysis.weaknesses),
                            recommendations: Some(
                                resp.improvements
                                    .into_iter()
                                    .map(|i| i.suggestion)
                                    .collect(),
                            ),
                            refined_content: Some(resp.refined_reasoning),
                            coherence_score: None,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        ReflectionResponse {
                            quality_score: 0.0,
                            thought_id: None,
                            session_id: None,
                            iterations_used: None,
                            strengths: None,
                            weaknesses: Some(vec![format!(
                                "reflection process failed: {e}. \
                                 Provide non-empty content. \
                                 Use operation='evaluate' to assess an existing session instead."
                            )]),
                            recommendations: None,
                            refined_content: None,
                            coherence_score: None,
                            metadata: None,
                        },
                        false,
                    ),
                }
            }
            "evaluate" => {
                let session_id = req.session_id.as_deref().unwrap_or("");
                let summary = req.content.as_deref();
                let evaluate_result = match tokio::time::timeout(
                    timeout_duration,
                    mode.evaluate_streaming(session_id, summary, Some(&progress)),
                )
                .await
                {
                    Ok(inner) => inner,
                    Err(_elapsed) => {
                        tracing::error!(
                            tool = "reasoning_reflection",
                            operation = "evaluate",
                            timeout_ms = timeout_ms,
                            "Tool execution timed out"
                        );
                        Err(ModeError::Timeout {
                            elapsed_ms: timeout_ms,
                        })
                    }
                };
                match evaluate_result {
                    Ok(resp) => (
                        ReflectionResponse {
                            quality_score: resp.session_assessment.overall_quality,
                            thought_id: Some(resp.thought_id),
                            session_id: Some(resp.session_id),
                            iterations_used: None,
                            strengths: Some(resp.strongest_elements),
                            weaknesses: Some(resp.areas_for_improvement),
                            recommendations: Some(resp.recommendations),
                            refined_content: None,
                            coherence_score: Some(resp.session_assessment.coherence),
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        ReflectionResponse {
                            quality_score: 0.0,
                            thought_id: None,
                            session_id: None,
                            iterations_used: None,
                            strengths: None,
                            weaknesses: Some(vec![format!(
                                "reflection evaluate failed: {e}. \
                                 Verify session_id is from a previous reasoning session. \
                                 Use operation='process' with content to start a new reflection."
                            )]),
                            recommendations: None,
                            refined_content: None,
                            coherence_score: None,
                            metadata: None,
                        },
                        false,
                    ),
                }
            }
            _ => (
                ReflectionResponse {
                    quality_score: 0.0,
                    thought_id: None,
                    session_id: None,
                    iterations_used: None,
                    strengths: None,
                    weaknesses: Some(vec![format!(
                        "Unknown operation: {operation}. Use 'process' or 'evaluate'."
                    )]),
                    recommendations: None,
                    refined_content: None,
                    coherence_score: None,
                    metadata: None,
                },
                false,
            ),
        };

        let elapsed_ms = timer.elapsed_ms();
        self.state
            .metrics
            .record(MetricEvent::new("reflection", elapsed_ms, success).with_operation(operation));

        // Add metadata on success
        if success {
            let iterations = response.iterations_used.unwrap_or(1) as usize;
            let quality = response.quality_score;

            match metadata_builders::build_metadata_for_reflection(
                &self.state.metadata_builder,
                req.content.as_deref().unwrap_or("").len(),
                operation,
                iterations,
                quality,
                response.session_id.clone(),
                elapsed_ms,
            )
            .await
            {
                Ok(metadata) => {
                    response.metadata = Some(metadata);
                }
                Err(e) => {
                    tracing::warn!(
                        tool = "reasoning_reflection",
                        operation = %operation,
                        error = %e,
                        "Metadata enrichment failed, returning response without metadata"
                    );
                }
            }
        }

        response
    }

    pub(super) async fn handle_checkpoint(&self, req: CheckpointRequest) -> CheckpointResponse {
        let timer = Timer::start();
        let mode = CheckpointMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let operation = req.operation.as_str();
        let (response, success) = match operation {
            "create" => {
                let name = req.name.as_deref().unwrap_or("checkpoint");
                let description = req.description.as_deref();
                // Create a basic context - in a full implementation, this would be
                // extracted from the session's current state
                let context = CheckpointContext::new(
                    vec![],
                    req.new_direction
                        .as_deref()
                        .unwrap_or("current exploration"),
                    vec![],
                );
                let resumption_hint = "Resume from this point";

                match mode
                    .create(&req.session_id, name, description, context, resumption_hint)
                    .await
                {
                    Ok(resp) => (
                        CheckpointResponse {
                            session_id: resp.session_id,
                            checkpoint_id: Some(resp.checkpoint_id),
                            checkpoints: None,
                            restored_state: None,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        CheckpointResponse {
                            session_id: req.session_id.clone(),
                            checkpoint_id: None,
                            checkpoints: None,
                            restored_state: Some(serde_json::json!({"error": e.to_string()})),
                            metadata: None,
                        },
                        false,
                    ),
                }
            }
            "list" => match mode.list(&req.session_id).await {
                Ok(resp) => (
                    CheckpointResponse {
                        session_id: resp.session_id,
                        checkpoint_id: None,
                        checkpoints: Some(
                            resp.checkpoints
                                .into_iter()
                                .map(|c| Checkpoint {
                                    id: c.id,
                                    name: c.name,
                                    description: c.description,
                                    created_at: c.created_at,
                                    thought_count: c.thought_count as u32,
                                })
                                .collect(),
                        ),
                        restored_state: None,
                        metadata: None,
                    },
                    true,
                ),
                Err(e) => (
                    CheckpointResponse {
                        session_id: req.session_id.clone(),
                        checkpoint_id: None,
                        checkpoints: None,
                        restored_state: Some(serde_json::json!({"error": e.to_string()})),
                        metadata: None,
                    },
                    false,
                ),
            },
            "restore" => {
                let checkpoint_id = req.checkpoint_id.as_deref().unwrap_or("");
                let new_direction = req.new_direction.as_deref();
                match mode.restore(checkpoint_id, new_direction).await {
                    Ok(resp) => (
                        CheckpointResponse {
                            session_id: resp.session_id,
                            checkpoint_id: Some(resp.checkpoint_id),
                            checkpoints: None,
                            restored_state: Some(serde_json::json!({
                                "context": {
                                    "key_findings": resp.restored_state.context.key_findings,
                                    "current_focus": resp.restored_state.context.current_focus,
                                    "open_questions": resp.restored_state.context.open_questions
                                },
                                "thought_count": resp.restored_state.thought_count,
                                "new_direction": resp.new_direction
                            })),
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        CheckpointResponse {
                            session_id: req.session_id.clone(),
                            checkpoint_id: None,
                            checkpoints: None,
                            restored_state: Some(serde_json::json!({"error": e.to_string()})),
                            metadata: None,
                        },
                        false,
                    ),
                }
            }
            _ => (
                CheckpointResponse {
                    session_id: req.session_id.clone(),
                    checkpoint_id: None,
                    checkpoints: None,
                    restored_state: Some(serde_json::json!({
                        "error": format!("Unknown operation: {}. Use 'create', 'list', or 'restore'.", req.operation)
                    })),
                    metadata: None,
                },
                false,
            ),
        };

        self.state.metrics.record(
            MetricEvent::new("checkpoint", timer.elapsed_ms(), success).with_operation(operation),
        );

        response
    }
}
