# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MCP Reasoning Server - A Rust-based MCP server providing structured reasoning capabilities via direct Anthropic Claude API calls. This project offers 15 consolidated reasoning tools (vs 40 in the predecessor mcp-langbase-reasoning).

**Status**: Production-ready. Fully implemented with 33,000+ lines of Rust code and 1,524 tests.

**Key Stats**:
- 83 source files, 33,000+ lines of code
- 1,524 tests
- 15 reasoning tools, 5 workflow presets
- 4-phase self-improvement system with safety mechanisms

**Key Documents**:
- `docs/DESIGN.md` - Complete technical specification
- `docs/IMPLEMENTATION_PLAN.md` - TDD execution guide (completed)
- `docs/LESSONS_LEARNED.md` - Patterns replicated from predecessor
- `docs/MODE_PATTERN.md` - Mode implementation template

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
cargo llvm-cov --fail-under-lines 100 # Coverage (100% required)

# Full validation (run before every commit)
cargo fmt --check && cargo clippy -- -D warnings && cargo llvm-cov --fail-under-lines 100

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
├── traits.rs            # Mockable traits (AnthropicClientTrait, StorageTrait, TimeProvider)
├── test_utils.rs        # Mock factories (test only)
├── error/mod.rs         # AppError, StorageError, ConfigError, ModeError
├── config/
│   ├── mod.rs           # Config struct + from_env()
│   └── validation.rs    # Validation logic
├── anthropic/
│   ├── client.rs        # AnthropicClient with retry + backoff
│   ├── types.rs         # Request/Response types, Vision support
│   ├── config.rs        # ModelConfig, ThinkingConfig (standard/deep/maximum)
│   └── streaming.rs     # SSE stream handling
├── storage/
│   ├── mod.rs           # Storage trait
│   ├── sqlite.rs        # Main implementation (<500 lines)
│   ├── session.rs       # Session CRUD
│   ├── thought.rs       # Thought CRUD
│   └── graph.rs         # Graph node/edge CRUD
├── prompts/
│   ├── mod.rs           # get_prompt_for_mode() router
│   ├── core.rs          # linear, tree, divergent, reflection prompts
│   └── advanced.rs      # graph, timeline, mcts, counterfactual prompts
├── modes/
│   ├── mod.rs           # ReasoningMode enum + exports
│   ├── core.rs          # ModeCore (shared deps) + extract_json() helper
│   ├── linear.rs        # Single-pass sequential
│   ├── tree.rs          # Branching (create/focus/list/complete)
│   ├── divergent.rs     # Multi-perspective + force_rebellion
│   ├── reflection.rs    # Meta-cognitive (process/evaluate)
│   ├── checkpoint.rs    # State management (create/list/restore)
│   ├── auto.rs          # Mode selection router
│   ├── graph.rs         # Graph-of-Thoughts (8 operations)
│   ├── detect.rs        # Bias/fallacy detection
│   ├── decision.rs      # weighted/pairwise/topsis/perspectives
│   ├── evidence.rs      # Credibility + Bayesian updates
│   ├── timeline.rs      # Temporal (create/branch/compare/merge)
│   ├── mcts.rs          # UCB1 search + auto_backtrack
│   └── counterfactual.rs # Pearl's Ladder causal analysis
├── server/
│   ├── mod.rs           # McpServer + graceful shutdown
│   ├── mcp.rs           # JSON-RPC protocol
│   ├── tools.rs         # 15 tool schemas (rmcp macros)
│   ├── handlers.rs      # HandlerRegistry (HashMap pattern)
│   └── transport.rs     # Stdio + HTTP transport
├── presets/
│   ├── mod.rs           # PresetMode (list/run)
│   └── builtin.rs       # 5 built-in presets
├── metrics/mod.rs       # Usage metrics collection
└── self_improvement/
    ├── mod.rs           # Re-exports
    ├── system.rs        # SelfImprovementSystem orchestrator
    ├── types.rs         # Severity, TriggerMetric, SuggestedAction
    ├── monitor.rs       # Phase 1: Metric collection
    ├── analyzer.rs      # Phase 2: LLM diagnosis
    ├── executor.rs      # Phase 3: Action execution + rollback
    ├── learner.rs       # Phase 4: Reward calculation
    ├── circuit_breaker.rs # Safety: halt on consecutive failures
    └── allowlist.rs     # Safety: validate action bounds
```

## The 15 Reasoning Tools

| Tool | Description | Operations |
|------|-------------|------------|
| `reasoning_linear` | Sequential step-by-step | single |
| `reasoning_tree` | Branching exploration | create, focus, list, complete |
| `reasoning_divergent` | Multi-perspective | force_rebellion option |
| `reasoning_reflection` | Meta-cognitive | process, evaluate |
| `reasoning_checkpoint` | State management | create, list, restore |
| `reasoning_auto` | Mode router | automatic selection |
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

### JSON Extraction Robustness
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
- **TDD workflow**: Write tests first → fail → implement → pass → 100% coverage
- **File size limits**: Max 500 lines per .rs file
- **100% test coverage**: Enforced via `cargo llvm-cov --fail-under-lines 100`
- **Structured logging**: Use `tracing` with structured fields, logs to stderr only

## Implementation Status

All phases complete:

| Phase | Component | Status |
|-------|-----------|--------|
| 0-3 | Scaffolding, Error Types, Config, Traits | ✅ Complete |
| 4-5 | Storage, Anthropic Client | ✅ Complete |
| 6-8 | ModeCore, Prompts, All 13 Modes | ✅ Complete |
| 9 | Server Infrastructure (rmcp) | ✅ Complete |
| 10 | Presets + Metrics | ✅ Complete |
| 11 | Self-Improvement System | ✅ Complete |
| 12 | Integration Tests | ✅ Complete |

## Test Organization

- Unit tests in same file: `#[cfg(test)] mod tests { ... }`
- Integration tests in `tests/` directory
- Use `#[serial]` for database tests
- Use `#[tokio::test]` for async tests
- Use `mockall::automock` for trait mocking

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
