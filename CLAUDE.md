# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MCP Reasoning Server - A Rust-based MCP server providing structured reasoning capabilities via direct Anthropic Claude API calls. This project offers 32 tools across reasoning (16), self-improvement (6), session management (4), and agent/team coordination (6).

**Status**: Complete. 38,000+ lines of Rust code and 2,620+ tests.

**Key Stats**:

- 119 source files, 38,000+ lines of code
- 2,620+ tests (90%+ line coverage)
- 16 core reasoning tools + 6 SI + 4 session + 6 agent/team = 32 tools total
- 6 workflow presets (code-review, debug-analysis, architecture-decision, strategic-decision, evidence-conclusion, brainstorming)
- 4-phase self-improvement system with safety mechanisms
- Tool chain tracking with pattern detection
- Error enhancement with contextual alternatives
- Streaming API with progress notifications
- Performance optimized: ~45% fewer allocations (2026-03-01)

**Key Documents**:

- `docs/` - [Documentation index](docs/README.md)
- `docs/reference/ARCHITECTURE.md` - System architecture overview
- `docs/reference/API_SPECIFICATION.md` - Complete tool schemas
- `docs/reference/IMPLEMENTATION_DETAILS.md` - Technical implementation
- `docs/reference/LESSONS_LEARNED.md` - Patterns replicated from predecessor
- `docs/reference/MODE_PATTERN.md` - Mode implementation template

## Build & Test Commands

```bash
# Build
cargo build                           # Debug build
cargo build --release                 # Release build

# Test
cargo test                            # Run all tests
cargo test -p mcp-reasoning <module>  # Test specific module (e.g., "error", "config", "modes")

# Quality checks
cargo fmt --check                     # Check formatting
cargo clippy -- -D warnings           # Lint with warnings as errors
cargo llvm-cov                        # Coverage report

# Full validation (run before every commit)
cargo fmt --check && cargo clippy -- -D warnings && cargo test

# Pre-commit hooks (recommended)
pip install pre-commit               # Install pre-commit
pre-commit install                   # Install git hooks
pre-commit run --all-files           # Run manually

# Database
cargo sqlx prepare --database-url "sqlite:./data/reasoning.db"  # Prepare SQLx queries

# File size check (max 500 lines per file)
wc -l src/**/*.rs | sort -n
```

## Environment Variables

```bash
# Required
ANTHROPIC_API_KEY=sk-ant-xxx

# Optional
DATABASE_PATH=./data/reasoning.db    # Default
LOG_LEVEL=info                        # error|warn|info|debug|trace
REQUEST_TIMEOUT_MS=30000              # Default (30s)
MAX_RETRIES=3                         # Default
MCP_TRANSPORT=stdio                   # stdio (default) or http
```

## Architecture

```
┌─────────────┐     stdin      ┌─────────────────┐
│ Claude Code │───────────────▶│   MCP Server    │──────▶ Anthropic API
│ or Desktop  │◀───────────────│     (Rust)      │
└─────────────┘     stdout     └────────┬────────┘
                                        │
                                        ▼
                                     SQLite
```

### Module Structure

```
src/
├── main.rs              # Entry point (<100 lines)
├── lib.rs               # Module declarations + lints
├── traits/              # Mockable traits (AnthropicClientTrait, StorageTrait, TimeProvider)
├── test_utils.rs        # Mock factories (test only)
├── error/
│   ├── mod.rs           # AppError, StorageError, ConfigError, ModeError
│   └── enhanced.rs      # ErrorEnhancer, ComplexityMetrics, contextual alternatives
├── config/
│   ├── mod.rs           # Config struct + from_env()
│   ├── secret.rs        # SecretString wrapper (redacts on Display)
│   ├── self_improvement.rs  # SelfImprovementConfig
│   └── validation.rs    # Validation logic
├── anthropic/
│   ├── client.rs        # AnthropicClient with retry + backoff + streaming
│   ├── types.rs         # Request/Response types, Vision support, StreamEvent
│   ├── config.rs        # ModelConfig, ThinkingConfig (standard/deep/maximum)
│   └── streaming.rs     # SSE parsing, StreamAccumulator
├── storage/
│   ├── mod.rs           # Storage trait + SqliteStorage struct
│   ├── core.rs          # Connection pool + migrations
│   ├── session.rs       # Session CRUD
│   ├── thought.rs       # Thought CRUD
│   ├── branch.rs        # Branch CRUD
│   ├── checkpoint.rs    # Checkpoint CRUD
│   ├── graph.rs         # Graph node/edge CRUD
│   ├── metrics.rs       # Metrics storage
│   ├── actions.rs       # SI action storage
│   ├── agent_metrics.rs # Agent performance storage
│   ├── trait_impl.rs    # StorageTrait implementation
│   └── types.rs         # Storage types
├── prompts/
│   ├── mod.rs           # ReasoningMode enum, Operation enum, get_prompt_for_mode() router
│   ├── core.rs          # linear, tree, divergent, reflection, checkpoint, auto prompts
│   ├── graph.rs         # Graph-of-Thoughts prompts (8 operations)
│   ├── detect.rs        # Bias/fallacy detection prompts
│   ├── decision.rs      # Decision analysis prompts (weighted/pairwise/topsis/perspectives)
│   ├── evidence.rs      # Evidence evaluation prompts (assess/probabilistic)
│   ├── timeline.rs      # Timeline prompts (create/branch/compare/merge)
│   ├── mcts.rs          # MCTS prompts (explore/auto_backtrack)
│   └── counterfactual.rs # Causal analysis prompts (Pearl's Ladder)
├── modes/
│   ├── mod.rs           # Mode exports
│   ├── core.rs          # ModeCore (shared deps) + extract_json() helper
│   ├── linear.rs        # Single-pass sequential
│   ├── tree.rs          # Branching (create/focus/list/complete/summarize)
│   ├── divergent.rs     # Multi-perspective + force_rebellion
│   ├── checkpoint.rs    # State management (create/list/restore)
│   ├── auto.rs          # Mode selection router
│   ├── meta.rs          # Meta-mode (selects based on empirical data)
│   ├── counterfactual.rs # Pearl's Ladder causal analysis
│   ├── reflection/      # Meta-cognitive (process/evaluate)
│   ├── graph/           # Graph-of-Thoughts (8 operations)
│   ├── detect/          # Bias/fallacy detection
│   ├── decision/        # weighted/pairwise/topsis/perspectives
│   ├── evidence/        # Credibility + Bayesian updates
│   ├── timeline/        # Temporal (create/branch/compare/merge)
│   ├── mcts/            # UCB1 search + auto_backtrack
│   └── memory/          # Session memory (list/resume/search/relate + embeddings)
├── server/
│   ├── mod.rs           # McpServer + graceful shutdown
│   ├── mcp.rs           # JSON-RPC protocol
│   ├── transport.rs     # Stdio + HTTP transport
│   ├── progress.rs      # ProgressEvent, ProgressReporter, milestones
│   ├── params.rs        # Tool parameter schemas
│   ├── requests.rs      # Request types with JsonSchema
│   ├── responses.rs     # Response types
│   ├── metadata_builders.rs # Response metadata helpers
│   ├── types.rs         # AppState with progress broadcast channel
│   └── tools/           # 32 tool schemas + per-category handlers
│       ├── mod.rs        # Tool definitions (rmcp macros)
│       ├── handlers_basic.rs    # linear, tree, divergent, reflection, checkpoint, auto
│       ├── handlers_cognitive.rs # detect, meta
│       ├── handlers_decision.rs # decision, evidence
│       ├── handlers_temporal.rs # timeline, mcts, counterfactual
│       ├── handlers_graph.rs    # graph
│       ├── handlers_sessions.rs # list_sessions, resume, search, relate
│       ├── handlers_agents.rs   # agent_invoke, agent_list, skill_run, team_run, team_list, agent_metrics
│       ├── handlers_si.rs       # SI status/diagnoses/approve/reject/trigger/rollback
│       └── handlers_infra.rs    # preset, metrics
├── agents/              # Agent coordination system (invoke/list/team)
├── skills/              # Composable skill system (run/discover/builtin)
├── metadata/            # Tool metadata, suggestions, timing defaults
├── presets/
│   └── mod.rs           # 6 built-in presets (code-review, debug-analysis, architecture-decision,
│                        #   strategic-decision, evidence-conclusion, brainstorming)
├── metrics/mod.rs       # Usage metrics + tool chain tracking (ToolTransition, ChainSummary)
└── self_improvement/
    ├── mod.rs           # Re-exports
    ├── system.rs        # SelfImprovementSystem orchestrator
    ├── manager.rs       # Cycle management + state machine
    ├── monitor.rs       # Phase 1: Metric collection
    ├── analyzer.rs      # Phase 2: LLM diagnosis
    ├── executor.rs      # Phase 3: Action execution + rollback
    ├── learner.rs       # Phase 4: Reward calculation
    ├── baseline.rs      # Performance baseline tracking
    ├── circuit_breaker.rs # Safety: halt on consecutive failures
    ├── allowlist.rs     # Safety: validate action bounds
    ├── types/           # Severity, TriggerMetric, SuggestedAction, etc.
    ├── storage/         # SI-specific storage layer
    ├── anthropic_calls/ # LLM interaction wrappers
    └── cli/             # CLI commands for SI management
```

## The 16 Core Reasoning Tools

| Tool | Description | Operations |
|------|-------------|------------|
| `reasoning_linear` | Sequential step-by-step | single |
| `reasoning_tree` | Branching exploration | create, focus, list, complete, summarize |
| `reasoning_divergent` | Multi-perspective | force_rebellion option |
| `reasoning_reflection` | Meta-cognitive | process, evaluate |
| `reasoning_checkpoint` | State management | create, list, restore |
| `reasoning_auto` | Mode router | automatic selection |
| `reasoning_meta` | Empirical mode selector | classifies problem, picks best tool from historical data |
| `reasoning_graph` | Graph-of-Thoughts | init, generate, score, aggregate, refine, prune, finalize, state |
| `reasoning_detect` | Bias/fallacy detection | biases, fallacies |
| `reasoning_decision` | Decision analysis | weighted, pairwise, topsis, perspectives |
| `reasoning_evidence` | Evidence evaluation | assess, probabilistic |
| `reasoning_timeline` | Temporal reasoning | create, branch, compare, merge |
| `reasoning_mcts` | Monte Carlo Tree Search | explore, auto_backtrack |
| `reasoning_counterfactual` | Causal analysis | Pearl's Ladder levels |
| `reasoning_preset` | Workflow presets | list, run |
| `reasoning_metrics` | Usage queries | by_mode, by_time, etc. |

## Key Design Patterns

### ModeCore Composition (from LESSONS_LEARNED.md)

```rust
pub struct ModeCore {
    storage: SqliteStorage,
    client: AnthropicClient,
}
impl LinearMode {
    core: ModeCore,  // Composition, not trait inheritance
}
```

### Tool Registry Pattern (avoids giant match statements)

```rust
let handlers: HashMap<&str, Box<dyn ToolHandler>> = create_handlers();
handlers.get(tool_name)?.handle(args).await
```

### JSON Extraction

```rust
fn extract_json(text: &str) -> Result<Value, ModeError> {
    // Fast path: raw JSON
    // Fallback: ```json blocks
    // Clear error with truncated preview
}
```

### Request Size Limits

```rust
const MAX_REQUEST_BYTES: usize = 100_000;  // 100KB
const MAX_MESSAGES: usize = 50;
const MAX_CONTENT_LENGTH: usize = 50_000;  // 50KB per message
```

### Extended Thinking Budgets

| Mode | Thinking Budget |
|------|-----------------|
| Linear, Tree, Auto, Checkpoint | None (fast) |
| Divergent, Graph | Standard (4096 tokens) |
| Reflection, Decision, Evidence | Deep (8192 tokens) |
| Counterfactual, MCTS | Maximum (16384 tokens) |

## Code Quality Requirements

- **Zero unsafe code**: `#![forbid(unsafe_code)]` in lib.rs
- **No panics**: No `.unwrap()` or `.expect()` in production paths
- **TDD workflow**: Write tests first, fail, implement, pass, 95%+ coverage
- **File size limits**: Max 500 lines per .rs file
- **High test coverage**: 2,620+ tests with 90%+ line coverage
- **Structured logging**: Use `tracing` with structured fields, logs to stderr only

## Implementation Status

All phases complete:

| Phase | Component | Status |
|-------|-----------|--------|
| 0-3 | Scaffolding, Error Types, Config, Traits | Complete |
| 4-5 | Storage, Anthropic Client | Complete |
| 6-8 | ModeCore, Prompts, All 13 Modes | Complete |
| 9 | Server Infrastructure (rmcp) | Complete |
| 10 | Presets + Metrics | Complete |
| 11 | Self-Improvement System | Complete |
| 12 | Integration Tests | Complete |

## Test Organization

- Unit tests in same file: `#[cfg(test)] mod tests { ... }`
- Integration tests in `tests/` directory
- Use `#[serial]` for database tests
- Use `#[tokio::test]` for async tests
- Use `mockall::automock` for trait mocking

### Error Handling in Tests

Test code uses `#[allow(clippy::unwrap_used, clippy::expect_used)]` because:

1. Test panics are acceptable and often preferable for clarity
2. `.expect()` provides better panic messages than `?` in tests
3. Reduces test verbosity while maintaining debuggability
4. Industry standard practice (see [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/documentation.html#examples-use-panics-not-try-not-unwrap-c-question-mark))

**Test Code Pattern:**

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_example() {
        let storage = SqliteStorage::new_in_memory().await.expect("create storage");
        let result = storage.get_session("id").await.expect("get session");
        assert_eq!(result.unwrap().id, "id");
    }
}
```

**Production Code Pattern:**

```rust
pub async fn operation() -> Result<Output, Error> {
    let value = fallible_operation()?;  // Never unwrap/expect
    Ok(value)
}
```

## Client Configuration

### Claude Code

```bash
claude mcp add mcp-reasoning \
  --transport stdio \
  --env ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -- /path/to/mcp-reasoning
```

### Claude Desktop (claude_desktop_config.json)

```json
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "/path/to/mcp-reasoning",
      "env": { "ANTHROPIC_API_KEY": "sk-ant-xxx" }
    }
  }
}
```
