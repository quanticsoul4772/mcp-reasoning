//! CLI commands for the self-improvement system.
//!
//! Provides commands for monitoring, controlling, and debugging
//! the self-improvement system from the command line.

mod commands;
mod duration;
mod errors;
mod help;
mod output_types;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;

// Re-export main types
pub use commands::SelfImproveCommands;
pub use duration::{format_duration, parse_duration};
pub use errors::CommandParseError;
pub use help::help_text;
pub use output_types::{
    ActionHistoryEntry, AnalyzerConfigOutput, BaselinesOutput, CircuitBreakerConfigOutput,
    CircuitBreakerOutput, ConfigOutput, ConfigOverrideOutput, DiagnosticsOutput,
    ExecutorConfigOutput, GlobalBaselinesOutput, HealthDiagnostics, HistoryOutput,
    LearnerConfigOutput, MonitorConfigOutput, PerformanceDiagnostics, ResourceDiagnostics,
    StatusOutput, ToolBaselinesOutput,
};
