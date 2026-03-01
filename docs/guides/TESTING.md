# Testing Guide

Comprehensive testing strategies and best practices for the MCP Reasoning Server.

---

## Table of Contents

- [Testing Philosophy](#testing-philosophy)
- [Test Organization](#test-organization)
- [Unit Testing](#unit-testing)
- [Integration Testing](#integration-testing)
- [Test Patterns](#test-patterns)
- [Coverage](#coverage)
- [Common Scenarios](#common-scenarios)

---

## Testing Philosophy

### Our Approach

**Test-Driven Development (TDD)**:

1. Write failing test
2. Implement feature
3. Verify test passes
4. Refactor

**Goals**:

- 95%+ line coverage
- Fast feedback (<1 second for unit tests)
- Clear, maintainable tests
- Confidence in changes

### What We Test

✅ **Always Test**:

- Public API behavior
- Error handling paths
- Edge cases and boundaries
- State management
- Database operations

❌ **Don't Test**:

- Third-party library internals
- Generated code (derive macros)
- Trivial getters/setters

---

## Test Organization

### Structure

```
src/
├── modes/
│   └── linear.rs           # Implementation + unit tests
└── lib.rs

tests/                      # Integration tests
├── integration/
│   ├── linear_mode.rs
│   └── graph_mode.rs
└── common/
    └── helpers.rs
```

### Unit Tests (Same File)

```rust
// src/modes/linear.rs

pub struct LinearMode { /* ... */ }

impl LinearMode {
    pub async fn process(&self, content: String) -> Result<Response, Error> {
        // Implementation
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_basic() {
        let mode = LinearMode::new(mock_deps());
        let result = mode.process("test".to_string()).await;

        assert!(result.is_ok());
        let response = result.expect("should succeed");
        assert!(!response.content.is_empty());
    }
}
```

### Integration Tests (tests/ Directory)

```rust
// tests/integration/linear_mode.rs

use mcp_reasoning::*;

#[tokio::test]
#[serial]
async fn test_linear_mode_end_to_end() {
    let storage = test_storage().await;
    let client = test_client();

    let mode = LinearMode::new(storage, client);
    let result = mode.process("Analyze this".to_string()).await;

    assert!(result.is_ok());
}
```

---

## Unit Testing

### Basic Pattern

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_function() {
        let result = sync_function(input);
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = async_function(input).await;
        assert!(result.is_ok());
    }
}
```

### Mocking

**Using mockall**:

```rust
use mockall::predicate::*;
use mockall::*;

#[automock]
trait StorageTrait {
    async fn save(&self, data: Data) -> Result<(), Error>;
}

#[tokio::test]
async fn test_with_mock() {
    let mut mock_storage = MockStorageTrait::new();

    mock_storage
        .expect_save()
        .times(1)
        .with(eq(expected_data))
        .returning(|_| Ok(()));

    let result = function_using_storage(&mock_storage).await;
    assert!(result.is_ok());
}
```

### Error Testing

```rust
#[tokio::test]
async fn test_error_empty_content() {
    let mode = LinearMode::new(test_deps());
    let result = mode.process("".to_string()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ModeError::InvalidInput { message } => {
            assert!(message.contains("empty"));
        }
        _ => panic!("Wrong error type"),
    }
}
```

### Edge Cases

```rust
#[tokio::test]
async fn test_max_content_length() {
    let mode = LinearMode::new(test_deps());
    let content = "a".repeat(100_000);  // Max allowed

    let result = mode.process(content).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_exceeds_max_length() {
    let mode = LinearMode::new(test_deps());
    let content = "a".repeat(100_001);  // Over limit

    let result = mode.process(content).await;
    assert!(result.is_err());
}
```

---

## Integration Testing

### Database Tests

**Use `#[serial]` to prevent race conditions**:

```rust
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_save_and_retrieve() {
    let storage = test_storage().await;

    // Save
    let session = Session::new("test-id");
    storage.save_session(&session).await.expect("save failed");

    // Retrieve
    let retrieved = storage
        .get_session("test-id")
        .await
        .expect("retrieve failed")
        .expect("session not found");

    assert_eq!(retrieved.id, "test-id");
}
```

### Test Utilities

```rust
// src/test_utils.rs

pub fn test_storage() -> SqliteStorage {
    SqliteStorage::new_in_memory()
        .await
        .expect("create test storage")
}

pub fn test_client() -> MockAnthropicClient {
    let mut mock = MockAnthropicClient::new();
    mock.expect_send()
        .returning(|_| Ok(test_response()));
    mock
}

pub fn test_response() -> AnthropicResponse {
    AnthropicResponse {
        content: "Test response".to_string(),
        // ...
    }
}
```

### End-to-End Tests

```rust
#[tokio::test]
#[serial]
async fn test_full_reasoning_workflow() {
    // Setup
    let storage = test_storage().await;
    let client = test_client();

    // Execute workflow
    let linear = LinearMode::new(storage.clone(), client.clone());
    let response1 = linear.process("Step 1").await.expect("step 1 failed");

    let tree = TreeMode::new(storage.clone(), client.clone());
    let response2 = tree
        .create("Step 2".to_string(), 3)
        .await
        .expect("step 2 failed");

    // Verify state
    let session = storage
        .get_session(&response1.session_id)
        .await
        .expect("get session failed");

    assert_eq!(session.thought_count, 2);
}
```

---

## Test Patterns

### Test Naming

```rust
// ✅ Good: Descriptive
#[test]
fn test_parse_json_with_valid_input() { }

#[test]
fn test_parse_json_returns_error_for_invalid_syntax() { }

// ❌ Bad: Vague
#[test]
fn test_parse() { }

#[test]
fn test_error() { }
```

### Arrange-Act-Assert (AAA)

```rust
#[tokio::test]
async fn test_process() {
    // Arrange
    let mode = LinearMode::new(test_deps());
    let input = "test content";

    // Act
    let result = mode.process(input.to_string()).await;

    // Assert
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.content.is_empty());
    assert!(response.confidence > 0.0);
}
```

### Parameterized Tests

```rust
use test_case::test_case;

#[test_case("short" ; "short input")]
#[test_case("a".repeat(1000) ; "long input")]
#[test_case("special chars: 你好 🎉" ; "unicode")]
fn test_content_variants(content: String) {
    let result = validate_content(&content);
    assert!(result.is_ok());
}
```

### Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_confidence_always_in_range(score in 0.0f64..1.0f64) {
        let response = create_response_with_confidence(score);
        assert!(response.confidence >= 0.0 && response.confidence <= 1.0);
    }
}
```

---

## Coverage

### Generate Coverage Report

```bash
# HTML report
cargo llvm-cov --html
open target/llvm-cov/html/index.html

# Terminal summary
cargo llvm-cov

# Specific module
cargo llvm-cov --html -- modes::linear
```

### Coverage Goals

| Component | Target |
|-----------|--------|
| Overall | 95%+ |
| New code | 98%+ |
| Error paths | 90%+ |
| Happy paths | 100% |

### What Coverage Doesn't Mean

❌ **High coverage ≠ Good tests**

```rust
// 100% coverage but useless test
#[test]
fn test_add() {
    add(1, 2);  // No assertions!
}
```

✅ **Good tests verify behavior**:

```rust
#[test]
fn test_add_returns_sum() {
    assert_eq!(add(1, 2), 3);
    assert_eq!(add(-1, 1), 0);
    assert_eq!(add(0, 0), 0);
}
```

---

## Common Scenarios

### Testing Async Functions

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_operation().await;
    assert!(result.is_ok());
}

// With timeout
#[tokio::test(flavor = "multi_thread")]
async fn test_with_timeout() {
    use tokio::time::{timeout, Duration};

    let result = timeout(
        Duration::from_secs(5),
        long_operation()
    ).await;

    assert!(result.is_ok(), "Operation timed out");
}
```

### Testing Error Messages

```rust
#[tokio::test]
async fn test_error_message_quality() {
    let result = operation_that_fails().await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let msg = error.to_string();

    // Verify helpful error message
    assert!(msg.contains("session"));
    assert!(msg.contains("not found"));
    assert!(!msg.contains("Internal error")); // Too vague
}
```

### Testing State Changes

```rust
#[tokio::test]
#[serial]
async fn test_state_progression() {
    let storage = test_storage().await;
    let mode = TreeMode::new(storage.clone());

    // Initial state
    let branches = storage.get_branches("session").await.unwrap();
    assert_eq!(branches.len(), 0);

    // After creation
    mode.create("content", 3).await.unwrap();
    let branches = storage.get_branches("session").await.unwrap();
    assert_eq!(branches.len(), 3);

    // After completion
    mode.complete(&branches[0].id, true).await.unwrap();
    let branch = storage.get_branch(&branches[0].id).await.unwrap().unwrap();
    assert_eq!(branch.status, BranchStatus::Completed);
}
```

### Testing Concurrent Operations

```rust
#[tokio::test]
async fn test_concurrent_requests() {
    let mode = Arc::new(LinearMode::new(test_deps()));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let mode = Arc::clone(&mode);
            tokio::spawn(async move {
                mode.process(format!("request {}", i)).await
            })
        })
        .collect();

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
```

---

## Running Tests

### All Tests

```bash
cargo test
```

### Specific Module

```bash
cargo test modes::linear
cargo test storage
cargo test integration::
```

### With Output

```bash
cargo test -- --nocapture
cargo test -- --show-output
```

### Single Test

```bash
cargo test test_specific_function
```

### Parallel vs Sequential

```bash
# Default: parallel
cargo test

# Sequential (for debugging)
cargo test -- --test-threads=1
```

---

## Best Practices

✅ **Do**:

- Write tests first (TDD)
- Test behavior, not implementation
- Use descriptive test names
- Test error paths
- Keep tests fast (<1s for unit tests)
- Use `#[serial]` for database tests
- Clean up test data

❌ **Don't**:

- Skip error case tests
- Test private functions directly
- Have flaky tests
- Use `sleep()` for synchronization
- Ignore test failures
- Commit commented-out tests

---

## Additional Resources

- [Development Guide](DEVELOPMENT.md) - Setup and workflow
- [Contributing Guide](CONTRIBUTING.md) - PR process
- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [tokio Testing](https://tokio.rs/tokio/topics/testing)

---

**Last Updated**: 2026-03-01
