//! Fix generation on a branch + validation (spec 001, T022, FR-008).
//!
//! Given a grounded reproducing test, ask the model for a fix touching only the
//! diagnosed component, apply it on a fresh branch, and validate: the reproducing
//! test must now pass, the full suite must be green, and fmt/clippy must pass. The
//! integrity guard (D6/FR-010) rejects any fix that touches the protected
//! acceptance/measurement surface, so a fix can never game its own success signal.
//!
//! Validation results are returned as flags — the admissibility gate
//! (`FixProposal::is_admissible`) and the operator, not this function, decide
//! whether a proposal proceeds. Only a protected-path violation is a hard error.

use std::path::Path;

use crate::self_improvement::analyzer::Localization;
use crate::self_improvement::heal::{is_protected, DefectRecord};
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message};

use super::{
    extract_json_object, required_str, run_checked, validate_rs_path, write_file, CommandRunner,
    GroundedTest, RepairError,
};

/// A fix applied to a branch, with its validation verdicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFix {
    /// The branch the fix was committed-in-progress on.
    pub branch: String,
    /// Workspace-relative paths the fix modified (never a protected path).
    pub changed_files: Vec<String>,
    /// Human summary of the change.
    pub change_summary: String,
    /// The reproducing test passes after the fix.
    pub reproducing_passes: bool,
    /// The full `cargo test` suite is green with the fix.
    pub suite_green: bool,
    /// `cargo fmt --check` and `cargo clippy -D warnings` both pass.
    pub quality_green: bool,
}

/// Generate a fix for `defect`, apply it on `branch`, and validate it (FR-008).
///
/// # Errors
/// - [`RepairError::Llm`] / [`RepairError::Parse`] for a failed or malformed fix.
/// - [`RepairError::UnsafePath`] for a path that escapes the workspace.
/// - [`RepairError::Protected`] if the fix touches the protected surface (D6) —
///   a hard stop, applied **before** any file is written.
/// - [`RepairError::Command`] if the branch cannot be created.
pub async fn generate_and_validate_fix<C, R>(
    client: &C,
    runner: &R,
    workspace: &Path,
    defect: &DefectRecord,
    localization: &Localization,
    grounded: &GroundedTest,
    branch: &str,
) -> Result<GeneratedFix, RepairError>
where
    C: AnthropicClientTrait,
    R: CommandRunner,
{
    let prompt = build_fix_prompt(defect, localization, grounded);
    let config = CompletionConfig::new()
        .with_max_tokens(4000)
        .with_temperature(0.0);
    let response = client
        .complete(vec![Message::user(&prompt)], config)
        .await?;

    let parsed = extract_json_object(&response.content)?;
    let change_summary = required_str(&parsed, "change_summary")?;
    let files = parse_fix_files(&parsed)?;

    // Integrity guard FIRST (D6): refuse — before writing anything — a fix that
    // would touch the acceptance/measurement surface.
    for (rel, _) in &files {
        if is_protected(rel) {
            return Err(RepairError::Protected(rel.clone()));
        }
    }

    // Create the branch; a non-zero exit (e.g. branch already exists) is a hard
    // failure — we will not write a fix onto an unknown ref.
    run_checked(
        runner,
        workspace,
        "git",
        &["checkout".to_string(), "-b".to_string(), branch.to_string()],
    )
    .await?;

    for (rel, contents) in &files {
        write_file(&workspace.join(rel), contents).await?;
    }
    let changed_files: Vec<String> = files.iter().map(|(rel, _)| rel.clone()).collect();

    // The reproducing test must now pass; if not, the fix did not work — return
    // the verdicts without running the (now-pointless) full suite/quality gates.
    let repro = runner
        .run(
            "cargo",
            &["test".to_string(), grounded.test_name.clone()],
            workspace,
        )
        .await?;
    if !repro.success() {
        return Ok(GeneratedFix {
            branch: branch.to_string(),
            changed_files,
            change_summary,
            reproducing_passes: false,
            suite_green: false,
            quality_green: false,
        });
    }

    let suite = runner
        .run("cargo", &["test".to_string()], workspace)
        .await?;
    let fmt = runner
        .run(
            "cargo",
            &["fmt".to_string(), "--check".to_string()],
            workspace,
        )
        .await?;
    let clippy = runner
        .run(
            "cargo",
            &[
                "clippy".to_string(),
                "--".to_string(),
                "-D".to_string(),
                "warnings".to_string(),
            ],
            workspace,
        )
        .await?;

    Ok(GeneratedFix {
        branch: branch.to_string(),
        changed_files,
        change_summary,
        reproducing_passes: true,
        suite_green: suite.success(),
        quality_green: fmt.success() && clippy.success(),
    })
}

fn build_fix_prompt(
    defect: &DefectRecord,
    localization: &Localization,
    grounded: &GroundedTest,
) -> String {
    format!(
        r#"You are fixing a recurring self-defect in an internal Rust reasoning server.

A reproducing test already FAILS on the current code. Produce the minimal fix that makes it pass, touching ONLY the diagnosed component.

## Defect
- Component (tool/mode): {component}
- Failure class: {class}
- Source hint: {hint}

## Reproducing test ({test_path})
{test_code}

## Hard constraints
- Touch ONLY production source for the diagnosed component.
- NEVER modify tests, src/metrics, src/eval, sensor.rs, circuit_breaker.rs, or allowlist.rs.
- No `.unwrap()`/`.expect()` in production paths; keep files under 500 lines.

## Output
Return ONLY a JSON object, no prose:
{{"change_summary": "<one sentence>", "files": [{{"path": "<workspace-relative .rs file>", "contents": "<full new file source>"}}]}}"#,
        component = defect.component,
        class = defect.failure_class,
        hint = localization.source_hint,
        test_path = grounded.test_path,
        test_code = grounded.test_code,
    )
}

/// Parse and path-validate the `files` array of the fix artifact.
fn parse_fix_files(parsed: &serde_json::Value) -> Result<Vec<(String, String)>, RepairError> {
    let array = parsed["files"]
        .as_array()
        .ok_or_else(|| RepairError::Parse("fix missing 'files' array".to_string()))?;
    if array.is_empty() {
        return Err(RepairError::Parse("fix touched no files".to_string()));
    }

    let mut files = Vec::with_capacity(array.len());
    for entry in array {
        let path = required_str(entry, "path")?;
        let contents = required_str(entry, "contents")?;
        // Reject escapes / non-.rs before the file is ever considered for writing.
        validate_rs_path(&path)?;
        files.push((path, contents));
    }
    Ok(files)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::testutil::{failing, passing, ScriptedRunner};
    use super::super::CommandOutput;
    use super::*;
    use crate::self_improvement::heal::FailureClass;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

    fn mock_client(content: &'static str) -> MockAnthropicClientTrait {
        let mut client = MockAnthropicClientTrait::new();
        client
            .expect_complete()
            .returning(move |_, _| Ok(CompletionResponse::new(content, Usage::new(10, 10))));
        client
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

    fn grounded() -> GroundedTest {
        GroundedTest {
            test_name: "heal_repro_parse".to_string(),
            test_path: "tests/heal_repro_parse.rs".to_string(),
            test_code: "#[test] fn heal_repro_parse() {}".to_string(),
            grounded: true,
        }
    }

    fn ok_output() -> CommandOutput {
        CommandOutput {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    const FIX_TO_PROD: &str = r#"{"change_summary": "broaden the JSON parser", "files": [{"path": "src/modes/linear.rs", "contents": "// fixed\n"}]}"#;

    #[tokio::test]
    async fn green_fix_passes_all_gates() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(FIX_TO_PROD);
        // git checkout, cargo test <name>, cargo test, cargo fmt, cargo clippy.
        let runner = ScriptedRunner::new(vec![
            ok_output(), // git checkout -b
            passing(),   // reproducing test passes
            passing(),   // full suite
            ok_output(), // fmt --check
            ok_output(), // clippy
        ]);

        let fix = generate_and_validate_fix(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            &grounded(),
            "heal/d1",
        )
        .await
        .unwrap();

        assert!(fix.reproducing_passes);
        assert!(fix.suite_green);
        assert!(fix.quality_green);
        assert_eq!(fix.changed_files, vec!["src/modes/linear.rs".to_string()]);
        // The branch was created, then the reproducing test was run by name.
        let calls = runner.calls();
        assert_eq!(calls[0].0, "git");
        assert_eq!(calls[0].1, vec!["checkout", "-b", "heal/d1"]);
        assert_eq!(calls[1].1, vec!["test", "heal_repro_parse"]);
        assert!(dir.path().join("src/modes/linear.rs").exists());
    }

    #[tokio::test]
    async fn integrity_guard_rejects_fix_touching_protected_path() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(
            r#"{"change_summary": "cheat", "files": [{"path": "src/metrics/mod.rs", "contents": "// gamed\n"}]}"#,
        );
        let runner = ScriptedRunner::new(vec![]);

        let err = generate_and_validate_fix(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            &grounded(),
            "heal/d1",
        )
        .await
        .unwrap_err();

        assert!(matches!(err, RepairError::Protected(p) if p == "src/metrics/mod.rs"));
        // Rejected before any command ran or any file was written.
        assert_eq!(runner.call_count(), 0);
        assert!(!dir.path().join("src/metrics/mod.rs").exists());
    }

    #[tokio::test]
    async fn fix_that_does_not_fix_reports_not_green_without_running_quality() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(FIX_TO_PROD);
        let runner = ScriptedRunner::new(vec![
            ok_output(), // git checkout -b
            failing(),   // reproducing test still fails
        ]);

        let fix = generate_and_validate_fix(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            &grounded(),
            "heal/d1",
        )
        .await
        .unwrap();

        assert!(!fix.reproducing_passes);
        assert!(!fix.suite_green);
        assert!(!fix.quality_green);
        // Stopped after the reproducing run — no full suite / fmt / clippy.
        assert_eq!(runner.call_count(), 2);
    }

    #[tokio::test]
    async fn suite_or_quality_failure_is_recorded_as_not_green() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(FIX_TO_PROD);
        let runner = ScriptedRunner::new(vec![
            ok_output(), // git checkout -b
            passing(),   // reproducing passes
            failing(),   // full suite breaks
            ok_output(), // fmt
            CommandOutput {
                status: 101,
                stdout: String::new(),
                stderr: "clippy error".to_string(),
            }, // clippy fails
        ]);

        let fix = generate_and_validate_fix(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            &grounded(),
            "heal/d1",
        )
        .await
        .unwrap();

        assert!(fix.reproducing_passes);
        assert!(!fix.suite_green);
        assert!(!fix.quality_green);
    }

    #[tokio::test]
    async fn branch_creation_failure_is_a_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(FIX_TO_PROD);
        let runner = ScriptedRunner::new(vec![CommandOutput {
            status: 128,
            stdout: String::new(),
            stderr: "fatal: a branch named 'heal/d1' already exists".to_string(),
        }]);

        let err = generate_and_validate_fix(
            &client,
            &runner,
            dir.path(),
            &defect(),
            &localization(),
            &grounded(),
            "heal/d1",
        )
        .await
        .unwrap_err();

        assert!(matches!(err, RepairError::Command { .. }));
        // The branch failed, so no fix file was written.
        assert!(!dir.path().join("src/modes/linear.rs").exists());
    }
}
