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
#![allow(clippy::module_name_repetitions)]

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
