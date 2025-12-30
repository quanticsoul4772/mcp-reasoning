# MCP Reasoning Server - Implementation Plan

**Purpose**: Step-by-step execution guide for Claude Code to implement the mcp-reasoning server.

**Methodology**: Test-Driven Development (TDD) with 100% coverage enforced at every checkpoint.

**Repository**: `https://github.com/quanticsoul4772/mcp-reasoning`

---

## Execution Protocol

### For Each File
```
1. Write tests FIRST (in same file or tests/ directory)
2. Run tests -> MUST FAIL (red)
3. Implement minimum code to pass
4. Run tests -> MUST PASS (green)
5. Run coverage -> MUST be 100%
6. Refactor if needed (maintain green + 100%)
7. Commit
```

### Checkpoint Commands
```bash
# After each file/step:
cargo build                           # Must compile
cargo test                            # Must pass
cargo clippy -- -D warnings           # Must pass
cargo llvm-cov --fail-under-lines 100 # Must pass

# Full validation:
cargo fmt --check && cargo clippy -- -D warnings && cargo llvm-cov --fail-under-lines 100
```

---

## Dependency Graph

```
Level 0: Scaffolding + Coverage Infrastructure (no deps)
    ↓
Level 1: Error Types (thiserror)
    ↓
Level 2: Config (Error)
    ↓
Level 3: Traits + Mock Infrastructure (Error, Config, chrono, mockall)
    ↓
Level 4: Storage Types, Anthropic Types (Traits)
    ↓
Level 5: Storage Impl, Anthropic Client + Extended Thinking + Vision (Types)
    ↓
Level 6: ModeCore, Prompts (Storage, Anthropic)
    ↓
Level 7: Individual Modes (ModeCore)
    ↓
Level 8: Tool Schemas, Handlers (Modes)
    ↓
Level 9: MCP Server, Transport Layer (rmcp, Handlers)
         ├── Stdio Transport (primary)
         └── HTTP Transport (optional)
    ↓
Level 10: Presets (5 built-in), Metrics (Server)
    ↓
Level 11: Self-Improvement (Everything)
    ↓
Level 12: Integration Tests (Everything)
    ↓
Level 12.5: Client Integration Testing (Claude Code, Claude Desktop)
    ↓
Level 13: Deployment (GitHub, Release)
```

---

## Phase 0: Scaffolding

**Goal**: Project compiles with empty stubs, CI/CD works, coverage infrastructure ready.

### Step 0.1: Create Repository Structure

```bash
# Create all directories
mkdir -p mcp-reasoning/{.cargo,.github/workflows,data,docs,migrations}
mkdir -p mcp-reasoning/src/{anthropic,config,error,metrics,modes,presets,prompts,self_improvement,server,storage}
mkdir -p mcp-reasoning/tests/{common,integration,modes}

# Create placeholder files
touch mcp-reasoning/data/.gitkeep

# Create migration file (from DESIGN.md Section 18.15)
touch mcp-reasoning/migrations/001_initial_schema.sql
```

### Step 0.2: Create Config Files

**Files to create** (copy from DESIGN.md Section 18):
1. `Cargo.toml` (Section 18.2)
2. `rust-toolchain.toml` (Section 18.3)
3. `.cargo/config.toml` (Section 18.4)
4. `.gitignore` (Section 18.5)
5. `.env.example` (Section 18.6)
6. `codecov.yml` (Section 18.18)
7. `.github/workflows/ci.yml` (Section 18.16)
8. `.github/workflows/coverage.yml` (Section 18.17)

### Step 0.3: Create Minimal Compiling Stubs

**Order matters** - create in this sequence:

1. **src/error/mod.rs** - Empty module
```rust
//! Error types for the MCP Reasoning Server.
```

2. **src/config/mod.rs** - Empty module
```rust
//! Configuration management.
```

3. **src/traits.rs** - Empty module
```rust
//! Trait definitions for mockable dependencies.
```

4. **src/anthropic/mod.rs** - Empty module
```rust
//! Anthropic API client.
```

5. **src/storage/mod.rs** - Empty module
```rust
//! Storage backend.
```

6. **src/modes/mod.rs** - Empty module
```rust
//! Reasoning modes.
```

7. **src/prompts/mod.rs** - Empty module
```rust
//! Prompt templates.
```

8. **src/presets/mod.rs** - Empty module
```rust
//! Workflow presets.
```

9. **src/metrics/mod.rs** - Empty module
```rust
//! Metrics collection.
```

10. **src/self_improvement/mod.rs** - Empty module
```rust
//! Self-improvement system.
```

11. **src/server/mod.rs** - Empty module
```rust
//! MCP server implementation.
```

12. **src/lib.rs** - Declare modules (simplified)
```rust
//! MCP Reasoning Server
#![forbid(unsafe_code)]

pub mod anthropic;
pub mod config;
pub mod error;
pub mod metrics;
pub mod modes;
pub mod presets;
pub mod prompts;
pub mod self_improvement;
pub mod server;
pub mod storage;
pub mod traits;
```

13. **src/main.rs** - Minimal main
```rust
//! MCP Reasoning Server binary.
fn main() {
    println!("mcp-reasoning starting...");
}
```

### Step 0.4: Verify Scaffold

```bash
cd mcp-reasoning
cargo build              # Should compile
cargo test               # Should pass (no tests)
cargo clippy             # Should pass
```

### Step 0.5: Rust Best Practices Configuration

**Add lint configuration to lib.rs** (from DESIGN.md Section 15):
```rust
//! MCP Reasoning Server
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]
```

**Verify Cargo.toml lints section**:
```toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

**File Size Limits** (from LESSONS_LEARNED.md):
| File Type | Max Lines | Action if Exceeded |
|-----------|-----------|-------------------|
| Any .rs file | 500 | Split into submodules |
| mod.rs | 200 | Move types to separate files |
| Test file | 500 | Organize with nested modules |
| main.rs | 100 | Keep entry point minimal |

**Enforcement**: Run `wc -l src/**/*.rs` periodically and refactor if limits exceeded.

**Structured Logging** (from LESSONS_LEARNED.md):
```rust
// Use tracing with structured fields (not println! or log!)
tracing::info!(
    mode = %mode_name,
    session_id = %session,
    latency_ms = elapsed.as_millis(),
    "Reasoning complete"
);

// All logs to stderr (stdout reserved for MCP JSON-RPC)
tracing_subscriber::fmt()
    .with_writer(std::io::stderr)
    .init();
```

**Zero Unsafe Code Policy** (from LESSONS_LEARNED.md):
- No `unsafe` blocks anywhere
- No `.unwrap()` in production paths (use `?` or `.unwrap_or()`)
- No `.expect()` in handlers (use proper error handling)
- No `panic!()` macros (return errors instead)

### Step 0.6: Coverage Infrastructure Setup

**Install coverage tooling**:
```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

**Create pre-commit hook** (scripts/pre-commit-coverage.sh):
```bash
#!/bin/bash
set -e
echo "Running coverage check..."
COVERAGE=$(cargo llvm-cov --json 2>/dev/null | jq -r '.data[0].totals.lines.percent')
if (( $(echo "$COVERAGE < 100" | bc -l) )); then
    echo "Coverage is ${COVERAGE}%, required 100%"
    cargo llvm-cov --show-missing-lines 2>/dev/null | grep -E "^\s+\d+\|" | head -20
    exit 1
fi
echo "Coverage: ${COVERAGE}%"
```

**Verify coverage commands work**:
```bash
cargo llvm-cov --version        # Verify installed
cargo llvm-cov                  # Should show 100% (no code yet)
```

### Step 0.7: Mock Infrastructure Scaffolding

**Create src/test_utils.rs** (from DESIGN.md Section 16.7):
```rust
//! Test utilities and mock factories.
#![cfg(test)]

// Placeholder - will be populated in Phase 3
```

**Create tests/common/mod.rs**:
```rust
//! Shared test utilities and fixtures.

// Placeholder - will be populated during testing phases
```

### Step 0.8: Git Initial Commit

```bash
git init
git add .
git commit -m "Initial scaffold with coverage infrastructure"
```

**Checkpoint 0 Complete**: Project compiles, coverage tooling works, lint configuration active.

---

## Phase 1: Error Types

**Goal**: All error types defined with 100% test coverage.

### Step 1.1: Error Types (src/error/mod.rs)

**TDD Workflow**:

1. **Write tests FIRST** at bottom of file
2. **Run tests** -> Should fail (types don't exist)
3. **Implement types** to make tests pass
4. **Run coverage** -> Must be 100%

**File content** (from DESIGN.md Section 18.9):
- `AppError` enum with variants: Anthropic, Storage, Config, Mcp, Mode
- `StorageError` enum with variants: Connection, Query, SessionNotFound, Migration
- `ConfigError` enum with variants: MissingEnvVar, Invalid
- `ModeError` enum with variants: Validation, ApiUnavailable, Timeout, SessionRequired, InvalidOperation

**Tests to write**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_impl_all;

    // Type assertions
    assert_impl_all!(AppError: Send, Sync, std::error::Error);
    assert_impl_all!(StorageError: Send, Sync, std::error::Error);
    assert_impl_all!(ConfigError: Send, Sync, std::error::Error);
    assert_impl_all!(ModeError: Send, Sync, std::error::Error);

    // Display tests for each variant
    #[test]
    fn test_app_error_display_anthropic() { ... }

    #[test]
    fn test_app_error_display_storage() { ... }

    // From impl tests
    #[test]
    fn test_app_error_from_storage_error() { ... }

    // ... one test per error variant
}
```

**Checkpoint**:
```bash
cargo test -p mcp-reasoning error
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 2: Configuration

**Goal**: Config loading and validation with 100% coverage.

### Step 2.1: Config Validation (src/config/validation.rs)

**Write tests FIRST**:
- `test_valid_config` - all fields valid
- `test_empty_api_key` - empty key rejected
- `test_timeout_too_low` - <1000ms rejected
- `test_timeout_too_high` - >300000ms rejected
- `test_retries_too_high` - >10 rejected
- `test_boundary_timeout_min` - 1000ms accepted
- `test_boundary_timeout_max` - 300000ms accepted

**Then implement** `validate_config()` function.

### Step 2.2: Config Loading (src/config/mod.rs)

**Write tests FIRST**:
- `test_config_from_env_with_all_vars`
- `test_config_from_env_defaults`
- `test_config_missing_api_key`
- `test_config_invalid_timeout_format`
- `test_config_invalid_retries_format`

**Then implement** `Config::from_env()`.

**Checkpoint**:
```bash
cargo test -p mcp-reasoning config
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 3: Traits

**Goal**: Mockable trait definitions for all external dependencies.

### Step 3.1: Core Traits (src/traits.rs)

Define traits with `#[cfg_attr(test, mockall::automock)]`:
- `AnthropicClientTrait` - async complete method
- `StorageTrait` - session and thought CRUD
- `TimeProvider` - time abstraction for testing

Define shared types:
- `Message` (role, content)
- `CompletionConfig` (max_tokens, temperature)
- `CompletionResponse` (content, usage)
- `Usage` (input_tokens, output_tokens)
- `Session` (id, created_at)
- `Thought` (id, session_id, content, mode, confidence, created_at)

**Tests**:
- Test `RealTimeProvider::now()` returns current time
- Test type constructors work
- Compile-time mock generation verification

**Checkpoint**:
```bash
cargo test -p mcp-reasoning traits
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 4: Storage Layer

**Goal**: SQLite implementation with full CRUD operations.

### Step 4.1: Storage Types (src/storage/types.rs)

Define all storage-specific types:
- `StoredSession`
- `StoredThought`
- `StoredBranch`
- `StoredCheckpoint`
- `StoredGraphNode`
- `StoredGraphEdge`

**Tests**: Serialization/deserialization, Default impls.

### Step 4.2: Storage Trait (src/storage/mod.rs)

Re-export types, define `Storage` struct and impl.

### Step 4.3: SQLite Implementation (src/storage/sqlite.rs)

**Write tests FIRST for each operation** (use `#[serial]` for DB isolation):

```rust
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_create_session() { ... }

#[tokio::test]
async fn test_get_session_exists() { ... }

#[tokio::test]
async fn test_get_session_not_found() { ... }

#[tokio::test]
async fn test_save_thought() { ... }

#[tokio::test]
async fn test_get_thoughts_for_session() { ... }

// ... many more tests
```

**Key operations to implement**:
- Connection/pool management
- Migration execution
- Session CRUD
- Thought CRUD
- Branch CRUD
- Checkpoint CRUD
- Graph node/edge CRUD
- Metrics storage

### Step 4.4: Session Operations (src/storage/session.rs)

Focused session operations, split from sqlite.rs if needed.

### Step 4.5: Thought Operations (src/storage/thought.rs)

Focused thought operations.

### Step 4.6: Graph Operations (src/storage/graph.rs)

Graph node and edge operations.

**Checkpoint**:
```bash
cargo sqlx prepare --database-url "sqlite:./data/reasoning.db"
cargo test -p mcp-reasoning storage
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 5: Anthropic Client

**Goal**: HTTP client with retry logic, streaming, extended thinking.

### Step 5.1: Anthropic Types (src/anthropic/types.rs)

Request/response types:
- `ApiRequest`
- `ApiResponse`
- `ApiError`
- `ContentBlock`
- `ThinkingBlock`

### Step 5.2: Anthropic Config (src/anthropic/config.rs)

**Write tests FIRST**:
- `test_model_config_defaults`
- `test_thinking_config_standard`
- `test_thinking_config_deep`
- `test_thinking_config_maximum`
- `test_thinking_config_minimum_enforced`

**Implement** (from DESIGN.md Section 11.2):

```rust
/// Model configuration per reasoning mode
#[derive(Debug, Clone)]
pub struct ModeConfig {
    pub model: String,
    pub temperature: Option<f64>,
    pub max_tokens: u32,
    pub thinking: Option<ThinkingConfig>,
    pub streaming: bool,
    pub tools: Option<Vec<ToolDefinition>>,
}

/// Extended Thinking configuration (Claude 3.5 Sonnet, Claude 4+)
#[derive(Debug, Clone, Serialize)]
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    pub type_: String,        // Always "enabled"
    pub budget_tokens: u32,   // Minimum 1024, recommended 2048-10000
}

impl ThinkingConfig {
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            type_: "enabled".to_string(),
            budget_tokens: budget_tokens.max(1024),  // Enforce minimum
        }
    }

    /// Standard budget (4096) for reflection/analysis modes
    pub fn standard() -> Self { Self::enabled(4096) }

    /// Deep budget (8192) for complex decision/evidence modes
    pub fn deep() -> Self { Self::enabled(8192) }

    /// Maximum budget (16384) for counterfactual/mcts modes
    pub fn maximum() -> Self { Self::enabled(16384) }
}
```

**Mode-specific thinking budgets** (from DESIGN.md Section 11.4):
| Mode | Thinking Budget |
|------|-----------------|
| Linear, Tree, Auto, Checkpoint | None (fast modes) |
| Divergent, Graph | Standard (4096) |
| Reflection, Decision, Evidence | Deep (8192) |
| Counterfactual, MCTS | Maximum (16384) |

### Step 5.2b: Vision Support Types (src/anthropic/types.rs)

**Add vision content types** (from DESIGN.md Section 11.2):

```rust
/// Vision content for image-based reasoning
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ImageSource {
    #[serde(rename = "base64")]
    Base64 { media_type: String, data: String },
    #[serde(rename = "url")]
    Url { url: String },
}
```

**Tests**:
- `test_content_part_text_serialization`
- `test_content_part_image_base64_serialization`
- `test_content_part_image_url_serialization`

### Step 5.3: Anthropic Client (src/anthropic/client.rs)

**Request Size Limits** (from LESSONS_LEARNED.md):
```rust
const MAX_REQUEST_BYTES: usize = 100_000;  // 100KB
const MAX_MESSAGES: usize = 50;
const MAX_CONTENT_LENGTH: usize = 50_000;  // 50KB per message

fn validate_request(req: &ApiRequest) -> Result<(), AnthropicError> {
    if req.messages.len() > MAX_MESSAGES {
        return Err(AnthropicError::InvalidRequest {
            message: format!("Too many messages: {} > {}", req.messages.len(), MAX_MESSAGES),
        });
    }
    for msg in &req.messages {
        if msg.content.len() > MAX_CONTENT_LENGTH {
            return Err(AnthropicError::InvalidRequest {
                message: format!("Message too large: {} > {}", msg.content.len(), MAX_CONTENT_LENGTH),
            });
        }
    }
    Ok(())
}
```

**Retry Logic with Exponential Backoff** (from LESSONS_LEARNED.md):
```rust
async fn execute_with_retry(&self, request: ApiRequest) -> Result<ApiResponse, AnthropicError> {
    let mut last_error = None;
    let mut delay = self.config.retry_delay_ms;

    for attempt in 0..=self.config.max_retries {
        if attempt > 0 {
            tracing::warn!(attempt, delay_ms = delay, "Retrying Anthropic request");
            tokio::time::sleep(Duration::from_millis(delay)).await;
            delay *= 2;  // Exponential backoff
        }

        match self.execute_once(&request).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                if !e.is_retryable() {
                    return Err(e);
                }
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or(AnthropicError::Network {
        message: "Unknown error after retries".to_string(),
    }))
}
```

**Write tests FIRST with mocked HTTP**:
- `test_complete_success`
- `test_complete_rate_limit_retry`
- `test_complete_server_error_retry`
- `test_complete_max_retries_exceeded`
- `test_complete_timeout`
- `test_complete_invalid_api_key`
- `test_validate_request_too_many_messages`
- `test_validate_request_message_too_large`
- `test_retry_exponential_backoff`

**Implement**:
- `AnthropicClient::new()`
- `AnthropicClient::complete()` with retry logic
- `validate_request()` for size limits
- Error mapping with `is_retryable()` method

### Step 5.4: Streaming Support (src/anthropic/streaming.rs)

**Write tests FIRST**:
- `test_stream_complete_message`
- `test_stream_partial_chunks`
- `test_stream_error_mid_stream`

**Implement**:
- `StreamEvent` enum
- `complete_streaming()` method

**Checkpoint**:
```bash
cargo test -p mcp-reasoning anthropic
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 6: Mode Infrastructure

**Goal**: ModeCore composition pattern and prompt templates.

### Step 6.1: Prompts (src/prompts/mod.rs, core.rs, advanced.rs)

Define prompt templates for each mode:
- Linear reasoning prompt
- Tree branching prompt
- Divergent perspectives prompt
- etc.

**Tests**: Each prompt returns valid non-empty string.

### Step 6.2: ModeCore (src/modes/core.rs)

Shared mode infrastructure:
- Storage reference
- Anthropic client reference
- Common helpers

**JSON Extraction Robustness** (from LESSONS_LEARNED.md):
```rust
/// Extract JSON from LLM response, handling multiple formats
pub fn extract_json(text: &str) -> Result<serde_json::Value, ModeError> {
    // Fast path: Try raw JSON parse first
    if let Ok(value) = serde_json::from_str(text) {
        return Ok(value);
    }

    // Fallback: Extract from ```json code blocks
    if let Some(start) = text.find("```json") {
        let start = start + 7;
        if let Some(end) = text[start..].find("```") {
            let json_str = &text[start..start + end].trim();
            return serde_json::from_str(json_str)
                .map_err(|e| ModeError::JsonParseFailed {
                    message: format!("Failed to parse JSON block: {}", e),
                });
        }
    }

    // Clear error with truncated preview
    let preview = if text.len() > 100 {
        format!("{}...", &text[..100])
    } else {
        text.to_string()
    };
    Err(ModeError::JsonParseFailed {
        message: format!("No valid JSON found in response: {}", preview),
    })
}

/// Serialize complex values for logging (truncated)
pub fn serialize_for_log<T: serde::Serialize>(value: &T, max_len: usize) -> String {
    match serde_json::to_string(value) {
        Ok(s) if s.len() <= max_len => s,
        Ok(s) => format!("{}...", &s[..max_len]),
        Err(_) => "<serialization failed>".to_string(),
    }
}
```

**Write tests FIRST**:
- `test_extract_json_raw_valid`
- `test_extract_json_code_block`
- `test_extract_json_nested_code_block`
- `test_extract_json_invalid_returns_error`
- `test_extract_json_error_includes_preview`
- `test_serialize_for_log_short`
- `test_serialize_for_log_truncates_long`

### Step 6.3: Mode Module Setup (src/modes/mod.rs)

- Re-exports
- `ReasoningMode` enum
- `FromStr` impl for mode parsing

**Checkpoint**:
```bash
cargo test -p mcp-reasoning modes::core
cargo test -p mcp-reasoning prompts
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 7: Core Modes

**Goal**: Implement 6 core reasoning modes with full test coverage.

### Step 7.1: Linear Mode (src/modes/linear.rs)

**Simplest mode - implement first as pattern for others.**

**Tests FIRST**:
```rust
#[tokio::test]
async fn test_linear_process_success() { ... }

#[tokio::test]
async fn test_linear_process_empty_content() { ... }

#[tokio::test]
async fn test_linear_process_api_error() { ... }

#[tokio::test]
async fn test_linear_process_with_session() { ... }

#[tokio::test]
async fn test_linear_process_creates_session() { ... }
```

**Implement**:
- `LinearMode::new()`
- `LinearMode::process()`
- `LinearResponse` struct

### Step 7.2: Tree Mode (src/modes/tree.rs)

**Tests FIRST for all 4 operations**:
- create: start exploration, generate branches
- focus: select branch
- list: show all branches
- complete: mark finished/abandoned

### Step 7.3: Divergent Mode (src/modes/divergent.rs)

**Tests FIRST**:
- Basic perspectives generation
- challenge_assumptions flag
- force_rebellion flag
- Multiple perspectives validation

### Step 7.4: Reflection Mode (src/modes/reflection.rs)

**Tests FIRST for both operations**:
- process: iterative refinement
- evaluate: session-wide assessment

### Step 7.5: Checkpoint Mode (src/modes/checkpoint.rs)

**Tests FIRST for all 3 operations**:
- create: save state
- list: show checkpoints
- restore: return to checkpoint

### Step 7.6: Auto Mode (src/modes/auto.rs)

**Tests FIRST**:
- Routes to linear for simple content
- Routes to tree for exploration
- Routes to divergent for creative
- Routes to reflection for meta-cognitive
- Handles hints parameter

**Checkpoint**:
```bash
cargo test -p mcp-reasoning modes
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 8: Advanced Modes

**Goal**: Implement 7 advanced reasoning modes.

### Step 8.1: Graph Mode (src/modes/graph.rs)

**Most complex mode - 8 operations**:
- init: create graph with root node
- generate: expand k nodes from current
- score: evaluate node quality
- aggregate: merge multiple nodes
- refine: improve via self-critique
- prune: remove low-scoring nodes
- finalize: extract conclusions
- state: get graph structure

**Tests**: One test per operation + error cases.

### Step 8.2: Detect Mode (src/modes/detect.rs)

**2 operations**:
- biases: detect cognitive biases
- fallacies: detect logical fallacies

**Tests**: Known biases detected, known fallacies detected.

### Step 8.3: Decision Mode (src/modes/decision.rs)

**4 operations**:
- weighted: weighted sum scoring
- pairwise: direct comparison
- topsis: ideal-point distance
- perspectives: stakeholder mapping

**Tests**: Each method produces ranked results.

### Step 8.4: Evidence Mode (src/modes/evidence.rs)

**2 operations**:
- assess: source credibility evaluation
- probabilistic: Bayesian updates

**Tests**: Credibility scoring, prior->posterior calculation.

### Step 8.5: Timeline Mode (src/modes/timeline.rs)

**4 operations**:
- create: new timeline
- branch: fork path
- compare: analyze divergence
- merge: synthesize branches

**Tests**: Branch creation, merge strategies.

### Step 8.6: MCTS Mode (src/modes/mcts.rs)

**2 operations**:
- explore: UCB1-guided search
- auto_backtrack: quality-triggered backtracking

**Tests**: Exploration expands nodes, backtrack triggers on quality drop.

### Step 8.7: Counterfactual Mode (src/modes/counterfactual.rs)

**Single operation**:
- analyze scenario + intervention -> consequences

**Tests**: Pearl's Ladder levels (association, intervention, counterfactual).

**Checkpoint**:
```bash
cargo test -p mcp-reasoning modes
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 9: Server Infrastructure

**Goal**: MCP protocol handling, tool registry, transport with rmcp SDK integration.

### Step 9.1: Tool Definitions with rmcp Macros (src/server/tools.rs)

**Use rmcp macro system** (from DESIGN.md Section 9):

```rust
use rmcp::prelude::*;

/// Reasoning server with all tools
#[derive(Clone)]
pub struct ReasoningServer {
    pub state: Arc<AppState>,
}

#[tool_router]
impl ReasoningServer {
    /// Single-pass sequential reasoning
    #[tool(
        name = "reasoning_linear",
        description = "Process a thought and get a logical continuation with confidence scoring.",
        annotations(
            title = "Linear Reasoning",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn reasoning_linear(
        &self,
        #[arg(description = "Thought content to process")] content: String,
        #[arg(description = "Session ID for context continuity")] session_id: Option<String>,
        #[arg(description = "Confidence threshold (0.0-1.0)")] confidence: Option<f64>,
    ) -> Result<LinearResponse, ToolError> {
        // Implementation calls mode
    }

    // ... 14 more tool definitions following same pattern
}
```

**Write tests FIRST**:
- `test_tool_linear_schema_valid`
- `test_tool_tree_schema_valid`
- `test_tool_divergent_schema_valid`
- ... (one per tool)
- `test_all_tools_have_descriptions`
- `test_tool_annotations_present`

**All 15 tools to define**:
1. `reasoning_linear` - Single-pass sequential reasoning
2. `reasoning_tree` - Branching exploration (create/focus/list/complete)
3. `reasoning_divergent` - Multi-perspective with force_rebellion
4. `reasoning_reflection` - Meta-cognitive (process/evaluate)
5. `reasoning_checkpoint` - State management (create/list/restore)
6. `reasoning_auto` - Mode selection router
7. `reasoning_graph` - Graph-of-Thoughts (8 operations)
8. `reasoning_detect` - Bias/fallacy detection
9. `reasoning_decision` - weighted/pairwise/topsis/perspectives
10. `reasoning_evidence` - Credibility assessment, Bayesian updates
11. `reasoning_timeline` - Temporal reasoning (create/branch/compare/merge)
12. `reasoning_mcts` - UCB1-guided search, auto_backtrack
13. `reasoning_counterfactual` - Pearl's Ladder causal analysis
14. `reasoning_preset` - Workflow preset execution
15. `reasoning_metrics` - Usage metrics queries

### Step 9.1b: Response Types with JsonSchema (src/server/tools.rs)

**Derive JsonSchema for automatic schema generation**:
```rust
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LinearResponse {
    pub thought_id: String,
    pub session_id: String,
    pub content: String,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub confidence: f64,
    pub next_step: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TreeResponse {
    pub session_id: String,
    pub branch_id: Option<String>,
    pub branches: Option<Vec<Branch>>,
    pub recommendation: Option<String>,
}

// ... similar for all response types
```

**Tests**: Each response type serializes to valid JSON schema.

### Step 9.2: Handler Registry (src/server/handlers.rs)

**Tool Registry Pattern** (from LESSONS_LEARNED.md - avoids 1600+ line match statement):
```rust
pub struct HandlerRegistry {
    handlers: HashMap<String, Box<dyn ToolHandler>>,
}

impl HandlerRegistry {
    pub fn new(state: Arc<AppState>) -> Self {
        let mut handlers: HashMap<String, Box<dyn ToolHandler>> = HashMap::new();
        handlers.insert("reasoning_linear".to_string(), Box::new(LinearHandler::new(state.clone())));
        handlers.insert("reasoning_tree".to_string(), Box::new(TreeHandler::new(state.clone())));
        // ... 13 more handlers
        Self { handlers }
    }

    pub async fn handle(&self, tool_name: &str, args: Value) -> Result<Value, McpError> {
        self.handlers
            .get(tool_name)
            .ok_or_else(|| McpError::UnknownTool { tool: tool_name.to_string() })?
            .handle(args)
            .await
    }
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn handle(&self, args: Value) -> Result<Value, McpError>;
}
```

**Parameter Struct Pattern** (from LESSONS_LEARNED.md - avoids 15+ identical structs):
```rust
// Common base for all reasoning tools
#[derive(Debug, Clone, Deserialize)]
pub struct ReasoningParams {
    pub content: String,
    pub session_id: Option<String>,
    pub confidence: Option<f64>,
}

// Mode-specific extensions via enum
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode")]
pub enum ModeParams {
    Linear(ReasoningParams),
    Tree {
        #[serde(flatten)]
        base: ReasoningParams,
        operation: Option<TreeOperation>,
        branch_id: Option<String>,
        num_branches: Option<u32>,
    },
    Divergent {
        #[serde(flatten)]
        base: ReasoningParams,
        num_perspectives: Option<u32>,
        challenge_assumptions: Option<bool>,
        force_rebellion: Option<bool>,
    },
    // ... other modes with specific fields
}

impl ModeParams {
    pub fn base(&self) -> &ReasoningParams {
        match self {
            ModeParams::Linear(base) => base,
            ModeParams::Tree { base, .. } => base,
            ModeParams::Divergent { base, .. } => base,
            // ...
        }
    }
}
```

**Tests**:
- `test_handler_registry_routes_correctly`
- `test_handler_registry_unknown_tool`
- `test_reasoning_params_deserialize`
- `test_mode_params_linear`
- `test_mode_params_tree_with_operation`
- `test_mode_params_base_extraction`

### Step 9.3: MCP Protocol (src/server/mcp.rs)

JSON-RPC message parsing and response formatting.

**Tests**:
- Parse valid request
- Parse invalid request
- Format success response
- Format error response

### Step 9.4: Transport Layer (src/server/transport.rs)

**Implement both transport types** (from DESIGN.md Section 8):

#### Step 9.4a: Stdio Transport (Primary for Claude Code)

```rust
use rmcp::transport::StdioTransport;

pub struct StdioHandler {
    config: TransportConfig,
}

impl StdioHandler {
    pub fn new() -> Self { ... }
    pub async fn run(&self, server: ReasoningServer) -> Result<(), TransportError> { ... }
}
```

**Tests**:
- `test_stdio_read_valid_jsonrpc`
- `test_stdio_read_invalid_json`
- `test_stdio_write_response`
- `test_stdio_handle_eof`
- `test_stdio_large_message`

#### Step 9.4b: Streamable HTTP Transport (from DESIGN.md Section 8.1)

```rust
use axum::{body::Body, http::{header, StatusCode}, response::Response};

pub struct StreamableHttpTransport {
    config: TransportConfig,
}

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub endpoint: String,           // Default: "/mcp"
    pub session_header: String,     // Default: "Mcp-Session-Id"
    pub timeout_ms: u64,            // Default: 300000 (5 min)
    pub max_message_size: usize,    // Default: 10MB
}

#[derive(Debug, Clone)]
pub enum TransportType {
    /// Streamable HTTP (recommended, March 2025+ spec)
    StreamableHttp {
        endpoint: String,
        session_management: SessionMode,
    },
    /// Legacy stdio transport (for CLI tools)
    Stdio,
}

#[derive(Debug, Clone)]
pub enum SessionMode {
    ServerManaged,
    ClientManaged,
    Stateless,
}

impl StreamableHttpTransport {
    pub async fn handle_request(
        &self,
        session_id: Option<String>,
        body: serde_json::Value,
    ) -> Result<Response<Body>, TransportError>;

    pub async fn handle_streaming_request(
        &self,
        session_id: Option<String>,
        body: serde_json::Value,
    ) -> Result<Response<Body>, TransportError>;
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Invalid JSON-RPC: {message}")]
    InvalidJsonRpc { message: String },

    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Message too large: {size} > {max}")]
    MessageTooLarge { size: usize, max: usize },

    #[error("Request timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}
```

**Tests**:
- `test_http_handle_request_success`
- `test_http_handle_request_invalid_json`
- `test_http_session_management`
- `test_http_streaming_response`
- `test_http_message_size_limit`
- `test_http_timeout`

#### Step 9.4c: Transport Selection

```rust
// In main.rs - choose transport based on environment
match std::env::var("MCP_TRANSPORT").as_deref() {
    Ok("http") | Ok("sse") => {
        let transport = StreamableHttpTransport::new(config);
        server.run_http(transport).await?;
    }
    _ => {
        // Default: stdio transport for CLI integration
        let transport = StdioTransport::new();
        server.run_stdio(transport).await?;
    }
}
```

**Tests**:
- `test_transport_selection_http`
- `test_transport_selection_stdio_default`

### Step 9.5: Server Assembly (src/server/mod.rs)

**Implement server with graceful shutdown** (from DESIGN.md Section 15.4):

```rust
use tokio::signal;
use tokio::sync::oneshot;

pub struct McpServer {
    state: Arc<AppState>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl McpServer {
    pub async fn new(config: Config) -> Result<Self, AppError> {
        let storage = SqliteStorage::new(&config.database_path).await?;
        let anthropic = AnthropicClient::new(config.api_key.clone(), config.into())?;
        let state = Arc::new(AppState::new(storage, anthropic));
        Ok(Self { state, shutdown_tx: None })
    }

    pub async fn run(self) -> Result<(), AppError> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Spawn signal handler
        tokio::spawn(async move {
            shutdown_signal().await;
            let _ = shutdown_tx.send(());
        });

        // Run server with graceful shutdown
        tokio::select! {
            result = self.serve_requests() => result,
            _ = shutdown_rx => {
                tracing::info!("Received shutdown signal, cleaning up...");
                self.cleanup().await?;
                Ok(())
            }
        }
    }

    async fn cleanup(&self) -> Result<(), AppError> {
        self.state.metrics.flush().await?;
        self.state.storage.close().await?;
        tracing::info!("Cleanup complete");
        Ok(())
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
```

**Tests**:
- `test_server_new_success`
- `test_server_new_missing_api_key`
- `test_server_run_and_shutdown`
- `test_server_cleanup_flushes_metrics`
- `test_shutdown_signal_ctrl_c`
- `test_shutdown_signal_terminate` (Unix only)

**Checkpoint**:
```bash
cargo test -p mcp-reasoning server
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 10: Presets & Metrics

**Goal**: Workflow presets and usage metrics.

### Step 10.1: Preset Definitions (src/presets/builtin.rs)

**Define 5 built-in presets** (from DESIGN.md Section 3.14):

```rust
/// Built-in workflow presets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub description: String,
    pub category: PresetCategory,
    pub steps: Vec<PresetStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetStep {
    pub mode: ReasoningMode,
    pub operation: Option<String>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PresetCategory {
    CodeQuality,
    Analysis,
    Decision,
    Research,
    Custom,
}
```

**Preset 1: code-review** (Category: CodeQuality)
```rust
PresetStep { mode: Linear, operation: None, config: None },           // Understand code
PresetStep { mode: Detect, operation: Some("biases"), config: None }, // Check for biases
PresetStep { mode: Divergent, operation: None, config: json!({"num_perspectives": 3}) }, // Alternative approaches
PresetStep { mode: Reflection, operation: Some("evaluate"), config: None }, // Final assessment
```

**Preset 2: debug-analysis** (Category: Analysis)
```rust
PresetStep { mode: Linear, operation: None, config: None },           // Understand problem
PresetStep { mode: Tree, operation: Some("create"), config: json!({"num_branches": 3}) }, // Hypotheses
PresetStep { mode: Evidence, operation: Some("assess"), config: None }, // Evaluate evidence
PresetStep { mode: Counterfactual, operation: None, config: None },   // What-if analysis
```

**Preset 3: architecture-decision** (Category: Decision)
```rust
PresetStep { mode: Divergent, operation: None, config: json!({"challenge_assumptions": true}) }, // Options
PresetStep { mode: Decision, operation: Some("weighted"), config: None }, // Score options
PresetStep { mode: Graph, operation: Some("init"), config: None },    // Map dependencies
PresetStep { mode: Mcts, operation: Some("explore"), config: None },  // Explore implications
PresetStep { mode: Reflection, operation: Some("evaluate"), config: None }, // Final decision
```

**Preset 4: strategic-decision** (Category: Decision)
```rust
PresetStep { mode: Decision, operation: Some("perspectives"), config: None }, // Stakeholder views
PresetStep { mode: Timeline, operation: Some("create"), config: None }, // Future scenarios
PresetStep { mode: Timeline, operation: Some("branch"), config: None }, // Alternative paths
PresetStep { mode: Evidence, operation: Some("probabilistic"), config: None }, // Risk assessment
PresetStep { mode: Decision, operation: Some("topsis"), config: None }, // Final ranking
```

**Preset 5: evidence-conclusion** (Category: Research)
```rust
PresetStep { mode: Evidence, operation: Some("assess"), config: None }, // Evaluate sources
PresetStep { mode: Detect, operation: Some("fallacies"), config: None }, // Check reasoning
PresetStep { mode: Graph, operation: Some("init"), config: None },    // Build argument map
PresetStep { mode: Graph, operation: Some("aggregate"), config: None }, // Synthesize
PresetStep { mode: Linear, operation: None, config: None },           // Final conclusion
```

**Tests**:
- `test_preset_code_review_steps`
- `test_preset_debug_analysis_steps`
- `test_preset_architecture_decision_steps`
- `test_preset_strategic_decision_steps`
- `test_preset_evidence_conclusion_steps`
- `test_preset_step_validation`
- `test_preset_serialization`

### Step 10.2: Preset Execution (src/presets/mod.rs)

- `PresetMode::list()` - return available presets
- `PresetMode::run()` - execute preset workflow

**Tests**: List returns presets, run executes steps.

### Step 10.3: Metrics Collection (src/metrics/mod.rs)

- Collect latency, success/failure
- Query by mode, time range
- Fallback metrics

**Tests**: Metrics recorded, queries return correct data.

**Checkpoint**:
```bash
cargo test -p mcp-reasoning presets metrics
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 11: Self-Improvement System

**Goal**: Autonomous 4-phase optimization loop.

### Step 11.1: Types (src/self_improvement/types.rs)

- `SelfImprovementAction`
- `ActionType`
- `ActionStatus`
- `SystemMetrics`
- `Lesson`

### Step 11.2: Monitor (src/self_improvement/monitor.rs)

Collect metrics and detect issues.

**Tests**: Baseline calculation, deviation detection.

### Step 11.3: Analyzer (src/self_improvement/analyzer.rs)

LLM-based diagnosis and action proposal.

**Tests**: Mock LLM returns valid actions.

### Step 11.4: Executor (src/self_improvement/executor.rs)

Execute approved actions with rollback capability.

**Tests**: Action execution, rollback on failure.

### Step 11.5: Learner (src/self_improvement/learner.rs)

Extract lessons from completed actions.

**Tests**: Reward calculation, lesson synthesis.

### Step 11.6: Circuit Breaker (src/self_improvement/circuit_breaker.rs)

Safety mechanism to prevent runaway changes.

**Tests**: Trip on threshold, reset after cooldown.

### Step 11.7: Allowlist (src/self_improvement/allowlist.rs)

Action validation against allowed types.

**Tests**: Allowed actions pass, disallowed rejected.

### Step 11.8: System Orchestration (src/self_improvement/system.rs)

Main loop coordinating all phases.

**Tests**: Full loop execution with mocks.

**Checkpoint**:
```bash
cargo test -p mcp-reasoning self_improvement
cargo llvm-cov --fail-under-lines 100
```

---

## Phase 12: Integration & Polish

**Goal**: End-to-end tests, documentation, release prep.

### Step 12.1: Integration Tests (tests/integration/)

- Full workflow: create session -> use modes -> checkpoint -> restore
- Multi-mode scenarios
- Error recovery paths

### Step 12.2: Main Binary (src/main.rs)

Complete entry point with:
- Logging initialization
- Config loading
- Server startup
- Signal handling

### Step 12.3: Documentation

- Complete README.md
- API reference examples
- doc comments compile

### Step 12.4: Final Validation

```bash
# Full validation suite
cargo fmt --check
cargo clippy -- -D warnings
cargo test --all-features
cargo llvm-cov --fail-under-lines 100
cargo doc --no-deps

# Build release
cargo build --release

# Test binary
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | ./target/release/mcp-reasoning
```

**Checkpoint**: All tests pass, 100% coverage, binary works.

---

## Phase 12.5: Client Integration Testing

**Goal**: Verify the server works correctly with Claude Code and Claude Desktop clients.

### Step 12.5.1: Stdio Protocol Verification

**Protocol behavior** (from DESIGN.md Section 17.6):
```
┌─────────────┐     stdin      ┌─────────────┐
│ Claude Code │───────────────▶│ MCP Server  │
│ or Desktop  │◀───────────────│             │
└─────────────┘     stdout     └─────────────┘
                      │
                   stderr (logs only)
```

**Critical requirement**: Server MUST NOT write anything to stdout except valid JSON-RPC messages. All logging goes to stderr.

**Manual verification tests**:
```bash
# Test 1: tools/list returns valid response
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | ./target/release/mcp-reasoning 2>/dev/null

# Test 2: Verify stderr contains logs (not stdout)
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | ./target/release/mcp-reasoning 2>&1 | grep -c "mcp_reasoning"

# Test 3: Invalid request returns error
echo '{"jsonrpc":"2.0","id":1,"method":"unknown"}' | ./target/release/mcp-reasoning 2>/dev/null
```

### Step 12.5.2: Claude Code Integration

**Add server to Claude Code** (from DESIGN.md Section 17.3):
```bash
# Windows
claude mcp add mcp-reasoning ^
  --transport stdio ^
  --env ANTHROPIC_API_KEY=%ANTHROPIC_API_KEY% ^
  -- %USERPROFILE%\.local\bin\mcp-reasoning.exe

# macOS/Linux
claude mcp add mcp-reasoning \
  --transport stdio \
  --env ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -- ~/.local/bin/mcp-reasoning
```

**Verification steps**:
```bash
# Check server is registered
claude mcp list
# Expected: mcp-reasoning: /path/to/mcp-reasoning -  Connected

# Get detailed config
claude mcp get mcp-reasoning
```

**Test tool invocation in Claude Code**:
```
# In Claude Code conversation:
> Use reasoning_linear to analyze "What makes a good software architecture?"

# Expected: Tool output with thought_id, session_id, content, and confidence
```

### Step 12.5.3: Claude Desktop Configuration

**Config file locations**:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

**Configuration format** (from DESIGN.md Section 17.4):
```json
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "/path/to/mcp-reasoning",
      "args": [],
      "env": {
        "ANTHROPIC_API_KEY": "sk-ant-xxx",
        "DATABASE_PATH": "./data/reasoning.db",
        "LOG_LEVEL": "info"
      }
    }
  }
}
```

**Windows-specific format**:
```json
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "C:\\Users\\username\\.local\\bin\\mcp-reasoning.exe",
      "env": {
        "ANTHROPIC_API_KEY": "sk-ant-xxx"
      }
    }
  }
}
```

### Step 12.5.4: Troubleshooting Verification

**Verify each failure mode** (from DESIGN.md Section 17.9):

| Test Case | Command | Expected Result |
|-----------|---------|-----------------|
| Binary not found | Use wrong path | "Failed to connect" |
| Missing API key | Remove env var | "Failed to connect" |
| Invalid API key | Use fake key | Tool invocation fails with auth error |
| Timeout | Set low timeout | Timeout error in tool response |

### Step 12.5.5: Multi-Session Testing

**Verify session persistence**:
1. Start Claude Code session
2. Use `reasoning_linear` with content
3. Note the `session_id` returned
4. Use `reasoning_checkpoint` to create checkpoint
5. Close Claude Code
6. Reopen Claude Code
7. Use `reasoning_checkpoint` with `operation: list`
8. Verify previous session/checkpoint exists

**Checkpoint**: Server integrates correctly with Claude Code and Claude Desktop.

---

## Phase 13: Deployment

**Goal**: Configure for Claude Code and push to GitHub.

### Step 13.1: Git Final Commit

```bash
git add .
git commit -m "Complete implementation with 100% coverage"
```

### Step 13.2: Create GitHub Repository

```bash
gh repo create quanticsoul4772/mcp-reasoning --public --source=. --push
```

### Step 13.3: Configure Claude Code

```bash
claude mcp add mcp-reasoning \
  --transport stdio \
  --env ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -- ./target/release/mcp-reasoning

claude mcp list  # Verify connected
```

### Step 13.4: Test in Claude Code

```
> Use reasoning_linear to analyze "What makes a good software architecture?"
```

---

## Verification Checklist

Before marking implementation complete:

### Build & Quality
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes (all tests)
- [ ] `cargo clippy -- -D warnings` passes (with pedantic, nursery lints)
- [ ] `cargo llvm-cov --fail-under-lines 100` passes
- [ ] `cargo doc --no-deps` generates docs without warnings
- [ ] `#![forbid(unsafe_code)]` in lib.rs
- [ ] No `.unwrap()` or `.expect()` in production code paths

### Binary Verification
- [ ] Binary responds to `tools/list` request with all 15 tools
- [ ] Binary outputs logs to stderr only (never stdout)
- [ ] Binary handles graceful shutdown (SIGTERM/SIGINT)
- [ ] Binary respects all environment variables

### rmcp SDK Integration
- [ ] All 15 tools use `#[tool]` macro correctly
- [ ] `#[tool_router]` generates proper routing
- [ ] Response types derive `JsonSchema` for automatic schema generation
- [ ] Tool annotations (read_only_hint, destructive_hint, etc.) present

### Transport Layer
- [ ] Stdio transport works with JSON-RPC messages
- [ ] HTTP transport handles requests correctly (if enabled)
- [ ] Transport selection based on MCP_TRANSPORT env var works

### Extended Thinking
- [ ] ThinkingConfig with standard/deep/maximum budgets works
- [ ] Mode-specific thinking budgets applied correctly
- [ ] Extended thinking responses parsed and returned

### Claude Code Integration
- [ ] `claude mcp add` succeeds
- [ ] `claude mcp list` shows connected
- [ ] Test tool invocation in Claude Code works
- [ ] Session persistence across Claude Code restarts works

### Claude Desktop Integration
- [ ] `claude_desktop_config.json` format documented
- [ ] Server starts correctly from Claude Desktop
- [ ] Tool invocation works in Claude Desktop

### Self-Improvement System
- [ ] Monitor collects metrics on every invocation
- [ ] Analyzer generates diagnoses with LLM
- [ ] Executor applies actions with rollback capability
- [ ] Learner calculates rewards and extracts lessons
- [ ] Circuit breaker trips after consecutive failures

### Documentation & Release
- [ ] README.md is complete and accurate
- [ ] Quick Start section works end-to-end
- [ ] Troubleshooting guide covers common issues
- [ ] GitHub repository created and pushed
- [ ] All 5 built-in presets documented

### Lessons Learned Compliance (from LESSONS_LEARNED.md)

**REPLICATE Items:**
- [ ] Error types designed first (hierarchical with thiserror)
- [ ] ModeCore composition pattern (not trait inheritance)
- [ ] Config struct with validation (fail fast on load)
- [ ] JSON extraction robustness (fast path + fallback)
- [ ] Retry logic with exponential backoff
- [ ] Structured logging with tracing (to stderr)
- [ ] Zero unsafe/unwrap/expect/panic policy enforced

**AVOID Items:**
- [ ] No file exceeds 500 lines (run `wc -l src/**/*.rs`)
- [ ] 15 consolidated tools (not 40 separate tools)
- [ ] Tool registry pattern (not giant match statement)
- [ ] Common ReasoningParams base (not 15+ identical structs)
- [ ] Prompts organized by category (not single file)
- [ ] Config validation on load (not runtime discovery)
- [ ] Request size limits enforced (100KB max, 50 messages max)
- [ ] Self-improvement 4-phase loop always enabled
