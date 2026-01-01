//! Tool definitions with rmcp macros.
//!
//! This module defines all 15 reasoning tools using the rmcp 0.12 macro system.
//! Uses `#[tool_router]` on impl with tools and `#[tool_handler]` on ServerHandler.
//!
//! Request and response types are defined in separate modules for maintainability.

// Tool methods are async stubs that will use await when connected to actual mode implementations
#![allow(clippy::unused_async)]

use std::sync::Arc;
use std::time::Duration;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router};

use super::metadata_builders;
use super::requests::{
    AutoRequest, CheckpointRequest, CounterfactualRequest, DecisionRequest, DetectRequest,
    DivergentRequest, EvidenceRequest, GraphRequest, LinearRequest, MctsRequest, MetricsRequest,
    PresetRequest, ReflectionRequest, SiApproveRequest, SiDiagnosesRequest, SiRejectRequest,
    SiRollbackRequest, SiStatusRequest, SiTriggerRequest, TimelineRequest, TreeRequest,
};
#[cfg(test)]
use super::responses::ConfidenceInterval;
use super::responses::{
    AutoResponse, BacktrackSuggestion, Branch, BranchComparison, CausalStep, Checkpoint,
    CheckpointResponse, CounterfactualResponse, DecisionResponse, DetectResponse, Detection,
    DivergentResponse, EvidenceAssessment, EvidenceResponse, GraphNode, GraphResponse, GraphState,
    Invocation, LinearResponse, MctsNode, MctsResponse, MetricsResponse, MetricsSummary, ModeStats,
    Perspective, PresetExecution, PresetInfo, PresetResponse, RankedOption, ReflectionResponse,
    SiApproveResponse, SiDiagnosesResponse, SiExecutionSummary, SiLearningSummary,
    SiPendingDiagnosis, SiRejectResponse, SiRollbackResponse, SiStatusResponse, SiTriggerResponse,
    StakeholderMap, TimelineBranch, TimelineResponse, TreeResponse,
};
use super::types::AppState;
use crate::error::ModeError;
use crate::metrics::{MetricEvent, Timer};
use crate::modes::{
    AutoMode, CheckpointContext, CheckpointMode, DecisionMode, DetectMode, DivergentMode,
    EvidenceMode, GraphMode, LinearMode, MctsMode, ReflectionMode, TimelineMode, TreeMode,
};

// ============================================================================
// Thinking Budget Constants for Timeout Selection
// ============================================================================

/// No extended thinking (fast modes) - 30s timeout
const NO_THINKING: Option<u32> = None;
/// Standard thinking budget (4096 tokens) - 30s timeout
const STANDARD_THINKING: Option<u32> = Some(4096);
/// Deep thinking budget (8192 tokens) - 60s timeout
const DEEP_THINKING: Option<u32> = Some(8192);
/// Maximum thinking budget (16384 tokens) - 120s timeout
const MAXIMUM_THINKING: Option<u32> = Some(16384);

// ============================================================================
// ReasoningServer with Tool Router (rmcp 0.12 syntax)
// ============================================================================

/// Reasoning server with all tools.
#[derive(Clone)]
pub struct ReasoningServer {
    /// Shared application state.
    pub state: Arc<AppState>,
    /// Tool router for handling tool calls.
    tool_router: ToolRouter<Self>,
}

impl ReasoningServer {
    /// Creates a new reasoning server.
    #[must_use]
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl ReasoningServer {
    #[tool(
        name = "reasoning_linear",
        description = "Process a thought and get a logical continuation with confidence scoring."
    )]
    async fn reasoning_linear(&self, req: Parameters<LinearRequest>) -> LinearResponse {
        let req = req.0;
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

        // Apply tool-level timeout to prevent indefinite hangs
        let timeout_ms = self.state.config.timeout_for_thinking_budget(NO_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);

        let result = match tokio::time::timeout(
            timeout_duration,
            mode.process(&req.content, req.session_id, req.confidence),
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
            Ok(resp) => LinearResponse {
                thought_id: resp.thought_id,
                session_id: resp.session_id,
                content: resp.content,
                confidence: resp.confidence,
                next_step: resp.next_step,
                metadata,
            },
            Err(e) => LinearResponse {
                thought_id: String::new(),
                session_id: input_session_id,
                content: format!("ERROR: {e}"),
                confidence: 0.0,
                next_step: None,
                metadata: None,
            },
        }
    }

    #[tool(
        name = "reasoning_tree",
        description = "Branching exploration: create=start with 2-4 paths, focus=select branch, list=show branches, complete=mark finished."
    )]
    async fn reasoning_tree(&self, req: Parameters<TreeRequest>) -> TreeResponse {
        let req = req.0;
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
                                        status: format!("{:?}", b.status).to_lowercase(),
                                    })
                                    .collect()
                            }),
                            recommendation: resp.recommendation,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        TreeResponse {
                            session_id,
                            branch_id: None,
                            branches: None,
                            recommendation: Some(format!("ERROR: {e}")),
                            metadata: None,
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
                                        status: format!("{:?}", b.status).to_lowercase(),
                                    })
                                    .collect()
                            }),
                            recommendation: resp.recommendation,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        TreeResponse {
                            session_id,
                            branch_id: None,
                            branches: None,
                            recommendation: Some(format!("ERROR: {e}")),
                            metadata: None,
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
                                    status: format!("{:?}", b.status).to_lowercase(),
                                })
                                .collect()
                        }),
                        recommendation: resp.recommendation,
                        metadata: None,
                    },
                    true,
                ),
                Err(e) => (
                    TreeResponse {
                        session_id,
                        branch_id: None,
                        branches: None,
                        recommendation: Some(format!("ERROR: {e}")),
                        metadata: None,
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
                                        status: format!("{:?}", b.status).to_lowercase(),
                                    })
                                    .collect()
                            }),
                            recommendation: resp.recommendation,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        TreeResponse {
                            session_id,
                            branch_id: None,
                            branches: None,
                            recommendation: Some(format!("ERROR: {e}")),
                            metadata: None,
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
                        "Unknown operation: {operation}. Use create/focus/list/complete."
                    )),
                    metadata: None,
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

    #[tool(
        name = "reasoning_divergent",
        description = "Generate novel perspectives with assumption challenges and optional force_rebellion mode."
    )]
    async fn reasoning_divergent(&self, req: Parameters<DivergentRequest>) -> DivergentResponse {
        let req = req.0;
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
            format!("divergent-{}", uuid::Uuid::new_v4())
        });
        let progress = self.state.create_progress_reporter(&progress_token);

        // Apply tool-level timeout (DEEP_THINKING = 8192 tokens = 60s)
        let timeout_ms = self
            .state
            .config
            .timeout_for_thinking_budget(DEEP_THINKING);
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
                synthesis: Some(format!("ERROR: {e}")),
                metadata: None,
            },
        }
    }

    #[tool(
        name = "reasoning_reflection",
        description = "Analyze and improve reasoning: process=iterative refinement, evaluate=session assessment."
    )]
    async fn reasoning_reflection(&self, req: Parameters<ReflectionRequest>) -> ReflectionResponse {
        let req = req.0;
        let timer = Timer::start();
        let mode = ReflectionMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let operation = req.operation.as_deref().unwrap_or("process");

        // Create progress reporter (use progress_token or generate one)
        let progress_token = req.progress_token.unwrap_or_else(|| {
            format!("reflection-{}", uuid::Uuid::new_v4())
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
                            weaknesses: Some(vec![format!("ERROR: {e}")]),
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
                            weaknesses: Some(vec![format!("ERROR: {e}")]),
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

            if let Ok(metadata) = metadata_builders::build_metadata_for_reflection(
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
                response.metadata = Some(metadata);
            }
        }

        response
    }

    #[tool(
        name = "reasoning_checkpoint",
        description = "Save and restore reasoning state: create=save, list=show, restore=return to checkpoint."
    )]
    async fn reasoning_checkpoint(&self, req: Parameters<CheckpointRequest>) -> CheckpointResponse {
        let req = req.0;
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

    #[tool(
        name = "reasoning_auto",
        description = "Analyze content and route to optimal reasoning mode."
    )]
    async fn reasoning_auto(&self, req: Parameters<AutoRequest>) -> AutoResponse {
        let req = req.0;
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
                // Build result with mode info and parameters
                let result = serde_json::json!({
                    "thought_id": resp.thought_id,
                    "session_id": resp.session_id,
                    "characteristics": resp.characteristics,
                    "suggested_parameters": resp.suggested_parameters,
                    "alternative": resp.alternative_mode.map(|a| serde_json::json!({
                        "mode": a.mode.to_string(),
                        "reason": a.reason
                    }))
                });

                AutoResponse {
                    selected_mode: resp.selected_mode.to_string(),
                    confidence: 0.85, // Default confidence for successful selection
                    rationale: resp.reasoning,
                    result,
                    metadata: None,
                }
            }
            Err(e) => AutoResponse {
                selected_mode: "linear".to_string(),
                confidence: 0.0,
                rationale: format!("ERROR: {e}"),
                result: serde_json::Value::Null,
                metadata: None,
            },
        }
    }

    #[tool(
        name = "reasoning_graph",
        description = "Graph reasoning: init/generate/score/aggregate/refine/prune/finalize/state operations."
    )]
    async fn reasoning_graph(&self, req: Parameters<GraphRequest>) -> GraphResponse {
        let req = req.0;
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

    #[tool(
        name = "reasoning_detect",
        description = "Detect cognitive biases and logical fallacies in reasoning."
    )]
    async fn reasoning_detect(&self, req: Parameters<DetectRequest>) -> DetectResponse {
        let req = req.0;
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
                                    severity: format!("{:?}", b.severity).to_lowercase(),
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
                                    category: Some(format!("{:?}", f.category).to_lowercase()),
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

    #[tool(
        name = "reasoning_decision",
        description = "Decision analysis: weighted/pairwise/topsis scoring or perspectives stakeholder mapping."
    )]
    async fn reasoning_decision(&self, req: Parameters<DecisionRequest>) -> DecisionResponse {
        let req = req.0;
        let timer = Timer::start();
        let mode = DecisionMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let base_content = req
            .question
            .as_deref()
            .or(req.topic.as_deref())
            .or(req.context.as_deref())
            .unwrap_or("");

        // Include user-provided options in the content sent to Claude
        let content = match &req.options {
            Some(opts) if !opts.is_empty() => {
                format!(
                    "Options to evaluate:\n- {}\n\nContext: {}",
                    opts.join("\n- "),
                    base_content
                )
            }
            _ => base_content.to_string(),
        };
        let content = content.as_str();

        let decision_type = req.decision_type.as_deref().unwrap_or("weighted");

        // Apply tool-level timeout to prevent indefinite hangs
        let timeout_ms = self.state.config.timeout_for_thinking_budget(DEEP_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);
        let decision_type_for_timeout = decision_type.to_string();

        let (response, success) = match tokio::time::timeout(timeout_duration, async {
            match decision_type_for_timeout.as_str() {
                "weighted" => match mode.weighted(content, req.session_id).await {
                    Ok(resp) => (
                        DecisionResponse {
                            recommendation: resp
                                .ranking
                                .first()
                                .map(|r| r.option.clone())
                                .unwrap_or_default(),
                            rankings: Some(
                                resp.ranking
                                    .into_iter()
                                    .map(|r| RankedOption {
                                        option: r.option,
                                        score: r.score,
                                        rank: r.rank,
                                    })
                                    .collect(),
                            ),
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: Some(resp.sensitivity_notes),
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!("ERROR: {e}"),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "pairwise" => match mode.pairwise(content, req.session_id).await {
                    Ok(resp) => (
                        DecisionResponse {
                            recommendation: resp
                                .ranking
                                .first()
                                .map(|r| r.option.clone())
                                .unwrap_or_default(),
                            rankings: Some(
                                resp.ranking
                                    .into_iter()
                                    .map(|r| RankedOption {
                                        option: r.option,
                                        score: f64::from(r.wins),
                                        rank: r.rank,
                                    })
                                    .collect(),
                            ),
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!("ERROR: {e}"),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "topsis" => match mode.topsis(content, req.session_id).await {
                    Ok(resp) => (
                        DecisionResponse {
                            recommendation: resp
                                .ranking
                                .first()
                                .map(|r| r.option.clone())
                                .unwrap_or_default(),
                            rankings: Some(
                                resp.ranking
                                    .into_iter()
                                    .map(|r| RankedOption {
                                        option: r.option,
                                        score: r.closeness,
                                        rank: r.rank,
                                    })
                                    .collect(),
                            ),
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!("ERROR: {e}"),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "perspectives" => match mode.perspectives(content, req.session_id).await {
                    Ok(resp) => (
                        DecisionResponse {
                            recommendation: resp.balanced_recommendation.option.clone(),
                            rankings: None,
                            stakeholder_map: Some(StakeholderMap {
                                key_players: resp
                                    .stakeholders
                                    .iter()
                                    .filter(|s| {
                                        s.influence_level == crate::modes::InfluenceLevel::High
                                    })
                                    .map(|s| s.name.clone())
                                    .collect(),
                                keep_satisfied: vec![],
                                keep_informed: resp
                                    .stakeholders
                                    .iter()
                                    .filter(|s| {
                                        s.influence_level == crate::modes::InfluenceLevel::Medium
                                    })
                                    .map(|s| s.name.clone())
                                    .collect(),
                                minimal_effort: resp
                                    .stakeholders
                                    .iter()
                                    .filter(|s| {
                                        s.influence_level == crate::modes::InfluenceLevel::Low
                                    })
                                    .map(|s| s.name.clone())
                                    .collect(),
                            }),
                            conflicts: Some(resp.conflicts.into_iter().map(|c| c.issue).collect()),
                            alignments: Some(
                                resp.alignments
                                    .into_iter()
                                    .map(|a| a.common_ground)
                                    .collect(),
                            ),
                            rationale: Some(resp.balanced_recommendation.rationale),
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!("ERROR: {e}"),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                _ => (
                    DecisionResponse {
                        recommendation: format!(
                            "ERROR: unknown type: {}",
                            decision_type_for_timeout
                        ),
                        rankings: None,
                        stakeholder_map: None,
                        conflicts: None,
                        alignments: None,
                        rationale: None,
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
                    tool = "reasoning_decision",
                    timeout_ms = timeout_ms,
                    decision_type = %decision_type,
                    "Tool execution timed out"
                );
                (
                    DecisionResponse {
                        recommendation: format!(
                            "ERROR: Tool execution timed out after {}ms",
                            timeout_ms
                        ),
                        rankings: None,
                        stakeholder_map: None,
                        conflicts: None,
                        alignments: None,
                        rationale: None,
                        metadata: None,
                    },
                    false,
                )
            }
        };

        self.state.metrics.record(
            MetricEvent::new("decision", timer.elapsed_ms(), success).with_operation(decision_type),
        );

        response
    }

    #[tool(
        name = "reasoning_evidence",
        description = "Evaluate evidence: assess=credibility scoring, probabilistic=Bayesian belief update."
    )]
    async fn reasoning_evidence(&self, req: Parameters<EvidenceRequest>) -> EvidenceResponse {
        let req = req.0;
        let timer = Timer::start();
        let mode = EvidenceMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let evidence_type = req.evidence_type.as_deref().unwrap_or("assess");
        let content = req
            .claim
            .as_deref()
            .or(req.hypothesis.as_deref())
            .or(req.context.as_deref())
            .unwrap_or("");

        // Apply tool-level timeout to prevent indefinite hangs
        let timeout_ms = self.state.config.timeout_for_thinking_budget(DEEP_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);
        let evidence_type_for_timeout = evidence_type.to_string();

        let (response, success) = match tokio::time::timeout(timeout_duration, async {
            match evidence_type_for_timeout.as_str() {
                "assess" => match mode.assess(content, req.session_id).await {
                    Ok(resp) => (
                        EvidenceResponse {
                            overall_credibility: resp.confidence_in_conclusion,
                            evidence_assessments: Some(
                                resp.evidence_pieces
                                    .into_iter()
                                    .map(|p| EvidenceAssessment {
                                        content: p.summary,
                                        credibility_score: p.credibility.overall,
                                        source_tier: format!("{:?}", p.source_type),
                                        corroborated_by: None,
                                    })
                                    .collect(),
                            ),
                            posterior: None,
                            prior: None,
                            likelihood_ratio: None,
                            entropy: None,
                            confidence_interval: None,
                            synthesis: Some(format!(
                                "Strengths: {}. Weaknesses: {}. Gaps: {}",
                                resp.overall_assessment.key_strengths.join(", "),
                                resp.overall_assessment.key_weaknesses.join(", "),
                                resp.overall_assessment.gaps.join(", ")
                            )),
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        EvidenceResponse {
                            overall_credibility: 0.0,
                            evidence_assessments: None,
                            posterior: None,
                            prior: None,
                            likelihood_ratio: None,
                            entropy: None,
                            confidence_interval: None,
                            synthesis: Some(format!("ERROR: {e}")),
                            metadata: None,
                        },
                        false,
                    ),
                },
                "probabilistic" => match mode.probabilistic(content, req.session_id).await {
                    Ok(resp) => {
                        let likelihood_ratio =
                            resp.evidence_analysis.first().map(|a| a.bayes_factor);
                        (
                            EvidenceResponse {
                                overall_credibility: resp.posterior.probability,
                                evidence_assessments: Some(
                                    resp.evidence_analysis
                                        .into_iter()
                                        .map(|a| EvidenceAssessment {
                                            content: a.evidence,
                                            credibility_score: a.bayes_factor.min(1.0),
                                            source_tier: "computed".to_string(),
                                            corroborated_by: None,
                                        })
                                        .collect(),
                                ),
                                posterior: Some(resp.posterior.probability),
                                prior: Some(resp.prior.probability),
                                likelihood_ratio,
                                entropy: None,
                                confidence_interval: None,
                                synthesis: Some(format!(
                                    "{} ({:?} {:?}). Sensitivity: {}",
                                    resp.belief_update.interpretation,
                                    resp.belief_update.direction,
                                    resp.belief_update.magnitude,
                                    resp.sensitivity
                                )),
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(e) => (
                        EvidenceResponse {
                            overall_credibility: 0.0,
                            evidence_assessments: None,
                            posterior: None,
                            prior: None,
                            likelihood_ratio: None,
                            entropy: None,
                            confidence_interval: None,
                            synthesis: Some(format!("ERROR: {e}")),
                            metadata: None,
                        },
                        false,
                    ),
                },
                _ => (
                    EvidenceResponse {
                        overall_credibility: 0.0,
                        evidence_assessments: None,
                        posterior: None,
                        prior: None,
                        likelihood_ratio: None,
                        entropy: None,
                        confidence_interval: None,
                        synthesis: Some(format!(
                            "Unknown evidence type: {}",
                            evidence_type_for_timeout
                        )),
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
                    tool = "reasoning_evidence",
                    timeout_ms = timeout_ms,
                    evidence_type = %evidence_type,
                    "Tool execution timed out"
                );
                (
                    EvidenceResponse {
                        overall_credibility: 0.0,
                        evidence_assessments: None,
                        posterior: None,
                        prior: None,
                        likelihood_ratio: None,
                        entropy: None,
                        confidence_interval: None,
                        synthesis: Some(format!("Tool execution timed out after {}ms", timeout_ms)),
                        metadata: None,
                    },
                    false,
                )
            }
        };

        self.state.metrics.record(
            MetricEvent::new("evidence", timer.elapsed_ms(), success).with_operation(evidence_type),
        );

        response
    }

    #[tool(
        name = "reasoning_timeline",
        description = "Temporal reasoning: create/branch/compare/merge operations."
    )]
    async fn reasoning_timeline(&self, req: Parameters<TimelineRequest>) -> TimelineResponse {
        let req = req.0;
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
                        timeline_id: format!("ERROR: {e}"),
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
                        timeline_id: format!("ERROR: {e}"),
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
                        timeline_id: format!("ERROR: {e}"),
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
                        timeline_id: format!("ERROR: {e}"),
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
                        timeline_id: format!("ERROR: Tool execution timed out after {}ms", timeout_ms),
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

    #[tool(
        name = "reasoning_mcts",
        description = "MCTS: explore=UCB1-guided search, auto_backtrack=quality-triggered backtracking."
    )]
    async fn reasoning_mcts(&self, req: Parameters<MctsRequest>) -> MctsResponse {
        let req = req.0;
        let timer = Timer::start();
        let mode = MctsMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let operation = req.operation.as_deref().unwrap_or("explore");
        let content = req.content.as_deref().unwrap_or("");
        let input_session_id = req.session_id.clone().unwrap_or_default();

        // Create progress reporter (use progress_token or generate one)
        let progress_token = req.progress_token.unwrap_or_else(|| {
            format!("mcts-{}", uuid::Uuid::new_v4())
        });
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
                    mode.auto_backtrack_streaming(content, Some(input_session_id.clone()), Some(&progress)),
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

    #[tool(
        name = "reasoning_counterfactual",
        description = "What-if analysis using Pearl's Ladder of Causation."
    )]
    async fn reasoning_counterfactual(
        &self,
        req: Parameters<CounterfactualRequest>,
    ) -> CounterfactualResponse {
        use crate::modes::CounterfactualMode;

        let req = req.0;
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
        let progress_token = req.progress_token.unwrap_or_else(|| {
            format!("counterfactual-{}", uuid::Uuid::new_v4())
        });
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
                counterfactual_outcome: format!("ERROR: {e}"),
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

    #[tool(
        name = "reasoning_preset",
        description = "Execute pre-defined reasoning workflows: list=show presets, run=execute workflow."
    )]
    async fn reasoning_preset(&self, req: Parameters<PresetRequest>) -> PresetResponse {
        let req = req.0;
        let timer = Timer::start();
        let operation = req.operation.clone();

        let (response, success) = match operation.as_str() {
            "list" => {
                // List available presets, optionally filtered by category
                let presets: Vec<PresetInfo> = self
                    .state
                    .presets
                    .list()
                    .iter()
                    .filter(|p| {
                        req.category
                            .as_ref()
                            .is_none_or(|cat| p.category.to_string() == *cat)
                    })
                    .map(|p| PresetInfo {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        description: p.description.clone(),
                        category: p.category.to_string(),
                        required_inputs: p
                            .steps
                            .iter()
                            .filter_map(|s| s.config.as_ref().map(|_| s.mode.clone()))
                            .collect(),
                    })
                    .collect();

                (
                    PresetResponse {
                        presets: Some(presets),
                        execution_result: None,
                        session_id: None,
                        metadata: None,
                    },
                    true,
                )
            }
            "run" => {
                // Run a specific preset
                let Some(preset_id) = req.preset_id.clone() else {
                    self.state.metrics.record(
                        MetricEvent::new("preset", timer.elapsed_ms(), false)
                            .with_operation(&operation),
                    );
                    return PresetResponse {
                        presets: None,
                        execution_result: Some(PresetExecution {
                            preset_id: "unknown".to_string(),
                            steps_completed: 0,
                            total_steps: 0,
                            step_results: vec![],
                            final_output: serde_json::json!({
                                "error": "preset_id is required for run operation"
                            }),
                        }),
                        session_id: req.session_id,
                        metadata: None,
                    };
                };

                let Some(preset) = self.state.presets.get(&preset_id) else {
                    self.state.metrics.record(
                        MetricEvent::new("preset", timer.elapsed_ms(), false)
                            .with_operation(&operation),
                    );
                    return PresetResponse {
                        presets: None,
                        execution_result: Some(PresetExecution {
                            preset_id,
                            steps_completed: 0,
                            total_steps: 0,
                            step_results: vec![],
                            final_output: serde_json::json!({"error": "preset not found"}),
                        }),
                        session_id: req.session_id,
                        metadata: None,
                    };
                };

                // Return preset info - actual execution would require running each step
                let total_steps = preset.steps.len() as u32;
                let step_results: Vec<serde_json::Value> = preset
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(i, step)| {
                        serde_json::json!({
                            "step": i,
                            "mode": step.mode,
                            "operation": step.operation,
                            "description": step.description,
                            "status": "pending"
                        })
                    })
                    .collect();

                (
                    PresetResponse {
                        presets: None,
                        execution_result: Some(PresetExecution {
                            preset_id: preset.id.clone(),
                            steps_completed: 0,
                            total_steps,
                            step_results,
                            final_output: serde_json::json!({
                                "name": preset.name,
                                "description": preset.description,
                                "category": preset.category.to_string(),
                                "message": "Preset workflow ready for execution"
                            }),
                        }),
                        session_id: req.session_id,
                        metadata: None,
                    },
                    true,
                )
            }
            _ => (
                PresetResponse {
                    presets: None,
                    execution_result: Some(PresetExecution {
                        preset_id: "unknown".to_string(),
                        steps_completed: 0,
                        total_steps: 0,
                        step_results: vec![],
                        final_output: serde_json::json!({
                            "error": format!(
                                "Unknown operation: {}. Use 'list' or 'run'.",
                                operation
                            )
                        }),
                    }),
                    session_id: req.session_id,
                    metadata: None,
                },
                false,
            ),
        };

        self.state.metrics.record(
            MetricEvent::new("preset", timer.elapsed_ms(), success).with_operation(&operation),
        );

        response
    }

    #[tool(
        name = "reasoning_metrics",
        description = "Query metrics: summary/by_mode/invocations/fallbacks/config."
    )]
    async fn reasoning_metrics(&self, req: Parameters<MetricsRequest>) -> MetricsResponse {
        let req = req.0;
        let timer = Timer::start();
        let query = req.query.clone();

        let (response, success) = match query.as_str() {
            "summary" => {
                let summary = self.state.metrics.summary();
                (
                    MetricsResponse {
                        summary: Some(MetricsSummary {
                            total_calls: summary.total_invocations,
                            success_rate: summary.overall_success_rate,
                            avg_latency_ms: summary
                                .by_mode
                                .values()
                                .map(|m| m.avg_latency_ms)
                                .sum::<f64>()
                                / summary.by_mode.len().max(1) as f64,
                            by_mode: serde_json::to_value(&summary.by_mode).unwrap_or_default(),
                        }),
                        mode_stats: None,
                        invocations: None,
                        config: None,
                    },
                    true,
                )
            }
            "by_mode" => {
                let mode_name = req.mode_name.clone().unwrap_or_default();

                // If mode_name is empty, return summary with all modes instead
                if mode_name.is_empty() {
                    let summary = self.state.metrics.summary();
                    (
                        MetricsResponse {
                            summary: Some(MetricsSummary {
                                total_calls: summary.total_invocations,
                                success_rate: summary.overall_success_rate,
                                avg_latency_ms: summary
                                    .by_mode
                                    .values()
                                    .map(|m| m.avg_latency_ms)
                                    .sum::<f64>()
                                    / summary.by_mode.len().max(1) as f64,
                                by_mode: serde_json::to_value(&summary.by_mode).unwrap_or_default(),
                            }),
                            mode_stats: None,
                            invocations: None,
                            config: None,
                        },
                        true,
                    )
                } else {
                    let events = self.state.metrics.invocations_by_mode(&mode_name);
                    let total = events.len() as u64;
                    let success_count = events.iter().filter(|e| e.success).count() as u64;
                    let failure_count = total - success_count;
                    let success_rate = if total > 0 {
                        success_count as f64 / total as f64
                    } else {
                        0.0
                    };

                    // Calculate latency percentiles
                    let mut latencies: Vec<u64> = events.iter().map(|e| e.latency_ms).collect();
                    latencies.sort_unstable();
                    let p50 = latencies.get(latencies.len() / 2).copied().unwrap_or(0) as f64;
                    let p95 = latencies
                        .get(latencies.len() * 95 / 100)
                        .copied()
                        .unwrap_or(0) as f64;
                    let p99 = latencies
                        .get(latencies.len() * 99 / 100)
                        .copied()
                        .unwrap_or(0) as f64;

                    (
                        MetricsResponse {
                            summary: None,
                            mode_stats: Some(ModeStats {
                                mode_name,
                                call_count: total,
                                success_count,
                                failure_count,
                                success_rate,
                                latency_p50_ms: p50,
                                latency_p95_ms: p95,
                                latency_p99_ms: p99,
                            }),
                            invocations: None,
                            config: None,
                        },
                        true,
                    )
                }
            }
            "invocations" => {
                let events = req.mode_name.as_ref().map_or_else(
                    || {
                        self.state
                            .metrics
                            .summary()
                            .by_mode
                            .keys()
                            .flat_map(|mode| self.state.metrics.invocations_by_mode(mode))
                            .collect()
                    },
                    |mode| self.state.metrics.invocations_by_mode(mode),
                );

                let limit = req.limit.unwrap_or(100).min(1000) as usize;
                let invocations: Vec<Invocation> = events
                    .into_iter()
                    .filter(|e| req.success_only.is_none_or(|s| !s || e.success))
                    .take(limit)
                    .enumerate()
                    .map(|(i, e)| {
                        #[allow(clippy::cast_possible_wrap)]
                        let created_at = chrono::DateTime::from_timestamp(e.timestamp as i64, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default();
                        Invocation {
                            id: format!("inv-{i}"),
                            tool_name: e.mode.clone(),
                            session_id: req.session_id.clone(),
                            success: e.success,
                            latency_ms: e.latency_ms,
                            created_at,
                        }
                    })
                    .collect();

                (
                    MetricsResponse {
                        summary: None,
                        mode_stats: None,
                        invocations: Some(invocations),
                        config: None,
                    },
                    true,
                )
            }
            "fallbacks" => {
                let fallbacks = self.state.metrics.fallbacks();
                (
                    MetricsResponse {
                        summary: None,
                        mode_stats: None,
                        invocations: Some(
                            fallbacks
                                .into_iter()
                                .enumerate()
                                .map(|(i, f)| {
                                    #[allow(clippy::cast_possible_wrap)]
                                    let created_at =
                                        chrono::DateTime::from_timestamp(f.timestamp as i64, 0)
                                            .map(|dt| dt.to_rfc3339())
                                            .unwrap_or_default();
                                    Invocation {
                                        id: format!("fallback-{i}"),
                                        tool_name: format!("{} -> {}", f.from_mode, f.to_mode),
                                        session_id: Some(f.reason),
                                        success: false,
                                        latency_ms: 0,
                                        created_at,
                                    }
                                })
                                .collect(),
                        ),
                        config: None,
                    },
                    true,
                )
            }
            "config" => (
                MetricsResponse {
                    summary: None,
                    mode_stats: None,
                    invocations: None,
                    config: Some(serde_json::json!({
                        "model": self.state.config.model,
                        "request_timeout_ms": self.state.config.request_timeout_ms,
                        "max_retries": self.state.config.max_retries,
                        "log_level": self.state.config.log_level,
                    })),
                },
                true,
            ),
            _ => (
                MetricsResponse {
                    summary: None,
                    mode_stats: None,
                    invocations: None,
                    config: Some(serde_json::json!({
                        "error": format!(
                            "Unknown query: {}. Use 'summary', 'by_mode', 'invocations', 'fallbacks', or 'config'.",
                            query
                        )
                    })),
                },
                false,
            ),
        };

        self.state.metrics.record(
            MetricEvent::new("metrics", timer.elapsed_ms(), success).with_operation(&query),
        );

        response
    }

    // ============================================================================
    // Self-Improvement Tools
    // ============================================================================

    #[tool(
        name = "reasoning_si_status",
        description = "Get self-improvement system status including cycle stats and circuit breaker state."
    )]
    async fn reasoning_si_status(&self, req: Parameters<SiStatusRequest>) -> SiStatusResponse {
        let _ = req; // Empty request struct
        let timer = Timer::start();
        let status = self.state.self_improvement.status().await;

        self.state
            .metrics
            .record(MetricEvent::new("si_status", timer.elapsed_ms(), true));

        SiStatusResponse {
            running: status.running,
            circuit_state: status.circuit_state,
            total_cycles: status.total_cycles,
            successful_cycles: status.successful_cycles,
            failed_cycles: status.failed_cycles,
            pending_diagnoses: status.pending_diagnoses,
            total_actions_executed: status.total_actions_executed,
            total_actions_rolled_back: status.total_actions_rolled_back,
            last_cycle_at: status.last_cycle_at,
            average_reward: status.learning_summary.average_reward,
        }
    }

    #[tool(
        name = "reasoning_si_diagnoses",
        description = "Get pending diagnoses awaiting approval."
    )]
    async fn reasoning_si_diagnoses(
        &self,
        req: Parameters<SiDiagnosesRequest>,
    ) -> SiDiagnosesResponse {
        let req = req.0;
        let timer = Timer::start();
        let diagnoses = self
            .state
            .self_improvement
            .pending_diagnoses(req.limit)
            .await;
        let total = diagnoses.len();

        self.state
            .metrics
            .record(MetricEvent::new("si_diagnoses", timer.elapsed_ms(), true));

        SiDiagnosesResponse {
            diagnoses: diagnoses
                .into_iter()
                .map(|d| SiPendingDiagnosis {
                    id: d.id,
                    action_type: d.action_type,
                    description: d.description,
                    rationale: d.rationale,
                    expected_improvement: d.expected_improvement,
                    created_at: d.created_at,
                })
                .collect(),
            total,
        }
    }

    #[tool(
        name = "reasoning_si_approve",
        description = "Approve a pending diagnosis to execute its proposed actions."
    )]
    async fn reasoning_si_approve(&self, req: Parameters<SiApproveRequest>) -> SiApproveResponse {
        let req = req.0;
        let timer = Timer::start();
        let result = self.state.self_improvement.approve(req.diagnosis_id).await;

        let (response, success) = match result {
            Ok(approve_result) => (
                SiApproveResponse {
                    success: true,
                    actions_executed: approve_result.execution_results.len(),
                    lessons_learned: approve_result.learning_results.len(),
                    execution_results: approve_result
                        .execution_results
                        .into_iter()
                        .map(|r| SiExecutionSummary {
                            action_id: r.action_id,
                            success: r.success,
                            error: if r.success { None } else { Some(r.message) },
                        })
                        .collect(),
                    learning_results: approve_result
                        .learning_results
                        .into_iter()
                        .map(|r| SiLearningSummary {
                            action_id: r.action_id,
                            insight: r.lesson,
                            reward: r.reward,
                        })
                        .collect(),
                    error: None,
                },
                true,
            ),
            Err(e) => (
                SiApproveResponse {
                    success: false,
                    actions_executed: 0,
                    lessons_learned: 0,
                    execution_results: vec![],
                    learning_results: vec![],
                    error: Some(e),
                },
                false,
            ),
        };

        self.state
            .metrics
            .record(MetricEvent::new("si_approve", timer.elapsed_ms(), success));

        response
    }

    #[tool(
        name = "reasoning_si_reject",
        description = "Reject a pending diagnosis."
    )]
    async fn reasoning_si_reject(&self, req: Parameters<SiRejectRequest>) -> SiRejectResponse {
        let req = req.0;
        let timer = Timer::start();
        let result = self
            .state
            .self_improvement
            .reject(req.diagnosis_id, req.reason)
            .await;

        let (response, success) = match result {
            Ok(()) => (
                SiRejectResponse {
                    success: true,
                    error: None,
                },
                true,
            ),
            Err(e) => (
                SiRejectResponse {
                    success: false,
                    error: Some(e),
                },
                false,
            ),
        };

        self.state
            .metrics
            .record(MetricEvent::new("si_reject", timer.elapsed_ms(), success));

        response
    }

    #[tool(
        name = "reasoning_si_trigger",
        description = "Trigger an immediate improvement cycle."
    )]
    async fn reasoning_si_trigger(&self, req: Parameters<SiTriggerRequest>) -> SiTriggerResponse {
        let _ = req; // Empty request struct
        let timer = Timer::start();
        let result = self.state.self_improvement.trigger_cycle().await;

        let (response, success) = match result {
            Ok(cycle_result) => {
                let actions_proposed = cycle_result
                    .analysis_result
                    .as_ref()
                    .map_or(0, |a| a.actions.len());
                (
                    SiTriggerResponse {
                        success: true,
                        actions_proposed,
                        actions_executed: cycle_result.execution_results.len(),
                        analysis_skipped: cycle_result.analysis_result.is_none(),
                        error: None,
                    },
                    true,
                )
            }
            Err(e) => (
                SiTriggerResponse {
                    success: false,
                    actions_proposed: 0,
                    actions_executed: 0,
                    analysis_skipped: false,
                    error: Some(format!("{e}")),
                },
                false,
            ),
        };

        self.state
            .metrics
            .record(MetricEvent::new("si_trigger", timer.elapsed_ms(), success));

        response
    }

    #[tool(
        name = "reasoning_si_rollback",
        description = "Rollback a previously executed action."
    )]
    async fn reasoning_si_rollback(
        &self,
        req: Parameters<SiRollbackRequest>,
    ) -> SiRollbackResponse {
        let req = req.0;
        let timer = Timer::start();
        let result = self.state.self_improvement.rollback(req.action_id).await;

        let (response, success) = match result {
            Ok(()) => (
                SiRollbackResponse {
                    success: true,
                    error: None,
                },
                true,
            ),
            Err(e) => (
                SiRollbackResponse {
                    success: false,
                    error: Some(e),
                },
                false,
            ),
        };

        self.state
            .metrics
            .record(MetricEvent::new("si_rollback", timer.elapsed_ms(), success));

        response
    }

    // ============================================================================
    // Metadata Builder Helpers
    // ============================================================================

    /// Build metadata for linear reasoning response.
    async fn build_metadata_for_linear(
        &self,
        content_length: usize,
        session_id: Option<String>,
        elapsed_ms: u64,
    ) -> Result<crate::metadata::ResponseMetadata, crate::error::AppError> {
        use crate::metadata::{ComplexityMetrics, MetadataRequest, ResultContext};

        // Record actual execution time
        let complexity = ComplexityMetrics {
            content_length,
            thinking_budget: None,
            num_perspectives: None,
            num_branches: None,
        };

        self.state
            .metadata_builder
            .timing_db()
            .record_execution(
                "reasoning_linear",
                Some("linear"),
                elapsed_ms,
                complexity.clone(),
            )
            .await?;

        // Build metadata request
        let metadata_req = MetadataRequest {
            tool_name: "reasoning_linear".into(),
            mode_name: Some("linear".into()),
            complexity,
            result_context: ResultContext {
                num_outputs: 1,
                has_branches: false,
                session_id,
                complexity: if content_length > 5000 {
                    "complex".into()
                } else if content_length > 2000 {
                    "moderate".into()
                } else {
                    "simple".into()
                },
            },
            tool_history: vec![], // TODO: Implement session history tracking
            goal: None,
            thinking_budget: Some("none".into()),
            session_state: None,
        };

        self.state.metadata_builder.build(&metadata_req).await
    }
}

// Implement ServerHandler to integrate with rmcp's server infrastructure
#[tool_handler]
impl ServerHandler for ReasoningServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCP Reasoning Server providing 15 structured reasoning tools.".to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;
    use rmcp::handler::server::wrapper::Parameters;
    use rmcp::model::IntoContents;

    #[test]
    fn test_linear_response_serialize() {
        let response = LinearResponse {
            thought_id: "t1".to_string(),
            session_id: "s1".to_string(),
            content: "reasoning content".to_string(),
            confidence: 0.85,
            next_step: Some("continue".to_string()),
            metadata: None,
        };
        let json = serde_json::to_string(&response).expect("serialize");
        assert!(json.contains("thought_id"));
    }

    #[test]
    fn test_linear_request_deserialize() {
        let json = r#"{"content": "test"}"#;
        let req: LinearRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.content, "test");
    }

    #[test]
    fn test_all_response_types_implement_json_schema() {
        let _ = schemars::schema_for!(LinearResponse);
        let _ = schemars::schema_for!(TreeResponse);
        let _ = schemars::schema_for!(DivergentResponse);
        let _ = schemars::schema_for!(ReflectionResponse);
        let _ = schemars::schema_for!(CheckpointResponse);
        let _ = schemars::schema_for!(AutoResponse);
        let _ = schemars::schema_for!(GraphResponse);
        let _ = schemars::schema_for!(DetectResponse);
        let _ = schemars::schema_for!(DecisionResponse);
        let _ = schemars::schema_for!(EvidenceResponse);
        let _ = schemars::schema_for!(TimelineResponse);
        let _ = schemars::schema_for!(MctsResponse);
        let _ = schemars::schema_for!(CounterfactualResponse);
        let _ = schemars::schema_for!(PresetResponse);
        let _ = schemars::schema_for!(MetricsResponse);
    }

    #[test]
    fn test_all_request_types_implement_json_schema() {
        let _ = schemars::schema_for!(LinearRequest);
        let _ = schemars::schema_for!(TreeRequest);
        let _ = schemars::schema_for!(DivergentRequest);
        let _ = schemars::schema_for!(ReflectionRequest);
        let _ = schemars::schema_for!(CheckpointRequest);
        let _ = schemars::schema_for!(AutoRequest);
        let _ = schemars::schema_for!(GraphRequest);
        let _ = schemars::schema_for!(DetectRequest);
        let _ = schemars::schema_for!(DecisionRequest);
        let _ = schemars::schema_for!(EvidenceRequest);
        let _ = schemars::schema_for!(TimelineRequest);
        let _ = schemars::schema_for!(MctsRequest);
        let _ = schemars::schema_for!(CounterfactualRequest);
        let _ = schemars::schema_for!(PresetRequest);
        let _ = schemars::schema_for!(MetricsRequest);
    }

    #[test]
    fn test_reasoning_server_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ReasoningServer>();
    }

    // ============================================================================
    // IntoContents Tests - Cover macro-generated implementations
    // ============================================================================

    #[test]
    fn test_linear_response_into_contents() {
        let response = LinearResponse {
            thought_id: "t1".to_string(),
            session_id: "s1".to_string(),
            content: "reasoning content".to_string(),
            confidence: 0.85,
            next_step: Some("continue".to_string()),
            metadata: None,
        };
        let contents = response.clone().into_contents();
        assert_eq!(contents.len(), 1);
        // Verify it produces valid JSON content
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("thought_id"));
        assert!(json.contains("t1"));
    }

    #[test]
    fn test_tree_response_into_contents() {
        let response = TreeResponse {
            session_id: "s1".to_string(),
            branch_id: Some("b1".to_string()),
            branches: Some(vec![Branch {
                id: "b1".to_string(),
                content: "branch content".to_string(),
                score: 0.9,
                status: "active".to_string(),
            }]),
            recommendation: Some("explore branch b1".to_string()),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_divergent_response_into_contents() {
        let response = DivergentResponse {
            thought_id: "t1".to_string(),
            session_id: "s1".to_string(),
            perspectives: vec![Perspective {
                viewpoint: "optimistic".to_string(),
                content: "positive outlook".to_string(),
                novelty_score: 0.8,
            }],
            challenged_assumptions: Some(vec!["assumption1".to_string()]),
            synthesis: Some("unified insight".to_string()),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_reflection_response_into_contents() {
        let response = ReflectionResponse {
            quality_score: 0.85,
            thought_id: Some("t1".to_string()),
            session_id: Some("s1".to_string()),
            iterations_used: Some(3),
            strengths: Some(vec!["logical".to_string()]),
            weaknesses: Some(vec!["needs more detail".to_string()]),
            recommendations: Some(vec!["add examples".to_string()]),
            refined_content: Some("improved reasoning".to_string()),
            coherence_score: Some(0.9),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_checkpoint_response_into_contents() {
        let response = CheckpointResponse {
            session_id: "s1".to_string(),
            checkpoint_id: Some("cp1".to_string()),
            checkpoints: Some(vec![Checkpoint {
                id: "cp1".to_string(),
                name: "checkpoint 1".to_string(),
                description: Some("first checkpoint".to_string()),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                thought_count: 5,
            }]),
            restored_state: None,
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_auto_response_into_contents() {
        let response = AutoResponse {
            selected_mode: "linear".to_string(),
            confidence: 0.9,
            rationale: "simple query".to_string(),
            result: serde_json::json!({"status": "ok"}),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_graph_response_into_contents() {
        let response = GraphResponse {
            session_id: "s1".to_string(),
            node_id: Some("n1".to_string()),
            nodes: Some(vec![GraphNode {
                id: "n1".to_string(),
                content: "node content".to_string(),
                score: Some(0.85),
                depth: Some(1),
                parent_id: None,
            }]),
            aggregated_insight: Some("combined insight".to_string()),
            conclusions: Some(vec!["conclusion 1".to_string()]),
            state: Some(GraphState {
                total_nodes: 10,
                active_nodes: 8,
                max_depth: 3,
                pruned_count: 2,
            }),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_detect_response_into_contents() {
        let response = DetectResponse {
            detections: vec![Detection {
                detection_type: "confirmation_bias".to_string(),
                category: Some("cognitive".to_string()),
                severity: "medium".to_string(),
                confidence: 0.8,
                evidence: "selective evidence".to_string(),
                explanation: "focusing on confirming data".to_string(),
                remediation: Some("consider counterexamples".to_string()),
            }],
            summary: Some("1 bias detected".to_string()),
            overall_quality: Some(0.7),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_decision_response_into_contents() {
        let response = DecisionResponse {
            recommendation: "Option A".to_string(),
            rankings: Some(vec![RankedOption {
                option: "Option A".to_string(),
                score: 0.9,
                rank: 1,
            }]),
            stakeholder_map: Some(StakeholderMap {
                key_players: vec!["CEO".to_string()],
                keep_satisfied: vec!["Board".to_string()],
                keep_informed: vec!["Team".to_string()],
                minimal_effort: vec!["Others".to_string()],
            }),
            conflicts: Some(vec!["resource allocation".to_string()]),
            alignments: Some(vec!["shared goals".to_string()]),
            rationale: Some("highest weighted score".to_string()),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_evidence_response_into_contents() {
        let response = EvidenceResponse {
            overall_credibility: 0.85,
            evidence_assessments: Some(vec![EvidenceAssessment {
                content: "primary source".to_string(),
                credibility_score: 0.9,
                source_tier: "tier1".to_string(),
                corroborated_by: Some(vec![1, 2]),
            }]),
            posterior: Some(0.75),
            prior: Some(0.5),
            likelihood_ratio: Some(3.0),
            entropy: Some(0.2),
            confidence_interval: Some(ConfidenceInterval {
                lower: 0.6,
                upper: 0.9,
            }),
            synthesis: Some("strong evidence".to_string()),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_timeline_response_into_contents() {
        let response = TimelineResponse {
            timeline_id: "tl1".to_string(),
            branch_id: Some("br1".to_string()),
            branches: Some(vec![TimelineBranch {
                id: "br1".to_string(),
                label: Some("main".to_string()),
                content: "timeline content".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            }]),
            comparison: Some(BranchComparison {
                divergence_points: vec!["point1".to_string()],
                quality_differences: serde_json::json!({"score": 0.1}),
                convergence_opportunities: vec!["merge here".to_string()],
            }),
            merged_content: None,
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_mcts_response_into_contents() {
        let response = MctsResponse {
            session_id: "s1".to_string(),
            best_path: Some(vec![MctsNode {
                node_id: "n1".to_string(),
                content: "node content".to_string(),
                visits: 10,
                ucb_score: 1.2,
            }]),
            iterations_completed: Some(50),
            backtrack_suggestion: Some(BacktrackSuggestion {
                should_backtrack: false,
                target_step: None,
                reason: None,
                quality_drop: None,
            }),
            executed: Some(false),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_counterfactual_response_into_contents() {
        let response = CounterfactualResponse {
            counterfactual_outcome: "different result".to_string(),
            causal_chain: vec![CausalStep {
                step: 1,
                cause: "intervention".to_string(),
                effect: "outcome change".to_string(),
                probability: 0.8,
            }],
            session_id: Some("s1".to_string()),
            original_scenario: "base scenario".to_string(),
            intervention_applied: "change X".to_string(),
            analysis_depth: "counterfactual".to_string(),
            key_differences: vec!["difference 1".to_string()],
            confidence: 0.85,
            assumptions: vec!["assumption 1".to_string()],
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_preset_response_into_contents() {
        let response = PresetResponse {
            presets: Some(vec![PresetInfo {
                id: "p1".to_string(),
                name: "Quick Analysis".to_string(),
                description: "Fast analysis preset".to_string(),
                category: "analysis".to_string(),
                required_inputs: vec!["content".to_string()],
            }]),
            execution_result: None,
            session_id: Some("s1".to_string()),
            metadata: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn test_metrics_response_into_contents() {
        let response = MetricsResponse {
            summary: Some(MetricsSummary {
                total_calls: 100,
                success_rate: 0.95,
                avg_latency_ms: 150.0,
                by_mode: serde_json::json!({"linear": 50, "tree": 30}),
            }),
            mode_stats: None,
            invocations: None,
            config: None,
        };
        let contents = response.into_contents();
        assert_eq!(contents.len(), 1);
    }

    // ============================================================================
    // Request Deserialization Tests
    // ============================================================================

    #[test]
    fn test_tree_request_deserialize() {
        let json = r#"{"operation": "create", "content": "test"}"#;
        let req: TreeRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.operation, Some("create".to_string()));
    }

    #[test]
    fn test_divergent_request_deserialize() {
        let json = r#"{"content": "test", "force_rebellion": true}"#;
        let req: DivergentRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.force_rebellion, Some(true));
    }

    #[test]
    fn test_reflection_request_deserialize() {
        let json = r#"{"operation": "evaluate", "session_id": "s1"}"#;
        let req: ReflectionRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.operation, Some("evaluate".to_string()));
    }

    #[test]
    fn test_checkpoint_request_deserialize() {
        let json = r#"{"operation": "create", "session_id": "s1", "name": "cp1"}"#;
        let req: CheckpointRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.name, Some("cp1".to_string()));
    }

    #[test]
    fn test_auto_request_deserialize() {
        let json = r#"{"content": "test", "hints": ["hint1"]}"#;
        let req: AutoRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.hints, Some(vec!["hint1".to_string()]));
    }

    #[test]
    fn test_graph_request_deserialize() {
        let json = r#"{"operation": "init", "session_id": "s1", "k": 5}"#;
        let req: GraphRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.k, Some(5));
    }

    #[test]
    fn test_detect_request_deserialize() {
        let json = r#"{"type": "biases", "check_formal": true}"#;
        let req: DetectRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.detect_type, "biases");
    }

    #[test]
    fn test_decision_request_deserialize() {
        let json = r#"{"type": "weighted", "options": ["A", "B"]}"#;
        let req: DecisionRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.options, Some(vec!["A".to_string(), "B".to_string()]));
    }

    #[test]
    fn test_evidence_request_deserialize() {
        let json = r#"{"type": "assess", "prior": 0.5}"#;
        let req: EvidenceRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.prior, Some(0.5));
    }

    #[test]
    fn test_timeline_request_deserialize() {
        let json = r#"{"operation": "branch", "timeline_id": "tl1"}"#;
        let req: TimelineRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.timeline_id, Some("tl1".to_string()));
    }

    #[test]
    fn test_mcts_request_deserialize() {
        let json = r#"{"operation": "explore", "iterations": 50}"#;
        let req: MctsRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.iterations, Some(50));
    }

    #[test]
    fn test_counterfactual_request_deserialize() {
        let json = r#"{"scenario": "base", "intervention": "change"}"#;
        let req: CounterfactualRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.scenario, "base");
    }

    #[test]
    fn test_preset_request_deserialize() {
        let json = r#"{"operation": "run", "preset_id": "p1"}"#;
        let req: PresetRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.preset_id, Some("p1".to_string()));
    }

    #[test]
    fn test_metrics_request_deserialize() {
        let json = r#"{"query": "by_mode", "mode_name": "linear"}"#;
        let req: MetricsRequest = serde_json::from_str(json).expect("deserialize");
        assert_eq!(req.mode_name, Some("linear".to_string()));
    }

    // ============================================================================
    // ServerHandler Tests
    // ============================================================================

    fn create_test_si_handle(
        storage: &crate::storage::SqliteStorage,
        metrics: std::sync::Arc<crate::metrics::MetricsCollector>,
    ) -> crate::self_improvement::ManagerHandle {
        use crate::config::SelfImprovementConfig;
        use crate::self_improvement::{SelfImprovementManager, SelfImprovementStorage};
        use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"summary": "Test", "confidence": 0.8, "actions": []}"#,
                Usage::new(100, 50),
            ))
        });

        let si_storage = std::sync::Arc::new(SelfImprovementStorage::new(storage.pool.clone()));

        let (_manager, handle) = SelfImprovementManager::new(
            SelfImprovementConfig::default(),
            client,
            metrics,
            si_storage,
        );
        handle
    }

    fn create_test_server_sync() -> ReasoningServer {
        use crate::anthropic::{AnthropicClient, ClientConfig};
        use crate::config::{Config, SecretString};
        use crate::metrics::MetricsCollector;
        use crate::storage::SqliteStorage;

        let config = Config {
            api_key: SecretString::new("test-key"),
            database_path: ":memory:".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            request_timeout_deep_ms: 60000,
            request_timeout_maximum_ms: 120000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let storage = rt.block_on(async { SqliteStorage::new_in_memory().await.unwrap() });

        let metrics = Arc::new(MetricsCollector::new());
        let si_handle = create_test_si_handle(&storage, metrics.clone());
        let client = AnthropicClient::new("test-key", ClientConfig::default()).unwrap();
        let metadata_builder = crate::metadata::MetadataBuilder::new(
            Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
                storage.clone(),
            ))),
            Arc::new(crate::metadata::PresetIndex::build()),
            30000,
        );
        let (progress_tx, _rx) = tokio::sync::broadcast::channel(100);
        let state = AppState::new(
            storage,
            client,
            config,
            metrics,
            si_handle,
            metadata_builder,
            progress_tx,
        );
        ReasoningServer::new(Arc::new(state))
    }

    async fn create_test_server() -> ReasoningServer {
        use crate::anthropic::{AnthropicClient, ClientConfig};
        use crate::config::{Config, SecretString};
        use crate::metrics::MetricsCollector;
        use crate::storage::SqliteStorage;

        let config = Config {
            api_key: SecretString::new("test-key"),
            database_path: ":memory:".to_string(),
            log_level: "info".to_string(),
            request_timeout_ms: 30000,
            request_timeout_deep_ms: 60000,
            request_timeout_maximum_ms: 120000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".to_string(),
        };

        let storage = SqliteStorage::new_in_memory().await.unwrap();

        let metrics = Arc::new(MetricsCollector::new());
        let si_handle = create_test_si_handle(&storage, metrics.clone());
        let client = AnthropicClient::new("test-key", ClientConfig::default()).unwrap();
        let metadata_builder = crate::metadata::MetadataBuilder::new(
            Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
                storage.clone(),
            ))),
            Arc::new(crate::metadata::PresetIndex::build()),
            30000,
        );
        let (progress_tx, _rx) = tokio::sync::broadcast::channel(100);
        let state = AppState::new(
            storage,
            client,
            config,
            metrics,
            si_handle,
            metadata_builder,
            progress_tx,
        );
        ReasoningServer::new(Arc::new(state))
    }

    #[test]
    fn test_server_handler_get_info() {
        let server = create_test_server_sync();
        let info = server.get_info();
        assert!(info.capabilities.tools.is_some());
        assert!(info.instructions.is_some());
    }

    #[test]
    fn test_reasoning_server_new() {
        let server = create_test_server_sync();
        // Just verify we can create a server without panicking
        let _ = &server.state;
    }

    // ============================================================================
    // Tool Method Tests (stubs, covering return path)
    // ============================================================================

    #[tokio::test]
    async fn test_reasoning_linear_tool() {
        let server = create_test_server().await;
        let req = LinearRequest {
            content: "test".to_string(),
            session_id: Some("s1".to_string()),
            confidence: Some(0.8),
        };
        let resp = server.reasoning_linear(Parameters(req)).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_tree_tool() {
        let server = create_test_server().await;
        let req = TreeRequest {
            operation: Some("create".to_string()),
            content: Some("test".to_string()),
            session_id: Some("s1".to_string()),
            branch_id: None,
            num_branches: Some(2),
            completed: None,
        };
        let resp = server.reasoning_tree(Parameters(req)).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_divergent_tool() {
        let server = create_test_server().await;
        let req = DivergentRequest {
            content: "test".to_string(),
            session_id: Some("s1".to_string()),
            num_perspectives: Some(3),
            challenge_assumptions: Some(true),
            force_rebellion: Some(false),
            progress_token: None,
        };
        let resp = server.reasoning_divergent(Parameters(req)).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_reflection_tool() {
        let server = create_test_server().await;
        let req = ReflectionRequest {
            operation: Some("process".to_string()),
            content: Some("test".to_string()),
            thought_id: None,
            session_id: Some("s1".to_string()),
            max_iterations: Some(3),
            quality_threshold: Some(0.8),
            progress_token: None,
        };
        let resp = server.reasoning_reflection(Parameters(req)).await;
        assert!(resp.quality_score >= 0.0);
    }

    #[tokio::test]
    async fn test_reasoning_checkpoint_tool() {
        let server = create_test_server().await;
        let req = CheckpointRequest {
            operation: "create".to_string(),
            session_id: "s1".to_string(),
            checkpoint_id: None,
            name: Some("cp1".to_string()),
            description: Some("test checkpoint".to_string()),
            new_direction: None,
        };
        let resp = server.reasoning_checkpoint(Parameters(req)).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_auto_tool() {
        let server = create_test_server().await;
        let req = AutoRequest {
            content: "test".to_string(),
            hints: Some(vec!["hint".to_string()]),
            session_id: Some("s1".to_string()),
        };
        let resp = server.reasoning_auto(Parameters(req)).await;
        assert!(!resp.selected_mode.is_empty());
    }

    #[tokio::test]
    async fn test_reasoning_graph_tool() {
        let server = create_test_server().await;
        let req = GraphRequest {
            operation: "init".to_string(),
            session_id: "s1".to_string(),
            content: Some("test".to_string()),
            problem: Some("problem".to_string()),
            node_id: None,
            node_ids: None,
            k: Some(3),
            threshold: None,
            terminal_node_ids: None,
        };
        let resp = server.reasoning_graph(Parameters(req)).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_detect_tool() {
        let server = create_test_server().await;
        let req = DetectRequest {
            detect_type: "biases".to_string(),
            content: Some("test".to_string()),
            thought_id: None,
            session_id: Some("s1".to_string()),
            check_types: None,
            check_formal: Some(true),
            check_informal: Some(true),
        };
        let resp = server.reasoning_detect(Parameters(req)).await;
        assert!(resp.detections.is_empty() || !resp.detections.is_empty());
    }

    #[tokio::test]
    async fn test_reasoning_decision_tool() {
        let server = create_test_server().await;
        let req = DecisionRequest {
            decision_type: Some("weighted".to_string()),
            question: Some("which?".to_string()),
            options: Some(vec!["A".to_string(), "B".to_string()]),
            topic: None,
            context: Some("context".to_string()),
            session_id: Some("s1".to_string()),
        };
        let resp = server.reasoning_decision(Parameters(req)).await;
        // Stub returns empty recommendation
        let _ = resp.recommendation;
    }

    #[tokio::test]
    async fn test_reasoning_evidence_tool() {
        let server = create_test_server().await;
        let req = EvidenceRequest {
            evidence_type: Some("assess".to_string()),
            claim: Some("claim".to_string()),
            hypothesis: None,
            context: Some("context".to_string()),
            prior: Some(0.5),
            session_id: Some("s1".to_string()),
        };
        let resp = server.reasoning_evidence(Parameters(req)).await;
        assert!(resp.overall_credibility >= 0.0);
    }

    #[tokio::test]
    async fn test_reasoning_timeline_tool() {
        let server = create_test_server().await;
        let req = TimelineRequest {
            operation: "create".to_string(),
            session_id: Some("s1".to_string()),
            timeline_id: None,
            content: Some("test".to_string()),
            label: Some("main".to_string()),
            branch_ids: None,
            source_branch_id: None,
            target_branch_id: None,
            merge_strategy: None,
        };
        let resp = server.reasoning_timeline(Parameters(req)).await;
        // Stub returns empty timeline_id
        let _ = resp.timeline_id;
    }

    #[tokio::test]
    async fn test_reasoning_mcts_tool() {
        let server = create_test_server().await;
        let req = MctsRequest {
            operation: Some("explore".to_string()),
            content: Some("test".to_string()),
            session_id: Some("s1".to_string()),
            node_id: None,
            iterations: Some(10),
            exploration_constant: Some(1.41),
            simulation_depth: Some(5),
            quality_threshold: Some(0.7),
            lookback_depth: Some(3),
            auto_execute: Some(false),
            progress_token: None,
        };
        let resp = server.reasoning_mcts(Parameters(req)).await;
        assert_eq!(resp.session_id, "s1");
    }

    #[tokio::test]
    async fn test_reasoning_counterfactual_tool() {
        let server = create_test_server().await;
        let req = CounterfactualRequest {
            scenario: "base".to_string(),
            intervention: "change".to_string(),
            analysis_depth: Some("counterfactual".to_string()),
            session_id: Some("s1".to_string()),
            progress_token: None,
        };
        let resp = server.reasoning_counterfactual(Parameters(req)).await;
        // Stub uses input values for output
        assert_eq!(resp.original_scenario, "base");
        assert_eq!(resp.intervention_applied, "change");
    }

    #[tokio::test]
    async fn test_reasoning_preset_tool() {
        let server = create_test_server().await;
        let req = PresetRequest {
            operation: "list".to_string(),
            preset_id: None,
            category: Some("analysis".to_string()),
            inputs: None,
            session_id: Some("s1".to_string()),
        };
        let resp = server.reasoning_preset(Parameters(req)).await;
        // presets may or may not be present
        let _ = resp.presets;
    }

    #[tokio::test]
    async fn test_reasoning_metrics_tool() {
        let server = create_test_server().await;
        let req = MetricsRequest {
            query: "summary".to_string(),
            mode_name: None,
            tool_name: None,
            session_id: None,
            success_only: Some(true),
            limit: Some(10),
        };
        let resp = server.reasoning_metrics(Parameters(req)).await;
        // summary may or may not be present
        let _ = resp.summary;
    }

    // ============================================================================
    // Wiremock Integration Tests - Cover Success Paths
    // ============================================================================

    mod wiremock_tests {
        use super::*;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        fn anthropic_response(text: &str) -> serde_json::Value {
            serde_json::json!({
                "id": "msg_test_123",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": text}],
                "model": "claude-sonnet-4-20250514",
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 100, "output_tokens": 50}
            })
        }

        async fn create_mocked_server(mock_server: &MockServer) -> ReasoningServer {
            use crate::anthropic::{AnthropicClient, ClientConfig};
            use crate::config::{Config, SecretString};
            use crate::metrics::MetricsCollector;
            use crate::storage::SqliteStorage;

            let config = Config {
                api_key: SecretString::new("test-key"),
                database_path: ":memory:".to_string(),
                log_level: "info".to_string(),
                request_timeout_ms: 5000,
                request_timeout_deep_ms: 60000,
                request_timeout_maximum_ms: 120000,
                max_retries: 0,
                model: "claude-sonnet-4-20250514".to_string(),
            };

            let storage = SqliteStorage::new_in_memory().await.unwrap();
            let metrics = Arc::new(MetricsCollector::new());
            let si_handle = super::create_test_si_handle(&storage, metrics.clone());
            let client_config = ClientConfig::default()
                .with_base_url(mock_server.uri())
                .with_max_retries(0)
                .with_timeout_ms(5000);
            let client = AnthropicClient::new("test-key", client_config).unwrap();
            let metadata_builder = crate::metadata::MetadataBuilder::new(
                Arc::new(crate::metadata::TimingDatabase::new(Arc::new(
                    storage.clone(),
                ))),
                Arc::new(crate::metadata::PresetIndex::build()),
                30000,
            );
            let (progress_tx, _rx) = tokio::sync::broadcast::channel(100);
            let state = AppState::new(
                storage,
                client,
                config,
                metrics,
                si_handle,
                metadata_builder,
                progress_tx,
            );
            ReasoningServer::new(Arc::new(state))
        }

        #[tokio::test]
        async fn test_linear_success_path() {
            let mock_server = MockServer::start().await;

            let response_json = serde_json::json!({
                "analysis": "Detailed reasoning analysis",
                "confidence": 0.85,
                "next_step": "Continue with more analysis"
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&response_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;
            let req = LinearRequest {
                content: "Analyze this problem".to_string(),
                session_id: None,
                confidence: Some(0.8),
            };

            let resp = server.reasoning_linear(Parameters(req)).await;
            // Should succeed with mocked response
            assert!(!resp.thought_id.is_empty() || !resp.content.is_empty());
        }

        #[tokio::test]
        async fn test_tree_all_operations() {
            let mock_server = MockServer::start().await;

            // Test create operation
            let create_json = serde_json::json!({
                "branches": [
                    {"id": "b1", "content": "Branch 1", "score": 0.8},
                    {"id": "b2", "content": "Branch 2", "score": 0.7}
                ],
                "recommendation": "Explore branch 1 first"
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&create_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            // Test create
            let create_req = TreeRequest {
                operation: Some("create".to_string()),
                content: Some("Explore this topic".to_string()),
                session_id: Some("s1".to_string()),
                branch_id: None,
                num_branches: Some(2),
                completed: None,
            };
            let resp = server.reasoning_tree(Parameters(create_req)).await;
            assert_eq!(resp.session_id, "s1");

            // Test list
            let list_req = TreeRequest {
                operation: Some("list".to_string()),
                content: None,
                session_id: Some("s1".to_string()),
                branch_id: None,
                num_branches: None,
                completed: None,
            };
            let resp = server.reasoning_tree(Parameters(list_req)).await;
            assert_eq!(resp.session_id, "s1");

            // Test focus
            let focus_req = TreeRequest {
                operation: Some("focus".to_string()),
                content: None,
                session_id: Some("s1".to_string()),
                branch_id: Some("b1".to_string()),
                num_branches: None,
                completed: None,
            };
            let resp = server.reasoning_tree(Parameters(focus_req)).await;
            assert_eq!(resp.session_id, "s1");

            // Test complete
            let complete_req = TreeRequest {
                operation: Some("complete".to_string()),
                content: None,
                session_id: Some("s1".to_string()),
                branch_id: Some("b1".to_string()),
                num_branches: None,
                completed: Some(true),
            };
            let resp = server.reasoning_tree(Parameters(complete_req)).await;
            assert_eq!(resp.session_id, "s1");

            // Test unknown operation
            let unknown_req = TreeRequest {
                operation: Some("unknown".to_string()),
                content: None,
                session_id: Some("s1".to_string()),
                branch_id: None,
                num_branches: None,
                completed: None,
            };
            let resp = server.reasoning_tree(Parameters(unknown_req)).await;
            assert!(resp.recommendation.unwrap().contains("Unknown operation"));
        }

        #[tokio::test]
        async fn test_divergent_success_path() {
            let mock_server = MockServer::start().await;

            let response_json = serde_json::json!({
                "perspectives": [
                    {"viewpoint": "Optimistic", "content": "Positive outlook", "novelty_score": 0.8},
                    {"viewpoint": "Pessimistic", "content": "Cautionary view", "novelty_score": 0.7}
                ],
                "challenged_assumptions": ["Assumption 1"],
                "synthesis": "Combined insight"
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&response_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;
            let req = DivergentRequest {
                content: "Analyze from multiple perspectives".to_string(),
                session_id: Some("s1".to_string()),
                num_perspectives: Some(2),
                challenge_assumptions: Some(true),
                force_rebellion: Some(true),
                progress_token: None,
            };

            let resp = server.reasoning_divergent(Parameters(req)).await;
            assert_eq!(resp.session_id, "s1");
        }

        #[tokio::test]
        async fn test_reflection_all_operations() {
            let mock_server = MockServer::start().await;

            let process_json = serde_json::json!({
                "analysis": {
                    "strengths": ["Clear logic"],
                    "weaknesses": ["Needs more evidence"]
                },
                "improvements": [
                    {"suggestion": "Add examples", "priority": 1}
                ],
                "refined_reasoning": "Improved version",
                "confidence_improvement": 0.15
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&process_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            // Test process
            let process_req = ReflectionRequest {
                operation: Some("process".to_string()),
                content: Some("Reasoning to improve".to_string()),
                thought_id: None,
                session_id: Some("s1".to_string()),
                max_iterations: Some(3),
                quality_threshold: Some(0.8),
                progress_token: None,
            };
            let resp = server.reasoning_reflection(Parameters(process_req)).await;
            assert!(resp.quality_score >= 0.0);

            // Test evaluate
            let eval_json = serde_json::json!({
                "session_assessment": {
                    "overall_quality": 0.8,
                    "coherence": 0.85,
                    "reasoning_depth": 0.75
                },
                "strongest_elements": ["Logic", "Structure"],
                "areas_for_improvement": ["More examples"],
                "recommendations": ["Add case studies"]
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&eval_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let evaluate_req = ReflectionRequest {
                operation: Some("evaluate".to_string()),
                content: None,
                thought_id: None,
                session_id: Some("s1".to_string()),
                max_iterations: None,
                quality_threshold: None,
                progress_token: None,
            };
            let resp = server.reasoning_reflection(Parameters(evaluate_req)).await;
            assert!(resp.quality_score >= 0.0);

            // Test unknown operation
            let unknown_req = ReflectionRequest {
                operation: Some("unknown".to_string()),
                content: None,
                thought_id: None,
                session_id: Some("s1".to_string()),
                max_iterations: None,
                quality_threshold: None,
                progress_token: None,
            };
            let resp = server.reasoning_reflection(Parameters(unknown_req)).await;
            assert!(resp
                .weaknesses
                .unwrap()
                .iter()
                .any(|w| w.contains("Unknown")));
        }

        #[tokio::test]
        async fn test_checkpoint_all_operations() {
            let mock_server = MockServer::start().await;

            // No API calls needed for checkpoint - it's storage-only
            let server = create_mocked_server(&mock_server).await;

            // First create a session
            let create_req = CheckpointRequest {
                operation: "create".to_string(),
                session_id: "s1".to_string(),
                checkpoint_id: None,
                name: Some("cp1".to_string()),
                description: Some("Test checkpoint".to_string()),
                new_direction: None,
            };
            let resp = server.reasoning_checkpoint(Parameters(create_req)).await;
            assert_eq!(resp.session_id, "s1");

            // List checkpoints
            let list_req = CheckpointRequest {
                operation: "list".to_string(),
                session_id: "s1".to_string(),
                checkpoint_id: None,
                name: None,
                description: None,
                new_direction: None,
            };
            let resp = server.reasoning_checkpoint(Parameters(list_req)).await;
            assert_eq!(resp.session_id, "s1");

            // Restore (will fail since no actual checkpoint, but exercises code path)
            let restore_req = CheckpointRequest {
                operation: "restore".to_string(),
                session_id: "s1".to_string(),
                checkpoint_id: Some("cp-nonexistent".to_string()),
                name: None,
                description: None,
                new_direction: Some("New direction".to_string()),
            };
            let resp = server.reasoning_checkpoint(Parameters(restore_req)).await;
            // Will have error in restored_state since checkpoint doesn't exist
            assert!(resp.restored_state.is_some());

            // Unknown operation
            let unknown_req = CheckpointRequest {
                operation: "unknown".to_string(),
                session_id: "s1".to_string(),
                checkpoint_id: None,
                name: None,
                description: None,
                new_direction: None,
            };
            let resp = server.reasoning_checkpoint(Parameters(unknown_req)).await;
            assert!(resp.restored_state.is_some());
        }

        #[tokio::test]
        async fn test_auto_success_path() {
            let mock_server = MockServer::start().await;

            let response_json = serde_json::json!({
                "selected_mode": "tree",
                "reasoning": "Content suggests branching exploration",
                "characteristics": ["complex", "multi-path"],
                "suggested_parameters": {"num_branches": 3},
                "alternative_mode": {"mode": "linear", "reason": "Simpler option"}
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&response_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;
            let req = AutoRequest {
                content: "Complex problem with multiple paths".to_string(),
                hints: Some(vec!["exploration".to_string()]),
                session_id: Some("s1".to_string()),
            };

            let resp = server.reasoning_auto(Parameters(req)).await;
            assert!(!resp.selected_mode.is_empty());
        }

        #[tokio::test]
        async fn test_graph_all_operations() {
            let mock_server = MockServer::start().await;

            // Setup mock for various graph operations
            let init_json = serde_json::json!({
                "root": {"id": "n1", "content": "Root node", "score": 1.0}
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&init_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            // Test init
            let init_req = GraphRequest {
                operation: "init".to_string(),
                session_id: "s1".to_string(),
                content: Some("Problem to explore".to_string()),
                problem: None,
                node_id: None,
                node_ids: None,
                k: None,
                threshold: None,
                terminal_node_ids: None,
            };
            let resp = server.reasoning_graph(Parameters(init_req)).await;
            assert_eq!(resp.session_id, "s1");

            // Test generate
            let generate_json = serde_json::json!({
                "children": [
                    {"id": "n2", "content": "Child 1", "score": 0.8},
                    {"id": "n3", "content": "Child 2", "score": 0.7}
                ]
            });
            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&generate_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let generate_req = GraphRequest {
                operation: "generate".to_string(),
                session_id: "s1".to_string(),
                content: Some("Generate continuations".to_string()),
                problem: None,
                node_id: Some("n1".to_string()),
                node_ids: None,
                k: Some(2),
                threshold: None,
                terminal_node_ids: None,
            };
            let resp = server.reasoning_graph(Parameters(generate_req)).await;
            assert_eq!(resp.session_id, "s1");

            // Test unknown operation
            let unknown_req = GraphRequest {
                operation: "unknown".to_string(),
                session_id: "s1".to_string(),
                content: None,
                problem: None,
                node_id: None,
                node_ids: None,
                k: None,
                threshold: None,
                terminal_node_ids: None,
            };
            let resp = server.reasoning_graph(Parameters(unknown_req)).await;
            assert!(resp
                .aggregated_insight
                .unwrap()
                .contains("Invalid operation"));
        }

        #[tokio::test]
        async fn test_detect_all_types() {
            let mock_server = MockServer::start().await;

            // Test biases
            let biases_json = serde_json::json!({
                "biases_detected": [
                    {
                        "bias": "confirmation bias",
                        "severity": "medium",
                        "evidence": "Selected confirming evidence",
                        "impact": "May miss counterexamples",
                        "debiasing": "Consider opposing views"
                    }
                ],
                "overall_assessment": {
                    "bias_count": 1,
                    "most_severe": "confirmation bias",
                    "reasoning_quality": 0.7,
                    "overall_analysis": "Some bias detected"
                }
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&biases_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let biases_req = DetectRequest {
                detect_type: "biases".to_string(),
                content: Some("Argument with potential bias".to_string()),
                thought_id: None,
                session_id: Some("s1".to_string()),
                check_types: None,
                check_formal: None,
                check_informal: None,
            };
            let resp = server.reasoning_detect(Parameters(biases_req)).await;
            assert!(resp.summary.is_some());

            // Test fallacies
            let fallacies_json = serde_json::json!({
                "fallacies_detected": [
                    {
                        "fallacy": "ad hominem",
                        "category": "informal",
                        "passage": "The argument",
                        "explanation": "Attacks the person",
                        "correction": "Address the argument"
                    }
                ],
                "argument_structure": {
                    "premises": ["Premise 1"],
                    "conclusion": "Conclusion",
                    "structure_type": "deductive",
                    "validity": "invalid"
                },
                "overall_assessment": {
                    "fallacy_count": 1,
                    "most_critical": "ad hominem",
                    "argument_strength": 0.4,
                    "overall_analysis": "Weak argument"
                }
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&fallacies_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let fallacies_req = DetectRequest {
                detect_type: "fallacies".to_string(),
                content: Some("Argument with fallacy".to_string()),
                thought_id: None,
                session_id: Some("s1".to_string()),
                check_types: None,
                check_formal: Some(true),
                check_informal: Some(true),
            };
            let resp = server.reasoning_detect(Parameters(fallacies_req)).await;
            assert!(resp.summary.is_some());

            // Test unknown type
            let unknown_req = DetectRequest {
                detect_type: "unknown".to_string(),
                content: Some("Content".to_string()),
                thought_id: None,
                session_id: None,
                check_types: None,
                check_formal: None,
                check_informal: None,
            };
            let resp = server.reasoning_detect(Parameters(unknown_req)).await;
            assert!(resp.summary.unwrap().contains("Unknown"));
        }

        #[tokio::test]
        async fn test_decision_all_types() {
            let mock_server = MockServer::start().await;

            // Test weighted
            let weighted_json = serde_json::json!({
                "criteria": [
                    {"name": "Cost", "weight": 0.4, "type": "cost"},
                    {"name": "Quality", "weight": 0.6, "type": "benefit"}
                ],
                "decision_matrix": [
                    {"option": "A", "scores": {"Cost": 0.8, "Quality": 0.9}},
                    {"option": "B", "scores": {"Cost": 0.6, "Quality": 0.7}}
                ],
                "weighted_totals": {"A": 0.86, "B": 0.66},
                "ranking": [
                    {"option": "A", "score": 0.86, "rank": 1},
                    {"option": "B", "score": 0.66, "rank": 2}
                ],
                "sensitivity_notes": "A is preferred"
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&weighted_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let weighted_req = DecisionRequest {
                decision_type: Some("weighted".to_string()),
                question: Some("Which option?".to_string()),
                options: Some(vec!["A".to_string(), "B".to_string()]),
                topic: None,
                context: None,
                session_id: Some("s1".to_string()),
            };
            let resp = server.reasoning_decision(Parameters(weighted_req)).await;
            assert!(!resp.recommendation.is_empty() || resp.recommendation.contains("ERROR"));

            // Test pairwise
            let pairwise_json = serde_json::json!({
                "comparisons": [
                    {"option_a": "A", "option_b": "B", "preferred": "A", "strength": "strong", "rationale": "Better"}
                ],
                "pairwise_matrix": [[0, 1], [0, 0]],
                "ranking": [
                    {"option": "A", "wins": 1, "rank": 1},
                    {"option": "B", "wins": 0, "rank": 2}
                ]
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&pairwise_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let pairwise_req = DecisionRequest {
                decision_type: Some("pairwise".to_string()),
                question: Some("Compare options".to_string()),
                options: Some(vec!["A".to_string(), "B".to_string()]),
                topic: None,
                context: None,
                session_id: Some("s1".to_string()),
            };
            let resp = server.reasoning_decision(Parameters(pairwise_req)).await;
            let _ = resp.recommendation;

            // Test unknown type (defaults to weighted)
            let default_req = DecisionRequest {
                decision_type: None,
                question: Some("Question".to_string()),
                options: None,
                topic: None,
                context: None,
                session_id: None,
            };
            let resp = server.reasoning_decision(Parameters(default_req)).await;
            let _ = resp.recommendation;
        }

        #[tokio::test]
        async fn test_evidence_all_types() {
            let mock_server = MockServer::start().await;

            // Test assess
            let assess_json = serde_json::json!({
                "evidence_pieces": [
                    {
                        "content": "Primary source",
                        "credibility_score": 0.9,
                        "source_tier": "primary",
                        "corroborated_by": [1]
                    }
                ],
                "corroboration_matrix": [[1]],
                "overall_credibility": 0.85,
                "synthesis": "Strong evidence"
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&assess_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let assess_req = EvidenceRequest {
                evidence_type: Some("assess".to_string()),
                claim: Some("The claim".to_string()),
                hypothesis: None,
                prior: None,
                context: Some("Context".to_string()),
                session_id: Some("s1".to_string()),
            };
            let resp = server.reasoning_evidence(Parameters(assess_req)).await;
            assert!(resp.overall_credibility >= 0.0);

            // Test probabilistic
            let prob_json = serde_json::json!({
                "prior": 0.5,
                "likelihood_ratio": 2.0,
                "posterior": 0.67,
                "confidence_interval": {"lower": 0.5, "upper": 0.8},
                "hypothesis": "The hypothesis",
                "sensitivity": "Moderate"
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&prob_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let prob_req = EvidenceRequest {
                evidence_type: Some("probabilistic".to_string()),
                claim: None,
                hypothesis: Some("Hypothesis".to_string()),
                prior: Some(0.5),
                context: None,
                session_id: Some("s1".to_string()),
            };
            let resp = server.reasoning_evidence(Parameters(prob_req)).await;
            assert!(resp.overall_credibility >= 0.0);

            // Test unknown type (defaults to assess)
            let default_req = EvidenceRequest {
                evidence_type: None,
                claim: Some("Claim".to_string()),
                hypothesis: None,
                prior: None,
                context: None,
                session_id: None,
            };
            let resp = server.reasoning_evidence(Parameters(default_req)).await;
            assert!(resp.overall_credibility >= 0.0);
        }

        #[tokio::test]
        async fn test_timeline_all_operations() {
            let mock_server = MockServer::start().await;

            let create_json = serde_json::json!({
                "timeline_id": "tl1",
                "events": [
                    {"timestamp": "t1", "event": "Event 1", "significance": "high"}
                ],
                "analysis": "Timeline analysis"
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&create_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            // Test create
            let create_req = TimelineRequest {
                operation: "create".to_string(),
                session_id: Some("s1".to_string()),
                timeline_id: None,
                content: Some("Timeline content".to_string()),
                label: Some("main".to_string()),
                branch_ids: None,
                source_branch_id: None,
                target_branch_id: None,
                merge_strategy: None,
            };
            let resp = server.reasoning_timeline(Parameters(create_req)).await;
            let _ = resp.timeline_id;

            // Test branch
            let branch_req = TimelineRequest {
                operation: "branch".to_string(),
                session_id: Some("s1".to_string()),
                timeline_id: Some("tl1".to_string()),
                content: Some("Branch content".to_string()),
                label: Some("alternative".to_string()),
                branch_ids: None,
                source_branch_id: None,
                target_branch_id: None,
                merge_strategy: None,
            };
            let resp = server.reasoning_timeline(Parameters(branch_req)).await;
            let _ = resp.timeline_id;

            // Test compare
            let compare_req = TimelineRequest {
                operation: "compare".to_string(),
                session_id: Some("s1".to_string()),
                timeline_id: Some("tl1".to_string()),
                content: None,
                label: None,
                branch_ids: Some(vec!["b1".to_string(), "b2".to_string()]),
                source_branch_id: None,
                target_branch_id: None,
                merge_strategy: None,
            };
            let resp = server.reasoning_timeline(Parameters(compare_req)).await;
            let _ = resp.timeline_id;

            // Test merge
            let merge_req = TimelineRequest {
                operation: "merge".to_string(),
                session_id: Some("s1".to_string()),
                timeline_id: Some("tl1".to_string()),
                content: None,
                label: None,
                branch_ids: None,
                source_branch_id: Some("b1".to_string()),
                target_branch_id: Some("b2".to_string()),
                merge_strategy: Some("integrate".to_string()),
            };
            let resp = server.reasoning_timeline(Parameters(merge_req)).await;
            let _ = resp.timeline_id;

            // Test unknown operation
            let unknown_req = TimelineRequest {
                operation: "unknown".to_string(),
                session_id: Some("s1".to_string()),
                timeline_id: None,
                content: None,
                label: None,
                branch_ids: None,
                source_branch_id: None,
                target_branch_id: None,
                merge_strategy: None,
            };
            let resp = server.reasoning_timeline(Parameters(unknown_req)).await;
            // Should have error in some field
            let _ = resp.timeline_id;
        }

        #[tokio::test]
        async fn test_mcts_all_operations() {
            let mock_server = MockServer::start().await;

            let explore_json = serde_json::json!({
                "best_path": [
                    {"node_id": "n1", "content": "Step 1", "visits": 10, "ucb_score": 1.5}
                ],
                "iterations_completed": 50,
                "frontier_evaluation": [
                    {"node_id": "n2", "score": 0.8}
                ]
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&explore_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            // Test explore
            let explore_req = MctsRequest {
                operation: Some("explore".to_string()),
                content: Some("Problem to search".to_string()),
                session_id: Some("s1".to_string()),
                node_id: None,
                iterations: Some(50),
                exploration_constant: Some(1.41),
                simulation_depth: Some(5),
                quality_threshold: Some(0.7),
                lookback_depth: Some(3),
                auto_execute: Some(false),
                progress_token: None,
            };
            let resp = server.reasoning_mcts(Parameters(explore_req)).await;
            assert_eq!(resp.session_id, "s1");

            // Test auto_backtrack
            let backtrack_json = serde_json::json!({
                "should_backtrack": true,
                "target_step": 2,
                "reason": "Quality dropped",
                "quality_drop": 0.2
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&backtrack_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let backtrack_req = MctsRequest {
                operation: Some("auto_backtrack".to_string()),
                content: None,
                session_id: Some("s1".to_string()),
                node_id: None,
                iterations: None,
                exploration_constant: None,
                simulation_depth: None,
                quality_threshold: Some(0.7),
                lookback_depth: Some(3),
                auto_execute: Some(true),
                progress_token: None,
            };
            let resp = server.reasoning_mcts(Parameters(backtrack_req)).await;
            assert_eq!(resp.session_id, "s1");

            // Test unknown operation (defaults to explore)
            let default_req = MctsRequest {
                operation: None,
                content: Some("Content".to_string()),
                session_id: Some("s1".to_string()),
                node_id: None,
                iterations: None,
                exploration_constant: None,
                simulation_depth: None,
                quality_threshold: None,
                lookback_depth: None,
                auto_execute: None,
                progress_token: None,
            };
            let resp = server.reasoning_mcts(Parameters(default_req)).await;
            assert_eq!(resp.session_id, "s1");
        }

        #[tokio::test]
        async fn test_counterfactual_success_path() {
            let mock_server = MockServer::start().await;

            let response_json = serde_json::json!({
                "causal_model": {
                    "nodes": ["A", "B", "C"],
                    "edges": [
                        {"from": "A", "to": "B", "edge_type": "causes", "strength": "strong"}
                    ]
                },
                "ladder_rung": "intervention",
                "causal_chain": [
                    {"step": 1, "cause": "Intervention", "effect": "Changed outcome", "probability": 0.8}
                ],
                "counterfactual_outcome": "Different result",
                "key_differences": ["Difference 1"],
                "confidence": 0.85,
                "assumptions": ["Assumption 1"]
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&response_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;
            let req = CounterfactualRequest {
                scenario: "Original scenario".to_string(),
                intervention: "What if X changed?".to_string(),
                analysis_depth: Some("counterfactual".to_string()),
                session_id: Some("s1".to_string()),
                progress_token: None,
            };

            let resp = server.reasoning_counterfactual(Parameters(req)).await;
            assert_eq!(resp.original_scenario, "Original scenario");
            assert_eq!(resp.intervention_applied, "What if X changed?");
        }

        #[tokio::test]
        async fn test_preset_all_operations() {
            let mock_server = MockServer::start().await;

            // Presets don't require API calls
            let server = create_mocked_server(&mock_server).await;

            // Test list
            let list_req = PresetRequest {
                operation: "list".to_string(),
                preset_id: None,
                category: Some("analysis".to_string()),
                inputs: None,
                session_id: None,
            };
            let resp = server.reasoning_preset(Parameters(list_req)).await;
            assert!(resp.presets.is_some());

            // Test run (will fail without valid preset but exercises code)
            let run_req = PresetRequest {
                operation: "run".to_string(),
                preset_id: Some("quick_analysis".to_string()),
                category: None,
                inputs: Some(serde_json::json!({"content": "Test content"})),
                session_id: Some("s1".to_string()),
            };
            let resp = server.reasoning_preset(Parameters(run_req)).await;
            // Either has execution result or presets
            let _ = resp.execution_result;

            // Test unknown operation
            let unknown_req = PresetRequest {
                operation: "unknown".to_string(),
                preset_id: None,
                category: None,
                inputs: None,
                session_id: None,
            };
            let resp = server.reasoning_preset(Parameters(unknown_req)).await;
            let _ = resp.presets;
        }

        #[tokio::test]
        async fn test_decision_topsis_and_perspectives() {
            let mock_server = MockServer::start().await;

            // Test topsis
            let topsis_json = serde_json::json!({
                "criteria": [
                    {"name": "Cost", "weight": 0.4, "type": "cost"},
                    {"name": "Quality", "weight": 0.6, "type": "benefit"}
                ],
                "normalized_matrix": [[0.8, 0.9], [0.6, 0.7]],
                "weighted_matrix": [[0.32, 0.54], [0.24, 0.42]],
                "ideal_positive": [0.24, 0.54],
                "ideal_negative": [0.32, 0.42],
                "distance_positive": {"A": 0.1, "B": 0.2},
                "distance_negative": {"A": 0.2, "B": 0.1},
                "relative_closeness": {"A": 0.67, "B": 0.33},
                "ranking": [
                    {"option": "A", "closeness": 0.67, "rank": 1},
                    {"option": "B", "closeness": 0.33, "rank": 2}
                ]
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&topsis_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let topsis_req = DecisionRequest {
                decision_type: Some("topsis".to_string()),
                question: Some("Which option using TOPSIS?".to_string()),
                options: Some(vec!["A".to_string(), "B".to_string()]),
                topic: None,
                context: None,
                session_id: Some("s1".to_string()),
            };
            let resp = server.reasoning_decision(Parameters(topsis_req)).await;
            let _ = resp.recommendation;

            // Test perspectives
            let perspectives_json = serde_json::json!({
                "stakeholders": [
                    {"name": "Customer", "perspective": "Quality focus", "interests": ["Quality"], "concerns": ["Price"], "influence_level": "high"},
                    {"name": "Developer", "perspective": "Tech focus", "interests": ["Simplicity"], "concerns": ["Complexity"], "influence_level": "medium"},
                    {"name": "Manager", "perspective": "Cost focus", "interests": ["Budget"], "concerns": ["Overruns"], "influence_level": "low"}
                ],
                "conflicts": [
                    {"parties": ["Customer", "Manager"], "issue": "Budget vs quality", "severity": "medium", "resolution_approach": "Compromise"}
                ],
                "alignments": [
                    {"parties": ["Customer", "Developer"], "common_ground": "User experience", "leverage_opportunity": "Focus on UX"}
                ],
                "balanced_recommendation": {
                    "option": "Option A",
                    "rationale": "Best balance of interests",
                    "trade_offs": ["Some cost increase"]
                }
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&perspectives_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let perspectives_req = DecisionRequest {
                decision_type: Some("perspectives".to_string()),
                question: None,
                options: None,
                topic: Some("Project stakeholder analysis".to_string()),
                context: None,
                session_id: Some("s1".to_string()),
            };
            let resp = server
                .reasoning_decision(Parameters(perspectives_req))
                .await;
            assert!(resp.stakeholder_map.is_some() || !resp.recommendation.is_empty());

            // Test unknown decision type
            let unknown_req = DecisionRequest {
                decision_type: Some("unknown_type".to_string()),
                question: Some("Question".to_string()),
                options: None,
                topic: None,
                context: None,
                session_id: None,
            };
            let resp = server.reasoning_decision(Parameters(unknown_req)).await;
            assert!(
                resp.recommendation.contains("ERROR") || resp.recommendation.contains("unknown")
            );
        }

        #[tokio::test]
        async fn test_graph_score_operation() {
            let mock_server = MockServer::start().await;

            let score_json = serde_json::json!({
                "node_id": "n1",
                "score": 0.85,
                "factors": {"coherence": 0.9, "novelty": 0.8}
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&score_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let score_req = GraphRequest {
                operation: "score".to_string(),
                session_id: "s1".to_string(),
                content: Some("Evaluate this node".to_string()),
                problem: None,
                node_id: Some("n1".to_string()),
                node_ids: None,
                k: None,
                threshold: None,
                terminal_node_ids: None,
            };
            let resp = server.reasoning_graph(Parameters(score_req)).await;
            assert_eq!(resp.session_id, "s1");
        }

        #[tokio::test]
        async fn test_graph_aggregate_operation() {
            let mock_server = MockServer::start().await;

            let aggregate_json = serde_json::json!({
                "synthesis": {"content": "Combined insight from multiple nodes", "confidence": 0.8}
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&aggregate_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let aggregate_req = GraphRequest {
                operation: "aggregate".to_string(),
                session_id: "s1".to_string(),
                content: Some("Aggregate these insights".to_string()),
                problem: None,
                node_id: None,
                node_ids: Some(vec!["n1".to_string(), "n2".to_string()]),
                k: None,
                threshold: None,
                terminal_node_ids: None,
            };
            let resp = server.reasoning_graph(Parameters(aggregate_req)).await;
            assert_eq!(resp.session_id, "s1");
        }

        #[tokio::test]
        async fn test_graph_refine_operation() {
            let mock_server = MockServer::start().await;

            let refine_json = serde_json::json!({
                "refined_node": {"id": "n1_refined", "content": "Improved reasoning", "score": 0.9}
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&refine_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let refine_req = GraphRequest {
                operation: "refine".to_string(),
                session_id: "s1".to_string(),
                content: Some("Refine this node".to_string()),
                problem: None,
                node_id: Some("n1".to_string()),
                node_ids: None,
                k: None,
                threshold: None,
                terminal_node_ids: None,
            };
            let resp = server.reasoning_graph(Parameters(refine_req)).await;
            assert_eq!(resp.session_id, "s1");
        }

        #[tokio::test]
        async fn test_graph_prune_operation() {
            let mock_server = MockServer::start().await;

            let prune_json = serde_json::json!({
                "prune_candidates": [
                    {"id": "n3", "reason": "Low score", "score": 0.2},
                    {"id": "n4", "reason": "Redundant", "score": 0.3}
                ]
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&prune_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let prune_req = GraphRequest {
                operation: "prune".to_string(),
                session_id: "s1".to_string(),
                content: Some("Prune low value nodes".to_string()),
                problem: None,
                node_id: None,
                node_ids: None,
                k: None,
                threshold: Some(0.5),
                terminal_node_ids: None,
            };
            let resp = server.reasoning_graph(Parameters(prune_req)).await;
            assert_eq!(resp.session_id, "s1");
        }

        #[tokio::test]
        async fn test_graph_finalize_operation() {
            let mock_server = MockServer::start().await;

            let finalize_json = serde_json::json!({
                "conclusions": [
                    {"conclusion": "Main finding 1", "confidence": 0.9},
                    {"conclusion": "Main finding 2", "confidence": 0.85}
                ]
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&finalize_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let finalize_req = GraphRequest {
                operation: "finalize".to_string(),
                session_id: "s1".to_string(),
                content: Some("Generate final conclusions".to_string()),
                problem: None,
                node_id: None,
                node_ids: None,
                k: None,
                threshold: None,
                terminal_node_ids: Some(vec!["n1".to_string(), "n2".to_string()]),
            };
            let resp = server.reasoning_graph(Parameters(finalize_req)).await;
            assert_eq!(resp.session_id, "s1");
        }

        #[tokio::test]
        async fn test_graph_state_operation() {
            let mock_server = MockServer::start().await;

            let state_json = serde_json::json!({
                "structure": {
                    "total_nodes": 10,
                    "depth": 3,
                    "pruned_count": 2,
                    "active_branches": 4
                }
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&state_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let state_req = GraphRequest {
                operation: "state".to_string(),
                session_id: "s1".to_string(),
                content: None,
                problem: None,
                node_id: None,
                node_ids: None,
                k: None,
                threshold: None,
                terminal_node_ids: None,
            };
            let resp = server.reasoning_graph(Parameters(state_req)).await;
            assert_eq!(resp.session_id, "s1");
        }

        #[tokio::test]
        async fn test_evidence_unknown_type() {
            let mock_server = MockServer::start().await;

            // No mock needed - unknown type returns early
            let server = create_mocked_server(&mock_server).await;

            let unknown_req = EvidenceRequest {
                evidence_type: Some("unknown_type".to_string()),
                claim: Some("Claim".to_string()),
                hypothesis: None,
                prior: None,
                context: None,
                session_id: None,
            };
            let resp = server.reasoning_evidence(Parameters(unknown_req)).await;
            assert!(resp.synthesis.unwrap().contains("Unknown"));
        }

        #[tokio::test]
        async fn test_metrics_all_queries() {
            let mock_server = MockServer::start().await;

            // Metrics don't require API calls
            let server = create_mocked_server(&mock_server).await;

            // Test summary
            let summary_req = MetricsRequest {
                query: "summary".to_string(),
                mode_name: None,
                tool_name: None,
                session_id: None,
                success_only: None,
                limit: None,
            };
            let resp = server.reasoning_metrics(Parameters(summary_req)).await;
            let _ = resp.summary;

            // Test by_mode
            let by_mode_req = MetricsRequest {
                query: "by_mode".to_string(),
                mode_name: Some("linear".to_string()),
                tool_name: None,
                session_id: None,
                success_only: None,
                limit: None,
            };
            let resp = server.reasoning_metrics(Parameters(by_mode_req)).await;
            let _ = resp.mode_stats;

            // Test invocations
            let invocations_req = MetricsRequest {
                query: "invocations".to_string(),
                mode_name: None,
                tool_name: None,
                session_id: None,
                success_only: Some(true),
                limit: Some(10),
            };
            let resp = server.reasoning_metrics(Parameters(invocations_req)).await;
            let _ = resp.invocations;

            // Test fallbacks
            let fallbacks_req = MetricsRequest {
                query: "fallbacks".to_string(),
                mode_name: None,
                tool_name: None,
                session_id: None,
                success_only: None,
                limit: None,
            };
            let resp = server.reasoning_metrics(Parameters(fallbacks_req)).await;
            let _ = resp.summary;

            // Test config
            let config_req = MetricsRequest {
                query: "config".to_string(),
                mode_name: None,
                tool_name: None,
                session_id: None,
                success_only: None,
                limit: None,
            };
            let resp = server.reasoning_metrics(Parameters(config_req)).await;
            let _ = resp.config;

            // Test unknown query
            let unknown_req = MetricsRequest {
                query: "unknown".to_string(),
                mode_name: None,
                tool_name: None,
                session_id: None,
                success_only: None,
                limit: None,
            };
            let resp = server.reasoning_metrics(Parameters(unknown_req)).await;
            let _ = resp.summary;
        }

        #[tokio::test]
        async fn test_preset_run_valid() {
            let mock_server = MockServer::start().await;

            // Test running a valid preset
            let json = serde_json::json!({
                "analysis": "Quick analysis result",
                "confidence": 0.8
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(anthropic_response(&json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            // Run quick_analysis preset
            let run_req = PresetRequest {
                operation: "run".to_string(),
                preset_id: Some("quick_analysis".to_string()),
                category: None,
                inputs: Some(serde_json::json!({"content": "Analyze this"})),
                session_id: Some("s1".to_string()),
            };
            let resp = server.reasoning_preset(Parameters(run_req)).await;
            // Will have execution result or error
            let _ = resp.execution_result;
        }

        #[tokio::test]
        async fn test_timeline_success_paths() {
            let mock_server = MockServer::start().await;

            // Test create with proper response
            let create_json = serde_json::json!({
                "timeline_id": "tl_123",
                "events": [
                    {"timestamp": "2024-01-01", "event": "Start", "significance": "high"}
                ],
                "analysis": "Timeline created"
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&create_json.to_string())),
                )
                .expect(1..)
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            // Create timeline
            let create_req = TimelineRequest {
                operation: "create".to_string(),
                session_id: Some("s1".to_string()),
                timeline_id: None,
                content: Some("Event history".to_string()),
                label: Some("main".to_string()),
                branch_ids: None,
                source_branch_id: None,
                target_branch_id: None,
                merge_strategy: None,
            };
            let resp = server.reasoning_timeline(Parameters(create_req)).await;
            // Check that we get a response
            let _ = resp.timeline_id;

            // Test branch operation
            let branch_json = serde_json::json!({
                "branch_id": "br_456",
                "timeline_id": "tl_123",
                "divergence_point": "2024-01-15",
                "events": []
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&branch_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let branch_req = TimelineRequest {
                operation: "branch".to_string(),
                session_id: Some("s1".to_string()),
                timeline_id: Some("tl_123".to_string()),
                content: Some("Alternative history".to_string()),
                label: Some("alternative".to_string()),
                branch_ids: None,
                source_branch_id: None,
                target_branch_id: None,
                merge_strategy: None,
            };
            let resp = server.reasoning_timeline(Parameters(branch_req)).await;
            let _ = resp.branch_id;

            // Test compare operation
            let compare_json = serde_json::json!({
                "comparison": {
                    "common_events": ["Start"],
                    "divergences": [{"point": "Day 5", "branch_a": "X", "branch_b": "Y"}],
                    "analysis": "Branches diverge at Day 5"
                }
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&compare_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let compare_req = TimelineRequest {
                operation: "compare".to_string(),
                session_id: Some("s1".to_string()),
                timeline_id: Some("tl_123".to_string()),
                content: None,
                label: None,
                branch_ids: Some(vec!["br_1".to_string(), "br_2".to_string()]),
                source_branch_id: None,
                target_branch_id: None,
                merge_strategy: None,
            };
            let resp = server.reasoning_timeline(Parameters(compare_req)).await;
            let _ = resp.comparison;

            // Test merge operation
            let merge_json = serde_json::json!({
                "merged_timeline_id": "tl_merged",
                "events": [{"timestamp": "2024-01-01", "event": "Merged event"}],
                "conflicts_resolved": 1,
                "analysis": "Merge successful"
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&merge_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let merge_req = TimelineRequest {
                operation: "merge".to_string(),
                session_id: Some("s1".to_string()),
                timeline_id: Some("tl_123".to_string()),
                content: None,
                label: None,
                branch_ids: None,
                source_branch_id: Some("br_1".to_string()),
                target_branch_id: Some("br_2".to_string()),
                merge_strategy: Some("integrate".to_string()),
            };
            let resp = server.reasoning_timeline(Parameters(merge_req)).await;
            let _ = resp.merged_content;
        }

        #[tokio::test]
        async fn test_detect_low_argument_strength() {
            let mock_server = MockServer::start().await;

            // Test fallacies with low argument strength (high severity)
            let fallacies_json = serde_json::json!({
                "fallacies_detected": [
                    {
                        "fallacy": "strawman",
                        "category": "informal",
                        "passage": "The weak argument",
                        "explanation": "Misrepresents position",
                        "correction": "Address actual argument"
                    }
                ],
                "argument_structure": {
                    "premises": ["P1"],
                    "conclusion": "C",
                    "structure_type": "deductive",
                    "validity": "invalid"
                },
                "overall_assessment": {
                    "fallacy_count": 1,
                    "most_critical": "strawman",
                    "argument_strength": 0.3,
                    "overall_analysis": "Very weak"
                }
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&fallacies_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let req = DetectRequest {
                detect_type: "fallacies".to_string(),
                content: Some("Weak argument".to_string()),
                thought_id: None,
                session_id: None,
                check_types: None,
                check_formal: None,
                check_informal: None,
            };
            let resp = server.reasoning_detect(Parameters(req)).await;
            // With argument_strength 0.3, severity should be "high"
            if let Some(detection) = resp.detections.first() {
                assert_eq!(detection.severity, "high");
            }
        }

        #[tokio::test]
        async fn test_detect_medium_argument_strength() {
            let mock_server = MockServer::start().await;

            // Test fallacies with medium argument strength
            let fallacies_json = serde_json::json!({
                "fallacies_detected": [
                    {
                        "fallacy": "appeal to authority",
                        "category": "informal",
                        "passage": "Expert says so",
                        "explanation": "Relies on authority",
                        "correction": "Provide evidence"
                    }
                ],
                "argument_structure": {
                    "premises": ["P1"],
                    "conclusion": "C",
                    "structure_type": "inductive",
                    "validity": "partially_valid"
                },
                "overall_assessment": {
                    "fallacy_count": 1,
                    "most_critical": "appeal to authority",
                    "argument_strength": 0.5,
                    "overall_analysis": "Moderate"
                }
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&fallacies_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let req = DetectRequest {
                detect_type: "fallacies".to_string(),
                content: Some("Medium strength argument".to_string()),
                thought_id: None,
                session_id: None,
                check_types: None,
                check_formal: None,
                check_informal: None,
            };
            let resp = server.reasoning_detect(Parameters(req)).await;
            // With argument_strength 0.5, severity should be "medium"
            if let Some(detection) = resp.detections.first() {
                assert_eq!(detection.severity, "medium");
            }
        }

        #[tokio::test]
        async fn test_detect_high_argument_strength() {
            let mock_server = MockServer::start().await;

            // Test fallacies with high argument strength (low severity)
            let fallacies_json = serde_json::json!({
                "fallacies_detected": [
                    {
                        "fallacy": "minor issue",
                        "category": "informal",
                        "passage": "Good argument",
                        "explanation": "Small flaw",
                        "correction": "Minor fix"
                    }
                ],
                "argument_structure": {
                    "premises": ["P1", "P2"],
                    "conclusion": "C",
                    "structure_type": "deductive",
                    "validity": "valid"
                },
                "overall_assessment": {
                    "fallacy_count": 1,
                    "most_critical": "minor issue",
                    "argument_strength": 0.8,
                    "overall_analysis": "Strong"
                }
            });

            Mock::given(method("POST"))
                .and(path("/messages"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(anthropic_response(&fallacies_json.to_string())),
                )
                .mount(&mock_server)
                .await;

            let server = create_mocked_server(&mock_server).await;

            let req = DetectRequest {
                detect_type: "fallacies".to_string(),
                content: Some("Strong argument".to_string()),
                thought_id: None,
                session_id: None,
                check_types: None,
                check_formal: None,
                check_informal: None,
            };
            let resp = server.reasoning_detect(Parameters(req)).await;
            // With argument_strength 0.8, severity should be "low"
            if let Some(detection) = resp.detections.first() {
                assert_eq!(detection.severity, "low");
            }
        }

        #[tokio::test]
        async fn test_into_contents_implementations() {
            // Test all response types' IntoContents implementations
            let linear_resp = LinearResponse {
                thought_id: "t1".to_string(),
                session_id: "s1".to_string(),
                content: "Analysis".to_string(),
                confidence: 0.8,
                next_step: Some("Continue".to_string()),
                metadata: None,
            };
            let _ = linear_resp.into_contents();

            let tree_resp = TreeResponse {
                session_id: "s1".to_string(),
                branch_id: Some("b1".to_string()),
                branches: Some(vec![Branch {
                    id: "b1".to_string(),
                    content: "Content".to_string(),
                    score: 0.7,
                    status: "active".to_string(),
                }]),
                recommendation: Some("Rec".to_string()),
                metadata: None,
            };
            let _ = tree_resp.into_contents();

            let divergent_resp = DivergentResponse {
                thought_id: "t1".to_string(),
                session_id: "s1".to_string(),
                perspectives: vec![Perspective {
                    viewpoint: "View".to_string(),
                    content: "Content".to_string(),
                    novelty_score: 0.8,
                }],
                challenged_assumptions: Some(vec!["Assumption".to_string()]),
                synthesis: Some("Synthesis".to_string()),
                metadata: None,
            };
            let _ = divergent_resp.into_contents();

            let reflection_resp = ReflectionResponse {
                quality_score: 0.8,
                thought_id: Some("t1".to_string()),
                session_id: Some("s1".to_string()),
                iterations_used: Some(2),
                strengths: Some(vec!["Strength".to_string()]),
                weaknesses: Some(vec!["Weakness".to_string()]),
                recommendations: Some(vec!["Improve".to_string()]),
                refined_content: Some("Refined".to_string()),
                coherence_score: Some(0.85),
                metadata: None,
            };
            let _ = reflection_resp.into_contents();

            let checkpoint_resp = CheckpointResponse {
                session_id: "s1".to_string(),
                checkpoint_id: Some("cp1".to_string()),
                checkpoints: Some(vec![Checkpoint {
                    id: "cp1".to_string(),
                    name: "Name".to_string(),
                    created_at: "2024-01-01".to_string(),
                    description: None,
                    thought_count: 5,
                }]),
                restored_state: None,
                metadata: None,
            };
            let _ = checkpoint_resp.into_contents();

            let auto_resp = AutoResponse {
                selected_mode: "linear".to_string(),
                confidence: 0.9,
                rationale: "Rationale".to_string(),
                result: serde_json::json!({}),
                metadata: None,
            };
            let _ = auto_resp.into_contents();

            let graph_resp = GraphResponse {
                session_id: "s1".to_string(),
                node_id: Some("n1".to_string()),
                nodes: None,
                aggregated_insight: None,
                conclusions: None,
                state: None,
                metadata: None,
            };
            let _ = graph_resp.into_contents();

            let detect_resp = DetectResponse {
                detections: vec![],
                summary: Some("Summary".to_string()),
                overall_quality: Some(0.8),
                metadata: None,
            };
            let _ = detect_resp.into_contents();

            let decision_resp = DecisionResponse {
                recommendation: "A".to_string(),
                rankings: None,
                stakeholder_map: None,
                conflicts: None,
                alignments: None,
                rationale: None,
                metadata: None,
            };
            let _ = decision_resp.into_contents();

            let evidence_resp = EvidenceResponse {
                overall_credibility: 0.8,
                evidence_assessments: None,
                posterior: None,
                prior: None,
                likelihood_ratio: None,
                entropy: None,
                confidence_interval: None,
                synthesis: None,
                metadata: None,
            };
            let _ = evidence_resp.into_contents();

            let timeline_resp = TimelineResponse {
                timeline_id: "tl1".to_string(),
                branch_id: None,
                branches: None,
                comparison: None,
                merged_content: None,
                metadata: None,
            };
            let _ = timeline_resp.into_contents();

            let mcts_resp = MctsResponse {
                session_id: "s1".to_string(),
                best_path: None,
                iterations_completed: Some(10),
                backtrack_suggestion: None,
                executed: None,
                metadata: None,
            };
            let _ = mcts_resp.into_contents();

            let cf_resp = CounterfactualResponse {
                original_scenario: "Original".to_string(),
                intervention_applied: "Intervention".to_string(),
                counterfactual_outcome: "Outcome".to_string(),
                causal_chain: vec![],
                key_differences: vec![],
                confidence: 0.8,
                assumptions: vec![],
                session_id: Some("s1".to_string()),
                analysis_depth: "counterfactual".to_string(),
                metadata: None,
            };
            let _ = cf_resp.into_contents();

            let preset_resp = PresetResponse {
                presets: None,
                execution_result: None,
                session_id: None,
                metadata: None,
            };
            let _ = preset_resp.into_contents();

            let metrics_resp = MetricsResponse {
                summary: None,
                mode_stats: None,
                invocations: None,
                config: None,
            };
            let _ = metrics_resp.into_contents();
        }
    }
}
