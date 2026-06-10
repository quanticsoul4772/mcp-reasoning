//! Data model for the self-heal feature (spec 001): DefectRecord, FixProposal,
//! KnowledgeEntry, and their enums. Self-contained; the live executor wiring
//! (the propose-PR action) is a later increment.

use serde::{Deserialize, Serialize};

use super::redact::redact;

/// Maximum characters retained in a defect's redacted input excerpt.
pub const EXCERPT_MAX: usize = 400;

/// The class of a detected self-defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    /// Output could not be parsed (malformed/unparseable).
    Parse,
    /// Output violated its declared schema/contract.
    Schema,
    /// Failure attributed to model/provider drift, not a code defect.
    Drift,
}

impl std::fmt::Display for FailureClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse => write!(f, "parse"),
            Self::Schema => write!(f, "schema"),
            Self::Drift => write!(f, "drift"),
        }
    }
}

/// Lifecycle status of a [`DefectRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefectStatus {
    /// Seen, below the recurrence threshold.
    Observed,
    /// Crossed the recurrence threshold; eligible for a proposal.
    Recurring,
    /// A fix proposal has been opened.
    Proposed,
    /// An accepted fix resolved it.
    Resolved,
    /// Classified as drift; routed away from the repair path.
    DriftRouted,
}

/// Operator review state of a [`FixProposal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalReview {
    /// Awaiting operator decision.
    Proposed,
    /// Operator approved (still subject to the admissibility gate).
    Approved,
    /// Operator rejected.
    Rejected,
}

impl ProposalReview {
    /// Stable lowercase string form for persistence.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    /// Parse the persisted form; an unrecognized value falls back to `Proposed`
    /// (the safe default — never an accidental approval).
    #[must_use]
    pub fn from_db(s: &str) -> Self {
        match s {
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            _ => Self::Proposed,
        }
    }
}

/// A detected failure of the server's own output (FR-002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefectRecord {
    /// Stable id (content hash of the triggering input).
    pub id: String,
    /// Originating tool/mode (e.g. `reasoning_linear/linear`).
    pub component: String,
    /// Failure class.
    pub failure_class: FailureClass,
    /// Content hash of the triggering input (recurrence key).
    pub input_hash: String,
    /// Bounded, redacted excerpt — never raw/secret input (FR-016).
    pub excerpt: String,
    /// First observation timestamp (caller-provided epoch millis).
    pub first_seen: i64,
    /// Most recent observation timestamp.
    pub last_seen: i64,
    /// Recurrence counter within the recurrence window (FR-003).
    pub occurrences: u32,
    /// Lifecycle status.
    pub status: DefectStatus,
    /// Largest occurrence count for any single triggering input (spec 002, US2).
    /// The stable-path signal: a code defect repeats on the *same* input, whereas
    /// input-induced noise is spread across many inputs. Maintained by `DefectLog`;
    /// `1` for a standalone record built via [`DefectRecord::observe`].
    pub max_input_occurrences: u32,
    /// Number of distinct triggering inputs seen for this signature (spec 002).
    pub distinct_inputs: u32,
}

impl DefectRecord {
    /// Record a first observation of a defect, redacting the triggering input.
    #[must_use]
    pub fn observe(component: &str, class: FailureClass, raw_input: &str, now: i64) -> Self {
        let r = redact(raw_input, EXCERPT_MAX);
        Self {
            id: r.hash.clone(),
            component: component.to_string(),
            failure_class: class,
            input_hash: r.hash,
            excerpt: r.excerpt,
            first_seen: now,
            last_seen: now,
            occurrences: 1,
            status: DefectStatus::Observed,
            max_input_occurrences: 1,
            distinct_inputs: 1,
        }
    }

    /// The `(component, failure_class)` recurrence signature.
    #[must_use]
    pub fn signature(&self) -> String {
        format!("{}::{}", self.component, self.failure_class)
    }

    /// Note another occurrence at `now`.
    pub fn record_occurrence(&mut self, now: i64) {
        self.occurrences = self.occurrences.saturating_add(1);
        self.last_seen = now;
    }

    /// True once the recurrence threshold is met (FR-003); promotes
    /// `Observed → Recurring` when called as a check.
    #[must_use]
    pub fn is_recurring(&self, threshold: u32) -> bool {
        self.occurrences >= threshold && self.failure_class != FailureClass::Drift
    }

    /// True when this defect is eligible to *propose* a fix (spec 002, FR-004): a
    /// single triggering input recurred to `threshold` — a stable, repeatable code
    /// path — and it is not drift. Varied-input recurrence (many distinct inputs,
    /// none repeated to threshold) is NOT eligible: it is more likely input-induced
    /// than a server code defect, so it is recorded but held back from the loop.
    #[must_use]
    pub fn is_propose_eligible(&self, threshold: u32) -> bool {
        self.max_input_occurrences >= threshold && self.failure_class != FailureClass::Drift
    }

    /// Mark this defect as routed to the drift response (FR-012); it never enters
    /// the repair path.
    pub fn route_drift(&mut self) {
        self.failure_class = FailureClass::Drift;
        self.status = DefectStatus::DriftRouted;
    }
}

/// A candidate change for a recurring [`DefectRecord`].
// Flat verdict record: each bool is an independent gate result (grounded / suite /
// quality / weakens-invariant); a state machine would obscure, not clarify.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixProposal {
    /// Proposal id.
    pub id: String,
    /// The defect this addresses (the defect's content-hash id).
    pub defect_id: String,
    /// The `(component, failure_class)` signature of the defect class this guards.
    /// An accepted proposal becomes a [`KnowledgeEntry`] keyed by this, so a later
    /// defect of the same class can reuse the fix (FR-011).
    pub failure_signature: String,
    /// Generated branch name.
    pub branch: String,
    /// Human summary of the change.
    pub change_summary: String,
    /// Path/name of the reproducing test.
    pub reproducing_test_ref: String,
    /// The reproducing test demonstrably failed on the unpatched code (D4).
    pub grounded: bool,
    /// Full test suite passed with the fix.
    pub suite_green: bool,
    /// fmt + clippy + rustc passed.
    pub quality_green: bool,
    /// PR url once opened (none until then).
    pub pr_url: Option<String>,
    /// Operator review state.
    pub review_status: ProposalReview,
    /// The fix appears to weaken a validation/range/contract check (spec 002, US1).
    /// When true the proposal is NEVER admissible, regardless of the test gates —
    /// the reproducing test proves a behavior change, not that the change is correct.
    pub weakens_invariant: bool,
    /// Why the proposal was flagged/held (e.g. which invariant the fix would
    /// weaken) — operator-visible (FR-009). `None` when nothing was flagged.
    pub block_reason: Option<String>,
}

impl FixProposal {
    /// Admissibility gate (FR-008, spec 001/002): a fix may merge only if its
    /// reproducing test is grounded, the full suite is green, quality gates pass,
    /// AND it does not weaken a validation/range/contract check. Operator approval
    /// is required additionally and tracked separately.
    #[must_use]
    pub fn is_admissible(&self) -> bool {
        self.grounded && self.suite_green && self.quality_green && !self.weakens_invariant
    }
}

/// An accepted defect→fix→test mapping, reused on recurrence (FR-011).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// Entry id.
    pub id: String,
    /// `(component, failure_class)` signature this guards.
    pub failure_signature: String,
    /// Summary of the accepted fix.
    pub fix_summary: String,
    /// The reproducing test that guards the class.
    pub test_ref: String,
    /// Acceptance timestamp.
    pub accepted_at: i64,
}

impl KnowledgeEntry {
    /// Build the durable knowledge mapping for an accepted proposal (FR-011, T030).
    ///
    /// Keyed by the proposal's `failure_signature` so a later defect of the same
    /// class can reuse it. The caller must have verified admissibility + operator
    /// approval before recording acceptance.
    #[must_use]
    pub fn from_accepted_proposal(proposal: &FixProposal, accepted_at: i64) -> Self {
        Self {
            id: format!("knowledge-{}", proposal.id),
            failure_signature: proposal.failure_signature.clone(),
            fix_summary: proposal.change_summary.clone(),
            test_ref: proposal.reproducing_test_ref.clone(),
            accepted_at,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn observe_redacts_and_initializes() {
        let d = DefectRecord::observe(
            "reasoning_linear/linear",
            FailureClass::Parse,
            "sk-ant-XYZ123abc456 oops",
            100,
        );
        assert_eq!(d.occurrences, 1);
        assert_eq!(d.status, DefectStatus::Observed);
        assert!(!d.excerpt.contains("sk-ant-XYZ123"));
        assert_eq!(d.id, d.input_hash);
    }

    #[test]
    fn recurrence_and_signature() {
        let mut d = DefectRecord::observe("c", FailureClass::Schema, "bad", 1);
        assert!(!d.is_recurring(3));
        d.record_occurrence(2);
        d.record_occurrence(3);
        assert!(d.is_recurring(3));
        assert_eq!(d.signature(), "c::schema");
        assert_eq!(d.last_seen, 3);
    }

    #[test]
    fn drift_never_recurs_into_repair() {
        let mut d = DefectRecord::observe("c", FailureClass::Parse, "x", 1);
        d.record_occurrence(2);
        d.record_occurrence(3);
        d.route_drift();
        assert_eq!(d.status, DefectStatus::DriftRouted);
        assert!(!d.is_recurring(3));
    }

    #[test]
    fn proposal_review_str_roundtrips_and_defaults_safely() {
        for r in [
            ProposalReview::Proposed,
            ProposalReview::Approved,
            ProposalReview::Rejected,
        ] {
            assert_eq!(ProposalReview::from_db(r.as_str()), r);
        }
        // An unknown value never becomes an accidental approval.
        assert_eq!(ProposalReview::from_db("garbage"), ProposalReview::Proposed);
    }

    #[test]
    fn admissibility_requires_all_three() {
        let mut p = FixProposal {
            id: "p1".into(),
            defect_id: "d1".into(),
            failure_signature: "c::parse".into(),
            branch: "heal/d1".into(),
            change_summary: "fix".into(),
            reproducing_test_ref: "t".into(),
            grounded: true,
            suite_green: true,
            quality_green: true,
            pr_url: None,
            review_status: ProposalReview::Proposed,
            weakens_invariant: false,
            block_reason: None,
        };
        assert!(p.is_admissible());
        p.grounded = false;
        assert!(!p.is_admissible());
    }

    #[test]
    fn weakens_invariant_blocks_admissibility_despite_green_gates() {
        // spec 002 US1: even fully green, a fix that weakens a validation check
        // is NEVER admissible — the reproducing test proves a change, not correctness.
        let p = FixProposal {
            id: "p2".into(),
            defect_id: "d2".into(),
            failure_signature: "reasoning_linear/linear::schema".into(),
            branch: "heal/d2".into(),
            change_summary: "widen confidence range".into(),
            reproducing_test_ref: "t".into(),
            grounded: true,
            suite_green: true,
            quality_green: true,
            pr_url: None,
            review_status: ProposalReview::Proposed,
            weakens_invariant: true,
            block_reason: Some("widened (0.0..=1.0) → (0.0..=100.0)".into()),
        };
        assert!(!p.is_admissible());
    }
}
