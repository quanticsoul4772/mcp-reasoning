//! Meta-reasoning mode.
//!
//! Selects the best reasoning tool based on empirical effectiveness data.
//! Falls back to auto mode when no effectiveness data exists.

use serde::{Deserialize, Serialize};

use crate::error::ModeError;
use crate::metrics::{MetricsCollector, ToolEffectiveness};
use crate::modes::{extract_json, validate_content};
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message, StorageTrait};

/// Problem type classification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemClassification {
    /// Classified problem type.
    pub problem_type: String,
    /// Brief reasoning for classification.
    pub reasoning: String,
}

/// Result of meta-reasoning routing.
#[derive(Debug, Clone)]
pub struct MetaRouteResult {
    /// Selected tool/mode name.
    pub selected_tool: String,
    /// Classified problem type.
    pub problem_type: String,
    /// Confidence in the recommendation (0.0-1.0).
    pub confidence: f64,
    /// Reasoning for selection.
    pub reasoning: String,
    /// Whether we fell back to auto mode.
    pub fallback_to_auto: bool,
    /// Effectiveness data for the selected tool.
    pub effectiveness: Option<ToolEffectiveness>,
    /// All candidate tools considered.
    pub candidates: Vec<ToolEffectiveness>,
}

/// Meta-reasoning mode that routes based on empirical effectiveness.
pub struct MetaMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    #[allow(dead_code)]
    storage: S,
    client: C,
}

impl<S, C> MetaMode<S, C>
where
    S: StorageTrait,
    C: AnthropicClientTrait,
{
    /// Create a new meta-reasoning mode.
    pub fn new(storage: S, client: C) -> Self {
        Self { storage, client }
    }

    /// Route to optimal tool based on problem classification and effectiveness data.
    pub async fn route(
        &self,
        content: &str,
        problem_type_hint: Option<String>,
        min_confidence: Option<f64>,
        metrics: &MetricsCollector,
    ) -> Result<MetaRouteResult, ModeError> {
        validate_content(content)?;

        // Step 1: Classify problem type
        let classification = if let Some(hint) = problem_type_hint {
            ProblemClassification {
                problem_type: hint.clone(),
                reasoning: format!("User-provided hint: {hint}"),
            }
        } else {
            self.classify_problem(content).await?
        };

        tracing::info!(
            tool = "reasoning_meta",
            problem_type = %classification.problem_type,
            classification_reasoning = %classification.reasoning,
            "Problem classified"
        );

        // Step 2: Query effectiveness data
        let candidates = metrics.effectiveness_by_context(&classification.problem_type);
        let recommendation = metrics.recommend_tool(&classification.problem_type);

        if let Some((tool_name, confidence)) = recommendation {
            let threshold = min_confidence.unwrap_or(0.4);
            if confidence >= threshold {
                let effectiveness = candidates
                    .iter()
                    .find(|e| e.tool_name == tool_name)
                    .cloned();

                let sample_count = effectiveness.as_ref().map_or(0, |e| e.sample_count);

                return Ok(MetaRouteResult {
                    selected_tool: tool_name.clone(),
                    problem_type: classification.problem_type,
                    confidence,
                    reasoning: format!(
                        "Selected {} based on {} observations with {:.0}% success rate for this problem type. {}",
                        tool_name,
                        sample_count,
                        effectiveness.as_ref().map_or(0.0, |e| e.success_rate * 100.0),
                        classification.reasoning
                    ),
                    fallback_to_auto: false,
                    effectiveness,
                    candidates,
                });
            }
        }

        // Step 3: Fall back — not enough data
        let reason = if candidates.is_empty() {
            "No effectiveness data for this problem type"
        } else {
            "Insufficient confidence in available data"
        };

        tracing::info!(
            tool = "reasoning_meta",
            reason = reason,
            problem_type = %classification.problem_type,
            "Falling back to auto mode recommendation"
        );

        Ok(MetaRouteResult {
            selected_tool: "auto".to_string(),
            problem_type: classification.problem_type,
            confidence: 0.5,
            reasoning: format!(
                "{}. Use reasoning_auto for routing. {}",
                reason, classification.reasoning
            ),
            fallback_to_auto: true,
            effectiveness: None,
            candidates,
        })
    }

    /// Classify problem type using the LLM.
    async fn classify_problem(&self, content: &str) -> Result<ProblemClassification, ModeError> {
        // Truncate content for classification (no need for full input)
        let truncated = if content.len() > 2000 {
            &content[..2000]
        } else {
            content
        };

        let prompt = format!(
            r#"Classify this problem into ONE category. Respond with JSON only.

Categories:
- math: mathematical, analytical, calculation, proof
- code_review: code analysis, debugging, refactoring, implementation
- planning: strategy, workflow design, project planning, decision-making
- brainstorming: creative, exploration, ideation, divergent thinking
- summarization: synthesis, condensing, overview, extraction
- research: investigation, evidence gathering, literature review
- evaluation: comparison, assessment, judgment, scoring
- causal: cause-and-effect, root cause analysis, counterfactual
- temporal: timeline, scheduling, sequence analysis
- other: doesn't fit above categories

Content:
{truncated}

Respond: {{"problem_type": "category", "reasoning": "one sentence why"}}"#
        );

        let messages = vec![Message::user(prompt)];
        let config = CompletionConfig::default()
            .with_max_tokens(256)
            .with_temperature(0.2);

        let response = self.client.complete(messages, config).await.map_err(|e| {
            ModeError::ApiUnavailable {
                message: format!("Classification failed: {e}"),
            }
        })?;

        let json = extract_json(&response.content)?;

        let problem_type = json
            .get("problem_type")
            .and_then(|v| v.as_str())
            .unwrap_or("other")
            .to_string();

        let reasoning = json
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();

        Ok(ProblemClassification {
            problem_type,
            reasoning,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::metrics::MetricEvent;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, MockStorageTrait, Usage};

    fn create_mock_metrics_with_data() -> MetricsCollector {
        let metrics = MetricsCollector::new();
        // Add enough data points for recommendation (need 3+)
        for i in 0..5 {
            metrics.record(
                MetricEvent::new("linear", 100 + i * 10, true)
                    .with_problem_type("math")
                    .with_quality_rating(0.9),
            );
        }
        for i in 0..3 {
            metrics.record(
                MetricEvent::new("tree", 200 + i * 10, i < 2)
                    .with_problem_type("planning")
                    .with_quality_rating(0.7),
            );
        }
        metrics
    }

    #[tokio::test]
    async fn test_route_with_hint_and_data() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let metrics = create_mock_metrics_with_data();

        let mode = MetaMode::new(mock_storage, mock_client);
        let result = mode
            .route(
                "Solve x^2 + 3x + 2 = 0",
                Some("math".to_string()),
                None,
                &metrics,
            )
            .await;

        assert!(result.is_ok());
        let route = result.unwrap();
        assert_eq!(route.selected_tool, "linear");
        assert_eq!(route.problem_type, "math");
        assert!(!route.fallback_to_auto);
        assert!(route.confidence > 0.4);
    }

    #[tokio::test]
    async fn test_route_fallback_no_data() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let metrics = MetricsCollector::new(); // Empty metrics

        let mode = MetaMode::new(mock_storage, mock_client);
        let result = mode
            .route(
                "Something",
                Some("unknown_type".to_string()),
                None,
                &metrics,
            )
            .await;

        assert!(result.is_ok());
        let route = result.unwrap();
        assert_eq!(route.selected_tool, "auto");
        assert!(route.fallback_to_auto);
    }

    #[tokio::test]
    async fn test_route_with_classification() {
        let mock_storage = MockStorageTrait::new();
        let mut mock_client = MockAnthropicClientTrait::new();
        let metrics = create_mock_metrics_with_data();

        mock_client.expect_complete().returning(|_, _| {
            Ok(CompletionResponse::new(
                r#"{"problem_type": "math", "reasoning": "Contains equation solving"}"#,
                Usage::new(50, 80),
            ))
        });

        let mode = MetaMode::new(mock_storage, mock_client);
        let result = mode
            .route("Solve x^2 + 3x + 2 = 0", None, None, &metrics)
            .await;

        assert!(result.is_ok());
        let route = result.unwrap();
        assert_eq!(route.problem_type, "math");
        assert_eq!(route.selected_tool, "linear");
        assert!(!route.fallback_to_auto);
    }

    #[tokio::test]
    async fn test_route_high_confidence_threshold() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let metrics = create_mock_metrics_with_data();

        let mode = MetaMode::new(mock_storage, mock_client);
        // Set very high threshold that data can't meet
        let result = mode
            .route("Solve 2+2", Some("math".to_string()), Some(0.99), &metrics)
            .await;

        assert!(result.is_ok());
        let route = result.unwrap();
        assert!(route.fallback_to_auto);
    }

    #[tokio::test]
    async fn test_classify_empty_content() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let metrics = MetricsCollector::new();

        let mode = MetaMode::new(mock_storage, mock_client);
        let result = mode.route("", None, None, &metrics).await;

        assert!(result.is_err());
    }
}
