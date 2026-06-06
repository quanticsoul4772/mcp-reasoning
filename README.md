# MCP Reasoning Server

A Rust MCP server providing 35 tools for structured reasoning, self-improvement, session management, and agent coordination. 2,690+ tests, 95%+ coverage.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

---

## What It Does

Provides Claude with structured reasoning modes:

- **Linear** - Step-by-step sequential reasoning
- **Tree** - Explore multiple solution paths in parallel
- **Divergent** - Generate creative alternative perspectives
- **Graph** - Build and traverse complex reasoning chains
- **Decision** - Multi-criteria analysis (weighted, pairwise, TOPSIS)
- **Detect** - Identify cognitive biases and logical fallacies
- **Evidence** - Evaluate source credibility and Bayesian updates
- **MCTS** - Monte Carlo Tree Search with auto-backtracking
- **Counterfactual** - "What-if" causal analysis
- **Timeline** - Temporal reasoning with branching scenarios
- **Reflection** - Meta-cognitive quality improvement
- **Checkpoint** - Save and restore reasoning state
- **Auto** - Automatically select the best reasoning mode
- **Meta** - Empirically select the best mode from historical performance data
- **Confidence Route** - Confidence-aware routing: execute the auto-selected mode or escalate to tree
- **Preset** - Pre-configured multi-step workflows
- **Metrics** - Track usage and performance

Plus **session memory**: list and resume past reasoning sessions, and **semantically search or relate** them by meaning using Voyage AI embeddings (see [Semantic Memory](#semantic-memory)).

Each tool returns metadata: execution time estimates, next-step suggestions, and workflow recommendations.

---

## Quick Start

### Prerequisites

- [Anthropic API key](https://console.anthropic.com/) (required)
- [Voyage AI API key](https://www.voyageai.com/) (optional; **required only for the memory tools** `reasoning_search` / `reasoning_relate`)
- Choose one installation method below

### Installation

#### Option 1: One-Command Install

**macOS/Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.ps1 | iex
```

Downloads a pre-built binary to your PATH and optionally configures Claude Desktop.

#### Option 2: npm

```bash
# Global install
npm install -g @mcp-reasoning/server

# Or use without installing
npx @mcp-reasoning/server --version
```

Works with `npx` without a global install.

#### Option 3: Homebrew (macOS/Linux)

```bash
brew tap quanticsoul4772/mcp
brew install mcp-reasoning
```

#### Option 4: Chocolatey (Windows)

```powershell
choco install mcp-reasoning
```

#### Option 5: Docker

```bash
docker pull ghcr.io/quanticsoul4772/mcp-reasoning:latest

# Or use docker-compose
curl -O https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/docker-compose.yml
# Edit docker-compose.yml to add your API key
docker-compose up -d
```

#### Option 6: Build from Source

```bash
git clone https://github.com/quanticsoul4772/mcp-reasoning.git
cd mcp-reasoning
cargo build --release
# Binary at: target/release/mcp-reasoning
```

Requires [Rust 1.75+](https://www.rust-lang.org/tools/install).

---

### Configuration

#### Automatic Configuration

```bash
curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/configure.sh | bash
```

#### Manual Configuration

**For Claude Code:**

```bash
claude mcp add mcp-reasoning \
  --transport stdio \
  --env ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  -- mcp-reasoning
```

**For Claude Desktop:**

**macOS/Linux**: Edit `~/.config/Claude/claude_desktop_config.json`
**Windows**: Edit `%APPDATA%\Claude\claude_desktop_config.json`

Add:

```json
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "mcp-reasoning",
      "env": {
        "ANTHROPIC_API_KEY": "your-api-key-here"
      }
    }
  }
}
```

**Using npm/npx:**

```json
{
  "mcpServers": {
    "mcp-reasoning": {
      "command": "npx",
      "args": ["-y", "@mcp-reasoning/server"],
      "env": {
        "ANTHROPIC_API_KEY": "your-api-key-here"
      }
    }
  }
}
```

Restart Claude Desktop.

#### Verify Installation

```bash
# Check version
mcp-reasoning --version

# Run health checks
export ANTHROPIC_API_KEY=your-key-here  # or use .env file
mcp-reasoning --health
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ANTHROPIC_API_KEY` | **Yes** | - | Your Anthropic API key |
| `VOYAGE_API_KEY` | For memory tools | - | Enables `reasoning_search` / `reasoning_relate` and grounds `reasoning_divergent`'s novelty scores. Without it those three tools return a config error; the other 32 are unaffected |
| `VOYAGE_MODEL` | No | `voyage-4` | Voyage embedding model |
| `DATABASE_PATH` | No | `./data/reasoning.db` | SQLite database location |
| `LOG_LEVEL` | No | `info` | `error`, `warn`, `info`, `debug`, or `trace` |

---

## Usage

Once installed, ask Claude to use reasoning tools:

```
"Use linear reasoning to analyze the trade-offs between microservices and monolithic architecture"

"Create a reasoning tree to explore different database migration strategies"

"Use divergent thinking with force_rebellion to challenge our assumptions about this design"

"Run a decision analysis using TOPSIS to compare these three cloud providers"

"Detect any cognitive biases in my previous reasoning"

"Use counterfactual analysis: what if we had chosen Rust instead of Python?"

"Search past sessions for anything I reasoned about rate limiting"   # reasoning_search

"Relate this session to my earlier ones to spot conflicting conclusions"  # reasoning_relate
```

> `reasoning_search` / `reasoning_relate` require `VOYAGE_API_KEY` (see [Semantic Memory](#semantic-memory)).

### Example: Tree Reasoning

```
You: "Use tree reasoning to explore approaches to implement rate limiting"

Claude calls: reasoning_tree(operation="create", content="...", num_branches=3)

Response includes:
- 3 divergent branches (token bucket, sliding window, fixed window)
- Scores for each approach
- Recommendation on which branch to explore further
- Metadata: estimated time for next operations, suggested next tools
```

### Built-in Workflows

Six pre-configured workflows that chain multiple tools:

- `code-review` - Analyze code with bias detection
- `debug-analysis` - Hypothesis-driven debugging
- `architecture-decision` - Multi-factor architectural analysis
- `strategic-decision` - Stakeholder-aware planning
- `evidence-conclusion` - Research synthesis
- `brainstorming` - Creative exploration (divergent → tree → summarize → reflection)

Ask Claude: *"Run the architecture-decision preset to evaluate switching to Kubernetes"*

---

## Features

### Session Persistence

Reasoning state (sessions, thoughts, branches) is stored in SQLite, so sessions can be **resumed across conversations** with `reasoning_resume` (which reloads the session's thought chain). Checkpoints are a separate feature for saving and rolling back state *within* a session (`reasoning_checkpoint`).

### Semantic Memory

`reasoning_search` and `reasoning_relate` rank past sessions by **meaning**, not keywords, using [Voyage AI](https://www.voyageai.com/):

- Each session is embedded on `voyage-4` and cached (keyed on content **and** model). Search is cosine recall followed by a cross-encoder rerank (`rerank-2.5`); relate builds a depth-bounded, edge-capped session graph.
- **Requires `VOYAGE_API_KEY`** — there is no keyword fallback. Without the key, `reasoning_search`, `reasoning_relate`, and `reasoning_divergent` (whose novelty scores are embedding-grounded) return a clear config error; the other 32 tools work normally.
- A background worker warms embeddings off the request path, so the first search/relate after writing a session isn't slowed by embedding.

### Self-Improvement (4-Phase)

The server monitors its own reasoning quality and proposes tuning to its
configuration. It is **advisory by default** — it records recommendations rather
than silently changing the running server.

1. **Monitor** — Tracks per-reasoning-mode success/error rates and average execution time (plus the overall success rate and invocation count), and flags modes with a low success rate or high latency
2. **Analyze** — Uses Claude to diagnose anomalies and propose corrective actions (config/threshold adjustments)
3. **Execute** — Validates each proposed action against the allowlist and records it as a recommendation in `config_overrides`. Recommendations are **not** applied to the running server by default; set `SELF_IMPROVEMENT_APPLY_OVERRIDES=true` to apply recorded overrides over the config at the **next restart** (bounded to allowlisted, validated fields)
4. **Learn** — Computes a reward signal and a lesson per executed action (visible via `reasoning_si_status`) and feeds the per-action-type effectiveness back into the next **Analyze** step to steer later proposals. Note the reward is based on an **estimated** improvement — a fixed fraction of each action's own `expected_improvement`, not a measured post-change metric — so in practice it rewards actions that execute successfully and penalizes those that fail validation, rather than reflecting measured reasoning-quality gains (real post-change measurement is a planned follow-up). Per-action-type effectiveness is **persisted** (`si_action_type_stats`) and restored on startup, so the steering survives restarts; the recent-insights list re-warms in-process (textual lessons are not persisted)

Review recommendations with `reasoning_si_diagnoses` / `reasoning_si_overrides`;
apply or reject with `reasoning_si_approve` / `reasoning_si_reject`.

Safety mechanisms: an allowlist validates every proposed action (type + parameter
keys + bounds) before execution; a circuit breaker halts the cycle after
consecutive failures.

### Tool Chain Tracking

Automatically records reasoning tool sequences (e.g. `linear → reflection → decision`) and detects recurring patterns. Use `reasoning_metrics` to query chain summaries and spot workflow anti-patterns.

### Error Enhancement

Errors include contextual alternatives — if a tool fails due to incorrect parameters, the response suggests the correct call with example values. Complexity metrics help diagnose timeout causes.

### Extended Thinking Budgets

| Modes | Thinking Budget |
|-------|-----------------|
| `linear`, `tree`, `auto`, `checkpoint` | None (fast) |
| `graph` | Standard — 4096 tokens |
| `divergent`, `reflection`, `decision`, `evidence` | Deep — 8192 tokens |
| `counterfactual`, `mcts` | Maximum — 16384 tokens |

### Streaming

The long-running modes (`divergent`, `reflection`, `mcts`, `counterfactual`) emit milestone progress — a percentage plus a status label (e.g. "Starting API call", "Processing response") — as MCP `notifications/progress` while reasoning continues. These are sent only when the client opts in by supplying a progress token in the request `_meta` (per the MCP spec); the payload is progress status, not partial reasoning output.

### Implementation

- Zero `unsafe` code (`#![forbid(unsafe_code)]`)
- No `.unwrap()` / `.expect()` in production paths
- Const SQL queries, pre-allocated buffers
- 2,690+ tests, 95%+ line coverage

---

## Architecture

```
Claude (Desktop/Code)
    ↓ stdio/JSON-RPC
MCP Reasoning Server (Rust)
    ↓
┌─────────────────┐
│  17 Tools       │
│  ├─ Core (8)    │ ← Linear, Tree, Divergent, Reflection, Checkpoint, Auto, Meta, Confidence_route
│  ├─ Graph (1)   │ ← Graph-of-Thoughts
│  ├─ Analysis (3)│ ← Detect, Decision, Evidence
│  ├─ Advanced (3)│ ← Timeline, MCTS, Counterfactual
│  └─ Infra (2)   │ ← Preset, Metrics
└─────────────────┘
    ↓              ↓              ↓
Anthropic API   Voyage AI      SQLite DB
(Claude models) (memory:       (persistence)
                 embed+rerank)
```

**Tech Stack**: Rust, [rmcp SDK](https://crates.io/crates/rmcp), SQLite, Anthropic API, Voyage AI

---

## Documentation

- **[Documentation Index](docs/README.md)** - Complete documentation hub
- **[API Reference](docs/reference/TOOL_REFERENCE.md)** - Core reasoning tools with examples
- **[Architecture](docs/reference/ARCHITECTURE.md)** - System design
- **[Development Guide](docs/guides/DEVELOPMENT.md)** - Setup and contribution
- **[Testing Guide](docs/guides/TESTING.md)** - Testing strategies
- **[CHANGELOG](CHANGELOG.md)** - Version history

---

## Development

### Build and Test

```bash
# Build
cargo build

# Run tests
cargo test

# Check code quality
cargo fmt --check
cargo clippy -- -D warnings

# Run with coverage
cargo llvm-cov
```

### Code Quality

- Zero `unsafe` code (enforced via `#![forbid(unsafe_code)]`)
- No `.unwrap()` or `.expect()` in production paths (enforced via `#![deny(clippy::unwrap_used, clippy::expect_used)]`)
- `clippy -- -D warnings`, `cargo fmt`

### Contributing

See [CONTRIBUTING.md](docs/guides/CONTRIBUTING.md).

```bash
# Install pre-commit hooks
pip install pre-commit
pre-commit install

# Make changes, ensure tests pass
cargo test
cargo clippy -- -D warnings
cargo fmt --check

# Submit PR
```

---

## Troubleshooting

### "ANTHROPIC_API_KEY not found"

Set the environment variable or add it to `.env` file:

```bash
export ANTHROPIC_API_KEY=your-key-here
# or
echo "ANTHROPIC_API_KEY=your-key-here" > .env
```

### "Database error"

Ensure the `data/` directory exists:

```bash
mkdir -p data
```

Or set `DATABASE_PATH` to a writable location.

### "Command not found" in Claude Desktop

Use absolute paths in `claude_desktop_config.json`:

```bash
# Get absolute path
cd mcp-reasoning
pwd  # Copy this path
```

### Logs

Logs go to stderr. Set `LOG_LEVEL=debug` for detailed output:

```bash
LOG_LEVEL=debug /path/to/mcp-reasoning 2> server.log
```

---

## License

[MIT](LICENSE)

---

## Acknowledgments

Built with:

- [rmcp](https://crates.io/crates/rmcp) - Rust MCP SDK
- [Anthropic Claude API](https://www.anthropic.com/)
- Inspired by Graph-of-Thoughts, MCTS, and Pearl's Causal Framework

---

[docs/README.md](docs/README.md) | [Issues](https://github.com/quanticsoul4772/mcp-reasoning/issues)
