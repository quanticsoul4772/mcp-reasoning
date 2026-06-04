//! Record types for self-improvement database storage.
//!
//! This module contains the data structures for storing self-improvement
//! system data in the database.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::self_improvement::types::{ActionStatus, DiagnosisId, DiagnosisStatus, Severity};

// ============================================================================
// Invocation Records
// ============================================================================

/// Invocation event record for database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationRecord {
    /// Unique identifier.
    pub id: String,
    /// Tool name that was invoked.
    pub tool_name: String,
    /// Latency in milliseconds.
    pub latency_ms: i64,
    /// Whether the invocation succeeded.
    pub success: bool,
    /// Optional quality score (0.0 to 1.0).
    pub quality_score: Option<f64>,
    /// When the invocation occurred.
    pub created_at: DateTime<Utc>,
}

impl InvocationRecord {
    /// Create a new invocation record.
    #[must_use]
    pub fn new(
        tool_name: impl Into<String>,
        latency_ms: i64,
        success: bool,
        quality_score: Option<f64>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.into(),
            latency_ms,
            success,
            quality_score,
            created_at: Utc::now(),
        }
    }
}

// ============================================================================
// Diagnosis Records
// ============================================================================

/// Diagnosis record for database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisRecord {
    /// Unique identifier.
    pub id: DiagnosisId,
    /// Type of trigger (error_rate, latency, quality_score).
    pub trigger_type: String,
    /// JSON-serialized trigger data.
    pub trigger_json: String,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable description.
    pub description: String,
    /// Suspected root cause (LLM-generated).
    pub suspected_cause: Option<String>,
    /// JSON-serialized suggested action.
    pub suggested_action_json: String,
    /// Rationale for the action (LLM-generated).
    pub action_rationale: Option<String>,
    /// Current status.
    pub status: DiagnosisStatus,
    /// When the diagnosis was created.
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Action Records
// ============================================================================

/// Action execution record for database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Unique identifier.
    pub id: String,
    /// Associated diagnosis ID.
    pub diagnosis_id: DiagnosisId,
    /// Type of action (adjust_param, scale_resource, no_op).
    pub action_type: String,
    /// JSON-serialized action data.
    pub action_json: String,
    /// Outcome of execution.
    pub outcome: ActionStatus,
    /// JSON-serialized pre-execution metrics.
    pub pre_metrics_json: String,
    /// JSON-serialized post-execution metrics (if available).
    pub post_metrics_json: Option<String>,
    /// Execution time in milliseconds.
    pub execution_time_ms: i64,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// When the action was executed.
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Config Override Records
// ============================================================================

/// Config override record for database storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOverrideRecord {
    /// Configuration key.
    pub key: String,
    /// JSON-serialized value.
    pub value_json: String,
    /// Action ID that applied this override.
    pub applied_by_action: Option<String>,
    /// When the override was last updated.
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Stats Types
// ============================================================================

/// Invocation statistics summary.
#[derive(Debug, Clone, Default)]
pub struct InvocationStats {
    /// Total number of invocations.
    pub total_count: u64,
    /// Number of successful invocations.
    pub success_count: u64,
    /// Error rate (0.0 to 1.0).
    pub error_rate: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Average quality score (if available).
    pub avg_quality_score: Option<f64>,
}
