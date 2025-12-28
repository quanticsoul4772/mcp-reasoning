# MCP Reasoning Server

An MCP server that adds structured reasoning capabilities to Claude Code and Claude Desktop.

## Features

- **Structured Reasoning** - Linear, tree-based, and graph-based reasoning patterns
- **Decision Analysis** - Weighted scoring, pairwise comparison, TOPSIS, stakeholder mapping
- **Bias Detection** - Identify cognitive biases and logical fallacies in arguments
- **Counterfactual Analysis** - What-if scenarios using Pearl's causal framework
- **Session Persistence** - Save and restore reasoning state across sessions
- **Built-in Workflows** - Pre-configured presets for common reasoning tasks
- **Self-Improvement** - 4-phase optimization loop (Monitor → Analyze → Execute → Learn) with circuit breaker safety

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

## Installation

### Prerequisites

- Rust 1.75+ (for async traits)
- Anthropic API key

### Build from Source

```bash
git clone https://github.com/quanticsoul4772/mcp-reasoning.git
cd mcp-reasoning
cargo build --release
```

The binary will be at `target/release/mcp-reasoning`.

## Configuration

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ANTHROPIC_API_KEY` | Yes | - | Your Anthropic API key |
| `DATABASE_PATH` | No | `./data/reasoning.db` | SQLite database path |
| `LOG_LEVEL` | No | `info` | Logging level (error/warn/info/debug/trace) |
| `REQUEST_TIMEOUT_MS` | No | `30000` | Request timeout in milliseconds |
| `MAX_RETRIES` | No | `3` | Maximum API retry attempts |

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

## The 15 Reasoning Tools

### Core Reasoning (6 tools)

| Tool | Description |
|------|-------------|
| `reasoning_linear` | Sequential step-by-step reasoning with confidence scoring |
| `reasoning_tree` | Branching exploration (create/focus/list/complete) |
| `reasoning_divergent` | Multi-perspective generation with force_rebellion mode |
| `reasoning_reflection` | Meta-cognitive analysis (process/evaluate) |
| `reasoning_checkpoint` | State management (create/list/restore) |
| `reasoning_auto` | Automatic mode selection based on content |

### Graph Reasoning (1 tool)

| Tool | Description |
|------|-------------|
| `reasoning_graph` | Graph-of-Thoughts with 8 operations: init, generate, score, aggregate, refine, prune, finalize, state |

### Analysis Tools (3 tools)

| Tool | Description |
|------|-------------|
| `reasoning_detect` | Cognitive bias and logical fallacy detection |
| `reasoning_decision` | Decision analysis (weighted/pairwise/topsis/perspectives) |
| `reasoning_evidence` | Evidence evaluation with Bayesian updates |

### Advanced Reasoning (3 tools)

| Tool | Description |
|------|-------------|
| `reasoning_timeline` | Temporal reasoning (create/branch/compare/merge) |
| `reasoning_mcts` | Monte Carlo Tree Search with UCB1 exploration |
| `reasoning_counterfactual` | What-if analysis using Pearl's Ladder of Causation |

### Infrastructure (2 tools)

| Tool | Description |
|------|-------------|
| `reasoning_preset` | Pre-defined reasoning workflows (list/run) |
| `reasoning_metrics` | Usage metrics and observability |

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

### Divergent Perspectives

```json
{
  "tool": "reasoning_divergent",
  "arguments": {
    "content": "Should we migrate to a new database technology?",
    "num_perspectives": 4,
    "force_rebellion": true
  }
}
```

### Decision Analysis

```json
{
  "tool": "reasoning_decision",
  "arguments": {
    "type": "weighted",
    "question": "Which cloud provider should we choose?",
    "options": ["AWS", "GCP", "Azure"]
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

## Built-in Presets

| Preset | Description |
|--------|-------------|
| `problem_solving` | Structured problem decomposition workflow |
| `decision_making` | Multi-criteria decision analysis |
| `creative_exploration` | Divergent thinking with synthesis |
| `critical_analysis` | Bias detection and evidence evaluation |
| `strategic_planning` | Timeline-based strategic reasoning |

Run a preset:

```json
{
  "tool": "reasoning_preset",
  "arguments": {
    "operation": "run",
    "preset_id": "problem_solving",
    "inputs": {
      "problem": "How to reduce technical debt while maintaining velocity?"
    }
  }
}
```

## Development

### Build Commands

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run clippy lints
cargo clippy -- -D warnings

# Run with coverage (requires cargo-llvm-cov)
cargo llvm-cov --fail-under-lines 100
```

### Project Structure

```
src/
├── main.rs              # Entry point
├── lib.rs               # Module declarations
├── traits.rs            # Core traits (AnthropicClientTrait, StorageTrait)
├── error/               # Error types
├── config/              # Configuration
├── anthropic/           # Anthropic API client
├── storage/             # SQLite persistence
├── prompts/             # Mode-specific prompts
├── modes/               # Reasoning mode implementations
├── server/              # MCP server infrastructure
├── presets/             # Built-in workflow presets
├── metrics/             # Usage metrics
└── self_improvement/    # Self-optimization system
```

### Code Quality

- Zero unsafe code (`#![forbid(unsafe_code)]`)
- No panics in production paths
- 100% test coverage enforced
- Max 500 lines per file
- Structured logging via tracing

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch
3. Ensure all tests pass with `cargo test`
4. Ensure clippy passes with `cargo clippy -- -D warnings`
5. Submit a pull request

## Acknowledgments

- Built on the [rmcp](https://crates.io/crates/rmcp) MCP SDK
- Inspired by structured reasoning research and Graph-of-Thoughts
