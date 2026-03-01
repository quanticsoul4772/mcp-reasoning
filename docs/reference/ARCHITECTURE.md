# MCP Reasoning Server - Architecture

## Overview

MCP server providing structured reasoning capabilities via direct Anthropic Claude API calls.

**Key Differentiators from mcp-langbase-reasoning:**

- Direct Anthropic API (no Langbase middleman)
- Consolidated tool surface (15 tools vs 40)
- Anthropic Claude models (user preference)
- Simplified architecture

---

## System Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ MCP Client  │────▶│ MCP Server  │────▶│  Anthropic  │
│             │◀────│   (Rust)    │◀────│  Claude API │
└─────────────┘     └─────────────┘     └─────────────┘
      JSON-RPC             │
                           ▼
                    ┌─────────────┐
                    │   SQLite    │
                    └─────────────┘
```

**Components:**

- **MCP Client**: Claude Code or Claude Desktop
- **MCP Server**: Rust-based reasoning server
- **Anthropic API**: Direct Claude API integration
- **SQLite**: Session and reasoning state persistence

---

## Tool Surface (15 Tools)

### Core Reasoning (6 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_linear` | - | Single-pass sequential reasoning; process thought and get logical continuation |
| `reasoning_tree` | create, focus, list, complete | Branching exploration: create=start with 2-4 paths, focus=select branch, list=show branches, complete=mark finished |
| `reasoning_divergent` | - | Creative multi-perspective generation with assumption challenges and force_rebellion mode |
| `reasoning_reflection` | process, evaluate | Meta-cognitive: process=iterative refinement, evaluate=session-wide quality assessment |
| `reasoning_checkpoint` | create, list, restore | Backtracking: create=save state, list=show checkpoints, restore=return with optional new_direction |
| `reasoning_auto` | - | Analyze content and route to optimal reasoning mode (linear/tree/divergent/etc.) |

### Graph Reasoning (1 tool)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_graph` | init, generate, score, aggregate, refine, prune, finalize, state | Graph-of-Thoughts: init=create graph, generate=expand k nodes, score=evaluate quality, aggregate=merge nodes, refine=improve via self-critique, prune=remove weak nodes, finalize=extract conclusions, state=show structure |

### Analysis (3 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_detect` | biases, fallacies | Cognitive errors: biases=confirmation/anchoring/sunk-cost with remediation, fallacies=ad-hominem/straw-man/false-dichotomy with formal/informal categories |
| `reasoning_decision` | weighted, pairwise, topsis, perspectives | Decisions: weighted=scored ranking, pairwise=direct comparison, topsis=ideal-point distance, perspectives=stakeholder power/interest mapping |
| `reasoning_evidence` | assess, probabilistic | Evidence: assess=source credibility/corroboration/chain-of-custody, probabilistic=Bayesian prior->posterior with likelihoods |

### Advanced Reasoning (3 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_timeline` | create, branch, compare, merge | Temporal: create=new timeline, branch=fork path, compare=analyze divergence, merge=synthesize branches with strategy |
| `reasoning_mcts` | explore, auto_backtrack | MCTS: explore=UCB1-guided search with iterations/depth, auto_backtrack=quality-triggered backtracking with lookback |
| `reasoning_counterfactual` | - | What-if causal analysis using Pearl's Ladder: scenario + intervention -> causal consequences |

### Infrastructure (2 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_preset` | list, run | Workflows: list=show presets by category, run=execute preset with automatic step sequencing and dependency management |
| `reasoning_metrics` | summary, by_mode, invocations, fallbacks, config | Observability: summary=all stats, by_mode=mode stats, invocations=call history with filters, fallbacks=usage breakdown, config=debug info |

---

## Key Design Decisions

### 1. Direct Anthropic Integration

- No intermediate service (Langbase)
- Reduced latency and complexity
- Direct access to latest Claude features
- Simplified error handling

### 2. Consolidated Tool Surface

- 15 tools (vs 40 in predecessor)
- Operations as parameters instead of separate tools
- Easier to discover and use
- Consistent API patterns

### 3. Persistent State Management

- SQLite for session persistence
- Checkpoint system for backtracking
- Branch tracking for tree reasoning
- Historical metrics for learning

### 4. Self-Improvement System

- 4-phase loop: Monitor → Analyze → Execute → Learn
- Circuit breaker for safety
- Rate limiting and allowlists
- Rollback capability

---

## See Also

- [API Specification](./API_SPECIFICATION.md) - Complete tool schemas
- [Implementation Details](./IMPLEMENTATION_DETAILS.md) - Technical implementation
- [Tool Reference](./TOOL_REFERENCE.md) - Usage guide and examples
