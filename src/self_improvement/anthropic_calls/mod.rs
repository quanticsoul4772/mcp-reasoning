//! Anthropic API calls for self-improvement system.
//!
//! Provides LLM-powered operations for:
//! - Diagnosis generation
//! - Action selection
//! - Decision validation
//! - Learning synthesis
//!
//! # Security
//!
//! This module implements input sanitization to prevent prompt injection attacks:
//! - `escape_for_prompt()` escapes format string markers and truncates long content
//! - `sanitize_multiline()` neutralizes instruction separator patterns
//! - `extract_json()` enforces size limits to prevent DoS

mod client;
mod parsers;
mod prompts;
mod security;
mod types;

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

// Re-export main types
pub use client::AnthropicCalls;
pub use types::{
    DiagnosisContent, HealthContext, LearningContext, LearningSynthesis, MetricsContext,
    TriggerContext, ValidationResult,
};
