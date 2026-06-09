//! Open the proposal as a reviewable PR (spec 001, T023, D5/FR-007).
//!
//! Stage exactly the proposal's files (the fix + the reproducing test), commit on
//! the proposal branch, and open a PR via `gh` for operator review. This step
//! **never merges and never edits anything** — it only publishes the branch for a
//! human to approve. The `gh` argument vector comes from
//! [`crate::self_improvement::heal::pr_create_args`], whose test asserts no
//! auto-merge flag is ever present.

use std::path::Path;

use crate::self_improvement::heal::pr_create_args;

use super::{run_checked, CommandRunner, RepairError};

/// Stage `files`, commit them on `branch`, and open a PR; return the PR URL.
///
/// `files` are the workspace-relative paths the proposal introduced (the fix
/// files and the reproducing test) — only these are staged, never `git add -A`.
///
/// # Errors
/// - [`RepairError::Parse`] if `files` is empty (nothing to propose).
/// - [`RepairError::Command`] if `git add`/`git commit`/`gh pr create` fail.
/// - [`RepairError::Command`] if `gh` returns no URL.
pub async fn open_pr<R: CommandRunner>(
    runner: &R,
    workspace: &Path,
    branch: &str,
    title: &str,
    body: &str,
    files: &[String],
) -> Result<String, RepairError> {
    if files.is_empty() {
        return Err(RepairError::Parse(
            "cannot open a PR with no files to commit".to_string(),
        ));
    }

    // Stage exactly the proposal's files — nothing else in the tree.
    let mut add_args = vec!["add".to_string(), "--".to_string()];
    add_args.extend(files.iter().cloned());
    run_checked(runner, workspace, "git", &add_args).await?;

    // Commit on the proposal branch.
    run_checked(
        runner,
        workspace,
        "git",
        &["commit".to_string(), "-m".to_string(), title.to_string()],
    )
    .await?;

    // Open the PR for review. `pr_create_args` never includes a merge flag
    // (FR-007); `gh` prints the new PR URL on stdout.
    let pr_args = pr_create_args(branch, title, body);
    let out = run_checked(runner, workspace, "gh", &pr_args).await?;

    let url = out.stdout.trim().to_string();
    if url.is_empty() {
        return Err(RepairError::Command {
            program: "gh".to_string(),
            message: "pr create returned no URL".to_string(),
        });
    }
    Ok(url)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::testutil::ScriptedRunner;
    use super::super::CommandOutput;
    use super::*;

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

    fn files() -> Vec<String> {
        vec![
            "src/modes/linear.rs".to_string(),
            "tests/heal_repro_parse.rs".to_string(),
        ]
    }

    #[tokio::test]
    async fn opens_pr_and_returns_url_without_merging() {
        let dir = tempfile::tempdir().unwrap();
        let runner = ScriptedRunner::new(vec![
            ok(),                                    // git add
            ok(),                                    // git commit
            gh_url("https://github.com/o/r/pull/7"), // gh pr create
        ]);

        let url = open_pr(
            &runner,
            dir.path(),
            "heal/d1",
            "fix: tolerate trailing prose",
            "Closes a recurring parse defect.",
            &files(),
        )
        .await
        .unwrap();

        assert_eq!(url, "https://github.com/o/r/pull/7");

        let calls = runner.calls();
        // Staged exactly the proposal files (no `git add -A`).
        assert_eq!(calls[0].0, "git");
        assert_eq!(calls[0].1[0], "add");
        assert!(calls[0].1.contains(&"src/modes/linear.rs".to_string()));
        assert!(calls[0]
            .1
            .contains(&"tests/heal_repro_parse.rs".to_string()));
        assert_eq!(calls[1].1[0], "commit");
        // The PR is opened, and NOTHING in the gh invocation requests a merge.
        assert_eq!(calls[2].0, "gh");
        assert!(calls[2].1.iter().all(|a| !a.contains("merge")));
    }

    #[tokio::test]
    async fn errors_when_gh_returns_no_url() {
        let dir = tempfile::tempdir().unwrap();
        let runner = ScriptedRunner::new(vec![ok(), ok(), ok()]); // gh stdout empty

        let err = open_pr(&runner, dir.path(), "heal/d1", "fix", "body", &files())
            .await
            .unwrap_err();

        assert!(matches!(err, RepairError::Command { .. }));
    }

    #[tokio::test]
    async fn commit_failure_aborts_before_opening_pr() {
        let dir = tempfile::tempdir().unwrap();
        let runner = ScriptedRunner::new(vec![
            ok(), // git add
            CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: "nothing to commit".to_string(),
            }, // git commit fails
        ]);

        let err = open_pr(&runner, dir.path(), "heal/d1", "fix", "body", &files())
            .await
            .unwrap_err();

        assert!(matches!(err, RepairError::Command { .. }));
        // gh was never reached.
        assert_eq!(runner.call_count(), 2);
    }

    #[tokio::test]
    async fn refuses_empty_file_set() {
        let dir = tempfile::tempdir().unwrap();
        let runner = ScriptedRunner::new(vec![]);

        let err = open_pr(&runner, dir.path(), "heal/d1", "fix", "body", &[])
            .await
            .unwrap_err();

        assert!(matches!(err, RepairError::Parse(_)));
        assert_eq!(runner.call_count(), 0);
    }
}
