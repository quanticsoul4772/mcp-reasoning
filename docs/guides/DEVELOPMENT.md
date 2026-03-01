# Development Guide

Complete guide for setting up and working with the MCP Reasoning Server codebase.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Initial Setup](#initial-setup)
- [Building](#building)
- [Testing](#testing)
- [Code Quality](#code-quality)
- [Development Workflow](#development-workflow)
- [Debugging](#debugging)
- [Common Issues](#common-issues)

---

## Prerequisites

### Required

- **Rust 1.75+** - For async traits support

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update
  ```

- **Anthropic API Key** - Get from [Anthropic Console](https://console.anthropic.com/)

### Recommended

- **Pre-commit** - For automated code quality checks

  ```bash
  pip install pre-commit
  ```

- **cargo-llvm-cov** - For coverage reports

  ```bash
  cargo install cargo-llvm-cov
  ```

- **Claude Code or Claude Desktop** - For testing integration

---

## Initial Setup

### 1. Clone the Repository

```bash
git clone https://github.com/quanticsoul4772/mcp-reasoning.git
cd mcp-reasoning
```

### 2. Set Up Environment

Create `.env` file:

```bash
cp .env.example .env
```

Edit `.env` and add your API key:

```bash
ANTHROPIC_API_KEY=your_api_key_here
DATABASE_PATH=./data/reasoning.db
LOG_LEVEL=info
```

### 3. Install Pre-commit Hooks

```bash
pre-commit install
```

This sets up automatic checks on every commit:

- `cargo fmt` - Code formatting
- `cargo clippy` - Linting
- Whitespace and line ending fixes
- File size checks
- Markdown linting

See [Pre-commit Setup](PRE_COMMIT_SETUP.md) for details.

### 4. Prepare Database

```bash
cargo sqlx prepare --database-url "sqlite:./data/reasoning.db"
```

This generates `.sqlx/` directory with compile-time query verification.

---

## Building

### Debug Build (Development)

```bash
cargo build
```

Binary: `target/debug/mcp-reasoning`

### Release Build (Production)

```bash
cargo build --release
```

Binary: `target/release/mcp-reasoning`

- Optimized for speed (~10x faster)
- Smaller binary size
- LTO enabled
- Debug symbols stripped

### Build Times

| Build Type | Clean Build | Incremental |
|------------|-------------|-------------|
| Debug | ~2-3 min | ~10-30 sec |
| Release | ~5-8 min | ~30-60 sec |

---

## Testing

### Run All Tests

```bash
cargo test
```

**Expected output**: `2020 passed` in ~0.8 seconds

### Run Specific Module

```bash
# Test a specific module
cargo test modes::linear

# Test storage layer
cargo test storage

# Test with output
cargo test -- --nocapture

# Test single function
cargo test test_linear_mode
```

### Test Organization

```rust
// Unit tests (in same file as code)
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_example() {
        // Tests can use .unwrap() and .expect()
    }
}
```

Tests use:

- `#[tokio::test]` for async tests
- `#[serial]` for database tests (from `serial_test` crate)
- `#[allow(clippy::unwrap_used)]` in test modules

See [Testing Guide](TESTING.md) for detailed strategies.

### Coverage Report

```bash
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

**Target**: 95%+ line coverage

---

## Code Quality

### Format Code

```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt
```

### Lint Code

```bash
# Run clippy with warnings as errors
cargo clippy -- -D warnings

# Run clippy pedantic
cargo clippy -- -W clippy::pedantic

# Auto-fix issues
cargo clippy --fix
```

### File Size Check

Max 500 lines per file:

```bash
# Check file sizes
wc -l src/**/*.rs | sort -n

# Files approaching limit
wc -l src/**/*.rs | awk '$1 > 400'
```

### Pre-commit Validation

Run all checks manually:

```bash
pre-commit run --all-files
```

---

## Development Workflow

### 1. Create Feature Branch

```bash
git checkout -b feature/my-feature
```

### 2. Make Changes

Follow TDD approach:

1. Write failing test
2. Implement feature
3. Verify test passes
4. Refactor

### 3. Run Quality Checks

```bash
# Before committing
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

### 4. Commit Changes

Pre-commit hooks will run automatically:

```bash
git add .
git commit -m "feat: Add new feature"
```

If hooks fail, fix issues and re-commit.

### 5. Push and Create PR

```bash
git push origin feature/my-feature
```

Then create PR on GitHub.

---

## Debugging

### Enable Debug Logging

```bash
LOG_LEVEL=debug cargo run
```

Or in `.env`:

```bash
LOG_LEVEL=debug
```

Levels: `error`, `warn`, `info`, `debug`, `trace`

### Run with Debugger

**VS Code**: Use `launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug mcp-reasoning",
      "cargo": {
        "args": ["build", "--bin=mcp-reasoning"],
        "filter": {
          "name": "mcp-reasoning",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "ANTHROPIC_API_KEY": "your_key_here"
      }
    }
  ]
}
```

**CLI**:

```bash
rust-lldb target/debug/mcp-reasoning
```

### Inspect SQLite Database

```bash
sqlite3 data/reasoning.db

# List tables
.tables

# Query sessions
SELECT * FROM sessions;

# Schema
.schema sessions
```

### Check Logs

Logs go to stderr by default:

```bash
cargo run 2> logs.txt
```

---

## Common Issues

### Issue: `cargo test` fails with database errors

**Solution**: Prepare SQLx queries:

```bash
cargo sqlx prepare --database-url "sqlite:./data/reasoning.db"
```

### Issue: Pre-commit hooks fail

**Solution**: Run hooks manually to see details:

```bash
pre-commit run --all-files
```

Fix issues and commit again.

### Issue: `ANTHROPIC_API_KEY` not found

**Solution**: Create `.env` file with your API key:

```bash
echo "ANTHROPIC_API_KEY=your_key_here" > .env
```

### Issue: Slow builds

**Solutions**:

- Use `cargo build` (not release) for development
- Enable incremental compilation (default)
- Use `cargo check` for faster feedback
- Consider using `sccache` or `mold` linker

### Issue: Test failures in CI but passing locally

**Causes**:

- Missing `.sqlx/` directory - commit it to git
- Different Rust version - check `rust-toolchain.toml`
- Race conditions - use `#[serial]` for database tests

### Issue: Out of memory during build

**Solution**: Reduce parallel jobs:

```bash
cargo build -j 2
```

---

## Additional Resources

- [Testing Guide](TESTING.md) - Comprehensive testing strategies
- [Contributing Guide](CONTRIBUTING.md) - PR workflow and guidelines
- [Architecture](../reference/ARCHITECTURE.md) - System design
- [Lessons Learned](../reference/LESSONS_LEARNED.md) - Design patterns

---

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/quanticsoul4772/mcp-reasoning/issues)
- **Discussions**: [GitHub Discussions](https://github.com/quanticsoul4772/mcp-reasoning/discussions)
- **Documentation**: [docs/README.md](../README.md)

---

**Last Updated**: 2026-03-01
