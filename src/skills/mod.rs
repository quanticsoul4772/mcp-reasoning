//! Composable skill system.
//!
//! Skills extend presets with context passing, conditional execution,
//! and error handling strategies.
//!
//! # Skills vs Presets
//!
//! Skills build on the preset concept but add:
//! - **Context passing**: Results flow between steps via named keys
//! - **Conditions**: Steps can be conditional on previous results
//! - **Error strategies**: Per-step error handling (fail/skip/retry)
//! - **Discovery**: New skills can be mined from tool chain patterns

pub mod builtin;
pub mod discovery;
pub mod executor;
pub mod registry;
pub mod types;

pub use self::executor::SkillExecutor;
pub use self::registry::SkillRegistry;
pub use self::types::{ErrorStrategy, Skill, SkillContext, SkillStep, StepCondition};
