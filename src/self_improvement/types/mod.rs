//! Self-improvement system types.
//!
//! Core types for the 4-phase optimization loop (DESIGN.md Section 14):
//! - Monitor: Metric collection and baseline tracking
//! - Analyze: LLM-powered diagnosis
//! - Execute: Action execution with safety
//! - Learn: Reward calculation and lesson synthesis
//!
//! # Module Organization
//!
//! - `enums`: Core enumeration types (Severity, TriggerMetric, ParamValue, etc.)
//! - `diagnosis`: Diagnosis and action types (SuggestedAction, SelfDiagnosis)
//! - `rewards`: Reward calculation types (RewardBreakdown, NormalizedReward)
//! - `metrics`: Metrics snapshot types (MetricsSnapshot, ToolMetrics, Baselines)
//! - `legacy`: Legacy types for backward compatibility

mod diagnosis;
mod enums;
mod legacy;
mod metrics;
mod rewards;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests;

// Re-export all public types from submodules
pub use diagnosis::{SelfDiagnosis, SuggestedAction};
pub use enums::{
    ConfigScope, DiagnosisId, DiagnosisStatus, ParamValue, ResourceType, Severity, TriggerMetric,
};
// Export legacy types as the primary types (for backward compatibility)
// The new ActionType/ActionStatus from enums are for future DESIGN.md 14.2 use
pub use legacy::{
    ActionStatus, ActionType, LegacyTriggerMetric, Lesson, SelfImprovementAction, SystemMetrics,
};
pub use metrics::{Baselines, MetricsSnapshot, ToolMetrics};
pub use rewards::{NormalizedReward, RewardBreakdown, RewardWeights};

// Re-export new enum types with explicit names for future use
pub use enums::{ActionStatus as NewActionStatus, ActionType as NewActionType};
