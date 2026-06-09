//! Reproducing-test synthesis with execution grounding (spec 001, T021, D4/FR-006).
//!
//! Generate a test that reproduces a recurring defect, write it into the
//! workspace, run it on the **unpatched** tree, and REQUIRE it to fail. A test
//! that passes (or never compiles/runs) proves nothing and aborts the proposal —
//! this is the guard against APR patch-overfitting (Constitution III).

use std::path::Path;

use crate::self_improvement::analyzer::Localization;
use crate::self_improvement::heal::DefectRecord;
use crate::traits::{AnthropicClientTrait, CompletionConfig, Message};

use super::{
    extract_json_object, remove_file, required_str, validate_rs_path, write_file, CommandOutput,
    CommandRunner, RepairError,
};

/// A reproducing test that has been execution-grounded: it demonstrably failed on
/// the unpatched tree (`grounded == true` on every value returned by
/// [`synthesize_reproducing_test`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundedTest {
    /// The test function name (used to filter `cargo test`).
    pub test_name: String,
    /// Workspace-relative path the test was written to.
    pub test_path: String,
    /// The test source.
    pub test_code: String,
    /// Always true: failure on the unpatched tree was verified (D4).
    pub grounded: bool,
}

/// Synthesize a reproducing test for `defect` (focused by `localization`), write
/// it under `workspace`, run it via `runner`, and require it to fail on the
/// unpatched tree (D4/FR-006).
///
/// Returns the grounded test on success, leaving the file in place for the fix +
/// PR steps. On any non-grounded outcome the written file is removed and an error
/// is returned — no proposal proceeds.
///
/// # Errors
/// - [`RepairError::Parse`] / [`RepairError::UnsafePath`] for a malformed or unsafe
///   synthesized artifact.
/// - [`RepairError::NotGrounded`] if the test passed on the unpatched tree.
/// - [`RepairError::TestDidNotRun`] if it failed to compile/run (not a real repro).
pub async fn synthesize_reproducing_test<C, R>(
    client: &C,
    runner: &R,
    workspace: &Path,
    defect: &DefectRecord,
    localization: &Localization,
) -> Result<GroundedTest, RepairError>
where
    C: AnthropicClientTrait,
    R: CommandRunner,
{
    let prompt = build_prompt(defect, localization);
    let config = CompletionConfig::new()
        .with_max_tokens(1500)
        .with_temperature(0.0);
    let response = client
        .complete(vec![Message::user(&prompt)], config)
        .await?;

    let parsed = extract_json_object(&response.content)?;
    let test_name = required_str(&parsed, "test_name")?;
    let test_code = required_str(&parsed, "test_code")?;
    let rel_path = required_str(&parsed, "test_path")?;

    let rel = validate_rs_path(&rel_path)?;
    let abs = workspace.join(&rel);

    write_file(&abs, &test_code).await?;

    let out = match runner
        .run("cargo", &["test".to_string(), test_name.clone()], workspace)
        .await
    {
        Ok(o) => o,
        Err(e) => {
            let _ = remove_file(&abs).await;
            return Err(e);
        }
    };

    if let Err(e) = interpret_grounding(&out) {
        let _ = remove_file(&abs).await;
        return Err(e);
    }

    Ok(GroundedTest {
        test_name,
        test_path: rel_path,
        test_code,
        grounded: true,
    })
}

fn build_prompt(defect: &DefectRecord, localization: &Localization) -> String {
    format!(
        r#"You are writing a Rust test that REPRODUCES a recurring self-defect in an internal reasoning server, so a fix can be proven.

## Defect
- Component (tool/mode): {component}
- Failure class: {class}
- Occurrences: {occurrences}
- Source hint: {hint}
- Redacted input excerpt: {excerpt}

## Requirements
- The test MUST fail on the CURRENT (unpatched) code — it encodes the bug as an assertion that currently does not hold.
- It must compile against the existing crate and be self-contained.
- Do NOT modify production behavior; only assert the expected, correct output.

## Output
Return ONLY a JSON object, no prose:
{{"test_name": "<unique snake_case fn name>", "test_path": "<workspace-relative .rs path, e.g. tests/heal_repro_<name>.rs>", "test_code": "<full Rust source for the test file>"}}"#,
        component = defect.component,
        class = defect.failure_class,
        occurrences = defect.occurrences,
        hint = localization.source_hint,
        excerpt = defect.excerpt,
    )
}

/// Interpret a `cargo test` run as a grounding verdict (D4):
/// - exit 0 → the test passed on the unpatched tree → NOT grounded (abort).
/// - failed with a `test result: FAILED` summary → a genuine reproduction.
/// - failed without that summary → it did not compile/run; failure is not a repro.
fn interpret_grounding(out: &CommandOutput) -> Result<(), RepairError> {
    if out.success() {
        return Err(RepairError::NotGrounded);
    }
    if out.combined().contains("test result: FAILED") {
        Ok(())
    } else {
        Err(RepairError::TestDidNotRun(
            "no `test result: FAILED` summary — likely a compile error".to_string(),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::testutil::{
        failing as failing_output, passing as passing_output, ScriptedRunner,
    };
    use super::*;
    use crate::self_improvement::heal::FailureClass;
    use crate::traits::{CompletionResponse, MockAnthropicClientTrait, Usage};

    fn compile_error_output() -> CommandOutput {
        CommandOutput {
            status: 101,
            stdout: String::new(),
            stderr: "error[E0432]: unresolved import `crate::nope`".to_string(),
        }
    }

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

    const VALID_ARTIFACT: &str = r##"{"test_name": "heal_repro_parse", "test_path": "tests/heal_repro_parse.rs", "test_code": "#[test]\nfn heal_repro_parse() { assert!(false); }\n"}"##;

    #[tokio::test]
    async fn grounds_when_test_fails_on_unpatched_tree() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(VALID_ARTIFACT);
        let runner = ScriptedRunner::new(vec![failing_output()]);

        let result =
            synthesize_reproducing_test(&client, &runner, dir.path(), &defect(), &localization())
                .await
                .unwrap();

        assert!(result.grounded);
        assert_eq!(result.test_name, "heal_repro_parse");
        assert_eq!(result.test_path, "tests/heal_repro_parse.rs");
        // The file remains in place for the fix + PR steps.
        assert!(dir.path().join("tests/heal_repro_parse.rs").exists());
        // `cargo test <name>` was the grounding command.
        assert_eq!(runner.call_count(), 1);
    }

    #[tokio::test]
    async fn aborts_not_grounded_when_test_passes_on_unpatched_tree() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(VALID_ARTIFACT);
        let runner = ScriptedRunner::new(vec![passing_output()]);

        let err =
            synthesize_reproducing_test(&client, &runner, dir.path(), &defect(), &localization())
                .await
                .unwrap_err();

        assert!(matches!(err, RepairError::NotGrounded));
        // The non-grounded file is cleaned up.
        assert!(!dir.path().join("tests/heal_repro_parse.rs").exists());
    }

    #[tokio::test]
    async fn errors_when_reproducing_test_does_not_compile() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(VALID_ARTIFACT);
        let runner = ScriptedRunner::new(vec![compile_error_output()]);

        let err =
            synthesize_reproducing_test(&client, &runner, dir.path(), &defect(), &localization())
                .await
                .unwrap_err();

        assert!(matches!(err, RepairError::TestDidNotRun(_)));
        assert!(!dir.path().join("tests/heal_repro_parse.rs").exists());
    }

    #[tokio::test]
    async fn rejects_path_that_escapes_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(
            r#"{"test_name": "x", "test_path": "../evil.rs", "test_code": "fn x() {}"}"#,
        );
        let runner = ScriptedRunner::new(vec![]);

        let err =
            synthesize_reproducing_test(&client, &runner, dir.path(), &defect(), &localization())
                .await
                .unwrap_err();

        assert!(matches!(err, RepairError::UnsafePath(_)));
        // No command ran and nothing was written.
        assert_eq!(runner.call_count(), 0);
    }

    #[tokio::test]
    async fn errors_on_missing_test_code() {
        let dir = tempfile::tempdir().unwrap();
        let client = mock_client(r#"{"test_name": "x", "test_path": "tests/x.rs"}"#);
        let runner = ScriptedRunner::new(vec![]);

        let err =
            synthesize_reproducing_test(&client, &runner, dir.path(), &defect(), &localization())
                .await
                .unwrap_err();

        assert!(matches!(err, RepairError::Parse(_)));
        assert_eq!(runner.call_count(), 0);
    }
}
