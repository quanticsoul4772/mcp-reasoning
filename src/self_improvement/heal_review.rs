//! Operator review + knowledge reuse for self-heal proposals (spec 001, US3).
//!
//! This is the acceptance side of the loop (T029–T031):
//! - [`accept_proposal`]: an operator approves a persisted [`FixProposal`]. The
//!   merge is permitted ONLY if the proposal is admissible
//!   (`grounded ∧ suite_green ∧ quality_green`, FR-008/FR-009); on acceptance a
//!   durable [`KnowledgeEntry`] is recorded for reuse (T030).
//! - [`reject_proposal`]: an operator rejects it.
//! - [`find_reusable_fix`]: a new defect matching a previously-accepted class is
//!   recognized so the loop can skip re-diagnosis (T031, SC-006).
//!
//! The loop can never self-approve: these functions are the operator override
//! path. The cycle only ever produces `Proposed` proposals.

use crate::error::StorageError;
use crate::self_improvement::heal::{DefectRecord, KnowledgeEntry, ProposalReview};
use crate::self_improvement::storage::SelfImprovementStorage;

/// Why a proposal could not be accepted/rejected.
#[derive(Debug)]
pub enum ReviewError {
    /// No proposal with that id is persisted.
    NotFound(String),
    /// The proposal did not pass the admissibility gate, so it may not be
    /// accepted no matter the operator's intent (FR-008/FR-009).
    NotAdmissible(String),
    /// An underlying storage failure.
    Storage(StorageError),
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "fix proposal '{id}' not found"),
            Self::NotAdmissible(id) => write!(
                f,
                "fix proposal '{id}' is not admissible (grounded ∧ suite_green ∧ quality_green); cannot accept"
            ),
            Self::Storage(e) => write!(f, "storage error: {e}"),
        }
    }
}

impl std::error::Error for ReviewError {}

impl From<StorageError> for ReviewError {
    fn from(e: StorageError) -> Self {
        Self::Storage(e)
    }
}

/// Operator-accept a proposal: gate on admissibility, mark it approved, and record
/// the durable knowledge mapping for reuse. Returns the recorded entry.
///
/// `now` is the acceptance timestamp (epoch millis), injected for testability.
///
/// # Errors
/// [`ReviewError::NotFound`] if the id is unknown; [`ReviewError::NotAdmissible`]
/// if the gate fails (no approval is recorded in that case); storage errors.
pub async fn accept_proposal(
    storage: &SelfImprovementStorage,
    proposal_id: &str,
    now: i64,
) -> Result<KnowledgeEntry, ReviewError> {
    let proposal = storage
        .get_fix_proposal(proposal_id)
        .await?
        .ok_or_else(|| ReviewError::NotFound(proposal_id.to_string()))?;

    // The admissibility gate is non-negotiable: a non-green proposal can never be
    // accepted, even by an operator (the merge would regress the suite).
    if !proposal.is_admissible() {
        return Err(ReviewError::NotAdmissible(proposal_id.to_string()));
    }

    storage
        .update_proposal_review(proposal_id, ProposalReview::Approved)
        .await?;

    let entry = KnowledgeEntry::from_accepted_proposal(&proposal, now);
    storage.upsert_knowledge_entry(&entry).await?;
    Ok(entry)
}

/// Operator-reject a proposal. No knowledge is recorded.
///
/// # Errors
/// [`ReviewError::NotFound`] if the id is unknown; storage errors.
pub async fn reject_proposal(
    storage: &SelfImprovementStorage,
    proposal_id: &str,
) -> Result<(), ReviewError> {
    storage
        .get_fix_proposal(proposal_id)
        .await?
        .ok_or_else(|| ReviewError::NotFound(proposal_id.to_string()))?;

    storage
        .update_proposal_review(proposal_id, ProposalReview::Rejected)
        .await?;
    Ok(())
}

/// If a previously-accepted fix already guards `defect`'s failure class, return it
/// so the loop can reference it and skip re-diagnosis (FR-011, SC-006).
///
/// # Errors
/// Propagates storage errors.
pub async fn find_reusable_fix(
    storage: &SelfImprovementStorage,
    defect: &DefectRecord,
) -> Result<Option<KnowledgeEntry>, StorageError> {
    storage
        .get_knowledge_by_signature(&defect.signature())
        .await
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::self_improvement::heal::{FailureClass, FixProposal};
    use crate::storage::SqliteStorage;
    use serial_test::serial;

    async fn storage() -> SelfImprovementStorage {
        let sqlite = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        SelfImprovementStorage::new(sqlite.pool.clone())
    }

    fn proposal(id: &str, admissible: bool) -> FixProposal {
        FixProposal {
            id: id.to_string(),
            defect_id: "hash123".to_string(),
            failure_signature: "reasoning_linear/linear::parse".to_string(),
            branch: format!("heal/{id}"),
            change_summary: "broaden the JSON parser".to_string(),
            reproducing_test_ref: "tests/heal_repro_parse.rs".to_string(),
            grounded: true,
            suite_green: admissible,
            quality_green: true,
            pr_url: Some("https://github.com/o/r/pull/9".to_string()),
            review_status: ProposalReview::Proposed,
        }
    }

    #[tokio::test]
    #[serial]
    async fn accept_admissible_marks_approved_and_records_knowledge() {
        let s = storage().await;
        s.upsert_fix_proposal(&proposal("p1", true)).await.unwrap();

        let entry = accept_proposal(&s, "p1", 1_700_000_000_000).await.unwrap();
        assert_eq!(entry.failure_signature, "reasoning_linear/linear::parse");
        assert_eq!(entry.test_ref, "tests/heal_repro_parse.rs");

        // The proposal is now Approved...
        let reloaded = s.get_fix_proposal("p1").await.unwrap().unwrap();
        assert_eq!(reloaded.review_status, ProposalReview::Approved);
        // ...and the knowledge is retrievable for reuse.
        let known = s
            .get_knowledge_by_signature("reasoning_linear/linear::parse")
            .await
            .unwrap();
        assert!(known.is_some());
    }

    #[tokio::test]
    #[serial]
    async fn accept_non_admissible_is_refused_and_records_nothing() {
        let s = storage().await;
        s.upsert_fix_proposal(&proposal("p2", false)).await.unwrap();

        let err = accept_proposal(&s, "p2", 1).await.unwrap_err();
        assert!(matches!(err, ReviewError::NotAdmissible(_)));

        // No approval, no knowledge written (FR-009).
        let reloaded = s.get_fix_proposal("p2").await.unwrap().unwrap();
        assert_eq!(reloaded.review_status, ProposalReview::Proposed);
        assert!(s
            .get_knowledge_by_signature("reasoning_linear/linear::parse")
            .await
            .unwrap()
            .is_none());
    }

    #[test]
    fn review_error_display_and_from() {
        assert!(ReviewError::NotFound("p1".to_string())
            .to_string()
            .contains("p1"));
        assert!(ReviewError::NotAdmissible("p2".to_string())
            .to_string()
            .contains("admissible"));
        let storage_err = StorageError::QueryFailed {
            query: "q".to_string(),
            message: "boom".to_string(),
        };
        let review: ReviewError = storage_err.into();
        assert!(matches!(review, ReviewError::Storage(_)));
        assert!(review.to_string().contains("storage error"));
    }

    #[tokio::test]
    #[serial]
    async fn accept_missing_proposal_errors() {
        let s = storage().await;
        let err = accept_proposal(&s, "nope", 1).await.unwrap_err();
        assert!(matches!(err, ReviewError::NotFound(_)));
    }

    #[tokio::test]
    #[serial]
    async fn reject_marks_rejected_without_knowledge() {
        let s = storage().await;
        s.upsert_fix_proposal(&proposal("p3", true)).await.unwrap();

        reject_proposal(&s, "p3").await.unwrap();
        let reloaded = s.get_fix_proposal("p3").await.unwrap().unwrap();
        assert_eq!(reloaded.review_status, ProposalReview::Rejected);
        assert!(s
            .get_knowledge_by_signature("reasoning_linear/linear::parse")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    #[serial]
    async fn reuse_recognizes_a_previously_accepted_class() {
        let s = storage().await;
        s.upsert_fix_proposal(&proposal("p4", true)).await.unwrap();
        accept_proposal(&s, "p4", 1).await.unwrap();

        // A NEW defect of the same (component, class) — different input/hash —
        // finds the accepted fix, so the loop can skip re-diagnosis.
        let recurrence =
            DefectRecord::observe("reasoning_linear/linear", FailureClass::Parse, "other", 9);
        let reusable = find_reusable_fix(&s, &recurrence).await.unwrap();
        assert!(reusable.is_some());

        // A different class does NOT match.
        let other = DefectRecord::observe("reasoning_tree/tree", FailureClass::Schema, "x", 9);
        assert!(find_reusable_fix(&s, &other).await.unwrap().is_none());
    }
}
