//! Types for Anthropic API calls in the self-improvement system.
//!
//! Contains data structures for diagnosis, validation, learning synthesis,
//! and various context types used in LLM interactions.

use serde::{Deserialize, Serialize};

use crate::self_improvement::types::TriggerMetric;

// ============================================================================
// Diagnosis Content
// ============================================================================

/// LLM-generated diagnosis content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisContent {
    /// Human-readable description of the issue.
    pub description: String,
    /// Suspected root cause.
    pub suspected_cause: String,
    /// Confidence in the diagnosis (0.0 to 1.0).
    pub confidence: f64,
    /// Supporting evidence.
    pub evidence: Vec<String>,
}

// ============================================================================
// Validation Result
// ============================================================================

/// LLM validation result for a suggested action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the action is approved.
    pub approved: bool,
    /// Risk level (low, medium, high).
    pub risk_level: String,
    /// Reasoning for the decision.
    pub reasoning: String,
    /// Suggested modifications (if any).
    pub modifications: Option<Vec<String>>,
}

// ============================================================================
// Learning Synthesis
// ============================================================================

/// LLM-generated learning synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSynthesis {
    /// Lessons learned from this action.
    pub lessons: Vec<String>,
    /// Recommendations for future actions.
    pub recommendations: Vec<String>,
    /// Pattern identified (if any).
    pub pattern: Option<String>,
    /// Confidence in the synthesis (0.0 to 1.0).
    pub confidence: f64,
}

// ============================================================================
// Health Report (for context)
// ============================================================================

/// Simplified health report for LLM context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthContext {
    /// Current error rate.
    pub error_rate: f64,
    /// Baseline error rate.
    pub baseline_error_rate: f64,
    /// Current latency P95.
    pub latency_p95_ms: i64,
    /// Baseline latency P95.
    pub baseline_latency_ms: i64,
    /// Current quality score.
    pub quality_score: f64,
    /// Baseline quality score.
    pub baseline_quality: f64,
    /// Triggered metrics.
    pub triggers: Vec<TriggerContext>,
}

/// Triggered metric context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerContext {
    /// Metric type.
    pub metric_type: String,
    /// Deviation percentage.
    pub deviation_pct: f64,
    /// Severity.
    pub severity: String,
}

impl From<&TriggerMetric> for TriggerContext {
    fn from(trigger: &TriggerMetric) -> Self {
        Self {
            metric_type: trigger.metric_type().to_string(),
            deviation_pct: trigger.deviation_pct(),
            severity: trigger.severity().to_string(),
        }
    }
}

// ============================================================================
// Learning Outcome Context
// ============================================================================

/// Learning outcome context for LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningContext {
    /// Action type that was executed.
    pub action_type: String,
    /// Reward value (-1.0 to 1.0).
    pub reward: f64,
    /// Pre-execution metrics.
    pub pre_metrics: MetricsContext,
    /// Post-execution metrics.
    pub post_metrics: MetricsContext,
    /// Action details.
    pub action_details: String,
}

/// Metrics context for LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsContext {
    /// Error rate.
    pub error_rate: f64,
    /// Latency P95.
    pub latency_p95_ms: i64,
    /// Quality score.
    pub quality_score: f64,
}
