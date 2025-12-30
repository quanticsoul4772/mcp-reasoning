//! System prompts and prompt builders for LLM calls.

use super::security::sanitize_multiline;
use super::types::{DiagnosisContent, HealthContext, LearningContext};
use crate::self_improvement::types::{SuggestedAction, TriggerMetric};

// ============================================================================
// System Prompts
// ============================================================================

pub const DIAGNOSIS_SYSTEM_PROMPT: &str = r"You are a system reliability expert analyzing metrics for a reasoning server.
Analyze the provided data and respond with ONLY valid JSON, no other text.
Focus on identifying the most critical issue and its root cause.";

pub const ACTION_SYSTEM_PROMPT: &str = r"You are a system reliability expert selecting actions for detected issues.
Respond with ONLY valid JSON, no other text.
Prefer conservative, reversible adjustments.";

pub const VALIDATION_SYSTEM_PROMPT: &str = r"You are a system safety validator reviewing proposed actions.
Respond with ONLY valid JSON, no other text.
Validate for safety, reversibility, and appropriateness.";

pub const LEARNING_SYSTEM_PROMPT: &str = r"You are a system learning analyst synthesizing lessons from executed actions.
Respond with ONLY valid JSON, no other text.
Focus on patterns and actionable recommendations.";

// ============================================================================
// Prompt Builders
// ============================================================================

pub fn build_diagnosis_prompt(health: &HealthContext) -> String {
    let triggers_json = serde_json::to_string_pretty(&health.triggers).unwrap_or_default();

    format!(
        r#"Current System State:
- Error Rate: {:.2}% (baseline: {:.2}%)
- Latency P95: {}ms (baseline: {}ms)
- Quality Score: {:.2} (baseline: {:.2})

Triggered Metrics:
{triggers_json}

Respond with JSON:
{{
    "description": "Human-readable description of the issue",
    "suspected_cause": "Most likely root cause",
    "confidence": 0.0-1.0,
    "evidence": ["Supporting evidence point 1", "Point 2"]
}}"#,
        health.error_rate * 100.0,
        health.baseline_error_rate * 100.0,
        health.latency_p95_ms,
        health.baseline_latency_ms,
        health.quality_score,
        health.baseline_quality,
    )
}

pub fn build_action_prompt(diagnosis: &DiagnosisContent, trigger: &TriggerMetric) -> String {
    let metric_type = trigger.metric_type();
    let severity = trigger.severity();

    // Sanitize user-controlled content to prevent prompt injection
    let safe_description = sanitize_multiline(&diagnosis.description);
    let safe_cause = sanitize_multiline(&diagnosis.suspected_cause);

    format!(
        r#"Diagnosis:
- Description: {safe_description}
- Suspected Cause: {safe_cause}
- Confidence: {:.0}%

Triggered Metric: {} (Severity: {})

Available Actions:
1. adjust_param - Adjust a configuration parameter
2. scale_resource - Scale a resource
3. no_op - Take no action (if issue is transient)

Respond with JSON:
{{
    "action_type": "adjust_param" | "scale_resource" | "no_op",
    "key": "parameter key (for adjust_param)",
    "old_value": current value,
    "new_value": proposed value,
    "scope": "global" | "mode:name" | "tool:name",
    "resource": "resource type (for scale_resource)",
    "reason": "reason for no_op",
    "rationale": "Explanation for this action"
}}"#,
        diagnosis.confidence * 100.0,
        metric_type,
        severity,
    )
}

pub fn build_validation_prompt(action: &SuggestedAction, context: &str) -> String {
    let action_json = serde_json::to_string_pretty(action).unwrap_or_default();

    // Sanitize the context parameter to prevent prompt injection
    let safe_context = sanitize_multiline(context);

    format!(
        r#"Proposed Action:
{action_json}

Context:
{safe_context}

Respond with JSON:
{{
    "approved": true | false,
    "risk_level": "low" | "medium" | "high",
    "reasoning": "Explanation for decision",
    "modifications": ["Suggested modification 1"] (optional)
}}"#
    )
}

pub fn build_learning_prompt(learning: &LearningContext) -> String {
    // Sanitize action_details which is user-controlled
    let safe_action_details = sanitize_multiline(&learning.action_details);

    format!(
        r#"Action Executed: {}
Reward: {:.2} (positive = improvement, negative = regression)

Pre-Execution Metrics:
- Error Rate: {:.2}%
- Latency P95: {}ms
- Quality Score: {:.2}

Post-Execution Metrics:
- Error Rate: {:.2}%
- Latency P95: {}ms
- Quality Score: {:.2}

Action Details:
{safe_action_details}

Respond with JSON:
{{
    "lessons": ["Lesson 1", "Lesson 2"],
    "recommendations": ["Future recommendation 1"],
    "pattern": "Identified pattern (if any)" | null,
    "confidence": 0.0-1.0
}}"#,
        learning.action_type,
        learning.reward,
        learning.pre_metrics.error_rate * 100.0,
        learning.pre_metrics.latency_p95_ms,
        learning.pre_metrics.quality_score,
        learning.post_metrics.error_rate * 100.0,
        learning.post_metrics.latency_p95_ms,
        learning.post_metrics.quality_score,
    )
}
