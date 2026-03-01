# Contributing Guide

Thank you for considering contributing to the MCP Reasoning Server! This guide will help you get started.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Process](#development-process)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Documentation](#documentation)
- [Review Process](#review-process)

---

## Code of Conduct

We are committed to providing a welcoming and inspiring community for all. Please be respectful and constructive in all interactions.

---

## Getting Started

### Prerequisites

1. Read the [Development Guide](DEVELOPMENT.md) for environment setup
2. Familiarize yourself with the [Architecture](../reference/ARCHITECTURE.md)
3. Review [Lessons Learned](../reference/LESSONS_LEARNED.md) for design patterns

### Finding Work

- Check [Issues](https://github.com/quanticsoul4772/mcp-reasoning/issues) for open tasks
- Look for issues labeled `good first issue` or `help wanted`
- Ask questions in [Discussions](https://github.com/quanticsoul4772/mcp-reasoning/discussions)

---

## Development Process

### 1. Fork and Clone

```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/mcp-reasoning.git
cd mcp-reasoning

# Add upstream remote
git remote add upstream https://github.com/quanticsoul4772/mcp-reasoning.git
```

### 2. Create Branch

```bash
git checkout -b feature/your-feature-name
```

**Branch naming conventions:**

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation only
- `refactor/` - Code refactoring
- `test/` - Test improvements
- `perf/` - Performance improvements

### 3. Make Changes

Follow **Test-Driven Development (TDD)**:

1. Write failing test
2. Implement feature
3. Verify test passes
4. Refactor if needed

**Example**:

```rust
// 1. Write failing test
#[tokio::test]
async fn test_new_feature() {
    let result = new_feature().await.expect("should work");
    assert_eq!(result.value, 42);
}

// 2. Implement
pub async fn new_feature() -> Result<Output, Error> {
    // Implementation
}

// 3. Verify: cargo test new_feature
// 4. Refactor if needed
```

### 4. Commit Changes

Use [Conventional Commits](https://www.conventionalcommits.org/):

```bash
git commit -m "feat: Add new reasoning mode"
git commit -m "fix: Resolve timeout issue in graph mode"
git commit -m "docs: Update API reference"
git commit -m "test: Add coverage for checkpoint mode"
git commit -m "perf: Optimize SQL query allocation"
```

**Format**: `<type>: <description>`

**Types**:

- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation
- `test` - Tests
- `refactor` - Code restructuring
- `perf` - Performance
- `chore` - Maintenance

### 5. Stay Updated

```bash
git fetch upstream
git rebase upstream/main
```

---

## Pull Request Process

### Before Submitting

✅ **Checklist**:

- [ ] All tests pass: `cargo test`
- [ ] Code formatted: `cargo fmt`
- [ ] Lints pass: `cargo clippy -- -D warnings`
- [ ] Coverage maintained: `cargo llvm-cov` (95%+)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if user-facing)
- [ ] Commits follow conventions
- [ ] Branch is up to date with main

### Create Pull Request

1. Push your branch:

   ```bash
   git push origin feature/your-feature-name
   ```

2. Go to GitHub and click "New Pull Request"

3. Fill in the template:
   - **Title**: Clear, descriptive
   - **Description**: What, why, how
   - **Related Issues**: Link with `Fixes #123`
   - **Testing**: How to test
   - **Screenshots**: If UI changes

### PR Title Format

```
feat: Add counterfactual reasoning mode
fix: Resolve database connection timeout
docs: Update installation instructions
```

### During Review

- Respond to feedback promptly
- Make requested changes
- Push new commits (don't force-push during review)
- Mark conversations as resolved

### After Approval

- Maintainer will merge (squash and merge)
- Delete your feature branch
- Pull latest main

---

## Coding Standards

### Rust Guidelines

**General**:

- No `unsafe` code: `#![forbid(unsafe_code)]`
- No `.unwrap()` or `.expect()` in production code
- No `panic!()` in production code
- Use `?` operator for error propagation

**Error Handling**:

```rust
// ✅ Good
pub async fn operation() -> Result<Output, Error> {
    let value = fallible_operation()?;
    Ok(value)
}

// ❌ Bad
pub async fn operation() -> Output {
    fallible_operation().unwrap()  // Never in production!
}
```

**Test Code Exception**:

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    // Tests CAN use .unwrap() and .expect()
    #[tokio::test]
    async fn test_example() {
        let result = operation().await.expect("should work");
        assert_eq!(result, expected);
    }
}
```

### File Organization

**Max 500 lines per file**:

- If approaching limit, split into submodules
- Use clear module hierarchy
- Keep related code together

**Example**:

```
src/modes/
├── mod.rs          # Re-exports only
├── core.rs         # Shared dependencies
├── linear.rs       # <500 lines
└── tree/           # Split if >500 lines
    ├── mod.rs
    ├── create.rs
    └── focus.rs
```

### Logging

Use structured logging with `tracing`:

```rust
tracing::info!(
    mode = %mode_name,
    session_id = %session,
    latency_ms = elapsed.as_millis(),
    "Reasoning complete"
);
```

**Levels**:

- `error` - Failures requiring attention
- `warn` - Concerning but recoverable
- `info` - Significant events
- `debug` - Detailed diagnostics
- `trace` - Very verbose

### Documentation

**Public API must have docs**:

```rust
/// Process content with sequential reasoning.
///
/// # Arguments
///
/// * `content` - The content to analyze
/// * `session_id` - Optional session for context
///
/// # Returns
///
/// Returns a `LinearResponse` with reasoning and confidence.
///
/// # Errors
///
/// Returns `ModeError` if:
/// - Content is empty
/// - API request fails
/// - Database error occurs
pub async fn process(
    content: String,
    session_id: Option<String>,
) -> Result<LinearResponse, ModeError>
```

---

## Testing Requirements

### Coverage

- **Minimum**: 95% line coverage
- **Target**: 98%+ for new code
- Check with: `cargo llvm-cov --html`

### Test Organization

```rust
// Unit tests in same file
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature() {
        // Test code
    }

    #[tokio::test]
    #[serial]  // For database tests
    async fn test_database_operation() {
        // Database test
    }
}
```

### Test Requirements

✅ **Every PR must include**:

- Unit tests for new functions
- Integration tests for new features
- Error case tests
- Edge case tests

See [Testing Guide](TESTING.md) for detailed strategies.

---

## Documentation

### Update Required

When you change:

- **API** → Update `docs/reference/API_SPECIFICATION.md`
- **Architecture** → Update `docs/reference/ARCHITECTURE.md`
- **User-facing** → Update `README.md` and `CHANGELOG.md`
- **Development** → Update relevant guides

### Documentation Style

- Use clear headings
- Include code examples
- Keep files under 500 lines
- Link to related docs

---

## Review Process

### What We Look For

✅ **Code Quality**:

- Follows Rust idioms
- Clear, self-documenting code
- Appropriate error handling
- No unnecessary complexity

✅ **Testing**:

- Good coverage
- Tests are clear and maintainable
- Edge cases covered

✅ **Documentation**:

- Public APIs documented
- CHANGELOG updated
- README updated if needed

✅ **Performance**:

- No obvious inefficiencies
- Allocations minimized
- Database queries optimized

### Review Timeline

- Initial review: 1-3 days
- Follow-up reviews: 1-2 days
- Merge: After approval + CI passes

### After Merge

Your contribution will be:

- Included in next release
- Listed in CHANGELOG
- Credited in git history
- Appreciated by the community! 🎉

---

## Questions?

- **Issues**: [GitHub Issues](https://github.com/quanticsoul4772/mcp-reasoning/issues)
- **Discussions**: [GitHub Discussions](https://github.com/quanticsoul4772/mcp-reasoning/discussions)
- **Email**: See repository maintainers

---

**Thank you for contributing!** 🙏

Every contribution, no matter how small, makes this project better.

---

**Last Updated**: 2026-03-01
