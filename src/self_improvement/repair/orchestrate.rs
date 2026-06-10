//! Propose-PR orchestration (spec 001, T024): chain the repair steps into a
//! single `FixProposal`.
//!
//! `localize → synthesize → fix → (if admissible) open PR`. The reproducing-test
//! gate (D4), the integrity guard (D6), and the admissibility gate (FR-008) are
//! enforced by the steps this composes; a PR is opened ONLY for an admissible
//! fix, and it is never merged (D5/FR-007). This function performs no policy
//! decision of its own beyond "open the PR only if admissible" — operator
//! approval still governs any merge.

use std::path::Path;

use crate::self_improvement::analyzer::Localization;
use crate::self_improvement::heal::{DefectRecord, FixProposal, ProposalReview};
use crate::traits::AnthropicClientTrait;

use super::{
    generate_and_validate_fix, open_pr, synthesize_reproducing_test, CommandRunner, RepairError,
};

/// Run the full propose-PR pipeline for `defect` on `branch`, returning the
/// resulting [`FixProposal`].
///
/// A PR is opened (and `pr_url` set) only when the fix is admissible
/// (`grounded ∧ suite_green ∧ quality_green`). A non-admissible fix returns a
/// proposal with `pr_url == None` so the caller can record the attempt without a
/// PR ever being opened.
///
/// # Errors
/// Propagates [`RepairError`] from any step — notably `NotGrounded` (the
/// reproducing test did not fail on the unpatched tree) and `Protected` (the fix
/// tried to touch the measurement surface), both of which abort before a PR.
pub async fn propose_pr<C, R>(
    client: &C,
    runner: &R,
    workspace: &Path,
    defect: &DefectRecord,
    localization: &Localization,
    branch: &str,
) -> Result<FixProposal, RepairError>
where
    C: AnthropicClientTrait,
    R: CommandRunner,
{
    // 1. Reproducing test, grounded on the unpatched tree (aborts if it passes).
    let grounded =
        synthesize_reproducing_test(client, runner, workspace, defect, localization).await?;

    // 2. Fix on a branch + validation. The integrity guard inside this step hard-
    //    errors before writing anything if the fix touches a protected path.
    let fix = generate_and_validate_fix(
        client,
        runner,
        workspace,
        defect,
        localization,
        &grounded,
        branch,
    )
    .await?;

    let mut proposal = FixProposal {
        id: format!("proposal-{}", branch.replace('/', "_")),
        defect_id: defect.id.clone(),
        failure_signature: defect.signature(),
        branch: branch.to_string(),
        change_summary: fix.change_summary.clone(),
        reproducing_test_ref: grounded.test_path.clone(),
        grounded: grounded.grounded,
        suite_green: fix.suite_green,
        quality_green: fix.quality_green,
        pr_url: None,
        review_status: ProposalReview::Proposed,
        weakens_invariant: fix.weakens_invariant,
        block_reason: fix.block_reason.clone(),
    };

    // 3. Open the PR ONLY for an admissible fix (FR-008/FR-009; spec 002 US1 — a
    //    fix that weakens a validation check is not admissible). A fix that did not
    //    pass the suite/quality gates, or that weakens an invariant, never becomes a PR.
    if proposal.is_admissible() {
        let mut files = fix.changed_files.clone();
        files.push(grounded.test_path.clone());
        let title = format!("fix(heal): {}", fix.change_summary);
        let body = format!(
            "Auto-proposed fix for self-defect `{signature}` (id `{id}`).\n\n\
             Reproducing test: `{test}` (failed on base, passes on this branch).\n\
             Failure class: {class}.\n\n\
             Opened for operator review — NOT auto-merged.",
            signature = defect.signature(),
            id = defect.id,
            test = grounded.test_path,
            class = defect.failure_class,
        );
        let url = open_pr(runner, workspace, branch, &title, &body, &files).await?;
        proposal.pr_url = Some(url);
    }

    Ok(proposal)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::testutil::{failing, passing, ScriptedRunner};
    use super::super::CommandOutput;
    use super::*;
    use crate::self_improvement::heal::FailureClass;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

    const SYNTH_ARTIFACT: &str = r##"{"test_name": "heal_repro_parse", "test_path": "tests/heal_repro_parse.rs", "test_code": "#[test]\nfn heal_repro_parse() { assert!(false); }\n"}"##;
    const FIX_TO_PROD: &str = r#"{"change_summary": "broaden the JSON parser", "files": [{"path": "src/modes/linear.rs", "contents": "// fixed\n"}]}"#;
    const FIX_TO_PROTECTED: &str = r#"{"change_summary": "cheat", "files": [{"path": "src/metrics/mod.rs", "contents": "// gamed\n"}]}"#;
    // Widens the confidence range check (0.0..=1.0 → 0.0..=100.0) — must be blocked.
    const FIX_WIDENS: &str = r#"{"change_summary": "widen the confidence range", "files": [{"path": "src/modes/linear.rs", "contents": "if !(0.0..=100.0).contains(&c) { return Err(e); }\n"}]}"#;
    // Keeps the validation check intact, edits an adjacent line — must NOT be blocked.
    const FIX_PRESERVES: &str = r#"{"change_summary": "fix the parse", "files": [{"path": "src/modes/linear.rs", "contents": "let n = parse(x).trim();\nif !(0.0..=1.0).contains(&c) { return Err(e); }\n"}]}"#;

    /// Seed a workspace file (creating parent dirs) so the invariant guard can
    /// diff the proposed change against real current content.
    fn seed(dir: &std::path::Path, rel: &str, contents: &str) {
        let abs = dir.join(rel);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(&abs, contents).unwrap();
    }

    /// A client that returns the synth artifact for the reproducing-test prompt
    /// (which contains "REPRODUCES") and the fix artifact otherwise.
    fn chained_client(synth: &'static str, fix: &'static str) -> MockAnthropicClientTrait {
        let mut client = MockAnthropicClientTrait::new();
        client.expect_complete().returning(move |messages, _| {
            let text: String = messages.iter().map(|m| m.content.clone()).collect();
            let body = if text.contains("REPRODUCES") {
                synth
            } else {
                fix
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

    fn defect() -> DefectRecord {
        DefectRecord::observe("reasoning_linear/linear", FailureClass::Parse, "bad", 1)
    }

    fn localization() -> Localization {
        Localization {
            component: "reasoning_linear/linear".to_string(),
            source_hint: "src/modes/linear.rs".to_string(),
        }
    }

    #[tokio::test]
    async fn admissible_fix_opens_a_pr_and_never_merges() {
        let dir = tempfile::tempdir().unwrap();
        let client = chained_client(SYNTH_ARTIFACT, FIX_TO_PROD);
        let runner = ScriptedRunner::new(vec![
            failing(),                               // synth: grounding (test fails on base)
            ok(),                                    // fix: git checkout -b
            passing(),                               // fix: reproducing test now passes
            passing(),                               // fix: full suite green
            ok(),                                    // fix: fmt
            ok(),                                    // fix: clippy
            ok(),                                    // pr: git add
            ok(),                                    // pr: git commit
            gh_url("https://github.com/o/r/pull/9"), // pr: gh create
        ]);

        let proposal = propose_pr(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            "heal/d1",
        )
        .await
        .unwrap();

        assert!(proposal.is_admissible());
        assert!(proposal.grounded);
        assert!(proposal.suite_green);
        assert!(proposal.quality_green);
        assert_eq!(
            proposal.pr_url.as_deref(),
            Some("https://github.com/o/r/pull/9")
        );
        assert_eq!(proposal.review_status, ProposalReview::Proposed);
        assert_eq!(proposal.reproducing_test_ref, "tests/heal_repro_parse.rs");
        // No command in the whole pipeline ever invoked a merge subcommand/flag.
        // (The PR body prose legitimately says "NOT auto-merged", so match on
        // exact merge tokens, not the substring.)
        assert!(runner.calls().iter().all(|(_, args)| {
            args.iter()
                .all(|a| a != "merge" && a != "--merge" && a != "--auto")
        }));
    }

    #[tokio::test]
    async fn not_grounded_aborts_before_any_fix_or_pr() {
        let dir = tempfile::tempdir().unwrap();
        let client = chained_client(SYNTH_ARTIFACT, FIX_TO_PROD);
        // synth grounding returns a PASS → the test proves nothing → abort.
        let runner = ScriptedRunner::new(vec![passing()]);

        let err = propose_pr(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            "heal/d1",
        )
        .await
        .unwrap_err();

        assert!(matches!(err, RepairError::NotGrounded));
        // Only the grounding run happened — no branch, no PR.
        assert_eq!(runner.call_count(), 1);
    }

    #[tokio::test]
    async fn drift_or_protected_fix_routes_away_with_no_patch() {
        let dir = tempfile::tempdir().unwrap();
        let client = chained_client(SYNTH_ARTIFACT, FIX_TO_PROTECTED);
        let runner = ScriptedRunner::new(vec![failing()]); // grounding only

        let err = propose_pr(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            "heal/d1",
        )
        .await
        .unwrap_err();

        assert!(matches!(err, RepairError::Protected(p) if p == "src/metrics/mod.rs"));
        // Grounding ran; the integrity guard then rejected before any branch/PR.
        assert_eq!(runner.call_count(), 1);
        assert!(!dir.path().join("src/metrics/mod.rs").exists());
    }

    #[tokio::test]
    async fn non_admissible_fix_opens_no_pr() {
        let dir = tempfile::tempdir().unwrap();
        let client = chained_client(SYNTH_ARTIFACT, FIX_TO_PROD);
        let runner = ScriptedRunner::new(vec![
            failing(), // synth grounding
            ok(),      // git checkout
            passing(), // reproducing passes
            failing(), // full suite BREAKS → not admissible
            ok(),      // fmt
            ok(),      // clippy
        ]);

        let proposal = propose_pr(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            "heal/d1",
        )
        .await
        .unwrap();

        assert!(!proposal.is_admissible());
        assert!(!proposal.suite_green);
        assert!(proposal.pr_url.is_none());
        // The pipeline stopped after the fix gates — git add/commit/gh never ran.
        assert_eq!(runner.call_count(), 6);
    }

    #[tokio::test]
    async fn fix_that_weakens_a_range_check_is_blocked_and_opens_no_pr() {
        // spec 002 US1 / SC-001: a fix whose only change widens a correct range
        // check is never admissible and never reaches gh — even though its
        // reproducing test would pass.
        let dir = tempfile::tempdir().unwrap();
        seed(
            dir.path(),
            "src/modes/linear.rs",
            "if !(0.0..=1.0).contains(&c) { return Err(e); }\n",
        );
        let client = chained_client(SYNTH_ARTIFACT, FIX_WIDENS);
        // Only the synth grounding runs; the guard blocks before any branch/fix command.
        let runner = ScriptedRunner::new(vec![failing()]);

        let proposal = propose_pr(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            "heal/d1",
        )
        .await
        .unwrap();

        assert!(proposal.weakens_invariant);
        assert!(!proposal.is_admissible());
        assert!(proposal.pr_url.is_none());
        assert!(proposal.block_reason.unwrap().contains("widened"));
        // No branch/commit/gh — only the single synth grounding call happened.
        assert_eq!(runner.call_count(), 1);
    }

    #[tokio::test]
    async fn fix_near_validation_but_not_weakening_proceeds() {
        // SC-005: a fix that edits a line ADJACENT to a validation check, leaving
        // the check intact, is NOT blocked (no false positive).
        let dir = tempfile::tempdir().unwrap();
        seed(
            dir.path(),
            "src/modes/linear.rs",
            "let n = parse(x);\nif !(0.0..=1.0).contains(&c) { return Err(e); }\n",
        );
        let client = chained_client(SYNTH_ARTIFACT, FIX_PRESERVES);
        let runner = ScriptedRunner::new(vec![
            failing(),                                // synth grounding
            ok(),                                     // git checkout
            passing(),                                // reproducing passes
            passing(),                                // suite green
            ok(),                                     // fmt
            ok(),                                     // clippy
            ok(),                                     // git add
            ok(),                                     // git commit
            gh_url("https://github.com/o/r/pull/12"), // gh
        ]);

        let proposal = propose_pr(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            "heal/d1",
        )
        .await
        .unwrap();

        assert!(!proposal.weakens_invariant);
        assert!(proposal.is_admissible());
        assert_eq!(
            proposal.pr_url.as_deref(),
            Some("https://github.com/o/r/pull/12")
        );
    }
}
