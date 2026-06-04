//! Metrics snapshot types for the self-improvement system.
//!
//! This module contains types for capturing and tracking system metrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Baselines (DESIGN.md 14.3)
// ============================================================================

/// Baseline values for comparison.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Baselines {
    /// Baseline error rate.
    pub error_rate: f64,
    /// Baseline P95 latency.
    pub latency_p95_ms: i64,
    /// Baseline quality score.
    pub quality_score: f64,
    /// Sample count used to calculate baselines.
    pub sample_count: u64,
    /// When baselines were last updated.
    pub updated_at: DateTime<Utc>,
}

impl Baselines {
    /// Create new baselines.
    #[must_use]
    pub fn new(
        error_rate: f64,
        latency_p95_ms: i64,
        quality_score: f64,
        sample_count: u64,
    ) -> Self {
        Self {
            error_rate,
            latency_p95_ms,
            quality_score,
            sample_count,
            updated_at: Utc::now(),
        }
    }
}
