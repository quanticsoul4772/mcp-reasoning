//! Metrics snapshot types for the self-improvement system.
//!
//! This module contains types for capturing and tracking system metrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// MetricsSnapshot (DESIGN.md 14.3)
// ============================================================================

/// Snapshot of system metrics at a point in time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Error rate (0.0 to 1.0).
    pub error_rate: f64,
    /// P95 latency in milliseconds.
    pub latency_p95_ms: i64,
    /// Average quality score (0.0 to 1.0).
    pub quality_score: f64,
    /// Total invocations in this period.
    pub invocation_count: u64,
    /// Timestamp of snapshot.
    pub timestamp: DateTime<Utc>,
    /// Per-tool metrics.
    pub tool_metrics: HashMap<String, ToolMetrics>,
}

impl MetricsSnapshot {
    /// Create a new metrics snapshot.
    #[must_use]
    pub fn new(
        error_rate: f64,
        latency_p95_ms: i64,
        quality_score: f64,
        invocation_count: u64,
    ) -> Self {
        Self {
            error_rate: error_rate.clamp(0.0, 1.0),
            latency_p95_ms: latency_p95_ms.max(0),
            quality_score: quality_score.clamp(0.0, 1.0),
            invocation_count,
            timestamp: Utc::now(),
            tool_metrics: HashMap::new(),
        }
    }
}

// ============================================================================
// ToolMetrics
// ============================================================================

/// Per-tool metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetrics {
    /// Error rate for this tool.
    pub error_rate: f64,
    /// Average latency for this tool.
    pub avg_latency_ms: i64,
    /// Invocation count for this tool.
    pub invocation_count: u64,
}

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
