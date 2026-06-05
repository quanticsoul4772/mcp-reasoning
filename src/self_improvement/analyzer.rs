//! Self-improvement analysis.
//!
//! Phase 2 of the 4-phase loop: LLM-based diagnosis and action proposal.

use super::learner::LearningGuidance;
use super::monitor::MonitorResult;
use super::types::{ActionType, LegacyTriggerMetric, SelfImprovementAction, Severity};
use crate::error::ModeError;
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message};

/// Analysis result containing proposed actions.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Proposed improvement actions.
    pub actions: Vec<SelfImprovementAction>,
    /// Summary of the analysis.
    pub summary: String,
    /// Confidence in the analysis (0.0-1.0).
    pub confidence: f64,
}

/// Analyzer for diagnosing issues and proposing actions.
pub struct Analyzer<C: AnthropicClientTrait> {
    client: C,
    max_actions: usize,
}

impl<C: AnthropicClientTrait> Analyzer<C> {
    /// Create a new analyzer.
    pub fn new(client: C) -> Self {
        Self {
            client,
            max_actions: 3,
        }
    }

    /// Set maximum number of actions to propose.
    #[must_use]
    pub fn with_max_actions(mut self, max: usize) -> Self {
        self.max_actions = max;
        self
    }

    /// Analyze monitoring results and propose actions.
    ///
    /// `guidance` carries what past cycles' executed actions taught us; when
    /// non-empty it is rendered into the prompt so proposals favor action types
    /// that have worked and avoid those that have repeatedly failed. Pass
    /// `&LearningGuidance::default()` on the first cycle (no history).
    pub async fn analyze(
        &self,
        monitor_result: &MonitorResult,
        guidance: &LearningGuidance,
    ) -> Result<AnalysisResult, ModeError> {
        if monitor_result.triggers.is_empty() {
            return Ok(AnalysisResult {
                actions: Vec::new(),
                summary: "No issues detected, no actions needed.".to_string(),
                confidence: 1.0,
            });
        }

        let prompt = self.build_analysis_prompt(monitor_result, guidance);
        let messages = vec![Message::user(&prompt)];
        let config = CompletionConfig::new()
            .with_max_tokens(2048)
            .with_temperature(0.3);

        let response = self.client.complete(messages, config).await?;

        self.parse_analysis_response(&response.content, monitor_result)
    }

    fn build_analysis_prompt(
        &self,
        monitor_result: &MonitorResult,
        guidance: &LearningGuidance,
    ) -> String {
        let triggers_desc = monitor_result
            .triggers
            .iter()
            .map(|t: &LegacyTriggerMetric| {
                format!(
                    "- {} ({:?}): {} (value: {:.3}, threshold: {:.3})",
                    t.name, t.severity, t.description, t.value, t.threshold
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let guidance_section = Self::render_guidance(guidance);

        format!(
            r#"Analyze the following system metrics issues and propose improvement actions.

## Current System State
- Success Rate: {:.1}%
- Average Latency: {:.0}ms
- Total Invocations: {}

## Detected Issues
{triggers_desc}
{guidance_section}

## Available Action Types
1. ConfigAdjust - Adjust configuration parameters (timeouts, retries)
2. PromptTune - Modify prompt templates for better results
3. ThresholdAdjust - Adjust decision thresholds (routing/quality)
4. LogObservation - Log observation for future analysis

## Instructions
Propose up to {max_actions} improvement actions. For each action:
1. Choose the most appropriate action type
2. Provide a clear description
3. Explain the rationale
4. Estimate expected improvement (0.0-1.0)
5. Provide concrete `parameters` (see below)

`parameters` is REQUIRED for `config_adjust`, `threshold_adjust`, and
`prompt_tune`. ONLY these exact keys are accepted — any other key causes the
action to be REJECTED, so use only these:
- `config_adjust`: one or more of `request_timeout_ms`,
  `request_timeout_deep_ms`, `request_timeout_maximum_ms`, `factory_timeout_ms`,
  `max_retries` mapped to a new value, e.g. `{{"request_timeout_ms": 45000}}`.
- `threshold_adjust`: one or more of `high_confidence_threshold` (confidence
  above which `reasoning_confidence_route` stops escalating to tree reasoning),
  `reflection_quality_threshold` (quality at which `reasoning_reflection` stops
  refining), `mcts_quality_threshold` (the `reasoning_mcts` auto-backtrack
  floor), `graph_prune_threshold` (the `reasoning_graph` prune quality floor) —
  each 0.0–1.0, e.g. `{{"mcts_quality_threshold": 0.6}}`.
- `prompt_tune`: `prompt_key`, `template`, and optionally `mode`.
`log_observation` needs no parameters. Also keep `expected_improvement` at or
below 0.5. An action without a non-empty `parameters` object (for the three
types above) is discarded.

Respond in JSON format:
```json
{{
  "summary": "Brief analysis summary",
  "confidence": 0.8,
  "actions": [
    {{
      "action_type": "config_adjust",
      "description": "What to change",
      "rationale": "Why this helps",
      "expected_improvement": 0.15,
      "parameters": {{"request_timeout_ms": 45000}}
    }}
  ]
}}
```"#,
            monitor_result.metrics.success_rate * 100.0,
            monitor_result.metrics.avg_latency_ms,
            monitor_result.metrics.total_invocations,
            max_actions = self.max_actions
        )
    }

    /// Render the learning-feedback section, or an empty string on the first
    /// cycle (no history) so the prompt — and thus behavior — is unchanged until
    /// there is something to learn from.
    fn render_guidance(guidance: &LearningGuidance) -> String {
        use std::fmt::Write as _;

        if guidance.is_empty() {
            return String::new();
        }

        let mut section = String::from("\n## What past self-improvement actions taught us\n");

        if !guidance.effectiveness.is_empty() {
            section.push_str(
                "Effectiveness by action type (prefer high reward / success; avoid the rest):\n",
            );
            for eff in &guidance.effectiveness {
                let _ = writeln!(
                    section,
                    "- {}: {} tried, {:.0}% succeeded, avg reward {:+.2}",
                    eff.action_type,
                    eff.attempts,
                    eff.success_rate * 100.0,
                    eff.avg_reward
                );
            }
        }

        if !guidance.recent_insights.is_empty() {
            section.push_str("Recent outcomes:\n");
            for insight in &guidance.recent_insights {
                let _ = writeln!(section, "- {insight}");
            }
        }

        section.push_str(
            "Prefer action types and parameters that have worked; avoid those that repeatedly fail.\n",
        );
        section
    }

    fn parse_analysis_response(
        &self,
        response: &str,
        monitor_result: &MonitorResult,
    ) -> Result<AnalysisResult, ModeError> {
        // Try to extract JSON from response
        let json_str = extract_json_block(response)?;
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| ModeError::JsonParseFailed {
                message: format!("Invalid JSON: {e}"),
            })?;

        let summary = parsed["summary"]
            .as_str()
            .unwrap_or("Analysis complete")
            .to_string();

        let confidence = parsed["confidence"].as_f64().unwrap_or(0.7).clamp(0.0, 1.0);

        let mut actions = Vec::new();
        if let Some(action_array) = parsed["actions"].as_array() {
            for (i, action_json) in action_array.iter().enumerate() {
                if i >= self.max_actions {
                    break;
                }
                if let Some(action) = self.parse_action(action_json, i) {
                    actions.push(action);
                }
            }
        }

        // If no actions parsed but we have triggers, create fallback action
        if actions.is_empty() && !monitor_result.triggers.is_empty() {
            actions.push(self.create_fallback_action(monitor_result));
        }

        Ok(AnalysisResult {
            actions,
            summary,
            confidence,
        })
    }

    fn parse_action(
        &self,
        json: &serde_json::Value,
        index: usize,
    ) -> Option<SelfImprovementAction> {
        let action_type_str = json["action_type"].as_str()?;
        let action_type = match action_type_str {
            "config_adjust" | "ConfigAdjust" => ActionType::ConfigAdjust,
            "prompt_tune" | "PromptTune" => ActionType::PromptTune,
            "threshold_adjust" | "ThresholdAdjust" => ActionType::ThresholdAdjust,
            "log_observation" | "LogObservation" => ActionType::LogObservation,
            _ => return None,
        };

        // These types do nothing without parameters and the executor rejects
        // them ("No parameters provided"); only log_observation runs bare.
        let requires_params = matches!(
            action_type,
            ActionType::ConfigAdjust | ActionType::ThresholdAdjust | ActionType::PromptTune
        );

        let description = json["description"]
            .as_str()
            .unwrap_or("Improvement action")
            .to_string();

        let rationale = json["rationale"]
            .as_str()
            .unwrap_or("Based on detected issues")
            .to_string();

        let expected_improvement = json["expected_improvement"]
            .as_f64()
            .unwrap_or(0.1)
            .clamp(0.0, 1.0);

        let id = format!(
            "action-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis()),
            index
        );

        let mut action = SelfImprovementAction::new(
            id,
            action_type,
            description,
            rationale,
            expected_improvement,
        );

        if let Some(params) = json.get("parameters") {
            if !params.is_null() {
                action = action.with_parameters(params.clone());
            }
        }

        // Drop a parameter-requiring action that arrived without a non-empty
        // parameters object — proposing it would only fail in the executor and
        // trip the circuit breaker. (An empty object does nothing, so it is
        // treated as missing here.)
        let has_params = action
            .parameters
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .is_some_and(|obj| !obj.is_empty());
        if requires_params && !has_params {
            return None;
        }

        Some(action)
    }

    fn create_fallback_action(&self, monitor_result: &MonitorResult) -> SelfImprovementAction {
        let highest_severity = monitor_result
            .triggers
            .iter()
            .map(|t| t.severity)
            .max()
            .unwrap_or(Severity::Info);

        // This runs when the analysis response could not be parsed, so concrete
        // parameters are unavailable. Use LogObservation — the only type that
        // executes without parameters — and record the severity in the message,
        // rather than a ConfigAdjust/ThresholdAdjust the executor would reject.
        let description = match highest_severity {
            Severity::Critical | Severity::High => {
                "Log critical/high-severity issue (analysis could not be parsed)"
            }
            Severity::Warning => "Log warning-level issue (analysis could not be parsed)",
            Severity::Info => "Log observation for monitoring",
        };
        let action_type = ActionType::LogObservation;

        let id = format!(
            "fallback-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis())
        );

        SelfImprovementAction::new(
            id,
            action_type,
            description,
            "Fallback action due to parse failure",
            0.1,
        )
    }
}

fn extract_json_block(text: &str) -> Result<String, ModeError> {
    // Try to find JSON in code blocks first
    if let Some(start) = text.find("```json") {
        if let Some(end) = text[start..]
            .find("```\n")
            .or_else(|| text[start..].rfind("```"))
        {
            let json_start = start + 7;
            let json_end = start + end;
            if json_end > json_start {
                return Ok(text[json_start..json_end].trim().to_string());
            }
        }
    }

    // Try to find raw JSON object
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                return Ok(text[start..=end].to_string());
            }
        }
    }

    Err(ModeError::JsonParseFailed {
        message: "No JSON found in response".to_string(),
    })
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
    use crate::self_improvement::types::{LegacyTriggerMetric, SystemMetrics};
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};
    use std::collections::HashMap;

    fn create_test_monitor_result(with_triggers: bool) -> MonitorResult {
        let triggers = if with_triggers {
            vec![LegacyTriggerMetric::new(
                "test_metric",
                0.5,
                0.8,
                Severity::High,
                "Test issue",
            )]
        } else {
            Vec::new()
        };

        MonitorResult {
            metrics: SystemMetrics::new(0.8, 100.0, 100, HashMap::new()),
            triggers,
            action_recommended: with_triggers,
        }
    }

    fn mock_response(content: &str) -> CompletionResponse {
        CompletionResponse::new(content, Usage::new(100, 50))
    }

    #[tokio::test]
    async fn test_analyzer_no_triggers() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().never();

        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(false);

        let result = analyzer
            .analyze(&monitor_result, &LearningGuidance::default())
            .await
            .unwrap();
        assert!(result.actions.is_empty());
        assert!((result.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_analyzer_with_triggers() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(
                r#"```json
{
    "summary": "Found issues",
    "confidence": 0.85,
    "actions": [
        {
            "action_type": "config_adjust",
            "description": "Increase timeout",
            "rationale": "Reduce timeouts",
            "expected_improvement": 0.15,
            "parameters": {"request_timeout_ms": 45000}
        }
    ]
}
```"#,
            ))
        });

        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer
            .analyze(&monitor_result, &LearningGuidance::default())
            .await
            .unwrap();
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.actions[0].action_type, ActionType::ConfigAdjust);
        assert!((result.confidence - 0.85).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_analyzer_max_actions() {
        let mut client = MockAnthropicClientTrait::new();
        client
            .expect_complete()
            .returning(|_, _| {
                Ok(mock_response(r#"{
    "summary": "Multiple actions",
    "confidence": 0.8,
    "actions": [
        {"action_type": "config_adjust", "description": "Action 1", "rationale": "R1", "expected_improvement": 0.1, "parameters": {"request_timeout_ms": 45000}},
        {"action_type": "prompt_tune", "description": "Action 2", "rationale": "R2", "expected_improvement": 0.1, "parameters": {"prompt_key": "linear", "template": "x"}},
        {"action_type": "threshold_adjust", "description": "Action 3", "rationale": "R3", "expected_improvement": 0.1, "parameters": {"quality_threshold": 0.7}},
        {"action_type": "log_observation", "description": "Action 4", "rationale": "R4", "expected_improvement": 0.1}
    ]
}"#))
            });

        let analyzer = Analyzer::new(client).with_max_actions(2);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer
            .analyze(&monitor_result, &LearningGuidance::default())
            .await
            .unwrap();
        assert_eq!(result.actions.len(), 2);
    }

    #[tokio::test]
    async fn test_analyzer_parse_error() {
        let mut client = MockAnthropicClientTrait::new();
        client
            .expect_complete()
            .returning(|_, _| Ok(mock_response("Invalid response with no JSON")));

        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer
            .analyze(&monitor_result, &LearningGuidance::default())
            .await;
        // Should fail to parse
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_analyzer_with_parameters() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(
                r#"{
    "summary": "Config adjustment",
    "confidence": 0.9,
    "actions": [
        {
            "action_type": "config_adjust",
            "description": "Increase timeout",
            "rationale": "Too many timeouts",
            "expected_improvement": 0.2,
            "parameters": {"timeout_ms": 30000, "retries": 5}
        }
    ]
}"#,
            ))
        });

        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer
            .analyze(&monitor_result, &LearningGuidance::default())
            .await
            .unwrap();
        assert!(result.actions[0].parameters.is_some());
    }

    #[test]
    fn test_extract_json_block_with_markers() {
        let text = r#"Here is the analysis:
```json
{"key": "value"}
```
Done."#;
        let result = extract_json_block(text).unwrap();
        assert!(result.contains("key"));
    }

    #[test]
    fn test_extract_json_block_raw() {
        let text = r#"The response is {"key": "value"} here."#;
        let result = extract_json_block(text).unwrap();
        assert_eq!(result, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_block_no_json() {
        let text = "No JSON here";
        let result = extract_json_block(text);
        assert!(result.is_err());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
    }

    #[test]
    fn test_action_type_parsing() {
        let json: serde_json::Value = serde_json::json!({
            "action_type": "prompt_tune",
            "description": "Test",
            "rationale": "Test",
            "expected_improvement": 0.1,
            "parameters": {"prompt_key": "linear", "template": "x"}
        });

        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let action = analyzer.parse_action(&json, 0);
        assert!(action.is_some());
        assert_eq!(action.unwrap().action_type, ActionType::PromptTune);
    }

    #[test]
    fn test_invalid_action_type() {
        let json: serde_json::Value = serde_json::json!({
            "action_type": "invalid_type",
            "description": "Test"
        });

        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let action = analyzer.parse_action(&json, 0);
        assert!(action.is_none());
    }

    #[tokio::test]
    async fn test_analyzer_client_error() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Err(ModeError::ApiUnavailable {
                message: "API failed".to_string(),
            })
        });

        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer
            .analyze(&monitor_result, &LearningGuidance::default())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_analyzer_empty_actions_creates_fallback() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(
                r#"{"summary": "Empty", "confidence": 0.7, "actions": []}"#,
            ))
        });

        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer
            .analyze(&monitor_result, &LearningGuidance::default())
            .await
            .unwrap();
        assert_eq!(result.actions.len(), 1);
        assert!(result.actions[0].id.starts_with("fallback-"));
    }

    #[test]
    fn test_create_fallback_action_critical() {
        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let monitor_result = MonitorResult {
            metrics: SystemMetrics::new(0.5, 200.0, 50, HashMap::new()),
            triggers: vec![LegacyTriggerMetric::new(
                "critical_metric",
                0.2,
                0.9,
                Severity::Critical,
                "Critical issue",
            )],
            action_recommended: true,
        };

        let action = analyzer.create_fallback_action(&monitor_result);
        // The fallback can't supply concrete parameters, so it must be an
        // executable LogObservation rather than a doomed ConfigAdjust.
        assert_eq!(action.action_type, ActionType::LogObservation);
    }

    #[test]
    fn test_create_fallback_action_warning() {
        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let monitor_result = MonitorResult {
            metrics: SystemMetrics::new(0.7, 150.0, 75, HashMap::new()),
            triggers: vec![LegacyTriggerMetric::new(
                "warning_metric",
                0.6,
                0.7,
                Severity::Warning,
                "Warning issue",
            )],
            action_recommended: true,
        };

        let action = analyzer.create_fallback_action(&monitor_result);
        assert_eq!(action.action_type, ActionType::LogObservation);
    }

    #[test]
    fn test_create_fallback_action_info() {
        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let monitor_result = MonitorResult {
            metrics: SystemMetrics::new(0.9, 50.0, 100, HashMap::new()),
            triggers: vec![LegacyTriggerMetric::new(
                "info_metric",
                0.85,
                0.9,
                Severity::Info,
                "Info observation",
            )],
            action_recommended: false,
        };

        let action = analyzer.create_fallback_action(&monitor_result);
        assert_eq!(action.action_type, ActionType::LogObservation);
    }

    #[test]
    fn test_parse_action_with_defaults() {
        // log_observation needs no parameters, so a bare action still parses
        // and the field defaults apply.
        let json: serde_json::Value = serde_json::json!({
            "action_type": "log_observation"
        });

        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let action = analyzer.parse_action(&json, 0).unwrap();
        assert_eq!(action.action_type, ActionType::LogObservation);
        assert_eq!(action.description, "Improvement action");
        assert_eq!(action.rationale, "Based on detected issues");
        assert!((action.expected_improvement - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_action_null_parameters() {
        let json: serde_json::Value = serde_json::json!({
            "action_type": "log_observation",
            "description": "Log it",
            "rationale": "For tracking",
            "expected_improvement": 0.05,
            "parameters": null
        });

        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let action = analyzer.parse_action(&json, 0).unwrap();
        assert_eq!(action.action_type, ActionType::LogObservation);
        assert!(action.parameters.is_none());
    }

    #[test]
    fn test_parse_action_drops_param_requiring_action_without_params() {
        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        // config_adjust with no parameters → dropped (would fail in the executor).
        let bare = serde_json::json!({ "action_type": "config_adjust" });
        assert!(analyzer.parse_action(&bare, 0).is_none());

        // An empty parameters object does nothing, so it counts as missing.
        let empty = serde_json::json!({ "action_type": "threshold_adjust", "parameters": {} });
        assert!(analyzer.parse_action(&empty, 0).is_none());

        // With real parameters it is kept.
        let ok = serde_json::json!({
            "action_type": "config_adjust",
            "parameters": {"request_timeout_ms": 45000}
        });
        let action = analyzer.parse_action(&ok, 0).expect("kept");
        assert_eq!(action.action_type, ActionType::ConfigAdjust);
        assert!(action.parameters.is_some());

        // log_observation needs no parameters and is always kept.
        let log = serde_json::json!({ "action_type": "log_observation" });
        assert!(analyzer.parse_action(&log, 0).is_some());
    }

    #[test]
    fn test_parse_action_missing_action_type() {
        let json: serde_json::Value = serde_json::json!({
            "description": "No type",
            "rationale": "Missing"
        });

        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let action = analyzer.parse_action(&json, 0);
        assert!(action.is_none());
    }

    #[test]
    fn test_parse_action_pascal_case_types() {
        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        // Test all pascal case variants
        for (type_str, expected) in [
            ("ConfigAdjust", ActionType::ConfigAdjust),
            ("PromptTune", ActionType::PromptTune),
            ("ThresholdAdjust", ActionType::ThresholdAdjust),
            ("LogObservation", ActionType::LogObservation),
        ] {
            // Include parameters so the param-requiring types are not dropped;
            // this test only exercises the type-string parsing.
            let json = serde_json::json!({
                "action_type": type_str,
                "parameters": {"request_timeout_ms": 45000}
            });
            let action = analyzer.parse_action(&json, 0).unwrap();
            assert_eq!(action.action_type, expected);
        }
    }

    #[test]
    fn test_extract_json_block_triple_backticks_no_newline() {
        let text = "```json{\"key\": \"value\"}```";
        let result = extract_json_block(text).unwrap();
        assert!(result.contains("key"));
    }

    #[tokio::test]
    async fn test_analyzer_invalid_json_response() {
        let mut client = MockAnthropicClientTrait::new();
        client
            .expect_complete()
            .returning(|_, _| Ok(mock_response(r#"{"invalid json with missing bracket"#)));

        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer
            .analyze(&monitor_result, &LearningGuidance::default())
            .await;
        assert!(matches!(result, Err(ModeError::JsonParseFailed { .. })));
    }

    #[tokio::test]
    async fn test_analyzer_response_missing_fields_uses_defaults() {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(|_, _| {
            Ok(mock_response(
                r#"{"actions": [{"action_type": "config_adjust"}]}"#,
            ))
        });

        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer
            .analyze(&monitor_result, &LearningGuidance::default())
            .await
            .unwrap();
        assert_eq!(result.summary, "Analysis complete");
        assert!((result.confidence - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_action_clamps_expected_improvement() {
        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let json = serde_json::json!({
            "action_type": "log_observation",
            "expected_improvement": 2.5
        });

        let action = analyzer.parse_action(&json, 0).unwrap();
        assert!((action.expected_improvement - 1.0).abs() < f64::EPSILON);

        let json_neg = serde_json::json!({
            "action_type": "log_observation",
            "expected_improvement": -0.5
        });

        let action_neg = analyzer.parse_action(&json_neg, 0).unwrap();
        assert!((action_neg.expected_improvement - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_build_analysis_prompt_format() {
        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);

        let monitor_result = MonitorResult {
            metrics: SystemMetrics::new(0.75, 150.5, 200, HashMap::new()),
            triggers: vec![
                LegacyTriggerMetric::new("metric1", 0.5, 0.8, Severity::High, "Issue 1"),
                LegacyTriggerMetric::new("metric2", 0.3, 0.6, Severity::Warning, "Issue 2"),
            ],
            action_recommended: true,
        };

        let prompt = analyzer.build_analysis_prompt(&monitor_result, &LearningGuidance::default());

        assert!(prompt.contains("Success Rate: 75.0%"));
        assert!(prompt.contains("Average Latency: 150ms")); // {:.0} rounds 150.5 to 150
        assert!(prompt.contains("Total Invocations: 200"));
        assert!(prompt.contains("metric1"));
        assert!(prompt.contains("metric2"));
        assert!(prompt.contains("High"));
        assert!(prompt.contains("Warning"));
        // Empty guidance → no learning-feedback section (first-cycle behavior).
        assert!(!prompt.contains("What past self-improvement actions taught us"));
    }

    #[test]
    fn test_build_analysis_prompt_omits_guidance_when_empty() {
        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let prompt = analyzer.build_analysis_prompt(&monitor_result, &LearningGuidance::default());
        assert!(!prompt.contains("What past self-improvement actions taught us"));
        assert!(!prompt.contains("Effectiveness by action type"));
    }

    #[test]
    fn test_build_analysis_prompt_renders_guidance_when_present() {
        use crate::self_improvement::learner::{ActionEffectiveness, LearningGuidance};

        let client = MockAnthropicClientTrait::new();
        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let guidance = LearningGuidance {
            effectiveness: vec![
                ActionEffectiveness {
                    action_type: "config_adjust".to_string(),
                    attempts: 5,
                    success_rate: 0.2,
                    avg_reward: -0.4,
                },
                ActionEffectiveness {
                    action_type: "threshold_adjust".to_string(),
                    attempts: 3,
                    success_rate: 1.0,
                    avg_reward: 0.55,
                },
            ],
            recent_insights: vec!["config_adjust action failed: bad parameter".to_string()],
        };

        let prompt = analyzer.build_analysis_prompt(&monitor_result, &guidance);
        assert!(prompt.contains("What past self-improvement actions taught us"));
        // Effectiveness lines (percent + signed avg reward).
        assert!(prompt.contains("config_adjust: 5 tried, 20% succeeded, avg reward -0.40"));
        assert!(prompt.contains("threshold_adjust: 3 tried, 100% succeeded, avg reward +0.55"));
        // Recent outcome surfaced.
        assert!(prompt.contains("config_adjust action failed: bad parameter"));
    }
}
