//! Helper functions for storage operations.
//!
//! Contains parsing and conversion utilities used by the storage module.

use chrono::{DateTime, Utc};

use crate::error::StorageError;
use crate::self_improvement::types::{ActionStatus, DiagnosisStatus, Severity};

/// Helper to create a QueryFailed error.
pub fn query_error(message: impl Into<String>) -> StorageError {
    StorageError::QueryFailed {
        query: "self_improvement".to_string(),
        message: message.into(),
    }
}

/// Parse an RFC3339 timestamp string into DateTime<Utc>.
pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| query_error(format!("Invalid datetime '{s}': {e}")))
}

/// Parse severity from string.
pub fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "info" => Severity::Info,
        "warning" => Severity::Warning,
        "high" => Severity::High,
        "critical" => Severity::Critical,
        _ => Severity::Info,
    }
}

/// Parse diagnosis status from string.
pub fn parse_diagnosis_status(s: &str) -> DiagnosisStatus {
    match s.to_lowercase().as_str() {
        "pending" => DiagnosisStatus::Pending,
        "approved" => DiagnosisStatus::Approved,
        "rejected" => DiagnosisStatus::Rejected,
        "executed" => DiagnosisStatus::Executed,
        "failed" => DiagnosisStatus::Failed,
        "rolled_back" | "rolledback" => DiagnosisStatus::RolledBack,
        _ => DiagnosisStatus::Pending,
    }
}

/// Parse action status from string.
pub fn parse_action_status(s: &str) -> ActionStatus {
    match s.to_lowercase().as_str() {
        "proposed" => ActionStatus::Proposed,
        "approved" => ActionStatus::Approved,
        "executed" | "completed" => ActionStatus::Completed,
        "failed" => ActionStatus::Failed,
        "rolled_back" | "rolledback" => ActionStatus::RolledBack,
        _ => ActionStatus::Proposed,
    }
}
