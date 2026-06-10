# MCP Reasoning Server - Architecture

## Overview

MCP server providing structured reasoning capabilities via direct Anthropic Claude API calls.

**Key Differentiators from mcp-langbase-reasoning:**

- Direct Anthropic API (no Langbase middleman)
- Consolidated reasoning surface (operations as parameters, not 40 separate tools)
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

## Tool Surface (35 Tools)

The surface has grown beyond the original consolidated reasoning set: 17 core
reasoning tools, 7 self-improvement tools, 4 session-management tools, and 7
agent/team tools (17 + 7 + 4 + 7 = 35).

### Core Reasoning — Sequential & Branching (6 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_linear` | - | Single-pass sequential reasoning; process thought and get logical continuation |
| `reasoning_tree` | create, focus, list, complete, summarize | Branching exploration: create=start with 2-4 paths, focus=select branch, list=show branches, complete=mark finished, summarize=synthesize |
| `reasoning_divergent` | - | Creative multi-perspective generation with assumption challenges and force_rebellion mode |
| `reasoning_reflection` | process, evaluate | Meta-cognitive: process=iterative refinement, evaluate=session-wide quality assessment |
| `reasoning_checkpoint` | create, list, restore | Backtracking: create=save state, list=show checkpoints, restore=return with optional new_direction |
| `reasoning_auto` | - | Analyze content and route to optimal reasoning mode (linear/tree/divergent/etc.) |

### Core Reasoning — Routing & Empirical Selection (2 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_meta` | - | Empirical mode selector: classifies the problem and picks the best tool from historical success data |
| `reasoning_confidence_route` | - | Confidence-aware routing: runs an auto-selected mode, escalating low-confidence results to tree reasoning |

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
| `reasoning_metrics` | summary, by_mode, invocations, fallbacks, chains, config | Observability: summary=all stats, by_mode=mode stats, invocations=call history with filters, fallbacks=usage breakdown, chains=tool-composition transition matrix (success rates, common sequences, anti-patterns), config=debug info |

### Self-Improvement (7 tools)

| Tool | Description |
|------|-------------|
| `reasoning_si_status` | Current self-improvement system state, phase, and circuit-breaker status |
| `reasoning_si_diagnoses` | List LLM-generated diagnoses of metric regressions |
| `reasoning_si_overrides` | List active manual overrides on tuned parameters |
| `reasoning_si_approve` | Approve a proposed self-improvement action |
| `reasoning_si_reject` | Reject a proposed self-improvement action |
| `reasoning_si_trigger` | Manually trigger a self-improvement cycle |
| `reasoning_si_rollback` | Roll back an applied self-improvement action |

### Session Management (4 tools)

| Tool | Description |
|------|-------------|
| `reasoning_list_sessions` | List stored reasoning sessions with metadata |
| `reasoning_resume` | Resume a prior session, restoring its mode state and thought chain |
| `reasoning_search` | Semantic search over sessions (Voyage embeddings + rerank); requires `VOYAGE_API_KEY` |
| `reasoning_relate` | Build a session-to-session relatedness graph (cosine + shared-mode/temporal); requires `VOYAGE_API_KEY` |

### Agent & Team (7 tools)

| Tool | Description |
|------|-------------|
| `reasoning_agent_invoke` | Invoke a single named agent |
| `reasoning_agent_list` | List available agents |
| `reasoning_agent_metrics` | Per-agent performance metrics |
| `reasoning_skill_run` | Run a composable skill |
| `reasoning_team_run` | Run a multi-agent team workflow |
| `reasoning_team_list` | List available teams |
| `reasoning_crew_invoke` | Invoke a crew (coordinated multi-agent group) |

---

## Key Design Decisions

### 1. Direct Anthropic Integration

- No intermediate service (Langbase)
- Reduced latency and complexity
- Direct access to latest Claude features
- Simplified error handling

### 2. Consolidated Tool Surface

- Reasoning surface consolidated via operations-as-parameters (vs 40 separate tools in the predecessor); the full surface is now 35 tools across reasoning, self-improvement, session, and agent/team groups
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
