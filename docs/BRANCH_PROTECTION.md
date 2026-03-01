# Branch Protection Configuration

This document describes the branch protection settings that should be enabled for the `main` branch.

## Current Status

A repository ruleset exists (ID: 12805654) but is currently **disabled**. The ruleset needs to be enabled via the GitHub web interface.

## How to Enable

### Via GitHub Web UI

1. Go to: <https://github.com/quanticsoul4772/mcp-reasoning/settings/rules>
2. Find the "mcp-reasoning" ruleset
3. Click "Edit"
4. Change "Enforcement status" from **Disabled** to **Active**
5. Click "Save changes"

### Current Ruleset Configuration

The existing ruleset includes:

- **Deletion protection** - Prevents branch deletion
- **Non-fast-forward protection** - Prevents force pushes

### Recommended Additional Rules

Consider adding these rules to strengthen branch protection:

1. **Require pull request reviews**
   - Minimum 1 approval required
   - Dismiss stale reviews on new commits

2. **Require status checks to pass**
   - Check: `cargo fmt --check`
   - Check: `cargo clippy -- -D warnings`
   - Check: `cargo test`
   - Check: Coverage (95% minimum)
   - Check: Security Audit

3. **Require signed commits** (optional but recommended)
   - Ensures commit authenticity
   - Prevents commit impersonation

4. **Restrict who can push**
   - Limit to repository maintainers
   - Require pull requests from everyone else

## Why Branch Protection?

Branch protection is critical for maintaining code quality:

- **Prevents accidental force pushes** - No history rewriting on main
- **Ensures code review** - All changes reviewed before merge
- **Enforces quality gates** - Tests and linting must pass
- **Maintains stability** - Main branch always in working state
- **Audit trail** - All changes traceable through PRs

## Verification

To verify branch protection is active:

```bash
# Check ruleset status (requires appropriate permissions)
gh api repos/quanticsoul4772/mcp-reasoning/rulesets/12805654

# Look for "enforcement": "active" in the response
```

Or test manually:

```bash
# Try to force push (should fail if protection is active)
git push --force origin main
# Expected: "cannot force-push to a protected branch"
```

## Troubleshooting

### API Access Issues

If you encounter 404 errors when trying to modify rulesets via API, you may need:

- Additional GitHub permissions (repo admin)
- Personal access token with `admin:repo_hook` scope
- Use the GitHub web UI instead

### Protection Not Working

If protection seems not to be enforced:

- Verify enforcement is set to "Active", not "Disabled"
- Check if bypass actors are configured (can override protections)
- Ensure you're testing with a non-admin account (admins can bypass)

## Further Reading

- [GitHub Branch Protection Rules](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches)
- [GitHub Repository Rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets)
