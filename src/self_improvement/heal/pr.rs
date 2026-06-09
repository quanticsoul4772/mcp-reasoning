//! GitHub PR creation for the self-heal repair action (spec 001, T002/T023).
//!
//! Proposals are opened as operator-reviewed pull requests and are **never**
//! auto-merged (FR-007). This module builds the deterministic `gh` invocation and
//! checks availability; actually opening a PR shells out to `gh` and is exercised
//! in integration (not unit) tests, since the unit-testable property here is that
//! the command never requests a merge.

use std::process::Command;

/// True if the `gh` CLI is available on PATH.
#[must_use]
pub fn gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Build the `gh pr create` argument vector for a heal proposal.
///
/// Opens the PR for operator review only — it MUST NOT include any auto-merge
/// flag (FR-007). Callers run this against the per-task branch.
#[must_use]
pub fn pr_create_args(branch: &str, title: &str, body: &str) -> Vec<String> {
    vec![
        "pr".to_string(),
        "create".to_string(),
        "--head".to_string(),
        branch.to_string(),
        "--title".to_string(),
        title.to_string(),
        "--body".to_string(),
        body.to_string(),
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn args_open_pr_for_review_without_auto_merge() {
        let args = pr_create_args("heal/d1", "fix: parse defect", "body");
        assert_eq!(args[0], "pr");
        assert_eq!(args[1], "create");
        assert!(args.iter().any(|a| a == "--head"));
        assert!(args.iter().any(|a| a == "heal/d1"));
        // Safety (FR-007): never auto-merge.
        assert!(!args.iter().any(|a| a.contains("merge")));
    }

    #[test]
    fn gh_available_returns_bool() {
        // Environment-dependent; just ensure it does not panic.
        let _ = gh_available();
    }
}
