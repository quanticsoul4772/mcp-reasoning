# Lessons Learned from mcp-langbase-reasoning

## Summary

The langbase project accumulated structural debt over time. This document captures what to replicate and what to avoid.

---

## What Worked Well

### 1. Error Handling Architecture
```rust
// Hierarchical error types with clear separation
AppError
+-- AnthropicError (API layer)
+-- StorageError (database layer)
+-- McpError (protocol layer)
+-- ModeError (business logic)
```
- Use `thiserror` for all error types
- Each subsystem owns its errors
- Clear `From` implementations for composition

### 2. ModeCore Composition Pattern
```rust
// Share dependencies via composition, not inheritance
pub struct ModeCore {
    storage: SqliteStorage,
    client: AnthropicClient,
}

impl LinearMode {
    core: ModeCore,  // Not: impl ReasoningMode trait
}
```
- Avoids trait complexity
- Single source of truth for dependencies
- Easy to test with mock core

### 3. Configuration as First-Class Citizen
```rust
// Unified config loaded once at startup
pub struct Config {
    pub anthropic: AnthropicConfig,
    pub storage: StorageConfig,
    pub request: RequestConfig,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError>
}
```
- Fail fast on missing required values
- Sensible defaults for optional values
- Struct-based, not scattered env reads

### 4. JSON Extraction
```rust
// Handle multiple response formats
fn extract_json(text: &str) -> Result<Value, Error> {
    // Try raw JSON first (fast path)
    // Fall back to ```json blocks
    // Clear error with truncated preview
}
```
- LLMs return inconsistent formats
- Fast path for clean responses
- Graceful fallback with good errors

### 5. Retry Logic with Exponential Backoff
```rust
// Retry with backoff
while retries <= max_retries {
    match execute().await {
        Ok(r) => return Ok(r),
        Err(e) => {
            last_error = Some(e);
            sleep(delay * 2^retries).await;
            retries += 1;
        }
    }
}
```
- Always have retry logic for external APIs
- Log each attempt
- Track last error for reporting

### 6. Structured Logging
```rust
tracing::info!(
    mode = %mode_name,
    session_id = %session,
    latency_ms = elapsed.as_millis(),
    "Reasoning complete"
);
```
- Use `tracing` not `log`
- Structured fields for filtering
- stderr for logs (stdout for MCP)

### 7. Zero Unsafe Code Policy
- No `unsafe` blocks
- No `.unwrap()` in production paths
- No `.expect()` in handlers
- No `panic!()` macros
- Use `unwrap_or()` or `?` operator

### 8. Test Organization
```rust
// Tests alongside code
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_something() {
        let result = operation().await.expect("should succeed");
        assert_eq!(result, expected);
    }
}
```
- 2000+ tests is not excessive
- Test error cases explicitly
- Use `serial_test` for DB tests
- Test code uses `.unwrap()`/`.expect()` - acceptable and preferred for test clarity

---

## What Caused Problems

### 1. Giant Files
**Problem:** `sqlite.rs` was 4533 lines, `storage/mod.rs` was 3513 lines

**Solution for new project:**
```
src/storage/
+-- mod.rs           # Trait + re-exports only (~100 lines)
+-- sqlite.rs        # Main implementation (~500 lines)
+-- session.rs       # Session operations (~300 lines)
+-- thought.rs       # Thought operations (~300 lines)
+-- graph.rs         # Graph operations (~400 lines)
+-- metrics.rs       # Metrics operations (~200 lines)
```

### 2. Tool Explosion (40 tools)
**Problem:** Every operation became its own tool

**Solution:** Consolidated to 15 tools with `operation` parameter
```json
// Before: 8 separate tools
"reasoning_got_init", "reasoning_got_generate", "reasoning_got_score"...

// After: 1 tool with operation
{ "name": "reasoning_graph", "operation": "init|generate|score|..." }
```

### 3. Handler Routing Monolith
**Problem:** 1600+ line match statement routing tools

**Solution:** Tool registry pattern
```rust
// Before
match tool_name {
    "reasoning_linear" => handle_linear(args).await,
    "reasoning_tree" => handle_tree(args).await,
    // 40 more arms...
}

// After
let handlers: HashMap<&str, Box<dyn ToolHandler>> = create_handlers();
handlers.get(tool_name)?.handle(args).await
```

### 4. Parameter Struct Proliferation
**Problem:** 15+ nearly-identical Params structs

**Solution:** Common base with mode-specific extensions
```rust
// Common fields
pub struct ReasoningParams {
    pub content: String,
    pub session_id: Option<String>,
    pub confidence: Option<f64>,
}

// Mode-specific via enum
pub enum ModeParams {
    Linear(ReasoningParams),
    Tree { base: ReasoningParams, num_branches: u32 },
    // ...
}
```

### 5. Prompts in Single File
**Problem:** 1579 lines of prompts in one file

**Solution:** Organize by category
```
src/prompts/
+-- mod.rs           # get_prompt_for_mode() router
+-- core.rs          # linear, tree, divergent, reflection
+-- analysis.rs      # detect, decision, evidence
+-- advanced.rs      # graph, timeline, mcts, counterfactual
```

### 6. No Config Validation
**Problem:** Invalid configs discovered at runtime

**Solution:** Validate on load
```rust
impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let config = Self::load_raw()?;
        config.validate()?;  // NEW
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.request.timeout_ms == 0 {
            return Err(ConfigError::Invalid("timeout_ms must be > 0"));
        }
        if self.request.timeout_ms > 300_000 {
            return Err(ConfigError::Invalid("timeout_ms max is 5 minutes"));
        }
        Ok(())
    }
}
```

### 7. No Request Size Limits
**Problem:** Unbounded message sizes to API

**Solution:** Add limits
```rust
const MAX_REQUEST_BYTES: usize = 100_000;  // 100KB
const MAX_MESSAGES: usize = 50;

fn validate_request(req: &Request) -> Result<(), Error> {
    if req.messages.len() > MAX_MESSAGES {
        return Err(Error::TooManyMessages);
    }
    // ...
}
```

### 8. Self-Improvement Architecture
**Problem:** 14 interconnected files made it hard to understand

**Solution for new project:** Implement from day one, but with clearer organization
- Self-improvement is important - implement immediately
- Simplify the file structure (fewer files, clearer boundaries)
- Keep the 4-phase loop: Monitor -> Analyzer -> Executor -> Learner
- Keep safety features: circuit breaker, rate limiting
- Better documentation of the architecture

---

## Architecture Decisions for New Project

### File Size Limits
| File Type | Max Lines | Action if Exceeded |
|-----------|-----------|-------------------|
| Any .rs file | 500 | Split into submodules |
| mod.rs | 200 | Move types to separate files |
| Test file | 500 | Organize with nested modules |

### Module Organization
```
src/
+-- main.rs              # Entry point only (<100 lines)
+-- lib.rs               # Module declarations only
+-- config/
|   +-- mod.rs           # Config struct + from_env()
|   +-- validation.rs    # Validation logic
+-- error/
|   +-- mod.rs           # All error types (<500 lines)
+-- anthropic/
|   +-- mod.rs           # Client + exports
|   +-- types.rs         # Request/Response types
|   +-- config.rs        # Model settings
+-- prompts/
|   +-- mod.rs           # Router function
|   +-- core.rs          # Core mode prompts
|   +-- advanced.rs      # Advanced mode prompts
+-- modes/
|   +-- mod.rs           # ModeCore + exports
|   +-- linear.rs        # One file per mode
|   +-- tree.rs
|   +-- ...
+-- server/
|   +-- mod.rs           # AppState
|   +-- mcp.rs           # Protocol handling
|   +-- tools.rs         # Tool definitions (schemas)
|   +-- handlers.rs      # Tool handlers (registry pattern)
+-- storage/
|   +-- mod.rs           # Trait definition
|   +-- sqlite.rs        # Implementation
|   +-- session.rs       # Session operations
|   +-- types.rs         # Storage types
+-- metrics/
    +-- mod.rs           # Simple metrics (no over-engineering)
```

### Testing Strategy
- Unit tests in same file as code
- Integration tests in `tests/` directory
- Aim for 80% coverage, not 100%
- Test error paths explicitly
- Use `#[tokio::test]` for async

### Dependencies (Minimal)
```toml
[dependencies]
# Core
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# API
anthropic-sdk-rust = "0.1"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }

# Error handling
thiserror = "1"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Utilities
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

---

## Checklist Before Implementation

- [ ] Error types designed first
- [ ] Config struct with validation
- [ ] File size limits enforced
- [ ] Tool registry pattern (not match statement)
- [ ] Self-improvement system (4-phase loop, always enabled)
- [ ] Prompts organized by category
- [ ] Storage split by operation type
- [ ] Request size limits defined
- [ ] Retry logic with backoff
- [ ] Structured logging throughout
- [ ] Zero unsafe/unwrap policy

---

## Test Error Handling Decision (2024-12-29)

**Decision:** Use `#[allow(clippy::unwrap_used, clippy::expect_used)]` in test modules.

**Context:** Cargo.toml enforces `#![deny(unwrap_used, expect_used)]` for production code. This initially caused 872 clippy errors in test code.

**Rationale:**
- Tests are allowed to panic - it's their job to fail loudly and clearly
- `.expect("descriptive message")` provides better diagnostics than `?` in tests
- Maintains test readability and reduces verbosity
- Industry standard practice (Rust API Guidelines endorses this approach)
- Enables strict production lints while keeping pragmatic test patterns

**Implementation:**
- Added `#[allow(clippy::unwrap_used, clippy::expect_used)]` to all `#[cfg(test)] mod tests` blocks
- Added `#![allow(...)]` to integration test files in `tests/` directory
- Production code remains panic-free with zero unwrap/expect calls

**Alternatives Considered:**
- Rewrite all tests to use `Result<()>` - rejected (40+ hours, reduced readability)
- Hybrid approach with Result for critical paths - possible future enhancement

**When to use Result<()> in tests:**
- Integration tests with complex error chains
- Tests that need to propagate errors through multiple operations
- Error recovery testing where full error context is valuable

See `docs/TEST_ERROR_HANDLING_PLAN.md` for complete analysis.

---

## Pedantic Lint Fixes (2024-12-29)

Fixed 8+ clippy pedantic warnings using a hybrid automated + manual approach.

**Issues resolved:**
- Removed unnecessary raw string hashes (`r#""#` -> `r""`) - 5 locations
- Added numeric separators for readability (`120000` -> `120_000`) - 3 locations
- Simplified pattern matches (removed unnecessary field patterns)
- Other auto-fixed style improvements (17 fixes total)

**Method:**
1. Used `cargo clippy --fix --allow-dirty` for automated fixes (handled most issues)
2. Manual fixes for long literals (clippy auto-fix doesn't handle these)
3. Verified with full test suite (all 1,658 tests passing)

**Result:** Clean clippy pedantic lints for target issues, no functional changes.

**Files modified:** 19 files (src/modes/core.rs, src/server/params.rs, src/self_improvement/*, tests/*)

**Prevention:**
- Consider adding clippy pedantic to CI/CD pipeline
- Use editor integration (rust-analyzer with clippy enabled)
- Pre-commit hooks for automated style checks

---

## Dependency Cleanup (2024-12-30)

Removed 4 unused dependencies from Cargo.toml:
- `anyhow` - not used, project uses thiserror for error handling
- `futures` - not directly imported anywhere
- `async-stream` - not used
- `bytes` - not used

**Method:** Searched codebase for imports, verified build and tests still pass after removal.

**Result:** Reduced dependency footprint, faster builds, smaller binary.
