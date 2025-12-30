# MCP Reasoning Tools - Complete API Reference

This document provides detailed API documentation for all 15 reasoning tools.

## Table of Contents

- [Core Reasoning Tools](#core-reasoning-tools)
  - [reasoning_linear](#reasoning_linear)
  - [reasoning_tree](#reasoning_tree)
  - [reasoning_divergent](#reasoning_divergent)
  - [reasoning_reflection](#reasoning_reflection)
  - [reasoning_checkpoint](#reasoning_checkpoint)
  - [reasoning_auto](#reasoning_auto)
- [Graph Reasoning](#graph-reasoning)
  - [reasoning_graph](#reasoning_graph)
- [Analysis Tools](#analysis-tools)
  - [reasoning_detect](#reasoning_detect)
  - [reasoning_decision](#reasoning_decision)
  - [reasoning_evidence](#reasoning_evidence)
- [Advanced Reasoning](#advanced-reasoning)
  - [reasoning_timeline](#reasoning_timeline)
  - [reasoning_mcts](#reasoning_mcts)
  - [reasoning_counterfactual](#reasoning_counterfactual)
- [Infrastructure Tools](#infrastructure-tools)
  - [reasoning_preset](#reasoning_preset)
  - [reasoning_metrics](#reasoning_metrics)

---

## Core Reasoning Tools

### reasoning_linear

Process content with sequential step-by-step reasoning and confidence scoring.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `content` | string | Yes | The content to analyze |
| `session_id` | string | No | Session ID for context continuity |
| `confidence` | number | No | Minimum confidence threshold (0.0-1.0) |

**Response:**

```json
{
  "thought_id": "uuid",
  "session_id": "string",
  "content": "Detailed step-by-step analysis...",
  "confidence": 0.85,
  "next_step": "Suggested next reasoning step"
}
```

**Example:**

```json
{
  "tool": "reasoning_linear",
  "arguments": {
    "content": "Analyze the performance implications of using async/await vs threads",
    "session_id": "perf-analysis-001",
    "confidence": 0.7
  }
}
```

---

### reasoning_tree

Branching exploration for multi-path analysis with 2-4 divergent paths.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `operation` | string | No | Operation: `create`, `focus`, `list`, `complete` (default: `create`) |
| `content` | string | For create | Content to explore |
| `session_id` | string | No | Session ID |
| `branch_id` | string | For focus/complete | Target branch ID |
| `num_branches` | number | No | Number of branches (2-4, default: 3) |
| `completed` | boolean | For complete | Mark branch as completed |

**Operations:**

- **create**: Generate 2-4 divergent exploration branches
- **focus**: Deep-dive into a specific branch
- **list**: Show all branches for a session
- **complete**: Mark a branch as finished

**Response:**

```json
{
  "session_id": "string",
  "branch_id": "uuid",
  "branches": [
    {
      "id": "branch-uuid",
      "content": "Branch exploration content",
      "score": 0.75,
      "status": "active"
    }
  ],
  "recommendation": "Suggested next branch to explore"
}
```

**Example:**

```json
{
  "tool": "reasoning_tree",
  "arguments": {
    "operation": "create",
    "content": "How should we implement user authentication?",
    "num_branches": 4,
    "session_id": "auth-design"
  }
}
```

---

### reasoning_divergent

Generate multiple distinct perspectives with optional assumption challenges.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `content` | string | Yes | Content to analyze from multiple perspectives |
| `session_id` | string | No | Session ID |
| `num_perspectives` | number | No | Number of perspectives (2-5, default: 3) |
| `challenge_assumptions` | boolean | No | Identify and challenge hidden assumptions |
| `force_rebellion` | boolean | No | Enable maximum contrarian thinking mode |

**Response:**

```json
{
  "thought_id": "uuid",
  "session_id": "string",
  "perspectives": [
    {
      "viewpoint": "Perspective name",
      "content": "Detailed reasoning from this viewpoint",
      "novelty_score": 0.8
    }
  ],
  "challenged_assumptions": ["Assumption 1", "Assumption 2"],
  "synthesis": "Unified insight combining all perspectives"
}
```

**Example:**

```json
{
  "tool": "reasoning_divergent",
  "arguments": {
    "content": "Should we rewrite our backend in Rust?",
    "num_perspectives": 4,
    "challenge_assumptions": true,
    "force_rebellion": true
  }
}
```

---

### reasoning_reflection

Meta-cognitive analysis with iterative refinement or session evaluation.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `operation` | string | No | Operation: `process` or `evaluate` (default: `process`) |
| `content` | string | For process | Reasoning content to analyze and improve |
| `session_id` | string | For evaluate | Session ID to evaluate |
| `thought_id` | string | No | Specific thought to analyze |
| `max_iterations` | number | No | Maximum refinement iterations (1-5) |
| `quality_threshold` | number | No | Target quality score (0.0-1.0) |

**Operations:**

- **process**: Analyze and improve specific reasoning content
- **evaluate**: Comprehensive assessment of an entire session

**Response:**

```json
{
  "quality_score": 0.85,
  "thought_id": "uuid",
  "session_id": "string",
  "iterations_used": 2,
  "strengths": ["Clear logic", "Good evidence"],
  "weaknesses": ["Missing edge cases"],
  "recommendations": ["Consider alternative approaches"],
  "refined_content": "Improved version of the reasoning",
  "coherence_score": 0.9
}
```

**Example:**

```json
{
  "tool": "reasoning_reflection",
  "arguments": {
    "operation": "process",
    "content": "We should use microservices because they are popular.",
    "max_iterations": 3,
    "quality_threshold": 0.8
  }
}
```

---

### reasoning_checkpoint

Save and restore reasoning state for non-linear exploration.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `operation` | string | Yes | Operation: `create`, `list`, `restore` |
| `session_id` | string | Yes | Session ID |
| `name` | string | For create | Checkpoint name |
| `description` | string | No | Checkpoint description |
| `checkpoint_id` | string | For restore | Checkpoint to restore |
| `new_direction` | string | For restore | New exploration direction after restore |

**Operations:**

- **create**: Save current reasoning state
- **list**: Show all checkpoints for a session
- **restore**: Return to a previous checkpoint

**Response:**

```json
{
  "session_id": "string",
  "checkpoint_id": "uuid",
  "checkpoints": [
    {
      "id": "uuid",
      "name": "Checkpoint name",
      "description": "Optional description",
      "created_at": "2024-01-15T10:30:00Z",
      "thought_count": 5
    }
  ],
  "restored_state": {
    "context": {...},
    "thought_count": 5
  }
}
```

**Example:**

```json
{
  "tool": "reasoning_checkpoint",
  "arguments": {
    "operation": "create",
    "session_id": "exploration-001",
    "name": "before-risky-approach",
    "description": "Saving state before exploring experimental solution"
  }
}
```

---

### reasoning_auto

Analyze content and automatically select the optimal reasoning mode.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `content` | string | Yes | Content to analyze for mode selection |
| `hints` | array | No | Optional hints to guide mode selection |
| `session_id` | string | No | Session ID |

**Response:**

```json
{
  "selected_mode": "divergent",
  "confidence": 0.85,
  "rationale": "Content requires multiple perspectives due to...",
  "result": {
    "thought_id": "uuid",
    "session_id": "string",
    "characteristics": ["creative", "multi-faceted"],
    "suggested_parameters": {
      "num_perspectives": 4
    },
    "alternative": {
      "mode": "tree",
      "reason": "Could also benefit from structured exploration"
    }
  }
}
```

**Example:**

```json
{
  "tool": "reasoning_auto",
  "arguments": {
    "content": "Design a fault-tolerant distributed caching system",
    "hints": ["architecture", "reliability"]
  }
}
```

---

## Graph Reasoning

### reasoning_graph

Graph-of-Thoughts for complex reasoning chains with multiple interconnected nodes.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `operation` | string | Yes | Operation type (see below) |
| `session_id` | string | Yes | Session ID |
| `content` | string | For most ops | Content to process |
| `problem` | string | No | Problem context |
| `node_id` | string | For score/refine | Target node ID |
| `node_ids` | array | For aggregate | Node IDs to aggregate |
| `k` | number | For generate | Number of continuations (1-10) |
| `threshold` | number | For prune | Prune threshold (0.0-1.0) |
| `terminal_node_ids` | array | For finalize | Terminal nodes for conclusions |

**Operations:**

| Operation | Description |
|-----------|-------------|
| `init` | Initialize a new reasoning graph with root node |
| `generate` | Generate k child nodes from current position |
| `score` | Evaluate quality of a specific node |
| `aggregate` | Synthesize insights from multiple nodes |
| `refine` | Improve a node based on feedback |
| `prune` | Remove low-quality branches below threshold |
| `finalize` | Extract conclusions from terminal nodes |
| `state` | Get current graph structure and statistics |

**Response:**

```json
{
  "session_id": "string",
  "node_id": "uuid",
  "nodes": [
    {
      "id": "uuid",
      "content": "Node content",
      "score": 0.8,
      "depth": 2,
      "parent_id": "parent-uuid"
    }
  ],
  "aggregated_insight": "Synthesized insight",
  "conclusions": ["Conclusion 1", "Conclusion 2"],
  "state": {
    "total_nodes": 15,
    "active_nodes": 12,
    "max_depth": 4,
    "pruned_count": 3
  }
}
```

**Example:**

```json
{
  "tool": "reasoning_graph",
  "arguments": {
    "operation": "init",
    "session_id": "complex-problem-001",
    "content": "Design a real-time recommendation engine",
    "problem": "E-commerce platform with 1M daily users"
  }
}
```

---

## Analysis Tools

### reasoning_detect

Detect cognitive biases and logical fallacies in reasoning.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | string | Yes | Detection type: `biases` or `fallacies` |
| `content` | string | No | Content to analyze |
| `thought_id` | string | No | Thought ID to analyze |
| `session_id` | string | No | Session ID |
| `check_types` | array | No | Specific bias/fallacy types to check |
| `check_formal` | boolean | No | Check formal fallacies (for type=fallacies) |
| `check_informal` | boolean | No | Check informal fallacies (for type=fallacies) |

**Response:**

```json
{
  "detections": [
    {
      "type": "Confirmation Bias",
      "category": "cognitive",
      "severity": "high",
      "confidence": 0.85,
      "evidence": "Text showing the bias",
      "explanation": "Why this is problematic",
      "remediation": "How to correct it"
    }
  ],
  "summary": "2 biases detected. Most severe: Confirmation Bias",
  "overall_quality": 0.6
}
```

**Example:**

```json
{
  "tool": "reasoning_detect",
  "arguments": {
    "type": "biases",
    "content": "Our solution is obviously correct because everyone agrees with it"
  }
}
```

---

### reasoning_decision

Multi-criteria decision analysis with various scoring methods.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | string | No | Analysis type (default: `weighted`) |
| `question` | string | No | Decision question |
| `topic` | string | No | Topic for perspectives analysis |
| `options` | array | No | Options to evaluate |
| `context` | string | No | Additional context |
| `session_id` | string | No | Session ID |

**Analysis Types:**

| Type | Description |
|------|-------------|
| `weighted` | Weighted multi-criteria scoring |
| `pairwise` | Pairwise comparison matrix |
| `topsis` | TOPSIS (distance to ideal solution) |
| `perspectives` | Stakeholder perspective mapping |

**Response:**

```json
{
  "recommendation": "Option A",
  "rankings": [
    {"option": "Option A", "score": 0.85, "rank": 1},
    {"option": "Option B", "score": 0.72, "rank": 2}
  ],
  "stakeholder_map": {
    "key_players": ["CTO", "Lead Developer"],
    "keep_satisfied": ["Finance"],
    "keep_informed": ["QA Team"],
    "minimal_effort": []
  },
  "conflicts": ["Speed vs Quality trade-off"],
  "alignments": ["All agree on security requirements"],
  "rationale": "Detailed decision rationale..."
}
```

**Example:**

```json
{
  "tool": "reasoning_decision",
  "arguments": {
    "type": "topsis",
    "question": "Which database should we use for our new service?",
    "options": ["PostgreSQL", "MongoDB", "DynamoDB"],
    "context": "High-read workload, need strong consistency"
  }
}
```

---

### reasoning_evidence

Evaluate evidence quality with credibility scoring or Bayesian updates.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | string | No | Evaluation type: `assess` or `probabilistic` (default: `assess`) |
| `claim` | string | For assess | Claim to evaluate evidence for |
| `hypothesis` | string | For probabilistic | Hypothesis to update |
| `prior` | number | For probabilistic | Prior probability (0.0-1.0) |
| `context` | string | No | Additional context |
| `session_id` | string | No | Session ID |

**Response:**

```json
{
  "overall_credibility": 0.75,
  "evidence_assessments": [
    {
      "content": "Evidence summary",
      "credibility_score": 0.8,
      "source_tier": "Primary",
      "corroborated_by": [1, 3]
    }
  ],
  "posterior": 0.82,
  "prior": 0.5,
  "likelihood_ratio": 2.3,
  "entropy": 0.65,
  "confidence_interval": {"lower": 0.7, "upper": 0.9},
  "synthesis": "Evidence strength assessment..."
}
```

**Example:**

```json
{
  "tool": "reasoning_evidence",
  "arguments": {
    "type": "probabilistic",
    "hypothesis": "The performance issue is caused by database queries",
    "prior": 0.3,
    "context": "Slow API responses observed during peak hours"
  }
}
```

---

## Advanced Reasoning

### reasoning_timeline

Temporal reasoning with branching timelines for scenario analysis.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `operation` | string | Yes | Operation: `create`, `branch`, `compare`, `merge` |
| `content` | string | No | Timeline content |
| `session_id` | string | No | Session ID |
| `timeline_id` | string | No | Timeline ID |
| `branch_ids` | array | For compare | Branch IDs to compare |
| `source_branch_id` | string | For merge | Source branch |
| `target_branch_id` | string | For merge | Target branch |
| `merge_strategy` | string | For merge | Merge strategy |
| `label` | string | No | Branch label |

**Operations:**

| Operation | Description |
|-----------|-------------|
| `create` | Create a new timeline |
| `branch` | Create alternative timeline branches |
| `compare` | Compare different timeline branches |
| `merge` | Merge timeline branches |

**Response:**

```json
{
  "timeline_id": "uuid",
  "branch_id": "uuid",
  "branches": [
    {
      "id": "uuid",
      "label": "Optimistic scenario",
      "content": "Timeline events...",
      "created_at": "2024-01-15T10:30:00Z"
    }
  ],
  "comparison": {
    "divergence_points": ["Key decision point"],
    "quality_differences": {...},
    "convergence_opportunities": ["Possible merge point"]
  },
  "merged_content": "Merged timeline synthesis"
}
```

**Example:**

```json
{
  "tool": "reasoning_timeline",
  "arguments": {
    "operation": "branch",
    "content": "Project timeline with two possible approaches",
    "session_id": "project-planning"
  }
}
```

---

### reasoning_mcts

Monte Carlo Tree Search with UCB1 exploration for optimal path finding.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `operation` | string | No | Operation: `explore` or `auto_backtrack` (default: `explore`) |
| `content` | string | No | Content to explore |
| `session_id` | string | No | Session ID |
| `node_id` | string | No | Starting node ID |
| `iterations` | number | No | Number of iterations (1-100) |
| `exploration_constant` | number | No | UCB1 exploration constant |
| `simulation_depth` | number | No | Simulation depth (1-20) |
| `quality_threshold` | number | No | Quality threshold for backtracking (0.0-1.0) |
| `auto_execute` | boolean | No | Auto-execute backtrack suggestion |
| `lookback_depth` | number | No | Lookback depth (1-10) |

**Response:**

```json
{
  "session_id": "string",
  "best_path": [
    {
      "node_id": "uuid",
      "content": "Path step description",
      "ucb_score": 0.89,
      "visits": 15
    }
  ],
  "iterations_completed": 50,
  "backtrack_suggestion": {
    "should_backtrack": true,
    "target_step": 3,
    "reason": "Quality dropped significantly",
    "quality_drop": 0.25
  },
  "executed": true
}
```

**Example:**

```json
{
  "tool": "reasoning_mcts",
  "arguments": {
    "operation": "explore",
    "content": "Find optimal caching strategy for distributed system",
    "iterations": 50,
    "simulation_depth": 5,
    "exploration_constant": 1.4
  }
}
```

---

### reasoning_counterfactual

What-if causal analysis using Pearl's Ladder of Causation.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `scenario` | string | Yes | Base scenario to analyze |
| `intervention` | string | Yes | What-if change to apply |
| `session_id` | string | No | Session ID |
| `analysis_depth` | string | No | Depth: `association`, `intervention`, `counterfactual` (default: `counterfactual`) |

**Analysis Depths (Pearl's Ladder):**

| Level | Description |
|-------|-------------|
| `association` | Observational correlations (seeing) |
| `intervention` | Causal effects of actions (doing) |
| `counterfactual` | What would have happened (imagining) |

**Response:**

```json
{
  "counterfactual_outcome": "Projected outcome description",
  "causal_chain": [
    {
      "step": 1,
      "cause": "Initial change",
      "effect": "First-order effect",
      "probability": 0.8
    }
  ],
  "session_id": "string",
  "original_scenario": "Base scenario",
  "intervention_applied": "What-if change",
  "analysis_depth": "counterfactual",
  "key_differences": ["Difference 1", "Difference 2"],
  "confidence": 0.75,
  "assumptions": ["Assumption 1", "Assumption 2"]
}
```

**Example:**

```json
{
  "tool": "reasoning_counterfactual",
  "arguments": {
    "scenario": "We launched the product with feature set A",
    "intervention": "What if we had launched with minimal viable product instead?",
    "analysis_depth": "counterfactual"
  }
}
```

---

## Infrastructure Tools

### reasoning_preset

Execute pre-defined reasoning workflows.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `operation` | string | Yes | Operation: `list` or `run` |
| `preset_id` | string | For run | Preset ID to execute |
| `category` | string | For list | Filter by category |
| `inputs` | object | For run | Preset-specific inputs |
| `session_id` | string | No | Session ID |

**Available Presets:**

| Preset ID | Category | Description |
|-----------|----------|-------------|
| `code-review` | code_quality | Analyze code with bias detection and alternatives |
| `debug-analysis` | analysis | Hypothesis-driven debugging with evidence |
| `architecture-decision` | decision | Multi-factor architectural decisions |
| `strategic-decision` | decision | Stakeholder-aware strategic planning |
| `evidence-conclusion` | research | Evidence-based research synthesis |

**Response:**

```json
{
  "presets": [
    {
      "id": "code-review",
      "name": "Code Review",
      "description": "Comprehensive code analysis",
      "category": "code_quality",
      "required_inputs": ["code"]
    }
  ],
  "execution_result": {
    "preset_id": "code-review",
    "steps_completed": 3,
    "total_steps": 3,
    "step_results": [...],
    "final_output": {...}
  },
  "session_id": "string"
}
```

**Example:**

```json
{
  "tool": "reasoning_preset",
  "arguments": {
    "operation": "run",
    "preset_id": "debug-analysis",
    "inputs": {
      "problem": "API returns 500 errors intermittently",
      "context": "Happens during high traffic periods"
    }
  }
}
```

---

### reasoning_metrics

Query usage metrics and observability data.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Query type (see below) |
| `mode_name` | string | For by_mode | Mode to query stats for |
| `tool_name` | string | No | Filter by tool name |
| `session_id` | string | No | Filter by session |
| `success_only` | boolean | No | Only show successful invocations |
| `limit` | number | No | Result limit (1-1000) |

**Query Types:**

| Query | Description |
|-------|-------------|
| `summary` | Overall usage statistics |
| `by_mode` | Statistics for a specific mode |
| `invocations` | List of invocation records |
| `fallbacks` | Fallback events |
| `config` | Current configuration |

**Response:**

```json
{
  "summary": {
    "total_calls": 1500,
    "success_rate": 0.97,
    "avg_latency_ms": 245.5,
    "by_mode": {
      "linear": {"calls": 500, "success_rate": 0.98},
      "tree": {"calls": 300, "success_rate": 0.95}
    }
  },
  "mode_stats": {
    "mode_name": "linear",
    "call_count": 500,
    "success_count": 490,
    "failure_count": 10,
    "success_rate": 0.98,
    "latency_p50_ms": 200,
    "latency_p95_ms": 450,
    "latency_p99_ms": 800
  },
  "invocations": [...],
  "config": {...}
}
```

**Example:**

```json
{
  "tool": "reasoning_metrics",
  "arguments": {
    "query": "by_mode",
    "mode_name": "decision",
    "limit": 100
  }
}
```

---

## Session Management

All tools support `session_id` for context continuity. Sessions enable:

- **Context Preservation**: Previous thoughts inform new reasoning
- **Checkpointing**: Save and restore reasoning state
- **Metrics Tracking**: Session-level analytics
- **Cross-Tool Coordination**: Share context between different reasoning modes

**Best Practices:**

1. Use consistent session IDs for related reasoning tasks
2. Create checkpoints before exploring risky approaches
3. Use `reasoning_auto` when unsure which mode to use
4. Combine tools in workflows (e.g., divergent -> decision -> reflection)

---

## Error Handling

All tools return errors in response fields when operations fail:

```json
{
  "session_id": "provided-session-id",
  "content": "ERROR: Description of what went wrong",
  "confidence": 0.0
}
```

Common error scenarios:
- Empty content provided
- Invalid operation for the tool
- API call failures (retried automatically)
- Parsing failures from LLM responses
