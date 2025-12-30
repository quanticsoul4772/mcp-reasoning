//! Help text for CLI commands.

/// Get help text for CLI commands.
pub fn help_text() -> &'static str {
    r#"Self-Improvement System Commands:

  status              Show current system status

  history [options]   Show action history
    --limit, -l N       Maximum records to show (default: 10)
    --outcome, -o TYPE  Filter by outcome (success, failed, rolled_back)

  diagnostics, diag   Show diagnostic information
    --verbose, -v       Show verbose output

  config              Show current configuration

  circuit-breaker, cb Show circuit breaker status

  baselines           Show current baselines

  pause DURATION      Temporarily pause the system
                      Duration: e.g., "1h", "30m", "2d"

  rollback ACTION_ID  Rollback a specific action

  approve DIAG_ID     Approve a pending diagnosis

  reject DIAG_ID [REASON]
                      Reject a pending diagnosis with optional reason

Examples:
  self-improve status
  self-improve history --limit 20 --outcome success
  self-improve diagnostics --verbose
  self-improve pause 1h
  self-improve approve diag-abc123
  self-improve reject diag-xyz789 "Risk too high"
"#
}
