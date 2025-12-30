//! Database operations for self-improvement system.
//!
//! Provides CRUD operations for:
//! - Invocation records (from Monitor)
//! - Diagnosis records (from Analyzer)
//! - Action records (from Executor)
//! - Learning records (from Learner)
//! - Config overrides (applied by Executor)

mod helpers;
mod operations;
mod records;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;

// Re-export main types
pub use operations::SelfImprovementStorage;
pub use records::{
    ActionRecord, ConfigOverrideRecord, DiagnosisRecord, InvocationRecord, InvocationStats,
    LearningRecord,
};
