//! Propose-cycle dispatch (spec 001, T024b): turn recurring defects into
//! persisted fix proposals.
//!
//! For each ranked, capped recurring defect (FR-013): reuse a prior accepted fix
//! if its class is already known (skip re-diagnosis, FR-011/SC-006), otherwise
//! `localize → propose_pr` and persist the resulting [`FixProposal`]. Drift is
//! excluded by the ranking (FR-012). A per-defect LLM/repair error is recorded
//! and the cycle continues; only a storage failure aborts.
//!
//! This is the orchestration brain; the live call site (gated by config, with a
//! real workspace + `SystemCommandRunner`) wires it into the running manager.

use std::path::Path;

use crate::error::StorageError;
use crate::self_improvement::analyzer::localize;
use crate::self_improvement::heal::{
    blast_radius, classify_eligibility, partition_drift, rank_and_cap, DefectRecord,
    EligibilityOutcome, FixProposal, DEFAULT_DRIFT_THRESHOLD, DEFAULT_RECURRENCE_THRESHOLD,
};
use crate::self_improvement::heal_review::find_reusable_fix;
use crate::self_improvement::repair::{propose_pr, CommandRunner};
use crate::self_improvement::storage::SelfImprovementStorage;
use crate::traits::AnthropicClientTrait;

/// Outcome counts for one propose cycle.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProposeCycleSummary {
    /// Proposals whose admissible fix opened a PR.
    pub proposed: usize,
    /// Proposals persisted without a PR (the fix did not pass the gates).
    pub not_admissible: usize,
    /// Defects skipped because their class is already guarded (knowledge reuse).
    pub reused: usize,
    /// Defects routed to the drift response (broad/model drift) — alerted and
    /// recorded, never patched (FR-012, FR-005).
    pub drift: usize,
    /// Defects held back from propose (varied-input / ambiguous) — recorded, not
    /// patched (spec 002, FR-006).
    pub held_back: usize,
    /// Defects whose localize/propose step errored (cycle continued).
    pub errored: usize,
    /// The proposals produced this cycle (persisted).
    pub proposals: Vec<FixProposal>,
    /// `(signature, reason)` for every defect held back or routed to drift, so the
    /// operator sees why nothing was proposed for it (FR-007/FR-009/SC-004).
    pub held_back_reasons: Vec<(String, String)>,
}

/// Run one propose cycle over `recurring`, capped at `max_proposals` (FR-013).
///
/// # Errors
/// Returns [`StorageError`] only on a storage failure (knowledge lookup or
/// proposal persistence). LLM/repair failures are counted in the summary, not
/// propagated, so one bad defect never sinks the cycle.
pub async fn run_propose_cycle<C, R>(
    client: &C,
    runner: &R,
    storage: &SelfImprovementStorage,
    recurring: &[DefectRecord],
    workspace: &Path,
    max_proposals: usize,
    latest_model_change: Option<i64>,
) -> Result<ProposeCycleSummary, StorageError>
where
    C: AnthropicClientTrait,
    R: CommandRunner,
{
    let mut summary = ProposeCycleSummary::default();

    // Drift response (FR-012, D3): a failure class broad across components — or one
    // already classed Drift — is model/provider drift, not a code bug. Alert +
    // record it and route it away from the repair path (no patch). Only the
    // localized code defects continue to the propose path.
    let (code_defects, drift_defects) = partition_drift(recurring, DEFAULT_DRIFT_THRESHOLD);
    for d in &drift_defects {
        tracing::warn!(
            signature = %d.signature(),
            component = %d.component,
            occurrences = d.occurrences,
            "self-heal: failure classified as DRIFT (broad/model) — routed away from repair, no patch (FR-012)"
        );
        summary.drift += 1;
        summary.held_back_reasons.push((
            d.signature(),
            "structural drift (broad across components)".to_string(),
        ));
    }

    // Attribution gate (spec 002, US2): keep only stable-path code defects; hold
    // back varied-input/ambiguous ones and route model-correlated ones to drift.
    // Runs BEFORE rank_and_cap so ranking only sees eligible defects (Constitution IV).
    let mut eligible: Vec<DefectRecord> = Vec::new();
    for d in &code_defects {
        let model_changed = latest_model_change.is_some_and(|t| t >= d.first_seen);
        match classify_eligibility(d, model_changed, DEFAULT_RECURRENCE_THRESHOLD) {
            EligibilityOutcome::Eligible => eligible.push(d.clone()),
            EligibilityOutcome::HeldBack(reason) => {
                tracing::info!(signature = %d.signature(), %reason, "self-heal: defect held back from propose (attribution)");
                summary.held_back += 1;
                summary.held_back_reasons.push((d.signature(), reason));
            }
            EligibilityOutcome::Drift(reason) => {
                tracing::warn!(signature = %d.signature(), %reason, "self-heal: model-drift correlated — routed to drift (FR-005)");
                summary.drift += 1;
                summary.held_back_reasons.push((d.signature(), reason));
            }
        }
    }

    // Rank by frequency × severity, cap at K (FR-013); drift + ineligible removed.
    let selected = rank_and_cap(
        &eligible,
        |d| blast_radius(recurring, d.failure_class),
        max_proposals,
    );

    for defect in &selected {
        // Knowledge reuse (FR-011/SC-006): a class already guarded by an accepted
        // fix needs no re-diagnosis.
        if find_reusable_fix(storage, defect).await?.is_some() {
            summary.reused += 1;
            continue;
        }

        let Ok(localization) = localize(client, defect).await else {
            summary.errored += 1;
            continue;
        };

        let branch = format!("heal/{}", branch_slug(&defect.id));
        match propose_pr(client, runner, workspace, defect, &localization, &branch).await {
            Ok(proposal) => {
                storage.upsert_fix_proposal(&proposal).await?;
                if proposal.pr_url.is_some() {
                    summary.proposed += 1;
                } else {
                    summary.not_admissible += 1;
                }
                summary.proposals.push(proposal);
            }
            Err(_) => {
                summary.errored += 1;
            }
        }
    }

    Ok(summary)
}

/// A filesystem/branch-safe slug from a defect id (a hex content hash).
fn branch_slug(defect_id: &str) -> String {
    let slug: String = defect_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(12)
        .collect();
    if slug.is_empty() {
        "defect".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::self_improvement::heal::{FailureClass, KnowledgeEntry, ProposalReview};
    use crate::self_improvement::repair::testutil::{failing, passing, ScriptedRunner};
    use crate::self_improvement::repair::CommandOutput;
    use crate::storage::SqliteStorage;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};
    use serial_test::serial;

    const LOCALIZE_ARTIFACT: &str =
        r#"{"component": "reasoning_linear/linear", "source_hint": "src/modes/linear.rs"}"#;
    const SYNTH_ARTIFACT: &str = r##"{"test_name": "heal_repro_parse", "test_path": "tests/heal_repro_parse.rs", "test_code": "#[test]\nfn heal_repro_parse() { assert!(false); }\n"}"##;
    const FIX_TO_PROD: &str = r#"{"change_summary": "broaden the JSON parser", "files": [{"path": "src/modes/linear.rs", "contents": "// fixed\n"}]}"#;

    /// Client routing to the right artifact per pipeline stage by prompt content.
    fn staged_client() -> MockAnthropicClientTrait {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(move |messages, _| {
            let text: String = messages.iter().map(|m| m.content.clone()).collect();
            let body = if text.contains("REPRODUCES") {
                SYNTH_ARTIFACT
            } else if text.contains("source_hint") {
                LOCALIZE_ARTIFACT
            } else {
                FIX_TO_PROD
            };
            Ok(CompletionResponse::new(body, Usage::new(10, 10)))
        });
        client
    }

    fn ok() -> CommandOutput {
        CommandOutput {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    fn gh_url(url: &str) -> CommandOutput {
        CommandOutput {
            status: 0,
            stdout: format!("{url}\n"),
            stderr: String::new(),
        }
    }

    async fn storage() -> SelfImprovementStorage {
        let sqlite = SqliteStorage::new_in_memory().await.expect("storage");
        SelfImprovementStorage::new(sqlite.pool.clone())
    }

    fn defect(component: &str, class: FailureClass, occ: u32) -> DefectRecord {
        let mut d = DefectRecord::observe(component, class, "bad input", 1);
        for _ in 1..occ {
            d.record_occurrence(2);
        }
        // Stable-path defect: all occurrences from one input → propose-eligible.
        d.max_input_occurrences = occ;
        d.distinct_inputs = 1;
        d
    }

    /// A varied-input defect: recurs in aggregate but no single input repeats, so
    /// it is held back from the propose path (spec 002, US2).
    fn varied_defect(component: &str, class: FailureClass, occ: u32) -> DefectRecord {
        let mut d = DefectRecord::observe(component, class, "bad input", 1);
        for _ in 1..occ {
            d.record_occurrence(2);
        }
        d.max_input_occurrences = 1;
        d.distinct_inputs = occ;
        d
    }

    fn full_admissible_runner() -> ScriptedRunner {
        ScriptedRunner::new(vec![
            failing(),                               // synth grounding
            ok(),                                    // git checkout
            passing(),                               // reproducing passes
            passing(),                               // suite green
            ok(),                                    // fmt
            ok(),                                    // clippy
            ok(),                                    // git add
            ok(),                                    // git commit
            gh_url("https://github.com/o/r/pull/5"), // gh
        ])
    }

    #[tokio::test]
    #[serial]
    async fn proposes_and_persists_for_a_fresh_recurring_defect() {
        let dir = tempfile::tempdir().unwrap();
        let s = storage().await;
        let client = staged_client();
        let runner = full_admissible_runner();
        let recurring = vec![defect("reasoning_linear/linear", FailureClass::Parse, 5)];

        let summary = run_propose_cycle(&client, &runner, &s, &recurring, dir.path(), 5, None)
            .await
            .unwrap();

        assert_eq!(summary.proposed, 1);
        assert_eq!(summary.reused, 0);
        assert_eq!(summary.errored, 0);
        assert_eq!(summary.proposals.len(), 1);
        // The proposal was persisted with its PR URL.
        let id = &summary.proposals[0].id;
        let stored = s.get_fix_proposal(id).await.unwrap().unwrap();
        assert!(stored.pr_url.is_some());
        assert_eq!(stored.review_status, ProposalReview::Proposed);
    }

    #[tokio::test]
    #[serial]
    async fn reuses_known_class_and_skips_diagnosis() {
        let dir = tempfile::tempdir().unwrap();
        let s = storage().await;
        // Seed knowledge for the class.
        s.upsert_knowledge_entry(&KnowledgeEntry {
            id: "k1".to_string(),
            failure_signature: "reasoning_linear/linear::parse".to_string(),
            fix_summary: "already fixed".to_string(),
            test_ref: "tests/guard.rs".to_string(),
            accepted_at: 1,
        })
        .await
        .unwrap();

        let client = staged_client();
        // No runner outputs — reuse must short-circuit before any command.
        let runner = ScriptedRunner::new(vec![]);
        let recurring = vec![defect("reasoning_linear/linear", FailureClass::Parse, 5)];

        let summary = run_propose_cycle(&client, &runner, &s, &recurring, dir.path(), 5, None)
            .await
            .unwrap();

        assert_eq!(summary.reused, 1);
        assert_eq!(summary.proposed, 0);
        assert_eq!(runner.call_count(), 0);
    }

    #[tokio::test]
    #[serial]
    async fn drift_is_excluded_from_proposals() {
        let dir = tempfile::tempdir().unwrap();
        let s = storage().await;
        let client = staged_client();
        let runner = ScriptedRunner::new(vec![]);
        // A drift-classed defect must never be proposed (FR-012).
        let recurring = vec![defect("reasoning_linear/linear", FailureClass::Drift, 9)];

        let summary = run_propose_cycle(&client, &runner, &s, &recurring, dir.path(), 5, None)
            .await
            .unwrap();

        assert_eq!(summary.proposed, 0);
        assert_eq!(summary.reused, 0);
        assert_eq!(summary.errored, 0);
        assert_eq!(summary.drift, 1);
        assert_eq!(runner.call_count(), 0);
    }

    #[tokio::test]
    #[serial]
    async fn broad_parse_failure_is_routed_to_drift_not_proposed() {
        let dir = tempfile::tempdir().unwrap();
        let s = storage().await;
        let client = staged_client();
        let runner = ScriptedRunner::new(vec![]);
        // Same Parse class across 3 distinct components → structural drift (D3),
        // even though none is literally FailureClass::Drift.
        let recurring = vec![
            defect("reasoning_linear/linear", FailureClass::Parse, 5),
            defect("reasoning_tree/tree", FailureClass::Parse, 5),
            defect("reasoning_graph/graph", FailureClass::Parse, 5),
        ];

        let summary = run_propose_cycle(&client, &runner, &s, &recurring, dir.path(), 5, None)
            .await
            .unwrap();

        assert_eq!(summary.drift, 3);
        assert_eq!(summary.proposed, 0);
        // Routed away before any repair command ran.
        assert_eq!(runner.call_count(), 0);
    }

    #[tokio::test]
    #[serial]
    async fn caps_proposals_per_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let s = storage().await;
        let client = staged_client();
        // Only one admissible run is scripted; with the cap at 1 only one defect
        // is attempted even though two recur.
        let runner = full_admissible_runner();
        let recurring = vec![
            defect("reasoning_linear/linear", FailureClass::Parse, 9),
            defect("reasoning_tree/tree", FailureClass::Schema, 3),
        ];

        let summary = run_propose_cycle(&client, &runner, &s, &recurring, dir.path(), 1, None)
            .await
            .unwrap();

        assert_eq!(
            summary.proposed + summary.not_admissible + summary.errored,
            1
        );
    }

    #[test]
    fn branch_slug_is_sanitized() {
        assert_eq!(branch_slug("abc123def456ghi"), "abc123def456");
        assert_eq!(branch_slug("../../etc"), "etc");
        assert_eq!(branch_slug(""), "defect");
    }

    #[tokio::test]
    #[serial]
    async fn varied_input_defect_is_held_back_not_proposed() {
        let dir = tempfile::tempdir().unwrap();
        let s = storage().await;
        let client = staged_client();
        let runner = ScriptedRunner::new(vec![]);
        // Recurs in aggregate but no single input repeats → input-induced; held back.
        let recurring = vec![varied_defect(
            "reasoning_linear/linear",
            FailureClass::Schema,
            5,
        )];

        let summary = run_propose_cycle(&client, &runner, &s, &recurring, dir.path(), 5, None)
            .await
            .unwrap();

        assert_eq!(summary.held_back, 1);
        assert_eq!(summary.proposed, 0);
        assert_eq!(runner.call_count(), 0);
        assert!(summary.held_back_reasons[0].1.contains("input-induced"));
    }

    #[tokio::test]
    #[serial]
    async fn model_change_routes_recurring_defect_to_drift() {
        let dir = tempfile::tempdir().unwrap();
        let s = storage().await;
        let client = staged_client();
        let runner = ScriptedRunner::new(vec![]);
        // A stable-path defect first_seen at t=1; a model change at t=5 overlaps it.
        let recurring = vec![defect("reasoning_linear/linear", FailureClass::Schema, 5)];

        let summary = run_propose_cycle(&client, &runner, &s, &recurring, dir.path(), 5, Some(5))
            .await
            .unwrap();

        assert_eq!(summary.drift, 1);
        assert_eq!(summary.proposed, 0);
        assert_eq!(runner.call_count(), 0);
    }
}
