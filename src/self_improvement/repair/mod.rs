//! Self-heal repair pipeline (feature `001-heal-parse-schema`, US2).
//!
//! Turns a recurring, localized defect into an operator-reviewable PR:
//! 1. [`test_synth`] synthesizes a reproducing test and **grounds** it — the test
//!    must fail on the unpatched tree (D4/FR-006), else the proposal aborts.
//! 2. [`fix_gen`] generates a fix on a branch and validates it (reproducing test
//!    passes + suite green + fmt/clippy green), guarded against protected paths.
//! 3. `gh pr create` for review — never merged (D5/FR-007).
//!
//! All external side effects (running `cargo`/`git`/`gh`) go through the
//! [`CommandRunner`] trait so the orchestration is unit-testable with a scripted
//! fake and tests never run real tooling or mutate the repository.

pub mod fix_gen;
pub mod orchestrate;
pub mod pr;
pub mod test_synth;

pub use fix_gen::{generate_and_validate_fix, GeneratedFix};
pub use orchestrate::propose_pr;
pub use pr::open_pr;
pub use test_synth::{synthesize_reproducing_test, GroundedTest};

use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;

use crate::error::ModeError;

/// Captured result of running an external command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// Process exit code (`-1` when terminated without one).
    pub status: i32,
    /// Captured stdout (lossy UTF-8).
    pub stdout: String,
    /// Captured stderr (lossy UTF-8).
    pub stderr: String,
}

impl CommandOutput {
    /// True when the process exited zero.
    #[must_use]
    pub fn success(&self) -> bool {
        self.status == 0
    }

    /// stdout and stderr joined, for log/heuristic scanning.
    #[must_use]
    pub fn combined(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

/// An injectable runner for external commands (`cargo`/`git`/`gh`).
///
/// The real impl shells out; tests supply a scripted fake so no real tooling runs
/// (Constitution III: the repair pipeline must be exercised without mutating the
/// repo).
#[async_trait]
pub trait CommandRunner: Send + Sync {
    /// Run `program args...` with working directory `cwd`, capturing output.
    async fn run(
        &self,
        program: &str,
        args: &[String],
        cwd: &Path,
    ) -> Result<CommandOutput, RepairError>;
}

/// The production [`CommandRunner`]: shells out via `tokio::process`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemCommandRunner;

#[async_trait]
impl CommandRunner for SystemCommandRunner {
    async fn run(
        &self,
        program: &str,
        args: &[String],
        cwd: &Path,
    ) -> Result<CommandOutput, RepairError> {
        let output = tokio::process::Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .await
            .map_err(|e| RepairError::Command {
                program: program.to_string(),
                message: e.to_string(),
            })?;

        Ok(CommandOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Errors from the repair pipeline.
#[derive(Debug)]
pub enum RepairError {
    /// The LLM call (test synthesis / fix generation) failed.
    Llm(ModeError),
    /// An external command could not be spawned or run.
    Command {
        /// The program that failed.
        program: String,
        /// Underlying message.
        message: String,
    },
    /// A filesystem operation on a workspace path failed.
    Io {
        /// The path involved.
        path: String,
        /// Underlying message.
        message: String,
    },
    /// The synthesized model output could not be parsed into a test artifact.
    Parse(String),
    /// A synthesized path was unsafe (escaped the workspace or not a `.rs` file).
    UnsafePath(String),
    /// A fix attempted to modify the protected acceptance/measurement surface
    /// (tests/metrics/eval/sensor/circuit_breaker/allowlist) — the integrity guard
    /// rejects it so a fix can never game its own success signal (D6/FR-010).
    Protected(String),
    /// The reproducing test did NOT fail on the unpatched tree, so it proves
    /// nothing and the proposal must abort (D4/FR-006).
    NotGrounded,
    /// The reproducing test failed to compile/run, so failure is not a genuine
    /// reproduction.
    TestDidNotRun(String),
}

impl std::fmt::Display for RepairError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Llm(e) => write!(f, "repair LLM step failed: {e}"),
            Self::Command { program, message } => {
                write!(f, "command '{program}' failed: {message}")
            }
            Self::Io { path, message } => write!(f, "io error on '{path}': {message}"),
            Self::Parse(m) => write!(f, "could not parse synthesized artifact: {m}"),
            Self::UnsafePath(p) => write!(f, "unsafe synthesized path: {p}"),
            Self::Protected(p) => {
                write!(f, "fix may not modify the protected path: {p}")
            }
            Self::NotGrounded => write!(
                f,
                "reproducing test passed on the unpatched tree; not grounded (aborting)"
            ),
            Self::TestDidNotRun(m) => {
                write!(f, "reproducing test did not compile/run: {m}")
            }
        }
    }
}

impl std::error::Error for RepairError {}

impl From<ModeError> for RepairError {
    fn from(e: ModeError) -> Self {
        Self::Llm(e)
    }
}

// ---------------------------------------------------------------------------
// Shared helpers (JSON extraction, path safety, fs) used by test_synth/fix_gen.
// ---------------------------------------------------------------------------

/// Extract the JSON object spanning the first `{` to the last `}` in `text`.
pub(super) fn extract_json_object(text: &str) -> Result<serde_json::Value, RepairError> {
    let start = text
        .find('{')
        .ok_or_else(|| RepairError::Parse("no JSON object in response".to_string()))?;
    let end = text
        .rfind('}')
        .ok_or_else(|| RepairError::Parse("no closing brace in response".to_string()))?;
    if end <= start {
        return Err(RepairError::Parse("malformed JSON span".to_string()));
    }
    serde_json::from_str(&text[start..=end]).map_err(|e| RepairError::Parse(e.to_string()))
}

/// Read a required, non-empty string field from a JSON object.
pub(super) fn required_str(value: &serde_json::Value, key: &str) -> Result<String, RepairError> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| RepairError::Parse(format!("missing or empty '{key}'")))
}

/// Reject any synthesized path that escapes the workspace or is not a `.rs` file.
/// Returns the workspace-relative path on success.
pub(super) fn validate_rs_path(rel: &str) -> Result<PathBuf, RepairError> {
    let path = Path::new(rel);
    if path.extension().and_then(|e| e.to_str()) != Some("rs") {
        return Err(RepairError::UnsafePath(format!(
            "{rel} (must be a .rs file)"
        )));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            _ => {
                return Err(RepairError::UnsafePath(format!(
                    "{rel} (must be workspace-relative, no '..' or absolute roots)"
                )));
            }
        }
    }
    Ok(path.to_path_buf())
}

/// Write `contents` to `abs`, creating parent directories as needed.
pub(super) async fn write_file(abs: &Path, contents: &str) -> Result<(), RepairError> {
    if let Some(parent) = abs.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| RepairError::Io {
                path: parent.display().to_string(),
                message: e.to_string(),
            })?;
    }
    tokio::fs::write(abs, contents)
        .await
        .map_err(|e| RepairError::Io {
            path: abs.display().to_string(),
            message: e.to_string(),
        })
}

/// Run `program args...` in `workspace` and require a zero exit, returning the
/// captured output. A non-zero exit becomes [`RepairError::Command`].
pub(super) async fn run_checked<R: CommandRunner>(
    runner: &R,
    workspace: &Path,
    program: &str,
    args: &[String],
) -> Result<CommandOutput, RepairError> {
    let out = runner.run(program, args, workspace).await?;
    if out.success() {
        Ok(out)
    } else {
        Err(RepairError::Command {
            program: program.to_string(),
            message: format!("exit {}: {}", out.status, out.stderr.trim()),
        })
    }
}

/// Remove `abs`, mapping io errors into [`RepairError::Io`].
pub(super) async fn remove_file(abs: &Path) -> Result<(), RepairError> {
    tokio::fs::remove_file(abs)
        .await
        .map_err(|e| RepairError::Io {
            path: abs.display().to_string(),
            message: e.to_string(),
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
pub(super) mod testutil {
    use super::{CommandOutput, CommandRunner, RepairError};
    use async_trait::async_trait;
    use std::collections::VecDeque;
    use std::path::Path;
    use std::sync::Mutex;

    /// A scripted [`CommandRunner`] — returns queued outputs and records the calls
    /// made. No real `cargo`/`git`/`gh` ever runs, so tests never mutate the repo.
    pub struct ScriptedRunner {
        outputs: Mutex<VecDeque<CommandOutput>>,
        calls: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl ScriptedRunner {
        pub(crate) fn new(outputs: Vec<CommandOutput>) -> Self {
            Self {
                outputs: Mutex::new(outputs.into()),
                calls: Mutex::new(Vec::new()),
            }
        }

        pub(crate) fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }

        pub(crate) fn calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl CommandRunner for ScriptedRunner {
        async fn run(
            &self,
            program: &str,
            args: &[String],
            _cwd: &Path,
        ) -> Result<CommandOutput, RepairError> {
            self.calls
                .lock()
                .unwrap()
                .push((program.to_string(), args.to_vec()));
            Ok(self
                .outputs
                .lock()
                .unwrap()
                .pop_front()
                .expect("a scripted output is available"))
        }
    }

    /// `cargo test`-style output: a passing summary at exit 0.
    pub fn passing() -> CommandOutput {
        CommandOutput {
            status: 0,
            stdout: "test result: ok. 1 passed; 0 failed".to_string(),
            stderr: String::new(),
        }
    }

    /// `cargo test`-style output: a genuine test failure summary at exit 101.
    pub fn failing() -> CommandOutput {
        CommandOutput {
            status: 101,
            stdout: "test result: FAILED. 0 passed; 1 failed; 0 ignored".to_string(),
            stderr: String::new(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::error::ModeError;

    #[test]
    fn command_output_success_and_combined() {
        let ok = CommandOutput {
            status: 0,
            stdout: "out".to_string(),
            stderr: "err".to_string(),
        };
        assert!(ok.success());
        let combined = ok.combined();
        assert!(combined.contains("out") && combined.contains("err"));

        let bad = CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        };
        assert!(!bad.success());
    }

    #[test]
    fn repair_error_display_covers_every_variant() {
        let variants = [
            RepairError::Command {
                program: "git".to_string(),
                message: "boom".to_string(),
            },
            RepairError::Io {
                path: "p".to_string(),
                message: "io".to_string(),
            },
            RepairError::Parse("bad".to_string()),
            RepairError::UnsafePath("../x".to_string()),
            RepairError::Protected("src/metrics/mod.rs".to_string()),
            RepairError::NotGrounded,
            RepairError::TestDidNotRun("compile".to_string()),
        ];
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }

    #[test]
    fn repair_error_from_mode_error_is_llm() {
        let e: RepairError = ModeError::JsonParseFailed {
            message: "x".to_string(),
        }
        .into();
        assert!(matches!(e, RepairError::Llm(_)));
        assert!(e.to_string().contains("LLM"));
    }

    #[test]
    fn json_and_path_helper_error_branches() {
        // Closing brace before opening, and no object at all.
        assert!(matches!(
            extract_json_object("} oops {").unwrap_err(),
            RepairError::Parse(_)
        ));
        assert!(matches!(
            extract_json_object("no json here").unwrap_err(),
            RepairError::Parse(_)
        ));
        // Non-.rs and workspace-escaping paths.
        assert!(matches!(
            validate_rs_path("foo.txt").unwrap_err(),
            RepairError::UnsafePath(_)
        ));
        assert!(matches!(
            validate_rs_path("../x.rs").unwrap_err(),
            RepairError::UnsafePath(_)
        ));
        // Missing / empty required field.
        let v = serde_json::json!({"a": ""});
        assert!(matches!(
            required_str(&v, "a").unwrap_err(),
            RepairError::Parse(_)
        ));
        assert!(matches!(
            required_str(&v, "missing").unwrap_err(),
            RepairError::Parse(_)
        ));
    }

    #[tokio::test]
    async fn fs_helpers_map_io_errors() {
        let dir = tempfile::tempdir().unwrap();

        // create_dir_all fails when an ancestor is a regular file.
        let file = dir.path().join("a_file");
        tokio::fs::write(&file, "x").await.unwrap();
        let under_file = file.join("child.rs");
        assert!(matches!(
            write_file(&under_file, "y").await.unwrap_err(),
            RepairError::Io { .. }
        ));

        // Writing to a directory path itself fails.
        assert!(matches!(
            write_file(dir.path(), "x").await.unwrap_err(),
            RepairError::Io { .. }
        ));

        // Removing a non-existent file fails.
        assert!(matches!(
            remove_file(&dir.path().join("nope.rs")).await.unwrap_err(),
            RepairError::Io { .. }
        ));
    }

    #[tokio::test]
    async fn system_runner_captures_success_and_spawn_failure() {
        let runner = SystemCommandRunner;
        let cwd = Path::new(".");

        // A real, read-only command guaranteed present wherever cargo builds.
        let out = runner
            .run(env!("CARGO"), &["--version".to_string()], cwd)
            .await
            .unwrap();
        assert!(out.success());
        assert!(out.stdout.to_lowercase().contains("cargo"));

        // A non-existent program cannot spawn → Command error.
        let err = runner
            .run("definitely-not-a-real-binary-zzz", &[], cwd)
            .await
            .unwrap_err();
        assert!(matches!(err, RepairError::Command { .. }));
    }
}
