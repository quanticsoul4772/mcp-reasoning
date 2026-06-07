//! Self-improvement system.
//!
//! This module provides a 4-phase autonomous optimization loop:
//!
//! 1. **Monitor**: Collect metrics and detect issues
//! 2. **Analyze**: LLM-based diagnosis and action proposal
//! 3. **Execute**: Apply approved actions with rollback capability
//! 4. **Learn**: Extract lessons from completed actions
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                Self-Improvement System                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌────────┐ │
//! │  │ Monitor  │───▶│ Analyzer │───▶│ Executor │───▶│ Learner│ │
//! │  └──────────┘    └──────────┘    └──────────┘    └────────┘ │
//! │       │                               │                      │
//! │       │         ┌──────────────┐      │                      │
//! │       └────────▶│Circuit Breaker│◀────┘                      │
//! │                 └──────────────┘                             │
//! │                        │                                     │
//! │                 ┌──────────────┐                             │
//! │                 │  Allowlist   │                             │
//! │                 └──────────────┘                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Safety Mechanisms
//!
//! - **Circuit Breaker**: Halts operations after consecutive failures
//! - **Allowlist**: Validates actions against allowed types and parameters
//! - **Rate Limiting**: Prevents excessive actions per time period
//! - **Approval Gate**: Optional human approval before execution
//!
//! # Example
//!
//! ```
//! use mcp_reasoning::self_improvement::{
//!     ActionType, ActionStatus, Severity, CircuitBreakerConfig,
//! };
//! use mcp_reasoning::metrics::MetricsCollector;
//!
//! // Create a metrics collector for tracking
//! let metrics = MetricsCollector::new();
//!
//! // Configuration types are available for customization
//! let breaker_config = CircuitBreakerConfig::default();
//! assert!(breaker_config.failure_threshold > 0);
//!
//! // Action types for the self-improvement system
//! assert!(matches!(ActionType::ConfigAdjust, ActionType::ConfigAdjust));
//! assert!(matches!(ActionStatus::Proposed, ActionStatus::Proposed));
//! assert!(matches!(Severity::Info, Severity::Info));
//! ```

mod allowlist;
mod analyzer;
pub mod baseline;
mod circuit_breaker;
pub mod cli;
mod executor;
mod learner;
pub mod manager;
mod monitor;
pub mod sensor;
pub mod storage;
mod system;
mod types;

// Re-export main types
pub use allowlist::{Allowlist, AllowlistConfig, ValidationError, ValidationErrorCode};
pub use analyzer::{AnalysisResult, Analyzer};
pub use baseline::{Baseline as BaselineTracker, BaselineCollection, BaselineConfig, ToolBaseline};
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitState,
};
pub use cli::{
    format_duration, help_text, parse_duration, CommandParseError, SelfImproveCommands,
    StatusOutput,
};
pub use executor::{ConfigState, ExecutionRecord, ExecutionResult, Executor};
pub use learner::{ActionTypeStats, Learner, LearnerConfig, LearningResult, LearningSummary};
pub use manager::{
    ApproveResult, ConfigRecommendation, ExecutionResultSummary, LearningResultSummary,
    LearningSummaryData, ManagerCommand, ManagerHandle, ManagerStatus, PendingDiagnosis,
    SelfImprovementManager,
};
pub use monitor::{Baseline, Monitor, MonitorConfig, MonitorResult};
pub use sensor::{measure_delta, MeasuredDelta};
pub use storage::{
    ActionRecord, ConfigOverrideRecord, DiagnosisRecord, InvocationRecord, InvocationStats,
    SelfImprovementStorage,
};
pub use system::{CycleResult, SelfImprovementSystem, SystemConfig};
pub use types::{
    ActionStatus, ActionType, Baselines, DiagnosisStatus, LegacyTriggerMetric, Lesson,
    SelfImprovementAction, Severity, SystemMetrics,
};

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;

    #[test]
    fn test_exports_available() {
        // Verify all major types are exported
        let _ = ActionType::ConfigAdjust;
        let _ = ActionStatus::Proposed;
        let _ = Severity::Info;
    }
}
