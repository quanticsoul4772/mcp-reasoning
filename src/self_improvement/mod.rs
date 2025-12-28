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
//! ```ignore
//! use mcp_reasoning::self_improvement::{SelfImprovementSystem, SystemConfig};
//! use mcp_reasoning::metrics::MetricsCollector;
//!
//! let metrics = MetricsCollector::new();
//! let client = AnthropicClient::new(...);
//! let mut system = SelfImprovementSystem::with_defaults(client);
//!
//! // Run improvement cycle
//! let result = system.run_cycle(&metrics).await?;
//!
//! if !result.pending_actions().is_empty() {
//!     // Review and approve actions
//!     system.approve_and_execute();
//! }
//! ```

mod allowlist;
mod analyzer;
pub mod anthropic_calls;
pub mod baseline;
mod circuit_breaker;
pub mod cli;
mod executor;
mod learner;
mod monitor;
pub mod storage;
mod system;
mod types;

// Re-export main types
pub use allowlist::{Allowlist, AllowlistConfig, ValidationError, ValidationErrorCode};
pub use analyzer::{AnalysisResult, Analyzer};
pub use anthropic_calls::{
    AnthropicCalls, DiagnosisContent, HealthContext, LearningContext, LearningSynthesis,
    MetricsContext, TriggerContext, ValidationResult,
};
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
pub use monitor::{Baseline, Monitor, MonitorConfig, MonitorResult};
pub use storage::{
    ActionRecord, ConfigOverrideRecord, DiagnosisRecord, InvocationRecord, InvocationStats,
    LearningRecord, SelfImprovementStorage,
};
pub use system::{CycleResult, SelfImprovementSystem, SystemConfig};
pub use types::{
    ActionStatus, ActionType, Baselines, ConfigScope, DiagnosisStatus, LegacyTriggerMetric, Lesson,
    MetricsSnapshot, NormalizedReward, ParamValue, ResourceType, RewardBreakdown, RewardWeights,
    SelfDiagnosis, SelfImprovementAction, Severity, SuggestedAction, SystemMetrics, ToolMetrics,
    TriggerMetric,
};

#[cfg(test)]
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
