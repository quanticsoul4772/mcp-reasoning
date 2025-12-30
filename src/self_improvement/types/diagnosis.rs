//! Diagnosis types for the self-improvement system.
//!
//! This module contains types related to system diagnostics and suggested actions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::enums::{
    duration_serde, ConfigScope, DiagnosisId, DiagnosisStatus, ParamValue, ResourceType, Severity,
    TriggerMetric,
};

// ============================================================================
// SuggestedAction (DESIGN.md 14.2)
// ============================================================================

/// Actions the system can take (ALL must be reversible).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SuggestedAction {
    /// Adjust a configuration parameter.
    AdjustParam {
        /// The configuration key to adjust.
        key: String,
        /// The previous value.
        old_value: ParamValue,
        /// The new value to set.
        new_value: ParamValue,
        /// The scope of the adjustment.
        scope: ConfigScope,
    },
    /// Scale a resource.
    ScaleResource {
        /// The resource to scale.
        resource: ResourceType,
        /// The previous resource value.
        old_value: u32,
        /// The new resource value.
        new_value: u32,
    },
    /// No action needed, but revisit later.
    NoOp {
        /// Reason for taking no action.
        reason: String,
        /// Duration to wait before revisiting.
        #[serde(with = "duration_serde")]
        revisit_after: Duration,
    },
}

impl SuggestedAction {
    /// Create a param adjustment action.
    #[must_use]
    pub fn adjust_param(
        key: impl Into<String>,
        old_value: ParamValue,
        new_value: ParamValue,
        scope: ConfigScope,
    ) -> Self {
        Self::AdjustParam {
            key: key.into(),
            old_value,
            new_value,
            scope,
        }
    }

    /// Create a resource scaling action.
    #[must_use]
    pub const fn scale_resource(resource: ResourceType, old_value: u32, new_value: u32) -> Self {
        Self::ScaleResource {
            resource,
            old_value,
            new_value,
        }
    }

    /// Create a no-op action.
    #[must_use]
    pub fn no_op(reason: impl Into<String>, revisit_after: Duration) -> Self {
        Self::NoOp {
            reason: reason.into(),
            revisit_after,
        }
    }

    /// Check if this is a no-op action.
    #[must_use]
    pub const fn is_no_op(&self) -> bool {
        matches!(self, Self::NoOp { .. })
    }

    /// Get action type name.
    #[must_use]
    pub fn action_type(&self) -> &'static str {
        match self {
            Self::AdjustParam { .. } => "adjust_param",
            Self::ScaleResource { .. } => "scale_resource",
            Self::NoOp { .. } => "no_op",
        }
    }
}

// ============================================================================
// SelfDiagnosis (DESIGN.md 14.2)
// ============================================================================

/// Complete diagnosis report from the Analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfDiagnosis {
    /// Unique diagnosis identifier.
    pub id: DiagnosisId,
    /// When the diagnosis was created.
    pub created_at: DateTime<Utc>,
    /// What triggered this diagnosis.
    pub trigger: TriggerMetric,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable description of the issue.
    pub description: String,
    /// Suspected root cause (LLM-generated).
    pub suspected_cause: Option<String>,
    /// Recommended action to take.
    pub suggested_action: SuggestedAction,
    /// Rationale for the suggested action (LLM-generated).
    pub action_rationale: Option<String>,
    /// Current status.
    pub status: DiagnosisStatus,
}

impl SelfDiagnosis {
    /// Create a new diagnosis.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        trigger: TriggerMetric,
        description: impl Into<String>,
        suggested_action: SuggestedAction,
    ) -> Self {
        let severity = trigger.severity();
        Self {
            id: id.into(),
            created_at: Utc::now(),
            trigger,
            severity,
            description: description.into(),
            suspected_cause: None,
            suggested_action,
            action_rationale: None,
            status: DiagnosisStatus::Pending,
        }
    }

    /// Add suspected cause.
    #[must_use]
    pub fn with_suspected_cause(mut self, cause: impl Into<String>) -> Self {
        self.suspected_cause = Some(cause.into());
        self
    }

    /// Add action rationale.
    #[must_use]
    pub fn with_action_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.action_rationale = Some(rationale.into());
        self
    }

    /// Approve this diagnosis for execution.
    pub fn approve(&mut self) {
        self.status = DiagnosisStatus::Approved;
    }

    /// Reject this diagnosis.
    pub fn reject(&mut self) {
        self.status = DiagnosisStatus::Rejected;
    }

    /// Mark as executed.
    pub fn mark_executed(&mut self) {
        self.status = DiagnosisStatus::Executed;
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self) {
        self.status = DiagnosisStatus::Failed;
    }

    /// Mark as rolled back.
    pub fn mark_rolled_back(&mut self) {
        self.status = DiagnosisStatus::RolledBack;
    }
}
