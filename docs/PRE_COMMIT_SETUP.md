# Pre-Commit Hooks Setup

This document describes the pre-commit hooks configured for the mcp-reasoning project.

## What Are Pre-Commit Hooks?

Pre-commit hooks are scripts that run automatically before each commit to catch issues early. They help maintain code quality by enforcing standards before code reaches CI.

## Installation

### 1. Install pre-commit

```bash
# macOS/Linux
pip install pre-commit

# Or using Homebrew (macOS)
brew install pre-commit

# Or using Cargo
cargo install pre-commit
```

### 2. Install git hook scripts

```bash
cd /path/to/mcp-reasoning
pre-commit install
```

This installs the pre-commit hook into your `.git/hooks/` directory. The hooks will now run automatically on `git commit`.

### 3. (Optional) Install gitleaks for secret scanning

```bash
# macOS
brew install gitleaks

# Linux
# Download from https://github.com/gitleaks/gitleaks/releases

# Windows
# Download from https://github.com/gitleaks/gitleaks/releases
# Or use: choco install gitleaks
```

## Configured Hooks

### Rust Formatting (`cargo fmt`)
- Automatically formats Rust code using rustfmt
- Ensures consistent code style across the project
- **Auto-fix**: Yes

### Rust Linting (`cargo clippy`)
- Runs clippy with `-D warnings` (treats warnings as errors)
- Catches common mistakes and enforces best practices
- **Auto-fix**: No (manual fixes required)

### Secret Scanning (`gitleaks`)
- Scans staged files for secrets and credentials
- Prevents accidentally committing API keys, passwords, etc.
- **Auto-fix**: No (manual removal required)

### General File Checks
- Trailing whitespace removal
- End-of-file fixer (ensures newline at EOF)
- YAML syntax validation
- TOML syntax validation
- Large file detection (max 500KB)
- Merge conflict detection
- Line ending normalization

### Markdown Linting (`markdownlint`)
- Lints and auto-fixes markdown files
- Ensures consistent markdown formatting
- **Auto-fix**: Yes

## Running Manually

You can run hooks manually without committing:

```bash
# Run all hooks on all files
pre-commit run --all-files

# Run specific hook
pre-commit run cargo-fmt --all-files
pre-commit run clippy --all-files
pre-commit run gitleaks --all-files

# Run on staged files only (same as pre-commit)
pre-commit run
```

## Skipping Hooks

**Not recommended**, but you can skip hooks in emergency situations:

```bash
git commit --no-verify -m "Emergency fix"
```

Use this sparingly - hooks are there to protect code quality!

## Updating Hooks

Pre-commit hooks can be updated to their latest versions:

```bash
pre-commit autoupdate
```

This updates the `rev` field in `.pre-commit-config.yaml` to the latest release.

## Troubleshooting

### Hook execution is slow
- First run is slower as it sets up environments
- Subsequent runs are much faster (cached)
- Consider running hooks in parallel (default behavior)

### Clippy fails on valid code
- Ensure you're using the same Rust version as CI
- Check `rust-toolchain.toml` for the required version
- Run `cargo clippy -- -D warnings` manually to see issues

### Gitleaks reports false positives
- Add exceptions to `.gitleaksignore` if needed
- Use `gitleaks protect --verbose` to see what was detected

## Integration with CI

These same checks run in CI (`.github/workflows/ci.yml`), so pre-commit hooks help you catch issues locally before pushing.

## Best Practices

1. **Install pre-commit early** - Do it right after cloning the repo
2. **Don't skip hooks** - Fix issues rather than bypassing checks
3. **Run manually before large commits** - `pre-commit run --all-files`
4. **Keep hooks updated** - Run `pre-commit autoupdate` periodically
5. **Share with team** - Document in README that pre-commit is required

## Further Reading

- [pre-commit documentation](https://pre-commit.com/)
- [gitleaks documentation](https://github.com/gitleaks/gitleaks)
- [markdownlint rules](https://github.com/DavidAnson/markdownlint/blob/main/doc/Rules.md)
