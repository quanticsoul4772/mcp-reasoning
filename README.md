# MCP Reasoning Server

A high-performance MCP server that provides 15 structured reasoning tools for Claude Code and Claude Desktop. Built in Rust for speed and reliability.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
![Tests](https://img.shields.io/badge/Tests-2020%20passing-brightgreen.svg)
![Coverage](https://img.shields.io/badge/Coverage-95%25-brightgreen.svg)

---

## What It Does

Enables Claude to reason systematically through complex problems using specialized reasoning modes:

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
- **Preset** - Pre-configured multi-step workflows
- **Metrics** - Track usage and performance

Each tool includes intelligent metadata: execution time predictions, next-step suggestions, and workflow recommendations that improve with use.

---

## Quick Start

### Prerequisites

- [Anthropic API key](https://console.anthropic.com/) (required)
- Choose one installation method below (no Rust required!)

### Installation

Choose the method that works best for you:

#### 🚀 Option 1: One-Command Install (Recommended)

**macOS/Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/install.ps1 | iex
```

This downloads the pre-built binary, installs it to your PATH, and offers to configure Claude Desktop automatically.

#### 📦 Option 2: npm (Easiest for Developers)

```bash
# Global install
npm install -g @mcp-reasoning/server

# Or use without installing
npx @mcp-reasoning/server --version
```

**Why npm?** Cross-platform, auto-updates, works with `npx` (no install needed).

#### 🍺 Option 3: Homebrew (macOS/Linux)

```bash
brew tap quanticsoul4772/mcp
brew install mcp-reasoning
```

Auto-updates with `brew upgrade`.

#### 🍫 Option 4: Chocolatey (Windows)

```powershell
choco install mcp-reasoning
```

Auto-updates with `choco upgrade`.

#### 🐳 Option 5: Docker

```bash
docker pull ghcr.io/quanticsoul4772/mcp-reasoning:latest

# Or use docker-compose
curl -O https://raw.githubusercontent.com/quanticsoul4772/mcp-reasoning/main/docker-compose.yml
# Edit docker-compose.yml to add your API key
docker-compose up -d
```

#### 🔨 Option 6: Build from Source

```bash
git clone https://github.com/quanticsoul4772/mcp-reasoning.git
cd mcp-reasoning
cargo build --release
# Binary at: target/release/mcp-reasoning
```

Requires [Rust 1.75+](https://www.rust-lang.org/tools/install).

---

### Configuration

#### Automatic Configuration (Easiest)

After installing the binary, run:

```bash
# Interactive wizard guides you through setup
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
```

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

Five pre-configured workflows orchestrate multiple tools:

- `code-review` - Analyze code with bias detection
- `debug-analysis` - Hypothesis-driven debugging
- `architecture-decision` - Multi-factor architectural analysis
- `strategic-decision` - Stakeholder-aware planning
- `evidence-conclusion` - Research synthesis

Ask Claude: *"Run the architecture-decision preset to evaluate switching to Kubernetes"*

---

## Features

### Session Persistence

All reasoning is saved to SQLite. Continue complex analyses across multiple conversations using checkpoints.

### Self-Improvement

The server monitors its own performance and automatically optimizes:

- Learns from actual execution times to improve predictions
- Detects anomalies and suggests corrections
- Includes circuit breaker and safety mechanisms

### Streaming

Long-running operations send progress updates in real-time.

### Performance Optimized

- ~45% fewer memory allocations than previous version
- Const SQL queries eliminate repeated parsing
- Pre-allocated buffers and vectors
- Zero unsafe code, 95%+ test coverage

---

## Architecture

```
Claude (Desktop/Code)
    ↓ stdio/JSON-RPC
MCP Reasoning Server (Rust)
    ↓
┌─────────────────┐
│  15 Tools       │
│  ├─ Core (6)    │ ← Linear, Tree, Divergent, Reflection, Checkpoint, Auto
│  ├─ Graph (1)   │ ← Graph-of-Thoughts
│  ├─ Analysis (3)│ ← Detect, Decision, Evidence
│  ├─ Advanced (3)│ ← Timeline, MCTS, Counterfactual
│  └─ Infra (2)   │ ← Preset, Metrics
└─────────────────┘
    ↓                    ↓
Anthropic API        SQLite DB
(Claude models)      (persistence)
```

**Tech Stack**: Rust, [rmcp SDK](https://crates.io/crates/rmcp), SQLite, Anthropic API

---

## Documentation

- **[Documentation Index](docs/README.md)** - Complete documentation hub
- **[API Reference](docs/reference/TOOL_REFERENCE.md)** - All 15 tools with examples
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

# Run all 2,020 tests
cargo test

# Check code quality
cargo fmt --check
cargo clippy -- -D warnings

# Run with coverage
cargo llvm-cov
```

### Code Quality

- Zero `unsafe` code (enforced)
- No `.unwrap()` or `.expect()` in production paths
- 2,020 tests with 95%+ coverage
- Max 500 lines per file
- Strict clippy lints

### Contributing

We welcome contributions! See [CONTRIBUTING.md](docs/guides/CONTRIBUTING.md) for guidelines.

**Quick setup:**

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

**Questions?** See [docs/README.md](docs/README.md) or [open an issue](https://github.com/quanticsoul4772/mcp-reasoning/issues).
