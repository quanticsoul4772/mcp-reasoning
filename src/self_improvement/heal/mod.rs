//! Self-healing of parse/schema failures (feature `001-heal-parse-schema`).
//!
//! Foundational data model and safety helpers for the operator-reviewed self-heal
//! loop:
//! - [`types`]: `DefectRecord`, `FixProposal`, `KnowledgeEntry` and their enums.
//! - [`redact()`]: secret-scrubbing + stable hashing of triggering input (FR-016).
//! - [`guard`]: the integrity guard protecting the acceptance/measurement surface
//!   (FR-010) so a fix can never game its own success signal.
//!
//! This module is self-contained. Wiring the propose-PR action into the live
//! executor (US2/US3) is a later increment, gated by the constitution.

pub mod detect;
pub mod eligibility;
pub mod guard;
pub mod invariant_guard;
pub mod plan;
pub mod pr;
pub mod redact;
pub mod sink;
pub mod types;

pub use detect::{DefectLog, DEFAULT_RECURRENCE_THRESHOLD};
pub use eligibility::{classify_eligibility, EligibilityOutcome};
pub use guard::{is_protected, protected_paths};
pub use invariant_guard::{scan_for_weakened_invariants, ChangedFile, InvariantVerdict};
pub use plan::{
    blast_radius, classify, is_drift_class, partition_drift, rank_and_cap, severity,
    DEFAULT_DRIFT_THRESHOLD,
};
pub use pr::{gh_available, pr_create_args};
pub use redact::{redact, RedactedInput};
pub use sink::DefectSink;
pub use types::{
    DefectRecord, DefectStatus, FailureClass, FixProposal, KnowledgeEntry, ProposalReview,
    EXCERPT_MAX,
};
