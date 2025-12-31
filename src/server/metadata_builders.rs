//! Metadata builder helper functions for tool responses.

use crate::error::AppError;
use crate::metadata::{
    ComplexityMetrics, MetadataBuilder, MetadataRequest, ResponseMetadata, ResultContext,
};

/// Build metadata for divergent reasoning response.
pub async fn build_metadata_for_divergent(
    builder: &MetadataBuilder,
    content_length: usize,
    num_perspectives: usize,
    force_rebellion: bool,
    session_id: Option<String>,
    elapsed_ms: u64,
) -> Result<ResponseMetadata, AppError> {
    // Record actual execution time
    let complexity = ComplexityMetrics {
        content_length,
        thinking_budget: None,
        num_perspectives: Some(num_perspectives as u32),
        num_branches: None,
    };

    builder
        .timing_db()
        .record_execution(
            "reasoning_divergent",
            if force_rebellion {
                Some("rebellion")
            } else {
                Some("standard")
            },
            elapsed_ms,
            complexity.clone(),
        )
        .await?;

    // Build metadata request
    let metadata_req = MetadataRequest {
        tool_name: "reasoning_divergent".into(),
        mode_name: Some(if force_rebellion {
            "rebellion".into()
        } else {
            "standard".into()
        }),
        complexity,
        result_context: ResultContext {
            num_outputs: num_perspectives,
            has_branches: true,
            session_id,
            complexity: if force_rebellion || num_perspectives > 4 {
                "complex".into()
            } else if num_perspectives > 2 || content_length > 3000 {
                "moderate".into()
            } else {
                "simple".into()
            },
        },
        tool_history: vec![],
        goal: None,
        thinking_budget: Some("standard".into()),
        session_state: None,
    };

    builder.build(&metadata_req).await
}

/// Build metadata for decision analysis response.
#[allow(dead_code)]
pub async fn build_metadata_for_decision(
    builder: &MetadataBuilder,
    content_length: usize,
    decision_type: &str,
    num_options: usize,
    session_id: Option<String>,
    elapsed_ms: u64,
) -> Result<ResponseMetadata, AppError> {
    // Record actual execution time
    let complexity = ComplexityMetrics {
        content_length,
        thinking_budget: None,
        num_perspectives: Some(num_options as u32),
        num_branches: None,
    };

    builder
        .timing_db()
        .record_execution(
            "reasoning_decision",
            Some(decision_type),
            elapsed_ms,
            complexity.clone(),
        )
        .await?;

    // Build metadata request
    let metadata_req = MetadataRequest {
        tool_name: "reasoning_decision".into(),
        mode_name: Some(decision_type.into()),
        complexity,
        result_context: ResultContext {
            num_outputs: num_options,
            has_branches: decision_type == "perspectives",
            session_id,
            complexity: match decision_type {
                "topsis" => "complex".into(),
                "perspectives" => "complex".into(),
                "pairwise" if num_options > 5 => "complex".into(),
                "pairwise" => "moderate".into(),
                _ => "moderate".into(),
            },
        },
        tool_history: vec![],
        goal: None,
        thinking_budget: Some("standard".into()),
        session_state: None,
    };

    builder.build(&metadata_req).await
}

/// Build metadata for tree reasoning response.
pub async fn build_metadata_for_tree(
    builder: &MetadataBuilder,
    content_length: usize,
    operation: &str,
    num_branches: usize,
    session_id: Option<String>,
    elapsed_ms: u64,
) -> Result<ResponseMetadata, AppError> {
    // Record actual execution time
    let complexity = ComplexityMetrics {
        content_length,
        thinking_budget: None,
        num_perspectives: None,
        num_branches: Some(num_branches as u32),
    };

    builder
        .timing_db()
        .record_execution(
            "reasoning_tree",
            Some(operation),
            elapsed_ms,
            complexity.clone(),
        )
        .await?;

    // Build metadata request
    let metadata_req = MetadataRequest {
        tool_name: "reasoning_tree".into(),
        mode_name: Some(operation.into()),
        complexity,
        result_context: ResultContext {
            num_outputs: num_branches,
            has_branches: true,
            session_id,
            complexity: match operation {
                "create" if num_branches > 3 => "complex".into(),
                "create" => "moderate".into(),
                "focus" => "simple".into(),
                "list" => "simple".into(),
                "complete" => "simple".into(),
                _ => "moderate".into(),
            },
        },
        tool_history: vec![],
        goal: None,
        thinking_budget: Some("standard".into()),
        session_state: None,
    };

    builder.build(&metadata_req).await
}

/// Build metadata for graph reasoning response.
pub async fn build_metadata_for_graph(
    builder: &MetadataBuilder,
    content_length: usize,
    operation: &str,
    num_nodes: usize,
    session_id: Option<String>,
    elapsed_ms: u64,
) -> Result<ResponseMetadata, AppError> {
    // Record actual execution time
    let complexity = ComplexityMetrics {
        content_length,
        thinking_budget: None,
        num_perspectives: None,
        num_branches: Some(num_nodes as u32),
    };

    builder
        .timing_db()
        .record_execution(
            "reasoning_graph",
            Some(operation),
            elapsed_ms,
            complexity.clone(),
        )
        .await?;

    // Build metadata request
    let metadata_req = MetadataRequest {
        tool_name: "reasoning_graph".into(),
        mode_name: Some(operation.into()),
        complexity,
        result_context: ResultContext {
            num_outputs: num_nodes.max(1),
            has_branches: true,
            session_id,
            complexity: match operation {
                "init" => "simple".into(),
                "generate" if num_nodes > 5 => "complex".into(),
                "generate" => "moderate".into(),
                "score" => "simple".into(),
                "aggregate" => "complex".into(),
                "refine" => "moderate".into(),
                "prune" => "moderate".into(),
                "finalize" => "complex".into(),
                "state" => "simple".into(),
                _ => "moderate".into(),
            },
        },
        tool_history: vec![],
        goal: None,
        thinking_budget: Some("standard".into()),
        session_state: None,
    };

    builder.build(&metadata_req).await
}

/// Build metadata for reflection reasoning response.
pub async fn build_metadata_for_reflection(
    builder: &MetadataBuilder,
    content_length: usize,
    operation: &str,
    iterations_used: usize,
    quality_score: f64,
    session_id: Option<String>,
    elapsed_ms: u64,
) -> Result<ResponseMetadata, AppError> {
    // Record actual execution time
    let complexity = ComplexityMetrics {
        content_length,
        thinking_budget: None,
        num_perspectives: None,
        num_branches: Some(iterations_used as u32),
    };

    builder
        .timing_db()
        .record_execution(
            "reasoning_reflection",
            Some(operation),
            elapsed_ms,
            complexity.clone(),
        )
        .await?;

    // Build metadata request
    let metadata_req = MetadataRequest {
        tool_name: "reasoning_reflection".into(),
        mode_name: Some(operation.into()),
        complexity,
        result_context: ResultContext {
            num_outputs: iterations_used.max(1),
            has_branches: false,
            session_id,
            complexity: match operation {
                "process" if iterations_used > 3 => "complex".into(),
                "process" if quality_score < 0.6 => "complex".into(),
                "process" => "moderate".into(),
                "evaluate" => "simple".into(),
                _ => "moderate".into(),
            },
        },
        tool_history: vec![],
        goal: None,
        thinking_budget: Some("standard".into()),
        session_state: None,
    };

    builder.build(&metadata_req).await
}
