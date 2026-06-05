//! Meta-reasoning mode.
//!
//! Selects the best reasoning tool based on empirical effectiveness data.
//! Falls back to auto mode when no effectiveness data exists.

use serde::{Deserialize, Serialize};

use crate::error::ModeError;
use crate::metrics::{MetricsCollector, ToolEffectiveness, TransitionStats};
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
        previous_tool: Option<String>,
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

        // Tool-chain signal: which tool has historically succeeded after the tool
        // the caller just ran. Used to enrich a confident pick and to resolve the
        // low-data fallback below. `None` when there's no prior tool or no data.
        let chain_hint = previous_tool
            .as_deref()
            .and_then(|prev| Self::chain_hint(metrics, prev));
        let chain_note = match (&chain_hint, previous_tool.as_deref()) {
            (Some((next_tool, stats)), Some(prev)) => format!(
                " Tool-chain history: {} succeeded after {} in {:.0}% of {} sessions.",
                next_tool,
                prev,
                stats.success_rate * 100.0,
                stats.count
            ),
            _ => String::new(),
        };

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
                        "Selected {} based on {} observations with {:.0}% success rate for this problem type. {}{}",
                        tool_name,
                        sample_count,
                        effectiveness.as_ref().map_or(0.0, |e| e.success_rate * 100.0),
                        classification.reasoning,
                        chain_note
                    ),
                    fallback_to_auto: false,
                    effectiveness,
                    candidates,
                });
            }
        }

        // Step 3: Fall back — not enough effectiveness data. If tool-chain history
        // points to a tool that reliably follows the previous one, route there
        // instead of a blind auto fallback. This is what makes the recorded
        // transitions an actual routing decision rather than inert data.
        if let Some((next_tool, stats)) = chain_hint {
            tracing::info!(
                tool = "reasoning_meta",
                next_tool = %next_tool,
                problem_type = %classification.problem_type,
                "Routing via tool-chain history (no confident effectiveness data)"
            );

            return Ok(MetaRouteResult {
                selected_tool: next_tool.clone(),
                problem_type: classification.problem_type,
                confidence: stats.success_rate,
                reasoning: format!(
                    "No confident effectiveness data for this problem type, but tool-chain \
                     history shows {} succeeded after {} in {:.0}% of {} sessions. {}",
                    next_tool,
                    previous_tool.as_deref().unwrap_or("the previous tool"),
                    stats.success_rate * 100.0,
                    stats.count,
                    classification.reasoning
                ),
                fallback_to_auto: false,
                effectiveness: None,
                candidates,
            });
        }

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

    /// Pick the tool that most reliably follows `previous_tool` from recorded
    /// tool-chain transitions, if one clears the minimum-observation floor.
    ///
    /// Ranks candidates by success rate, breaking ties by observation count.
    /// Returns `None` when no transition from `previous_tool` has been observed at
    /// least `MIN_OBSERVED` times — so sparse/cold-start data has no influence.
    fn chain_hint(
        metrics: &MetricsCollector,
        previous_tool: &str,
    ) -> Option<(String, TransitionStats)> {
        // Mirror `recommend_tool`'s sample floor so a single fluke transition
        // never steers routing.
        const MIN_OBSERVED: u32 = 3;

        metrics
            .transitions_from(previous_tool)
            .into_iter()
            .filter(|(_, stats)| stats.count >= MIN_OBSERVED)
            .max_by(|a, b| {
                a.1.success_rate
                    .partial_cmp(&b.1.success_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.count.cmp(&b.1.count))
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
            .route("Solve x^2 + 3x + 2 = 0", None, None, None, &metrics)
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
            .route(
                "Solve 2+2",
                Some("math".to_string()),
                Some(0.99),
                None,
                &metrics,
            )
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
        let result = mode.route("", None, None, None, &metrics).await;

        assert!(result.is_err());
    }

    /// Build a collector whose only data is a strong `decision -> linear`
    /// tool-chain (4 successful observations), with NO per-problem effectiveness
    /// data, so routing must fall through to the chain-history branch.
    fn metrics_with_chain_decision_to_linear(observations: usize) -> MetricsCollector {
        let metrics = MetricsCollector::new();
        for i in 0..observations {
            let session = format!("chain-s{i}");
            metrics.record_tool_use(&session, "decision", true);
            metrics.record_tool_use(&session, "linear", true); // decision -> linear, success
        }
        metrics
    }

    #[tokio::test]
    async fn test_route_uses_chain_history_in_fallback() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        // Chain data present, but no effectiveness data for this problem type.
        let metrics = metrics_with_chain_decision_to_linear(4);

        let mode = MetaMode::new(mock_storage, mock_client);
        let result = mode
            .route(
                "Something",
                Some("unknown_type".to_string()),
                None,
                Some("decision".to_string()),
                &metrics,
            )
            .await
            .unwrap();

        // The inert matrix now drives a real routing decision.
        assert_eq!(result.selected_tool, "linear");
        assert!(!result.fallback_to_auto);
        assert!(result.reasoning.contains("tool-chain history"));
    }

    #[tokio::test]
    async fn test_route_no_chain_influence_without_previous_tool() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        let metrics = metrics_with_chain_decision_to_linear(4);

        let mode = MetaMode::new(mock_storage, mock_client);
        // No previous_tool → chain data is ignored → unchanged auto fallback.
        let result = mode
            .route(
                "Something",
                Some("unknown_type".to_string()),
                None,
                None,
                &metrics,
            )
            .await
            .unwrap();

        assert_eq!(result.selected_tool, "auto");
        assert!(result.fallback_to_auto);
    }

    #[tokio::test]
    async fn test_route_chain_below_floor_falls_back_to_auto() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        // Only 2 observations — below the MIN_OBSERVED floor of 3.
        let metrics = metrics_with_chain_decision_to_linear(2);

        let mode = MetaMode::new(mock_storage, mock_client);
        let result = mode
            .route(
                "Something",
                Some("unknown_type".to_string()),
                None,
                Some("decision".to_string()),
                &metrics,
            )
            .await
            .unwrap();

        assert_eq!(result.selected_tool, "auto");
        assert!(result.fallback_to_auto);
    }

    #[tokio::test]
    async fn test_chain_hint_enriches_confident_pick_without_overriding() {
        let mock_storage = MockStorageTrait::new();
        let mock_client = MockAnthropicClientTrait::new();
        // Effectiveness data picks "linear" for math; chain data points elsewhere.
        let metrics = create_mock_metrics_with_data();
        for i in 0..4 {
            let session = format!("conf-s{i}");
            metrics.record_tool_use(&session, "decision", true);
            metrics.record_tool_use(&session, "tree", true); // decision -> tree
        }

        let mode = MetaMode::new(mock_storage, mock_client);
        let result = mode
            .route(
                "Solve x^2 + 3x + 2 = 0",
                Some("math".to_string()),
                None,
                Some("decision".to_string()),
                &metrics,
            )
            .await
            .unwrap();

        // Effectiveness pick wins; the chain hint only annotates the reasoning.
        assert_eq!(result.selected_tool, "linear");
        assert!(!result.fallback_to_auto);
        assert!(result.reasoning.contains("Tool-chain history"));
    }
}
