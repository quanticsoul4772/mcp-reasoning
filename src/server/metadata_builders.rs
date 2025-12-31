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
    use crate::metadata::{PresetIndex, TimingDatabase};
    use crate::storage::SqliteStorage;
    use std::sync::Arc;

    async fn create_test_builder() -> MetadataBuilder {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");

        // Manually create the table since migrations don't run for in-memory DB
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tool_timing_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tool_name TEXT NOT NULL,
                mode_name TEXT,
                duration_ms INTEGER NOT NULL,
                complexity_score INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )",
        )
        .execute(&storage.pool)
        .await
        .expect("create table");

        let timing_db = Arc::new(TimingDatabase::new(Arc::new(storage)));
        let preset_index = Arc::new(PresetIndex::build());
        MetadataBuilder::new(timing_db, preset_index, 30_000)
    }

    #[tokio::test]
    async fn test_build_metadata_for_divergent_standard() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_divergent(
            &builder,
            1000,
            3,
            false,
            Some("session-1".to_string()),
            100,
        )
        .await;
        assert!(result.is_ok(), "Error: {:?}", result.err());
        let metadata = result.unwrap();
        assert!(metadata.timing.estimated_duration_ms > 0);
    }

    #[tokio::test]
    async fn test_build_metadata_for_divergent_rebellion() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_divergent(
            &builder,
            2000,
            5,
            true,
            Some("session-2".to_string()),
            150,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_divergent_complex_perspectives() {
        let builder = create_test_builder().await;
        // Many perspectives makes it complex
        let result = build_metadata_for_divergent(&builder, 1000, 6, false, None, 200).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_divergent_moderate_content() {
        let builder = create_test_builder().await;
        // Large content with few perspectives = moderate
        let result = build_metadata_for_divergent(&builder, 4000, 2, false, None, 180).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_divergent_simple() {
        let builder = create_test_builder().await;
        // Small content, few perspectives = simple
        let result = build_metadata_for_divergent(&builder, 500, 2, false, None, 80).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_decision_weighted() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_decision(
            &builder,
            1500,
            "weighted",
            4,
            Some("session-3".to_string()),
            120,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_decision_topsis() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_decision(&builder, 2000, "topsis", 5, None, 200).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_decision_perspectives() {
        let builder = create_test_builder().await;
        let result =
            build_metadata_for_decision(&builder, 1800, "perspectives", 3, None, 150).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_decision_pairwise_many_options() {
        let builder = create_test_builder().await;
        // Many options = complex
        let result = build_metadata_for_decision(&builder, 1200, "pairwise", 8, None, 180).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_decision_pairwise_few_options() {
        let builder = create_test_builder().await;
        // Few options = moderate
        let result = build_metadata_for_decision(&builder, 1000, "pairwise", 3, None, 100).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_tree_create() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_tree(
            &builder,
            1000,
            "create",
            3,
            Some("session-4".to_string()),
            90,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_tree_create_many_branches() {
        let builder = create_test_builder().await;
        // Many branches = complex
        let result = build_metadata_for_tree(&builder, 1500, "create", 5, None, 120).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_tree_focus() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_tree(&builder, 800, "focus", 1, None, 50).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_tree_list() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_tree(&builder, 500, "list", 4, None, 30).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_tree_complete() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_tree(&builder, 600, "complete", 1, None, 40).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_tree_unknown() {
        let builder = create_test_builder().await;
        // Unknown operation = moderate
        let result = build_metadata_for_tree(&builder, 700, "unknown_op", 2, None, 60).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_init() {
        let builder = create_test_builder().await;
        let result =
            build_metadata_for_graph(&builder, 1200, "init", 1, Some("session-5".to_string()), 80)
                .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_generate_many() {
        let builder = create_test_builder().await;
        // Many nodes = complex
        let result = build_metadata_for_graph(&builder, 2000, "generate", 8, None, 150).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_generate_few() {
        let builder = create_test_builder().await;
        // Few nodes = moderate
        let result = build_metadata_for_graph(&builder, 1000, "generate", 3, None, 100).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_score() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_graph(&builder, 500, "score", 2, None, 40).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_aggregate() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_graph(&builder, 1500, "aggregate", 5, None, 120).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_refine() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_graph(&builder, 1000, "refine", 3, None, 80).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_prune() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_graph(&builder, 800, "prune", 4, None, 60).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_finalize() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_graph(&builder, 1200, "finalize", 2, None, 100).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_state() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_graph(&builder, 400, "state", 3, None, 30).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_unknown() {
        let builder = create_test_builder().await;
        // Unknown operation = moderate
        let result = build_metadata_for_graph(&builder, 600, "unknown", 2, None, 50).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_graph_zero_nodes() {
        let builder = create_test_builder().await;
        // Zero nodes should be capped to 1
        let result = build_metadata_for_graph(&builder, 500, "init", 0, None, 40).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_reflection_process() {
        let builder = create_test_builder().await;
        let result = build_metadata_for_reflection(
            &builder,
            1000,
            "process",
            2,
            0.75,
            Some("session-6".to_string()),
            100,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_reflection_process_many_iterations() {
        let builder = create_test_builder().await;
        // Many iterations = complex
        let result =
            build_metadata_for_reflection(&builder, 1500, "process", 5, 0.8, None, 150).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_reflection_process_low_quality() {
        let builder = create_test_builder().await;
        // Low quality score = complex
        let result =
            build_metadata_for_reflection(&builder, 1000, "process", 2, 0.4, None, 120).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_reflection_evaluate() {
        let builder = create_test_builder().await;
        let result =
            build_metadata_for_reflection(&builder, 800, "evaluate", 1, 0.9, None, 60).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_reflection_unknown() {
        let builder = create_test_builder().await;
        // Unknown operation = moderate
        let result =
            build_metadata_for_reflection(&builder, 700, "unknown", 1, 0.7, None, 50).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_build_metadata_for_reflection_zero_iterations() {
        let builder = create_test_builder().await;
        // Zero iterations should be capped to 1
        let result =
            build_metadata_for_reflection(&builder, 500, "process", 0, 0.8, None, 40).await;
        assert!(result.is_ok());
    }
}
