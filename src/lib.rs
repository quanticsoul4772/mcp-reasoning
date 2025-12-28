//! MCP Reasoning Server
//!
//! A Rust-based MCP server providing structured reasoning capabilities
//! via direct Anthropic Claude API calls.
//!
//! # Features
//!
//! - 15 consolidated reasoning tools (vs 40 in predecessor)
//! - Direct Anthropic API integration
//! - Extended thinking support with configurable budgets
//! - `SQLite` persistence for sessions and state
//! - Self-improvement 4-phase optimization loop
//!
//! # Quick Start
//!
//! ```bash
//! ANTHROPIC_API_KEY=sk-ant-xxx ./mcp-reasoning
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     stdin      ┌─────────────────┐
//! │ Claude Code │───────────────▶│   MCP Server    │──────▶ Anthropic API
//! │ or Desktop  │◀───────────────│     (Rust)      │
//! └─────────────┘     stdout     └────────┬────────┘
//!                                         │
//!                                         ▼
//!                                      SQLite
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
// Allowed pedantic lints for practical reasons
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_const_for_fn)] // Many simple functions could be const but don't need to be
#![allow(clippy::cast_precision_loss)] // u64/usize to f64 for metrics is acceptable
#![allow(clippy::cast_possible_truncation)] // u128 milliseconds to u64 is safe in practice
#![allow(clippy::doc_markdown)] // Backticks in docs not required for all identifiers
#![allow(clippy::missing_errors_doc)] // Error documentation not required for all functions
#![allow(clippy::too_many_lines)] // Allow longer functions when logically coherent
#![allow(clippy::must_use_candidate)] // Not all getters need #[must_use]
#![allow(clippy::trivially_copy_pass_by_ref)] // Small types by ref is fine
#![allow(clippy::unused_self)] // Methods may have &self for future use
#![allow(clippy::match_same_arms)] // Explicit match arms can be clearer
#![allow(clippy::uninlined_format_args)] // format!("{}", x) is fine
#![allow(clippy::manual_let_else)] // if-let is sometimes clearer than let-else
#![allow(clippy::redundant_clone)] // Sometimes needed for borrow checker
#![allow(clippy::suboptimal_flops)] // Readable math over optimal
#![allow(clippy::assigning_clones)] // clone() is often clearer than clone_from()

pub mod anthropic;
pub mod config;
pub mod error;
pub mod metrics;
pub mod modes;
pub mod presets;
pub mod prompts;
pub mod self_improvement;
pub mod server;
pub mod storage;
pub mod traits;

#[cfg(test)]
mod test_utils;
