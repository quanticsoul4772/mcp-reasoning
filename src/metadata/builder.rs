//! Builder for constructing response metadata.

use super::{
    timing::ComplexityMetrics, ContextMetadata, PresetIndex, ResponseMetadata, SuggestionEngine,
    SuggestionMetadata, TimingDatabase, TimingMetadata,
};
use crate::error::AppError;
use crate::metrics::MetricsCollector;
use std::sync::Arc;

/// Builder for constructing response metadata.
pub struct MetadataBuilder {
    timing_db: Arc<TimingDatabase>,
    suggestion_engine: Arc<SuggestionEngine>,
    factory_timeout_ms: u64,
    /// Runtime metrics used to enrich next-tool suggestions with observed
    /// tool-chain transitions. Optional: when absent (e.g. in unit tests), the
    /// builder falls back to purely static suggestions.
    metrics: Option<Arc<MetricsCollector>>,
}

impl MetadataBuilder {
    /// Create a new metadata builder.
    #[must_use]
    pub fn new(
        timing_db: Arc<TimingDatabase>,
        preset_index: Arc<PresetIndex>,
        factory_timeout_ms: u64,
    ) -> Self {
        let suggestion_engine = Arc::new(SuggestionEngine::new(preset_index));

        Self {
            timing_db,
            suggestion_engine,
            factory_timeout_ms,
            metrics: None,
        }
    }

    /// Attach the runtime metrics collector so next-tool suggestions are blended
    /// with empirically observed tool-chain transitions.
    #[must_use]
    pub fn with_metrics(mut self, metrics: Arc<MetricsCollector>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Build complete metadata for a tool response.
    ///
    /// # Errors
    ///
    /// Returns error if timing database query fails.
    pub async fn build(&self, request: &MetadataRequest) -> Result<ResponseMetadata, AppError> {
        // 1. Estimate timing
        let timing = self.build_timing_metadata(request).await?;

        // 2. Generate suggestions
        let suggestions = self.build_suggestion_metadata(request);

        // 3. Build context
        let context = self.build_context_metadata(request);

        Ok(ResponseMetadata {
            timing,
            suggestions,
            context,
        })
    }

    async fn build_timing_metadata(
        &self,
        request: &MetadataRequest,
    ) -> Result<TimingMetadata, AppError> {
        let (estimated_duration_ms, confidence) = self
            .timing_db
            .estimate_duration(
                &request.tool_name,
                request.mode_name.as_deref(),
                request.complexity.clone(),
            )
            .await?;

        Ok(TimingMetadata {
            estimated_duration_ms,
            confidence,
            will_timeout_on_factory: estimated_duration_ms > self.factory_timeout_ms,
            factory_timeout_ms: self.factory_timeout_ms,
        })
    }

    fn build_suggestion_metadata(&self, request: &MetadataRequest) -> SuggestionMetadata {
        let empirical = self
            .metrics
            .as_ref()
            .map(|m| m.transition_stats_from(&request.tool_name))
            .unwrap_or_default();
        // Destinations the self-improvement loop has actively suppressed from
        // this tool (sustained anti-patterns) — hard-blocked from suggestions.
        let suppressed = self
            .metrics
            .as_ref()
            .map(|m| m.suppressed_destinations_from(&request.tool_name))
            .unwrap_or_default();
        let next_tools = self.suggestion_engine.suggest_next_tools_blended(
            &request.tool_name,
            &request.result_context,
            &empirical,
            &suppressed,
        );

        // Prefer the session's actual tool sequence (reconstructed from observed
        // transitions) over the request's hint, so preset matching reflects what
        // the agent really did. Falls back to the request when unavailable.
        let session_history = match (
            self.metrics.as_ref(),
            request.result_context.session_id.as_deref(),
        ) {
            (Some(metrics), Some(session_id)) if !session_id.is_empty() => {
                let history = metrics.session_tool_history(session_id);
                if history.is_empty() {
                    request.tool_history.clone()
                } else {
                    history
                }
            }
            _ => request.tool_history.clone(),
        };

        let relevant_presets = self
            .suggestion_engine
            .suggest_presets(&session_history, request.goal.as_deref());

        SuggestionMetadata {
            next_tools,
            relevant_presets,
        }
    }

    fn build_context_metadata(&self, request: &MetadataRequest) -> ContextMetadata {
        ContextMetadata {
            mode_used: request.mode_name.clone().unwrap_or_else(|| "none".into()),
            thinking_budget: request.thinking_budget.clone(),
            session_state: request.session_state.clone(),
        }
    }

    /// Get timing database reference.
    #[must_use]
    pub fn timing_db(&self) -> &Arc<TimingDatabase> {
        &self.timing_db
    }
}

/// Request context for metadata generation.
#[derive(Debug, Clone, Default)]
pub struct MetadataRequest {
    /// Name of the tool being executed.
    pub tool_name: String,
    /// Mode name if applicable.
    pub mode_name: Option<String>,
    /// Complexity metrics for this request.
    pub complexity: ComplexityMetrics,
    /// Result context from execution.
    pub result_context: super::suggestions::ResultContext,
    /// Recent tool history for this session.
    pub tool_history: Vec<String>,
    /// User's stated goal if known.
    pub goal: Option<String>,
    /// Thinking budget level.
    pub thinking_budget: Option<String>,
    /// Session state information.
    pub session_state: Option<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::SqliteStorage;

    async fn create_test_builder() -> MetadataBuilder {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let timing_db = Arc::new(TimingDatabase::new(Arc::new(storage)));
        let preset_index = Arc::new(PresetIndex::build());

        MetadataBuilder::new(timing_db, preset_index, 30_000)
    }

    #[tokio::test]
    async fn test_build_suggestions_blend_observed_transitions() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let timing_db = Arc::new(TimingDatabase::new(Arc::new(storage)));
        let preset_index = Arc::new(PresetIndex::build());
        let metrics = Arc::new(crate::metrics::MetricsCollector::new());

        // Observe reasoning_tree → reasoning_mcts succeeding repeatedly.
        for i in 0..6 {
            let sid = format!("s{i}");
            metrics.record_tool_use(&sid, "reasoning_tree", true);
            metrics.record_tool_use(&sid, "reasoning_mcts", true);
        }

        let builder = MetadataBuilder::new(timing_db, preset_index, 30_000)
            .with_metrics(Arc::clone(&metrics));

        let request = MetadataRequest {
            tool_name: "reasoning_tree".into(),
            mode_name: Some("create".into()),
            result_context: super::super::suggestions::ResultContext {
                has_branches: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let metadata = builder.build(&request).await.expect("build");
        // The observed high-success transition is promoted to the front.
        assert_eq!(
            metadata
                .suggestions
                .next_tools
                .first()
                .map(|s| s.tool.as_str()),
            Some("reasoning_mcts")
        );
    }

    #[tokio::test]
    async fn test_build_metadata_simple() {
        let builder = create_test_builder().await;

        let request = MetadataRequest {
            tool_name: "reasoning_linear".into(),
            mode_name: Some("linear".into()),
            complexity: ComplexityMetrics {
                content_length: 500,
                ..Default::default()
            },
            ..Default::default()
        };

        let metadata = builder.build(&request).await.expect("build");

        assert!(metadata.timing.estimated_duration_ms > 0);
        assert!(!metadata.timing.will_timeout_on_factory);
        assert_eq!(metadata.context.mode_used, "linear");
    }

    #[tokio::test]
    async fn test_build_metadata_will_timeout() {
        let builder = create_test_builder().await;

        let request = MetadataRequest {
            tool_name: "reasoning_divergent".into(),
            mode_name: Some("divergent".into()),
            complexity: ComplexityMetrics {
                num_perspectives: Some(4),
                thinking_budget: Some(16384),
                content_length: 5000,
                ..Default::default()
            },
            ..Default::default()
        };

        let metadata = builder.build(&request).await.expect("build");

        // With 4 perspectives + max thinking, should exceed 30s
        assert!(metadata.timing.will_timeout_on_factory);
    }

    #[tokio::test]
    async fn test_build_metadata_with_suggestions() {
        let builder = create_test_builder().await;

        let request = MetadataRequest {
            tool_name: "reasoning_divergent".into(),
            mode_name: Some("divergent".into()),
            result_context: super::super::suggestions::ResultContext {
                num_outputs: 4,
                complexity: "complex".into(),
                ..Default::default()
            },
            ..Default::default()
        };

        let metadata = builder.build(&request).await.expect("build");

        assert!(!metadata.suggestions.next_tools.is_empty());
        assert!(metadata
            .suggestions
            .next_tools
            .iter()
            .any(|s| s.tool == "reasoning_checkpoint"));
    }

    #[tokio::test]
    async fn test_build_metadata_with_tool_history() {
        let builder = create_test_builder().await;

        let request = MetadataRequest {
            tool_name: "reasoning_decision".into(),
            tool_history: vec!["reasoning_divergent".into(), "reasoning_decision".into()],
            ..Default::default()
        };

        let metadata = builder.build(&request).await.expect("build");

        // Should suggest decision_analysis preset
        assert!(metadata
            .suggestions
            .relevant_presets
            .iter()
            .any(|p| p.preset_id == "decision_analysis"));
    }
}
