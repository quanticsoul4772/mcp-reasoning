//! Agent system for orchestrating reasoning tools.
//!
//! This module provides:
//! - Agent abstraction with roles and capabilities
//! - Agent registry for managing built-in and custom agents
//! - Agent executor for LLM-planned tool dispatch
//! - Team coordination for multi-agent tasks
//! - Learning and metrics for agent optimization
//!
//! # Architecture
//!
//! Agents compose reasoning tools into higher-level workflows:
//!
//! ```text
//! User Task
//!     |
//!     v
//! AgentExecutor (LLM plans steps)
//!     |
//!     v
//! Reasoning Modes (linear, tree, decision, etc.)
//! ```

pub mod communication;
pub mod coordinator;
pub mod core;
pub mod decomposer;
pub mod executor;
pub mod learning;
pub mod metrics;
pub mod registry;
pub mod team;
pub mod team_registry;
pub mod types;

pub use self::core::AgentCore;
pub use self::executor::AgentExecutor;
pub use self::registry::AgentRegistry;
pub use self::team_registry::TeamRegistry;
pub use self::types::{Agent, AgentCapability, AgentConfig, AgentRole, AgentStatus};
