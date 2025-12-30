//! Reward calculation types for the self-improvement system.
//!
//! This module contains types for calculating and representing rewards
//! from improvement actions.

use serde::{Deserialize, Serialize};

use super::enums::TriggerMetric;
use super::metrics::MetricsSnapshot;

// ============================================================================
// RewardBreakdown (DESIGN.md 14.2)
// ============================================================================

/// Breakdown of reward components.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct RewardBreakdown {
    /// Component from error rate improvement.
    pub error_rate_component: f64,
    /// Component from latency improvement.
    pub latency_component: f64,
    /// Component from quality improvement.
    pub quality_component: f64,
}

impl RewardBreakdown {
    /// Create a new reward breakdown.
    #[must_use]
    pub const fn new(error_rate: f64, latency: f64, quality: f64) -> Self {
        Self {
            error_rate_component: error_rate,
            latency_component: latency,
            quality_component: quality,
        }
    }

    /// Calculate weighted total.
    #[must_use]
    pub fn weighted_total(&self, weights: &RewardWeights) -> f64 {
        self.error_rate_component * weights.error_rate
            + self.latency_component * weights.latency
            + self.quality_component * weights.quality
    }
}

// ============================================================================
// RewardWeights (DESIGN.md 14.2)
// ============================================================================

/// Weights for reward calculation based on trigger type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardWeights {
    /// Weight for error rate component.
    pub error_rate: f64,
    /// Weight for latency component.
    pub latency: f64,
    /// Weight for quality component.
    pub quality: f64,
}

impl RewardWeights {
    /// Get weights optimized for the trigger type.
    #[must_use]
    pub fn for_trigger(trigger: &TriggerMetric) -> Self {
        match trigger {
            TriggerMetric::ErrorRate { .. } => Self {
                error_rate: 0.6,
                latency: 0.2,
                quality: 0.2,
            },
            TriggerMetric::Latency { .. } => Self {
                error_rate: 0.2,
                latency: 0.6,
                quality: 0.2,
            },
            TriggerMetric::QualityScore { .. } => Self {
                error_rate: 0.2,
                latency: 0.2,
                quality: 0.6,
            },
        }
    }
}

impl Default for RewardWeights {
    fn default() -> Self {
        Self {
            error_rate: 0.34,
            latency: 0.33,
            quality: 0.33,
        }
    }
}

// ============================================================================
// NormalizedReward (DESIGN.md 14.2)
// ============================================================================

/// Normalized reward for comparing improvements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedReward {
    /// Reward value (-1.0 to 1.0, positive = improvement).
    pub value: f64,
    /// Breakdown by component.
    pub breakdown: RewardBreakdown,
    /// Confidence based on sample size (0.0 to 1.0).
    pub confidence: f64,
}

impl NormalizedReward {
    /// Create a new normalized reward.
    #[must_use]
    pub fn new(value: f64, breakdown: RewardBreakdown, confidence: f64) -> Self {
        Self {
            value: value.clamp(-1.0, 1.0),
            breakdown,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Calculate reward from metrics snapshots.
    #[must_use]
    pub fn calculate(
        trigger: &TriggerMetric,
        pre_metrics: &MetricsSnapshot,
        post_metrics: &MetricsSnapshot,
        sample_count: u64,
    ) -> Self {
        let weights = RewardWeights::for_trigger(trigger);

        // Calculate component improvements
        let error_improvement = if pre_metrics.error_rate > 0.0 {
            (pre_metrics.error_rate - post_metrics.error_rate) / pre_metrics.error_rate
        } else if post_metrics.error_rate > 0.0 {
            -1.0
        } else {
            0.0
        };

        let latency_improvement = if pre_metrics.latency_p95_ms > 0 {
            (pre_metrics.latency_p95_ms - post_metrics.latency_p95_ms) as f64
                / pre_metrics.latency_p95_ms as f64
        } else if post_metrics.latency_p95_ms > 0 {
            -1.0
        } else {
            0.0
        };

        let quality_improvement = if pre_metrics.quality_score > 0.0 {
            (post_metrics.quality_score - pre_metrics.quality_score) / pre_metrics.quality_score
        } else if post_metrics.quality_score > 0.0 {
            1.0
        } else {
            0.0
        };

        let breakdown = RewardBreakdown::new(
            error_improvement.clamp(-1.0, 1.0),
            latency_improvement.clamp(-1.0, 1.0),
            quality_improvement.clamp(-1.0, 1.0),
        );

        let value = breakdown.weighted_total(&weights);

        // Confidence based on sample size (asymptotic to 1.0)
        let confidence = 1.0 - 1.0 / (1.0 + sample_count as f64 / 100.0);

        Self::new(value, breakdown, confidence)
    }

    /// Check if reward indicates improvement.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.value > 0.0
    }

    /// Check if reward indicates degradation.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.value < 0.0
    }

    /// Check if reward is significant (above noise threshold).
    #[must_use]
    pub fn is_significant(&self, threshold: f64) -> bool {
        self.value.abs() > threshold && self.confidence > 0.5
    }
}
