//! Self-improvement analysis.
//!
//! Phase 2 of the 4-phase loop: LLM-based diagnosis and action proposal.

use super::monitor::MonitorResult;
use super::types::{ActionType, SelfImprovementAction, Severity};
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
    pub async fn analyze(
        &self,
        monitor_result: &MonitorResult,
    ) -> Result<AnalysisResult, ModeError> {
        if monitor_result.triggers.is_empty() {
            return Ok(AnalysisResult {
                actions: Vec::new(),
                summary: "No issues detected, no actions needed.".to_string(),
                confidence: 1.0,
            });
        }

        let prompt = self.build_analysis_prompt(monitor_result);
        let messages = vec![Message::user(&prompt)];
        let config = CompletionConfig::new()
            .with_max_tokens(2048)
            .with_temperature(0.3);

        let response = self.client.complete(messages, config).await?;

        self.parse_analysis_response(&response.content, monitor_result)
    }

    fn build_analysis_prompt(&self, monitor_result: &MonitorResult) -> String {
        let triggers_desc = monitor_result
            .triggers
            .iter()
            .map(|t| {
                format!(
                    "- {} ({:?}): {} (value: {:.3}, threshold: {:.3})",
                    t.name, t.severity, t.description, t.value, t.threshold
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"Analyze the following system metrics issues and propose improvement actions.

## Current System State
- Success Rate: {:.1}%
- Average Latency: {:.0}ms
- Total Invocations: {}

## Detected Issues
{triggers_desc}

## Available Action Types
1. ConfigAdjust - Adjust configuration parameters (timeouts, retries, limits)
2. PromptTune - Modify prompt templates for better results
3. ThresholdAdjust - Adjust mode routing or quality thresholds
4. LogObservation - Log observation for future analysis

## Instructions
Propose up to {max_actions} improvement actions. For each action:
1. Choose the most appropriate action type
2. Provide a clear description
3. Explain the rationale
4. Estimate expected improvement (0.0-1.0)

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
      "parameters": {{"key": "value"}}
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
                .map(|d| d.as_millis())
                .unwrap_or(0),
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

        Some(action)
    }

    fn create_fallback_action(&self, monitor_result: &MonitorResult) -> SelfImprovementAction {
        let highest_severity = monitor_result
            .triggers
            .iter()
            .map(|t| &t.severity)
            .max_by(|a, b| severity_ord(a).cmp(&severity_ord(b)))
            .unwrap_or(&Severity::Low);

        let (action_type, description) = match highest_severity {
            Severity::Critical | Severity::High => (
                ActionType::ConfigAdjust,
                "Adjust configuration to address critical issues".to_string(),
            ),
            Severity::Medium => (
                ActionType::ThresholdAdjust,
                "Adjust thresholds to improve system performance".to_string(),
            ),
            Severity::Low => (
                ActionType::LogObservation,
                "Log observation for monitoring".to_string(),
            ),
        };

        let id = format!(
            "fallback-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
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

fn severity_ord(s: &Severity) -> u8 {
    match s {
        Severity::Low => 0,
        Severity::Medium => 1,
        Severity::High => 2,
        Severity::Critical => 3,
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
mod tests {
    use super::*;
    use crate::self_improvement::types::{SystemMetrics, TriggerMetric};
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};
    use std::collections::HashMap;

    fn create_test_monitor_result(with_triggers: bool) -> MonitorResult {
        let triggers = if with_triggers {
            vec![TriggerMetric::new(
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

        let result = analyzer.analyze(&monitor_result).await.unwrap();
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
            "expected_improvement": 0.15
        }
    ]
}
```"#,
            ))
        });

        let analyzer = Analyzer::new(client);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer.analyze(&monitor_result).await.unwrap();
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
        {"action_type": "config_adjust", "description": "Action 1", "rationale": "R1", "expected_improvement": 0.1},
        {"action_type": "prompt_tune", "description": "Action 2", "rationale": "R2", "expected_improvement": 0.1},
        {"action_type": "threshold_adjust", "description": "Action 3", "rationale": "R3", "expected_improvement": 0.1},
        {"action_type": "log_observation", "description": "Action 4", "rationale": "R4", "expected_improvement": 0.1}
    ]
}"#))
            });

        let analyzer = Analyzer::new(client).with_max_actions(2);
        let monitor_result = create_test_monitor_result(true);

        let result = analyzer.analyze(&monitor_result).await.unwrap();
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

        let result = analyzer.analyze(&monitor_result).await;
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

        let result = analyzer.analyze(&monitor_result).await.unwrap();
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
        assert!(severity_ord(&Severity::Critical) > severity_ord(&Severity::High));
        assert!(severity_ord(&Severity::High) > severity_ord(&Severity::Medium));
        assert!(severity_ord(&Severity::Medium) > severity_ord(&Severity::Low));
    }

    #[test]
    fn test_action_type_parsing() {
        let json: serde_json::Value = serde_json::json!({
            "action_type": "prompt_tune",
            "description": "Test",
            "rationale": "Test",
            "expected_improvement": 0.1
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
}
