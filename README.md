# MCP Reasoning Server

A high-performance MCP server that adds structured reasoning capabilities to Claude Code and Claude Desktop. Built in Rust with direct Anthropic API integration.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/Tests-1624%20passing-brightgreen.svg)](#development)
[![Coverage](https://img.shields.io/badge/Coverage-96%25-brightgreen.svg)](#development)

## Features

- **15 Structured Reasoning Tools** - Linear, tree-based, graph-based, and advanced reasoning patterns
- **Decision Analysis** - Weighted scoring, pairwise comparison, TOPSIS, stakeholder mapping
- **Bias & Fallacy Detection** - Identify cognitive biases and logical fallacies with remediation suggestions
- **Counterfactual Analysis** - What-if scenarios using Pearl's causal framework (Ladder of Causation)
- **Monte Carlo Tree Search** - UCB1-guided exploration with auto-backtracking
- **Session Persistence** - Save and restore reasoning state with checkpoints across sessions
- **Built-in Workflow Presets** - Pre-configured workflows for code review, debugging, and architecture decisions
- **Self-Improvement System** - 4-phase optimization loop (Monitor → Analyze → Execute → Learn) with circuit breaker safety
- **Extended Thinking** - Configurable thinking budgets (standard/deep/maximum) for complex reasoning

## Architecture

```
                 ╔═══════════════════════════════════╗
                 ║       MCP REASONING SERVER        ║
                 ╚═══════════════════════════════════╝

┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│  Claude Code    │◄───────►│    Transport    │────────►│  Anthropic API  │
│  or Desktop     │ JSON-RPC│  (stdio/HTTP)   │         │    (Claude)     │
└─────────────────┘         └────────┬────────┘         └─────────────────┘
                                     │
                                     ▼
                            ┌─────────────────┐
                            │   Tool Router   │
                            │   (15 tools)    │
                            └────────┬────────┘
          ┌──────────────────────────┼──────────────────────────┐
          │                          │                          │
          ▼                          ▼                          ▼
┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────┐
│   CORE REASONING    │  │   ANALYSIS TOOLS    │  │ ADVANCED REASONING  │
├─────────────────────┤  ├─────────────────────┤  ├─────────────────────┤
│ Linear   │ Tree     │  │ Detect   │ Decision │  │ Timeline │ MCTS     │
│ Divergent│ Reflect  │  │ Evidence │          │  │ Counter- │ Presets  │
│ Auto     │ Chkpoint │  │          │          │  │ factual  │ Metrics  │
│ Graph-of-Thoughts   │  │                     │  │                     │
└──────────┬──────────┘  └──────────┬──────────┘  └──────────┬──────────┘
           │                        │                        │
           └────────────────────────┼────────────────────────┘
                                    │
           ┌────────────────────────┴────────────────────────┐
           │                                                 │
           ▼                                                 ▼
┌─────────────────────────┐              ┌─────────────────────────────────┐
│      SQLite Storage     │◄── metrics ─►│    SELF-IMPROVEMENT SYSTEM      │
├─────────────────────────┤              ├─────────────────────────────────┤
│ Sessions  │ Thoughts    │              │ Monitor → Analyze → Execute →   │
│ Branches  │ Graphs      │              │    │        Learn    │          │
│ Checkpoints             │              │    └─► SAFETY ◄──────┘          │
│                         │              │   (Circuit Breaker, Allowlist)  │
└─────────────────────────┘              └─────────────────────────────────┘
```

## Quick Start

### Prerequisites

- **Rust 1.75+** (for async traits)
- **Anthropic API key**

### Installation

```bash
# Clone and build
git clone https://github.com/quanticsoul4772/mcp-reasoning.git
cd mcp-reasoning
cargo build --release

# Binary will be at target/release/mcp-reasoning
```

### Claude Code Integration

```bash
claude mcp add mcp-reasoning \
  --transport stdio \
  --env ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -- /path/to/mcp-reasoning
```

### Claude Desktop Integration

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "/path/to/mcp-reasoning",
      "env": {
        "ANTHROPIC_API_KEY": "sk-ant-xxx"
      }
    }
  }
}
```

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ANTHROPIC_API_KEY` | Yes | - | Your Anthropic API key |
| `DATABASE_PATH` | No | `./data/reasoning.db` | SQLite database path |
| `LOG_LEVEL` | No | `info` | Logging level (error/warn/info/debug/trace) |
| `REQUEST_TIMEOUT_MS` | No | `30000` | Request timeout in milliseconds |
| `MAX_RETRIES` | No | `3` | Maximum API retry attempts |

## The 15 Reasoning Tools

### Core Reasoning (6 tools)

| Tool | Description | Key Operations |
|------|-------------|----------------|
| `reasoning_linear` | Sequential step-by-step reasoning with confidence scoring | Single-pass with next step suggestion |
| `reasoning_tree` | Branching exploration for multi-path analysis | create, focus, list, complete |
| `reasoning_divergent` | Multi-perspective generation with assumption challenges | force_rebellion mode for contrarian views |
| `reasoning_reflection` | Meta-cognitive analysis and iterative refinement | process, evaluate |
| `reasoning_checkpoint` | State management for backtracking | create, list, restore |
| `reasoning_auto` | Intelligent mode selection based on content | Automatic routing |

### Graph Reasoning (1 tool)

| Tool | Description | Operations |
|------|-------------|------------|
| `reasoning_graph` | Graph-of-Thoughts for complex reasoning chains | init, generate, score, aggregate, refine, prune, finalize, state |

### Analysis Tools (3 tools)

| Tool | Description | Types |
|------|-------------|-------|
| `reasoning_detect` | Cognitive bias and logical fallacy detection | biases, fallacies |
| `reasoning_decision` | Multi-criteria decision analysis | weighted, pairwise, topsis, perspectives |
| `reasoning_evidence` | Evidence evaluation with Bayesian updates | assess, probabilistic |

### Advanced Reasoning (3 tools)

| Tool | Description | Operations |
|------|-------------|------------|
| `reasoning_timeline` | Temporal reasoning with branching timelines | create, branch, compare, merge |
| `reasoning_mcts` | Monte Carlo Tree Search with UCB1 exploration | explore, auto_backtrack |
| `reasoning_counterfactual` | What-if causal analysis (Pearl's Ladder) | association, intervention, counterfactual |

### Infrastructure (2 tools)

| Tool | Description | Operations |
|------|-------------|------------|
| `reasoning_preset` | Pre-defined reasoning workflows | list, run |
| `reasoning_metrics` | Usage metrics and observability | summary, by_mode, invocations, fallbacks, config |

## Usage Examples

### Linear Reasoning

```json
{
  "tool": "reasoning_linear",
  "arguments": {
    "content": "Analyze the trade-offs between microservices and monolithic architectures",
    "confidence": 0.8
  }
}
```

### Tree Exploration

```json
{
  "tool": "reasoning_tree",
  "arguments": {
    "operation": "create",
    "content": "What are the best approaches to handle user authentication?",
    "num_branches": 3
  }
}
```

### Divergent Perspectives with Force Rebellion

```json
{
  "tool": "reasoning_divergent",
  "arguments": {
    "content": "Should we migrate to a new database technology?",
    "num_perspectives": 4,
    "force_rebellion": true,
    "challenge_assumptions": true
  }
}
```

### Decision Analysis (TOPSIS)

```json
{
  "tool": "reasoning_decision",
  "arguments": {
    "type": "topsis",
    "question": "Which cloud provider should we choose?",
    "options": ["AWS", "GCP", "Azure"],
    "context": "Mid-size startup, ML-heavy workloads"
  }
}
```

### Counterfactual Analysis

```json
{
  "tool": "reasoning_counterfactual",
  "arguments": {
    "scenario": "Our startup chose Python for the backend",
    "intervention": "What if we had chosen Rust instead?",
    "analysis_depth": "counterfactual"
  }
}
```

### Monte Carlo Tree Search

```json
{
  "tool": "reasoning_mcts",
  "arguments": {
    "operation": "explore",
    "content": "Design an optimal caching strategy",
    "iterations": 50,
    "simulation_depth": 5
  }
}
```

## Built-in Presets

| Preset | Category | Description |
|--------|----------|-------------|
| `code-review` | CodeQuality | Analyze code with bias detection and alternative approaches |
| `debug-analysis` | Analysis | Hypothesis-driven debugging with evidence evaluation |
| `architecture-decision` | Decision | Multi-factor architectural decision making |
| `strategic-decision` | Decision | Stakeholder-aware strategic planning with risk assessment |
| `evidence-conclusion` | Research | Evidence-based research synthesis |

Run a preset:

```json
{
  "tool": "reasoning_preset",
  "arguments": {
    "operation": "run",
    "preset_id": "code-review",
    "inputs": {
      "code": "function example() { ... }"
    }
  }
}
```

## Self-Improvement System

The server includes a 4-phase autonomous optimization loop with comprehensive safety mechanisms:

```
+===========================================================================+
|                  SELF-IMPROVEMENT SYSTEM (4-Phase Loop)                   |
+===========================================================================+
|                                                                           |
|   +-------------------------------------------------------------------+   |
|   |                       OPTIMIZATION LOOP                           |   |
|   |                                                                   |   |
|   |   +----------+    +----------+    +----------+    +----------+    |   |
|   |   | MONITOR  |--->| ANALYZER |--->| EXECUTOR |--->| LEARNER  |--+ |   |
|   |   +----------+    +----------+    +----------+    +----------+  | |   |
|   |   | Metrics  |    | LLM      |    | Apply    |    | Reward   |  | |   |
|   |   | Anomaly  |    | Diagnose |    | Action   |    | Calc     |  | |   |
|   |   | Errors   |    | Root     |    | Rollback |    | Extract  |  | |   |
|   |   | Latency  |    | Cause    |    | Support  |    | Lessons  |  | |   |
|   |   | Quality  |    | Propose  |    |          |    |          |  | |   |
|   |   +----^-----+    +----------+    +----------+    +----------+  | |   |
|   |        |                                                        | |   |
|   |        +------------------ Update Baselines <-------------------+ |   |
|   |                                                                   |   |
|   +-------------------------------------------------------------------+   |
|                                                                           |
|   +===================================================================+   |
|   |                       SAFETY MECHANISMS                           |   |
|   +===============================+===================================+   |
|   | Circuit Breaker               | Allowlist                         |   |
|   | - Halts on consecutive fails  | - Validates action types          |   |
|   | - Auto-recovery after timeout | - Checks parameter bounds         |   |
|   +-------------------------------+-----------------------------------+   |
|   | Rate Limiting                 | Approval Gate                     |   |
|   | - Max actions per period      | - Human-in-loop (optional)        |   |
|   | - Prevents runaway execution  | - Required for high-risk actions  |   |
|   +-------------------------------+-----------------------------------+   |
|                                                                           |
+===========================================================================+
```

**Phase Details:**

| Phase | Function | Key Capabilities |
|-------|----------|------------------|
| **Monitor** | Collect & detect | Metrics aggregation, anomaly detection, error tracking, latency monitoring |
| **Analyzer** | Diagnose & propose | LLM-powered root cause analysis, action proposal generation |
| **Executor** | Apply & protect | Action execution with rollback support, state preservation |
| **Learner** | Extract & improve | Reward calculation, baseline updates, pattern learning |

**Safety Mechanisms:**

| Mechanism | Protection | Trigger |
|-----------|------------|---------|
| **Circuit Breaker** | Halts operations | Consecutive failures exceed threshold |
| **Allowlist** | Validates actions | Every action checked against permitted types/params |
| **Rate Limiting** | Prevents overload | Actions exceed count per time period |
| **Approval Gate** | Human oversight | High-risk actions (optional, configurable) |

## Development

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run all tests (1,624 tests)
cargo test

# Run specific test module
cargo test modes::linear

# Check formatting
cargo fmt --check

# Run clippy lints
cargo clippy -- -D warnings

# Run with coverage (requires cargo-llvm-cov)
cargo llvm-cov
```

### Project Structure

```
src/
├── main.rs              # Entry point (<100 lines)
├── lib.rs               # Module declarations + lints
├── traits.rs            # Core traits (AnthropicClientTrait, StorageTrait)
├── error/               # Unified error types (thiserror)
├── config/              # Configuration + validation
├── anthropic/           # Anthropic API client
│   ├── client.rs        # HTTP client with retry + backoff
│   ├── types.rs         # Request/Response types
│   ├── config.rs        # Model + thinking configuration
│   └── streaming.rs     # SSE stream handling
├── storage/             # SQLite persistence
│   ├── core.rs          # Connection pool + migrations
│   ├── session.rs       # Session CRUD
│   ├── thought.rs       # Thought CRUD
│   ├── branch.rs        # Branch management
│   ├── checkpoint.rs    # Checkpoint storage
│   └── graph.rs         # Graph node/edge storage
├── prompts/             # Mode-specific prompts
├── modes/               # 13 reasoning mode implementations
│   ├── linear.rs        # Sequential reasoning
│   ├── tree.rs          # Branching exploration
│   ├── divergent.rs     # Multi-perspective
│   ├── graph/           # Graph-of-Thoughts
│   ├── decision/        # Decision analysis
│   ├── detect/          # Bias/fallacy detection
│   ├── evidence/        # Evidence evaluation
│   ├── timeline/        # Temporal reasoning
│   ├── mcts/            # Monte Carlo Tree Search
│   └── counterfactual.rs# Causal analysis
├── server/              # MCP server infrastructure (rmcp)
├── presets/             # Built-in workflow presets
├── metrics/             # Usage metrics collection
└── self_improvement/    # 4-phase optimization system
    ├── monitor.rs       # Metric collection
    ├── analyzer.rs      # LLM diagnosis
    ├── executor.rs      # Action execution
    ├── learner.rs       # Lesson extraction
    ├── circuit_breaker.rs # Safety mechanism
    └── allowlist.rs     # Action validation
```

### Code Quality Standards

- **Zero unsafe code** - `#![forbid(unsafe_code)]` enforced
- **No panics** - No `.unwrap()` or `.expect()` in production paths
- **1,624 tests** - Comprehensive unit, integration, and handler test coverage (96%+ coverage)
- **Max 500 lines per file** - Enforced for maintainability
- **Structured logging** - Via `tracing` crate, logs to stderr
- **Clippy pedantic** - All pedantic lints enabled as warnings

### Extended Thinking Budgets

| Mode | Thinking Budget | Use Case |
|------|-----------------|----------|
| Linear, Tree, Auto, Checkpoint | None (fast) | Quick operations |
| Divergent, Graph | Standard (4096 tokens) | Creative exploration |
| Reflection, Decision, Evidence | Deep (8192 tokens) | Analytical work |
| Counterfactual, MCTS | Maximum (16384 tokens) | Complex reasoning |

## API Reference

- **[Tool Reference](docs/TOOL_REFERENCE.md)** - Complete API documentation for all 15 reasoning tools with parameters, response schemas, and examples
- **[Design Document](docs/DESIGN.md)** - Technical specification and architecture details

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Ensure all tests pass (`cargo test`)
4. Ensure clippy passes (`cargo clippy -- -D warnings`)
5. Commit your changes (use conventional commits)
6. Submit a pull request

## Acknowledgments

- Built on the [rmcp](https://crates.io/crates/rmcp) MCP SDK
- Inspired by structured reasoning research, Graph-of-Thoughts, and Monte Carlo Tree Search
- Uses Pearl's Ladder of Causation for counterfactual analysis
