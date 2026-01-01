# Silent Failures Fix Plan

This document outlines the implementation plan for fixing silent failures identified in the mcp-reasoning codebase.

## Problem Summary

Six categories of silent failures were identified:

1. **Unused Transport Timeout** - `TransportConfig.read_timeout_ms` is defined but never applied
2. **RwLock Poisoning Ignored** - Metrics recording silently fails when lock is poisoned
3. **Metadata Errors Swallowed** - `.ok()` converts errors to `None` without logging
4. **No Request Lifecycle Logging** - Cannot observe when API calls start/complete
5. **Database Errors Silent** - `TimingDatabase` falls back silently without logging
6. **Environment Config Parsing** - Invalid values silently replaced with defaults

---

## Fix 1: Apply Transport Timeout

### Problem
`TransportConfig.read_timeout_ms` (default 300,000ms = 5 min) is defined but the `serve()` function ignores `self.config` entirely.

### Design

**Approach**: Wrap tool execution in `tokio::time::timeout` at the tool handler level rather than transport level, since rmcp handles transport internally.

**File**: `src/server/tools.rs`

**Pattern**: Create a timeout wrapper for each tool handler method.

```rust
// New helper macro or function in tools.rs
use tokio::time::{timeout, Duration};

const TOOL_EXECUTION_TIMEOUT_MS: u64 = 300_000; // 5 minutes

async fn with_tool_timeout<T, F: Future<Output = T>>(
    tool_name: &str,
    future: F,
) -> Result<T, AppError> {
    let timeout_duration = Duration::from_millis(TOOL_EXECUTION_TIMEOUT_MS);

    match timeout(timeout_duration, future).await {
        Ok(result) => Ok(result),
        Err(_) => {
            tracing::error!(
                tool = tool_name,
                timeout_ms = TOOL_EXECUTION_TIMEOUT_MS,
                "Tool execution timed out"
            );
            Err(AppError::Timeout {
                operation: tool_name.to_string(),
                timeout_ms: TOOL_EXECUTION_TIMEOUT_MS,
            })
        }
    }
}
```

**Integration**: Apply to each `#[tool]` method:

```rust
#[tool(name = "reasoning_linear", ...)]
async fn reasoning_linear(&self, req: Parameters<LinearRequest>) -> LinearResponse {
    match with_tool_timeout("reasoning_linear", self.reasoning_linear_inner(req)).await {
        Ok(response) => response,
        Err(e) => LinearResponse::error(e.to_string()),
    }
}
```

### Error Type Addition

**File**: `src/error/mod.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // ... existing variants ...

    #[error("Operation '{operation}' timed out after {timeout_ms}ms")]
    Timeout {
        operation: String,
        timeout_ms: u64,
    },
}
```

### Tests Required
- Test that timeout is triggered after configured duration
- Test that timeout error is properly logged
- Test that timeout doesn't affect successful operations

---

## Fix 2: Add Logging to Metrics RwLock

### Problem
When `self.events.write()` or `self.fallbacks.write()` fails (lock poisoned), the failure is completely silent.

### Design

**File**: `src/metrics/mod.rs`

**Before**:
```rust
pub fn record(&self, event: MetricEvent) {
    if let Ok(mut events) = self.events.write() {
        events.push(event);
    }
}
```

**After**:
```rust
pub fn record(&self, event: MetricEvent) {
    match self.events.write() {
        Ok(mut events) => {
            events.push(event);
        }
        Err(poison_error) => {
            tracing::error!(
                mode = %event.mode,
                error = %poison_error,
                "Failed to record metric event: RwLock poisoned"
            );
        }
    }
}

pub fn record_fallback(&self, fallback: FallbackEvent) {
    match self.fallbacks.write() {
        Ok(mut fallbacks) => {
            fallbacks.push(fallback);
        }
        Err(poison_error) => {
            tracing::error!(
                from = %fallback.from_mode,
                to = %fallback.to_mode,
                error = %poison_error,
                "Failed to record fallback event: RwLock poisoned"
            );
        }
    }
}
```

**Also update `summary()` method**:
```rust
pub fn summary(&self) -> MetricsSummary {
    let events = match self.events.read() {
        Ok(e) => e.clone(),
        Err(poison_error) => {
            tracing::warn!(
                error = %poison_error,
                "Reading events from poisoned lock, using recovered data"
            );
            poison_error.into_inner().clone()
        }
    };
    // ... same pattern for fallbacks ...
}
```

### Tests Required
- Test that poisoned lock scenario is logged (mock tracing)
- Test that `into_inner()` recovery works for reads

---

## Fix 3: Log Metadata Errors

### Problem
Multiple places use `.ok()` to silently convert metadata building errors to `None`.

### Design

**File**: `src/server/tools.rs`

**Before** (multiple locations):
```rust
let metadata = self.build_metadata_for_linear(...)
    .await
    .ok();
```

**After**:
```rust
let metadata = match self.build_metadata_for_linear(...).await {
    Ok(m) => Some(m),
    Err(e) => {
        tracing::warn!(
            tool = "reasoning_linear",
            error = %e,
            "Failed to build metadata, returning response without enrichment"
        );
        None
    }
};
```

**Affected Methods** (by line number in tools.rs):
- Line 99-101: `reasoning_linear`
- Line 299: `reasoning_tree`
- Line 353-362: `reasoning_divergent`
- Line 512: `reasoning_reflection`
- Line 916: `reasoning_graph`

### Pattern for Consistent Logging

Create a helper function:

```rust
async fn build_metadata_with_logging<F, T>(
    tool_name: &str,
    build_fn: F,
) -> Option<T>
where
    F: Future<Output = Result<T, crate::error::AppError>>,
{
    match build_fn.await {
        Ok(metadata) => Some(metadata),
        Err(e) => {
            tracing::warn!(
                tool = tool_name,
                error = %e,
                "Metadata enrichment failed, returning response without metadata"
            );
            None
        }
    }
}
```

### Tests Required
- Test that metadata errors are logged with correct tool name
- Test that response still returns successfully without metadata

---

## Fix 4: Add Request Lifecycle Logging

### Problem
No visibility into when tool handlers start, when API calls are made, or when they complete.

### Design

**Pattern**: Add structured logging at entry, API call, and exit points.

**File**: `src/server/tools.rs`

```rust
#[tool(name = "reasoning_linear", ...)]
async fn reasoning_linear(&self, req: Parameters<LinearRequest>) -> LinearResponse {
    let req = req.0;
    let request_id = uuid::Uuid::new_v4().to_string();
    let content_length = req.content.len();

    tracing::info!(
        request_id = %request_id,
        tool = "reasoning_linear",
        content_length = content_length,
        session_id = ?req.session_id,
        "Tool invocation started"
    );

    let timer = Timer::start();

    // ... existing processing ...

    let elapsed_ms = timer.elapsed_ms();
    let success = result.is_ok();

    tracing::info!(
        request_id = %request_id,
        tool = "reasoning_linear",
        elapsed_ms = elapsed_ms,
        success = success,
        "Tool invocation completed"
    );

    // ... rest of handler ...
}
```

**File**: `src/anthropic/client.rs`

Add logging around API calls:

```rust
async fn execute_once(&self, request: &ApiRequest) -> Result<ReasoningResponse, AnthropicError> {
    let url = format!("{}/messages", self.config.base_url);

    tracing::debug!(
        url = %url,
        model = %request.model,
        max_tokens = ?request.max_tokens,
        thinking_budget = ?request.thinking.as_ref().map(|t| t.budget_tokens),
        "Starting Anthropic API request"
    );

    let start = std::time::Instant::now();

    let response = self.client
        .post(&url)
        // ... headers and body ...
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                url = %url,
                elapsed_ms = start.elapsed().as_millis() as u64,
                error = %e,
                "Anthropic API request failed"
            );
            // ... error conversion ...
        })?;

    tracing::debug!(
        url = %url,
        status = %response.status(),
        elapsed_ms = start.elapsed().as_millis() as u64,
        "Anthropic API response received"
    );

    // ... rest of method ...
}
```

### Log Levels
- `INFO`: Tool start/complete (always visible at default log level)
- `DEBUG`: API request details (visible with LOG_LEVEL=debug)
- `WARN`: Non-fatal issues (metadata failures, fallbacks)
- `ERROR`: Fatal issues (timeouts, RwLock poisoning)

### Tests Required
- Test that log messages contain expected fields
- Test that timing information is accurate

---

## Fix 5: Log Environment Parsing Failures

### Problem
Invalid environment variable values are silently replaced with defaults in `SelfImprovementConfig::from_env()`.

### Design

**File**: `src/config/self_improvement.rs`

**Before**:
```rust
let min_invocations_for_analysis = env::var("SELF_IMPROVEMENT_MIN_INVOCATIONS")
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(DEFAULT_MIN_INVOCATIONS);
```

**After**:
```rust
let min_invocations_for_analysis = match env::var("SELF_IMPROVEMENT_MIN_INVOCATIONS") {
    Ok(value) => match value.parse::<u64>() {
        Ok(parsed) => parsed,
        Err(e) => {
            tracing::warn!(
                var = "SELF_IMPROVEMENT_MIN_INVOCATIONS",
                value = %value,
                error = %e,
                default = DEFAULT_MIN_INVOCATIONS,
                "Invalid environment variable value, using default"
            );
            DEFAULT_MIN_INVOCATIONS
        }
    },
    Err(_) => DEFAULT_MIN_INVOCATIONS, // Not set, use default (no warning needed)
};
```

**Also apply to**:
- `SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS`
- `SELF_IMPROVEMENT_MAX_ACTIONS`
- `SELF_IMPROVEMENT_CIRCUIT_BREAKER_THRESHOLD`

**File**: `src/config/mod.rs`

Apply same pattern to:
- `REQUEST_TIMEOUT_MS`
- `REQUEST_TIMEOUT_DEEP_MS`
- `REQUEST_TIMEOUT_MAXIMUM_MS`
- `MAX_RETRIES`

### Tests Required
- Test that valid values are parsed correctly
- Test that invalid values log warning and use default
- Test that missing values silently use default (no warning)

---

## Implementation Order

### Phase 1: Critical Fixes (Observability)
1. **Fix 4: Request Lifecycle Logging** - Most impactful for debugging
2. **Fix 2: Metrics RwLock Logging** - Prevents silent data loss

### Phase 2: Error Visibility
3. **Fix 3: Metadata Error Logging** - Shows enrichment failures
4. **Fix 5: Environment Parsing Logging** - Shows config issues

### Phase 3: Timeout Protection
5. **Fix 1: Transport Timeout** - Prevents infinite hangs

---

## File Change Summary

| File | Changes |
|------|---------|
| `src/error/mod.rs` | Add `Timeout` variant to `AppError` |
| `src/server/tools.rs` | Add timeout wrapper, lifecycle logging, metadata error logging |
| `src/metrics/mod.rs` | Add RwLock failure logging |
| `src/anthropic/client.rs` | Add API request/response logging |
| `src/config/self_improvement.rs` | Add parse failure warnings |
| `src/config/mod.rs` | Add parse failure warnings |

---

## Success Criteria

After implementation:

1. **No silent hangs** - Tool execution times out with clear error after 5 minutes
2. **Metrics never lost silently** - Poisoned locks are logged
3. **Metadata failures visible** - Warnings show which tools failed enrichment
4. **Full request tracing** - Every tool call has start/end logs with timing
5. **Config issues surfaced** - Invalid env vars trigger warnings

---

## Estimated Effort

| Fix | Complexity | Estimated Lines Changed |
|-----|------------|------------------------|
| Fix 1: Timeout | Medium | ~100 |
| Fix 2: RwLock Logging | Low | ~30 |
| Fix 3: Metadata Logging | Low | ~50 |
| Fix 4: Lifecycle Logging | Medium | ~100 |
| Fix 5: Env Parsing | Low | ~60 |

**Total**: ~340 lines of code changes + tests
