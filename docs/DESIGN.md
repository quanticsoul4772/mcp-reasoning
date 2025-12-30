# MCP Reasoning Server - Design Document

## Overview

MCP server providing structured reasoning capabilities via direct Anthropic Claude API calls.

**Key Differentiators from mcp-langbase-reasoning:**
- Direct Anthropic API (no Langbase middleman)
- Consolidated tool surface (15 tools vs 40)
- Anthropic Claude models (user preference)
- Simplified architecture

---

## 1. Architecture

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

---

## 2. Consolidated Tool Surface (15 Tools)

### 2.1 Core Reasoning (6 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_linear` | - | Single-pass sequential reasoning; process thought and get logical continuation |
| `reasoning_tree` | create, focus, list, complete | Branching exploration: create=start with 2-4 paths, focus=select branch, list=show branches, complete=mark finished |
| `reasoning_divergent` | - | Creative multi-perspective generation with assumption challenges and force_rebellion mode |
| `reasoning_reflection` | process, evaluate | Meta-cognitive: process=iterative refinement, evaluate=session-wide quality assessment |
| `reasoning_checkpoint` | create, list, restore | Backtracking: create=save state, list=show checkpoints, restore=return with optional new_direction |
| `reasoning_auto` | - | Analyze content and route to optimal reasoning mode (linear/tree/divergent/etc.) |

### 2.2 Graph Reasoning (1 tool)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_graph` | init, generate, score, aggregate, refine, prune, finalize, state | Graph-of-Thoughts: init=create graph, generate=expand k nodes, score=evaluate quality, aggregate=merge nodes, refine=improve via self-critique, prune=remove weak nodes, finalize=extract conclusions, state=show structure |

### 2.3 Analysis (3 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_detect` | biases, fallacies | Cognitive errors: biases=confirmation/anchoring/sunk-cost with remediation, fallacies=ad-hominem/straw-man/false-dichotomy with formal/informal categories |
| `reasoning_decision` | weighted, pairwise, topsis, perspectives | Decisions: weighted=scored ranking, pairwise=direct comparison, topsis=ideal-point distance, perspectives=stakeholder power/interest mapping |
| `reasoning_evidence` | assess, probabilistic | Evidence: assess=source credibility/corroboration/chain-of-custody, probabilistic=Bayesian prior->posterior with likelihoods |

### 2.4 Advanced Reasoning (3 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_timeline` | create, branch, compare, merge | Temporal: create=new timeline, branch=fork path, compare=analyze divergence, merge=synthesize branches with strategy |
| `reasoning_mcts` | explore, auto_backtrack | MCTS: explore=UCB1-guided search with iterations/depth, auto_backtrack=quality-triggered backtracking with lookback |
| `reasoning_counterfactual` | - | What-if causal analysis using Pearl's Ladder: scenario + intervention -> causal consequences |

### 2.5 Infrastructure (2 tools)

| Tool | Operations | Description |
|------|------------|-------------|
| `reasoning_preset` | list, run | Workflows: list=show presets by category, run=execute preset with automatic step sequencing and dependency management |
| `reasoning_metrics` | summary, by_mode, invocations, fallbacks, config | Observability: summary=all stats, by_mode=mode stats, invocations=call history with filters, fallbacks=usage breakdown, config=debug info |

---

## 3. Tool Schemas

### 3.1 reasoning_linear

```json
{
  "name": "reasoning_linear",
  "description": "Single-pass sequential reasoning. Process a thought and get a logical continuation with confidence scoring.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "content": { "type": "string", "description": "Thought to process" },
      "session_id": { "type": "string", "description": "Session for context continuity" },
      "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
    },
    "required": ["content"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "thought_id": { "type": "string", "description": "Unique identifier for this thought" },
      "session_id": { "type": "string", "description": "Session this thought belongs to" },
      "content": { "type": "string", "description": "The reasoning continuation" },
      "confidence": { "type": "number", "minimum": 0, "maximum": 1, "description": "Model's confidence in the reasoning" },
      "next_step": { "type": "string", "description": "Suggested next reasoning step" }
    },
    "required": ["thought_id", "session_id", "content", "confidence"]
  },
  "annotations": {
    "title": "Linear Reasoning",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": false,
    "openWorldHint": true
  }
}
```

### 3.2 reasoning_tree

```json
{
  "name": "reasoning_tree",
  "description": "Branching exploration with multiple reasoning paths. Operations: CREATE starts new exploration from content, returns root branch_id and 2-4 divergent branches; FOCUS selects a specific branch by branch_id for continued reasoning; LIST shows all branches in the session with their status and scores; COMPLETE marks a branch as finished (completed=true) or abandoned (completed=false).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "operation": {
        "type": "string",
        "enum": ["create", "focus", "list", "complete"],
        "default": "create",
        "description": "create=start exploration, focus=select branch, list=show branches, complete=finish branch"
      },
      "content": { "type": "string", "description": "Content to explore (for create)" },
      "session_id": { "type": "string" },
      "branch_id": { "type": "string", "description": "Branch ID (for focus/complete)" },
      "num_branches": { "type": "integer", "minimum": 2, "maximum": 4, "default": 3 },
      "completed": { "type": "boolean", "default": true, "description": "For complete operation" }
    },
    "required": []
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "session_id": { "type": "string" },
      "branch_id": { "type": "string", "description": "Current/created branch ID" },
      "branches": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": { "type": "string" },
            "content": { "type": "string" },
            "score": { "type": "number" },
            "status": { "type": "string", "enum": ["active", "completed", "abandoned"] }
          }
        },
        "description": "List of branches (for create/list)"
      },
      "recommendation": { "type": "string", "description": "Suggested next branch to explore" }
    },
    "required": ["session_id"]
  },
  "annotations": {
    "title": "Tree Reasoning",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": false,
    "openWorldHint": true
  }
}
```

### 3.3 reasoning_divergent

```json
{
  "name": "reasoning_divergent",
  "description": "Creative reasoning generating novel perspectives. Challenges assumptions, synthesizes diverse viewpoints, and produces unconventional solutions with optional force_rebellion mode for maximum creativity.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "content": { "type": "string" },
      "session_id": { "type": "string" },
      "num_perspectives": { "type": "integer", "minimum": 2, "maximum": 5, "default": 3 },
      "challenge_assumptions": { "type": "boolean", "default": false },
      "force_rebellion": { "type": "boolean", "default": false }
    },
    "required": ["content"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "thought_id": { "type": "string" },
      "session_id": { "type": "string" },
      "perspectives": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "viewpoint": { "type": "string", "description": "Name/description of this perspective" },
            "content": { "type": "string", "description": "Reasoning from this perspective" },
            "novelty_score": { "type": "number", "minimum": 0, "maximum": 1 }
          }
        }
      },
      "challenged_assumptions": { "type": "array", "items": { "type": "string" } },
      "synthesis": { "type": "string", "description": "Unified insight from all perspectives" }
    },
    "required": ["thought_id", "session_id", "perspectives"]
  },
  "annotations": {
    "title": "Divergent Reasoning",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": false,
    "openWorldHint": true
  }
}
```

### 3.4 reasoning_reflection

```json
{
  "name": "reasoning_reflection",
  "description": "Meta-cognitive reasoning that analyzes and improves reasoning quality. Operations: PROCESS reflects on content or thought_id, identifies strengths/weaknesses, iteratively refines up to max_iterations until quality_threshold reached; EVALUATE assesses an entire session's reasoning quality, coherence, and provides recommendations.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "operation": {
        "type": "string",
        "enum": ["process", "evaluate"],
        "default": "process",
        "description": "process=reflect on content/thought, evaluate=assess session quality"
      },
      "content": { "type": "string", "description": "Content to reflect on (for process)" },
      "thought_id": { "type": "string", "description": "Existing thought to analyze" },
      "session_id": { "type": "string", "description": "Required for evaluate operation" },
      "max_iterations": { "type": "integer", "minimum": 1, "maximum": 5, "default": 3 },
      "quality_threshold": { "type": "number", "minimum": 0, "maximum": 1, "default": 0.8 }
    },
    "required": []
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "thought_id": { "type": "string" },
      "session_id": { "type": "string" },
      "quality_score": { "type": "number", "minimum": 0, "maximum": 1 },
      "iterations_used": { "type": "integer" },
      "strengths": { "type": "array", "items": { "type": "string" } },
      "weaknesses": { "type": "array", "items": { "type": "string" } },
      "recommendations": { "type": "array", "items": { "type": "string" } },
      "refined_content": { "type": "string", "description": "Improved reasoning (for process)" },
      "coherence_score": { "type": "number", "description": "Session coherence (for evaluate)" }
    },
    "required": ["quality_score"]
  },
  "annotations": {
    "title": "Reflection",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": false,
    "openWorldHint": true
  }
}
```

### 3.5 reasoning_checkpoint

```json
{
  "name": "reasoning_checkpoint",
  "description": "State management for non-linear exploration with backtracking. Operations: CREATE saves current reasoning state with a name and optional description, returns checkpoint_id for later restoration; LIST shows all available checkpoints for the session; RESTORE returns to a saved checkpoint by checkpoint_id and optionally explores a new_direction from that point.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "operation": {
        "type": "string",
        "enum": ["create", "list", "restore"],
        "description": "create=save state, list=show checkpoints, restore=return to checkpoint"
      },
      "session_id": { "type": "string" },
      "checkpoint_id": { "type": "string", "description": "For restore operation" },
      "name": { "type": "string", "description": "Checkpoint name (for create)" },
      "description": { "type": "string" },
      "new_direction": { "type": "string", "description": "New approach after restore" }
    },
    "required": ["operation", "session_id"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "checkpoint_id": { "type": "string", "description": "ID of created/restored checkpoint" },
      "session_id": { "type": "string" },
      "checkpoints": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": { "type": "string" },
            "name": { "type": "string" },
            "description": { "type": "string" },
            "created_at": { "type": "string", "format": "date-time" },
            "thought_count": { "type": "integer" }
          }
        },
        "description": "List of checkpoints (for list operation)"
      },
      "restored_state": { "type": "object", "description": "Session state after restore" }
    },
    "required": ["session_id"]
  },
  "annotations": {
    "title": "Checkpoint Management",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": true,
    "openWorldHint": false
  }
}
```

### 3.6 reasoning_auto

```json
{
  "name": "reasoning_auto",
  "description": "Automatically select best reasoning mode based on content analysis. Analyzes the input and routes to the optimal mode (linear, tree, divergent, reflection, graph, etc.) with explanation.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "content": { "type": "string" },
      "hints": { "type": "array", "items": { "type": "string" } },
      "session_id": { "type": "string" }
    },
    "required": ["content"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "selected_mode": { "type": "string", "enum": ["linear", "tree", "divergent", "reflection", "graph", "timeline", "mcts", "counterfactual"] },
      "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
      "rationale": { "type": "string", "description": "Why this mode was selected" },
      "result": { "type": "object", "description": "Result from executing the selected mode" }
    },
    "required": ["selected_mode", "confidence", "result"]
  },
  "annotations": {
    "title": "Auto Mode Selection",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": false,
    "openWorldHint": true
  }
}
```

### 3.7 reasoning_graph

```json
{
  "name": "reasoning_graph",
  "description": "Graph-of-Thoughts reasoning with nodes and edges. Operations: INIT creates new graph with root node from content; GENERATE creates k diverse continuations from node_id (or active nodes); SCORE evaluates node quality on relevance/validity/depth/novelty; AGGREGATE merges multiple node_ids into unified insight; REFINE improves a node through self-critique; PRUNE removes nodes below score threshold; FINALIZE marks terminal nodes and extracts conclusions; STATE returns current graph structure with node counts.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "operation": {
        "type": "string",
        "enum": ["init", "generate", "score", "aggregate", "refine", "prune", "finalize", "state"],
        "description": "init=create graph, generate=expand nodes, score=evaluate, aggregate=merge, refine=improve, prune=remove weak, finalize=extract conclusions, state=show structure"
      },
      "session_id": { "type": "string" },
      "content": { "type": "string", "description": "For init operation" },
      "problem": { "type": "string", "description": "Problem context" },
      "node_id": { "type": "string", "description": "Target node for operations" },
      "node_ids": { "type": "array", "items": { "type": "string" }, "description": "For aggregate" },
      "k": { "type": "integer", "minimum": 1, "maximum": 10, "default": 3, "description": "Continuations to generate" },
      "threshold": { "type": "number", "minimum": 0, "maximum": 1, "default": 0.3, "description": "Prune threshold" },
      "terminal_node_ids": { "type": "array", "items": { "type": "string" }, "description": "For finalize" },
      "config": {
        "type": "object",
        "properties": {
          "max_nodes": { "type": "integer", "default": 100 },
          "max_depth": { "type": "integer", "default": 10 },
          "prune_threshold": { "type": "number", "default": 0.3 }
        }
      }
    },
    "required": ["operation", "session_id"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "session_id": { "type": "string" },
      "node_id": { "type": "string", "description": "Created/modified node ID" },
      "nodes": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": { "type": "string" },
            "content": { "type": "string" },
            "score": { "type": "number" },
            "depth": { "type": "integer" },
            "parent_id": { "type": "string" }
          }
        },
        "description": "Generated/affected nodes"
      },
      "aggregated_insight": { "type": "string", "description": "For aggregate operation" },
      "conclusions": { "type": "array", "items": { "type": "string" }, "description": "For finalize" },
      "state": {
        "type": "object",
        "properties": {
          "total_nodes": { "type": "integer" },
          "active_nodes": { "type": "integer" },
          "max_depth": { "type": "integer" },
          "pruned_count": { "type": "integer" }
        },
        "description": "For state operation"
      }
    },
    "required": ["session_id"]
  },
  "annotations": {
    "title": "Graph-of-Thoughts",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": false,
    "openWorldHint": true
  }
}
```

### 3.8 reasoning_detect

```json
{
  "name": "reasoning_detect",
  "description": "Analyze reasoning for cognitive errors. Types: BIASES detects confirmation bias, anchoring, availability heuristic, sunk cost fallacy, etc. with severity scores and remediation suggestions; FALLACIES identifies ad hominem, straw man, false dichotomy, appeal to authority, circular reasoning with formal (check_formal) and informal (check_informal) categories.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "type": {
        "type": "string",
        "enum": ["biases", "fallacies"],
        "description": "biases=cognitive bias detection, fallacies=logical fallacy detection"
      },
      "content": { "type": "string" },
      "thought_id": { "type": "string" },
      "session_id": { "type": "string" },
      "check_types": { "type": "array", "items": { "type": "string" }, "description": "Specific types to check" },
      "check_formal": { "type": "boolean", "default": true, "description": "For fallacies" },
      "check_informal": { "type": "boolean", "default": true, "description": "For fallacies" }
    },
    "required": ["type"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "detections": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "type": { "type": "string", "description": "Name of bias/fallacy" },
            "category": { "type": "string", "description": "formal/informal for fallacies" },
            "severity": { "type": "string", "enum": ["low", "medium", "high", "critical"] },
            "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
            "evidence": { "type": "string", "description": "Text that triggered detection" },
            "explanation": { "type": "string" },
            "remediation": { "type": "string", "description": "How to fix" }
          }
        }
      },
      "summary": { "type": "string" },
      "overall_quality": { "type": "number", "minimum": 0, "maximum": 1 }
    },
    "required": ["detections"]
  },
  "annotations": {
    "title": "Bias/Fallacy Detection",
    "readOnlyHint": true,
    "destructiveHint": false,
    "idempotentHint": true,
    "openWorldHint": false
  }
}
```

### 3.9 reasoning_decision

```json
{
  "name": "reasoning_decision",
  "description": "Multi-criteria decision analysis and stakeholder mapping. Types: WEIGHTED uses weighted scoring across criteria to rank options; PAIRWISE compares options directly against each other; TOPSIS uses ideal/anti-ideal point distance for complex tradeoffs; PERSPECTIVES maps stakeholders to power/interest quadrants (KeyPlayer, KeepSatisfied, KeepInformed, MinimalEffort) and identifies conflicts/alignments.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "type": {
        "type": "string",
        "enum": ["weighted", "pairwise", "topsis", "perspectives"],
        "default": "weighted",
        "description": "weighted=scored ranking, pairwise=direct comparison, topsis=ideal point, perspectives=stakeholder analysis"
      },
      "question": { "type": "string", "description": "Decision question (for weighted/pairwise/topsis)" },
      "topic": { "type": "string", "description": "Analysis topic (for perspectives)" },
      "options": { "type": "array", "items": { "type": "string" }, "minItems": 2 },
      "criteria": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "name": { "type": "string" },
            "weight": { "type": "number", "minimum": 0, "maximum": 1 }
          }
        }
      },
      "stakeholders": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "name": { "type": "string" },
            "role": { "type": "string" },
            "power_level": { "type": "number" },
            "interest_level": { "type": "number" }
          }
        }
      },
      "session_id": { "type": "string" },
      "context": { "type": "string" }
    },
    "required": ["type"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "recommendation": { "type": "string", "description": "Best option/action" },
      "rankings": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "option": { "type": "string" },
            "score": { "type": "number" },
            "rank": { "type": "integer" }
          }
        }
      },
      "stakeholder_map": {
        "type": "object",
        "properties": {
          "key_players": { "type": "array", "items": { "type": "string" } },
          "keep_satisfied": { "type": "array", "items": { "type": "string" } },
          "keep_informed": { "type": "array", "items": { "type": "string" } },
          "minimal_effort": { "type": "array", "items": { "type": "string" } }
        },
        "description": "For perspectives type"
      },
      "conflicts": { "type": "array", "items": { "type": "string" } },
      "alignments": { "type": "array", "items": { "type": "string" } },
      "rationale": { "type": "string" }
    },
    "required": ["recommendation"]
  },
  "annotations": {
    "title": "Decision Analysis",
    "readOnlyHint": true,
    "destructiveHint": false,
    "idempotentHint": true,
    "openWorldHint": false
  }
}
```

### 3.10 reasoning_evidence

```json
{
  "name": "reasoning_evidence",
  "description": "Evidence quality assessment and Bayesian inference. Types: ASSESS evaluates evidence for a claim with source credibility scoring (primary/secondary/tertiary/expert/anecdotal), corroboration tracking, and chain of custody analysis; PROBABILISTIC performs Bayesian updates from prior probability through evidence likelihoods to compute posterior probability with entropy and uncertainty metrics.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "type": {
        "type": "string",
        "enum": ["assess", "probabilistic"],
        "default": "assess",
        "description": "assess=evidence quality evaluation, probabilistic=Bayesian belief update"
      },
      "claim": { "type": "string", "description": "For assess" },
      "hypothesis": { "type": "string", "description": "For probabilistic" },
      "prior": { "type": "number", "minimum": 0, "maximum": 1, "description": "Prior probability" },
      "evidence": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "content": { "type": "string" },
            "source": { "type": "string" },
            "source_type": { "type": "string", "enum": ["primary", "secondary", "tertiary", "expert", "anecdotal"] },
            "likelihood_if_true": { "type": "number" },
            "likelihood_if_false": { "type": "number" }
          }
        },
        "minItems": 1
      },
      "session_id": { "type": "string" },
      "context": { "type": "string" }
    },
    "required": ["type", "evidence"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "overall_credibility": { "type": "number", "minimum": 0, "maximum": 1 },
      "evidence_assessments": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "content": { "type": "string" },
            "credibility_score": { "type": "number" },
            "source_tier": { "type": "string" },
            "corroborated_by": { "type": "array", "items": { "type": "integer" } }
          }
        },
        "description": "For assess type"
      },
      "posterior": { "type": "number", "minimum": 0, "maximum": 1, "description": "For probabilistic" },
      "prior": { "type": "number", "description": "For probabilistic" },
      "likelihood_ratio": { "type": "number" },
      "entropy": { "type": "number", "description": "Uncertainty measure" },
      "confidence_interval": {
        "type": "object",
        "properties": {
          "lower": { "type": "number" },
          "upper": { "type": "number" }
        }
      },
      "synthesis": { "type": "string" }
    },
    "required": ["overall_credibility"]
  },
  "annotations": {
    "title": "Evidence Assessment",
    "readOnlyHint": true,
    "destructiveHint": false,
    "idempotentHint": true,
    "openWorldHint": false
  }
}
```

### 3.11 reasoning_timeline

```json
{
  "name": "reasoning_timeline",
  "description": "Timeline-based temporal reasoning with parallel path exploration. Operations: CREATE initializes a new timeline with content and optional metadata; BRANCH creates alternative exploration path from parent_branch_id or current point; COMPARE analyzes divergence points, quality differences, and convergence opportunities between branch_ids; MERGE synthesizes two branches (source into target) using strategy: synthesize/prefer_source/prefer_target/interleave.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "operation": {
        "type": "string",
        "enum": ["create", "branch", "compare", "merge"],
        "description": "create=new timeline, branch=fork path, compare=analyze branches, merge=combine branches"
      },
      "content": { "type": "string", "description": "For create/branch" },
      "session_id": { "type": "string" },
      "timeline_id": { "type": "string", "description": "For branch/compare/merge" },
      "branch_ids": { "type": "array", "items": { "type": "string" }, "description": "For compare" },
      "source_branch_id": { "type": "string", "description": "For merge" },
      "target_branch_id": { "type": "string", "description": "For merge" },
      "parent_branch_id": { "type": "string", "description": "For branch" },
      "merge_strategy": {
        "type": "string",
        "enum": ["synthesize", "prefer_source", "prefer_target", "interleave"],
        "default": "synthesize"
      },
      "label": { "type": "string" },
      "metadata": { "type": "object" }
    },
    "required": ["operation"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "timeline_id": { "type": "string" },
      "branch_id": { "type": "string" },
      "branches": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": { "type": "string" },
            "label": { "type": "string" },
            "content": { "type": "string" },
            "created_at": { "type": "string", "format": "date-time" }
          }
        }
      },
      "comparison": {
        "type": "object",
        "properties": {
          "divergence_points": { "type": "array", "items": { "type": "string" } },
          "quality_differences": { "type": "object" },
          "convergence_opportunities": { "type": "array", "items": { "type": "string" } }
        },
        "description": "For compare operation"
      },
      "merged_content": { "type": "string", "description": "For merge operation" }
    },
    "required": ["timeline_id"]
  },
  "annotations": {
    "title": "Timeline Reasoning",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": false,
    "openWorldHint": true
  }
}
```

### 3.12 reasoning_mcts

```json
{
  "name": "reasoning_mcts",
  "description": "Monte Carlo Tree Search for guided reasoning exploration. Operations: EXPLORE uses UCB1 formula (Q/N + c*sqrt(ln(N_parent)/N)) to balance exploitation of promising paths with exploration of novel ones, runs iterations rollouts with simulation_depth, controlled by exploration_constant (default sqrt(2)); AUTO_BACKTRACK monitors reasoning quality and suggests/executes backtracking when quality drops below quality_threshold, looking back lookback_depth steps.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "operation": {
        "type": "string",
        "enum": ["explore", "auto_backtrack"],
        "default": "explore",
        "description": "explore=MCTS search with UCB1, auto_backtrack=quality-triggered backtracking"
      },
      "content": { "type": "string", "description": "For explore" },
      "session_id": { "type": "string" },
      "node_id": { "type": "string" },
      "iterations": { "type": "integer", "minimum": 1, "maximum": 100, "default": 10 },
      "exploration_constant": { "type": "number", "minimum": 0, "maximum": 10, "default": 1.414 },
      "simulation_depth": { "type": "integer", "minimum": 1, "maximum": 20, "default": 5 },
      "quality_threshold": { "type": "number", "minimum": 0, "maximum": 1, "default": 0.5 },
      "auto_execute": { "type": "boolean", "default": false },
      "lookback_depth": { "type": "integer", "minimum": 1, "maximum": 10, "default": 5 }
    },
    "required": ["operation"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "session_id": { "type": "string" },
      "best_path": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "node_id": { "type": "string" },
            "content": { "type": "string" },
            "ucb_score": { "type": "number" },
            "visits": { "type": "integer" }
          }
        },
        "description": "For explore operation"
      },
      "iterations_completed": { "type": "integer" },
      "backtrack_suggestion": {
        "type": "object",
        "properties": {
          "should_backtrack": { "type": "boolean" },
          "target_step": { "type": "integer" },
          "reason": { "type": "string" },
          "quality_drop": { "type": "number" }
        },
        "description": "For auto_backtrack operation"
      },
      "executed": { "type": "boolean", "description": "Whether backtrack was auto-executed" }
    },
    "required": ["session_id"]
  },
  "annotations": {
    "title": "MCTS Exploration",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": false,
    "openWorldHint": true
  }
}
```

### 3.13 reasoning_counterfactual

```json
{
  "name": "reasoning_counterfactual",
  "description": "Counterfactual what-if analysis using Pearl's Ladder of Causation. Explores alternative scenarios by modifying assumptions and tracing causal consequences at three depths: ASSOCIATION (correlation), INTERVENTION (do-calculus), COUNTERFACTUAL (full causal reasoning).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "scenario": { "type": "string", "description": "Base scenario" },
      "intervention": { "type": "string", "description": "What-if change" },
      "session_id": { "type": "string" },
      "analysis_depth": {
        "type": "string",
        "enum": ["association", "intervention", "counterfactual"],
        "default": "counterfactual"
      },
      "causal_model": {
        "type": "object",
        "properties": {
          "variables": { "type": "array", "items": { "type": "string" } },
          "relationships": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "cause": { "type": "string" },
                "effect": { "type": "string" },
                "strength": { "type": "number" }
              }
            }
          }
        }
      }
    },
    "required": ["scenario", "intervention"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "session_id": { "type": "string" },
      "original_scenario": { "type": "string" },
      "intervention_applied": { "type": "string" },
      "analysis_depth": { "type": "string" },
      "causal_chain": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "step": { "type": "integer" },
            "cause": { "type": "string" },
            "effect": { "type": "string" },
            "probability": { "type": "number" }
          }
        }
      },
      "counterfactual_outcome": { "type": "string" },
      "key_differences": { "type": "array", "items": { "type": "string" } },
      "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
      "assumptions": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["counterfactual_outcome", "causal_chain"]
  },
  "annotations": {
    "title": "Counterfactual Analysis",
    "readOnlyHint": true,
    "destructiveHint": false,
    "idempotentHint": true,
    "openWorldHint": false
  }
}
```

### 3.14 reasoning_preset

```json
{
  "name": "reasoning_preset",
  "description": "Pre-defined multi-step reasoning workflows. Operations: LIST shows available presets (code-review, debug-analysis, architecture-decision, etc.) optionally filtered by category; RUN executes a preset_id workflow with automatic step sequencing, dependency management, and result aggregation using provided inputs.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "operation": {
        "type": "string",
        "enum": ["list", "run"],
        "description": "list=show available presets, run=execute preset workflow"
      },
      "preset_id": { "type": "string", "description": "For run operation" },
      "category": { "type": "string", "description": "Filter for list" },
      "inputs": { "type": "object", "description": "Preset inputs for run" },
      "session_id": { "type": "string" }
    },
    "required": ["operation"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "presets": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": { "type": "string" },
            "name": { "type": "string" },
            "description": { "type": "string" },
            "category": { "type": "string" },
            "required_inputs": { "type": "array", "items": { "type": "string" } }
          }
        },
        "description": "For list operation"
      },
      "execution_result": {
        "type": "object",
        "properties": {
          "preset_id": { "type": "string" },
          "steps_completed": { "type": "integer" },
          "total_steps": { "type": "integer" },
          "step_results": { "type": "array", "items": { "type": "object" } },
          "final_output": { "type": "object" }
        },
        "description": "For run operation"
      },
      "session_id": { "type": "string" }
    }
  },
  "annotations": {
    "title": "Workflow Presets",
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": false,
    "openWorldHint": true
  }
}
```

### 3.15 reasoning_metrics

```json
{
  "name": "reasoning_metrics",
  "description": "Observability, usage statistics, and debugging. Queries: SUMMARY returns aggregated stats across all modes (call counts, success rates, latency); BY_MODE returns detailed stats for a specific mode_name; INVOCATIONS returns call history with inputs/outputs/latency/status, filterable by tool_name/session_id/success_only with limit; FALLBACKS shows fallback usage breakdown and recommendations; CONFIG returns current pipe configuration for debugging.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "enum": ["summary", "by_mode", "invocations", "fallbacks", "config"],
        "description": "summary=all stats, by_mode=mode stats, invocations=call history, fallbacks=fallback usage, config=debug info"
      },
      "mode_name": { "type": "string", "description": "For by_mode query" },
      "tool_name": { "type": "string", "description": "Filter for invocations" },
      "session_id": { "type": "string", "description": "Filter for invocations" },
      "success_only": { "type": "boolean" },
      "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 100 }
    },
    "required": ["query"]
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "summary": {
        "type": "object",
        "properties": {
          "total_calls": { "type": "integer" },
          "success_rate": { "type": "number" },
          "avg_latency_ms": { "type": "number" },
          "by_mode": { "type": "object", "additionalProperties": { "type": "integer" } }
        },
        "description": "For summary query"
      },
      "mode_stats": {
        "type": "object",
        "properties": {
          "mode_name": { "type": "string" },
          "call_count": { "type": "integer" },
          "success_count": { "type": "integer" },
          "failure_count": { "type": "integer" },
          "success_rate": { "type": "number" },
          "latency_p50_ms": { "type": "number" },
          "latency_p95_ms": { "type": "number" },
          "latency_p99_ms": { "type": "number" }
        },
        "description": "For by_mode query"
      },
      "invocations": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": { "type": "string" },
            "tool_name": { "type": "string" },
            "session_id": { "type": "string" },
            "success": { "type": "boolean" },
            "latency_ms": { "type": "integer" },
            "created_at": { "type": "string", "format": "date-time" }
          }
        },
        "description": "For invocations query"
      },
      "config": { "type": "object", "description": "For config query" }
    }
  },
  "annotations": {
    "title": "Metrics & Observability",
    "readOnlyHint": true,
    "destructiveHint": false,
    "idempotentHint": true,
    "openWorldHint": false
  }
}
```

---

## 4. Module Structure

```
src/
├── main.rs                 # Entry point, CLI
├── lib.rs                  # Module exports
├── config/
│   └── mod.rs              # Configuration from env
├── error/
│   └── mod.rs              # Error types (AnthropicError, etc.)
├── anthropic/
│   ├── mod.rs              # Public exports
│   ├── client.rs           # AnthropicClient
│   ├── types.rs            # ReasoningMode, Request/Response types
│   └── config.rs           # Model/temperature settings per mode
├── prompts/
│   └── mod.rs              # All system prompts
├── modes/
│   ├── mod.rs              # Mode exports
│   ├── core.rs             # ModeCore (storage + client)
│   ├── linear.rs           # LinearMode
│   ├── tree.rs             # TreeMode (with focus/list/complete)
│   ├── divergent.rs        # DivergentMode
│   ├── reflection.rs       # ReflectionMode (with evaluate)
│   ├── checkpoint.rs       # CheckpointMode (create/list/restore)
│   ├── auto.rs             # AutoMode (router)
│   ├── graph.rs            # GraphMode (all GoT operations)
│   ├── detect.rs           # DetectMode (biases/fallacies)
│   ├── decision.rs         # DecisionMode (weighted/pairwise/topsis/perspectives)
│   ├── evidence.rs         # EvidenceMode (assess/probabilistic)
│   ├── timeline.rs         # TimelineMode (create/branch/compare/merge)
│   ├── mcts.rs             # MctsMode (explore/auto_backtrack)
│   └── counterfactual.rs   # CounterfactualMode
├── presets/
│   ├── mod.rs
│   ├── registry.rs         # Preset registry
│   └── executor.rs         # Preset execution
├── server/
│   ├── mod.rs              # AppState
│   ├── mcp.rs              # JSON-RPC protocol
│   └── handlers.rs         # 15 tool handlers
├── storage/
│   ├── mod.rs              # Storage trait
│   └── sqlite.rs           # SQLite implementation
├── metrics/
│   └── mod.rs              # Metrics collection
└── self_improvement/
    ├── mod.rs              # Re-exports, documentation
    ├── system.rs           # Orchestrator (SelfImprovementSystem)
    ├── types.rs            # Core types (Severity, TriggerMetric, SuggestedAction)
    ├── monitor.rs          # Phase 1: Metric collection, baseline tracking
    ├── analyzer.rs         # Phase 2: LLM-powered diagnosis
    ├── executor.rs         # Phase 3: Action execution with safety
    ├── learner.rs          # Phase 4: Reward calculation, lesson synthesis
    ├── circuit_breaker.rs  # Safety: Halt on consecutive failures
    ├── allowlist.rs        # Safety: Validate action bounds
    ├── baseline.rs         # Baseline calculation (EMA + rolling)
    ├── config.rs           # Configuration structs
    ├── storage.rs          # Database operations
    ├── cli.rs              # CLI commands (status, history, pause, rollback, etc.)
    └── anthropic_calls.rs  # LLM calls for diagnosis, action selection, learning
```

---

## 5. Environment Variables

```bash
# Required
ANTHROPIC_API_KEY=sk-ant-xxx

# Optional
DATABASE_PATH=./data/reasoning.db    # Default
LOG_LEVEL=info                        # Default
REQUEST_TIMEOUT_MS=60000              # Default (60s for Claude)
MAX_RETRIES=3                         # Default

# Model overrides (optional)
ANTHROPIC_MODEL=claude-sonnet-4-20250514           # Default model
ANTHROPIC_MODEL_LINEAR=claude-haiku-3-5-20241022   # Override for linear
ANTHROPIC_MODEL_DECISION=claude-sonnet-4-20250514  # Override for decision
```

---

## 6. Model Configuration

| Mode | Default Model | Temperature | Max Tokens |
|------|---------------|-------------|------------|
| linear | claude-sonnet-4 | 0.7 | 2000 |
| tree | claude-sonnet-4 | 0.8 | 3000 |
| divergent | claude-sonnet-4 | 0.9 | 3000 |
| reflection | claude-sonnet-4 | 0.6 | 2500 |
| checkpoint | claude-sonnet-4 | 0.7 | 2000 |
| auto | claude-haiku-3-5 | 0.5 | 1000 |
| graph | claude-sonnet-4 | 0.7 | 2500 |
| detect | claude-sonnet-4 | 0.5 | 3000 |
| decision | claude-sonnet-4 | 0.6 | 4000 |
| evidence | claude-sonnet-4 | 0.6 | 3000 |
| timeline | claude-sonnet-4 | 0.7 | 2500 |
| mcts | claude-sonnet-4 | 0.7 | 2500 |
| counterfactual | claude-sonnet-4 | 0.6 | 3000 |

---

## 7. Dependencies

```toml
[package]
name = "mcp-reasoning"
version = "0.1.0"
edition = "2021"

[dependencies]
# ============================================
# MCP SDK (Official Rust SDK)
# ============================================
rmcp = { version = "0.9", features = [
    "server",           # Server-side MCP
    "macros",           # #[tool], #[prompt], #[tool_router] macros
    "axum",             # Axum integration for HTTP transport
    "transport-sse",    # SSE transport (legacy support)
    "transport-io",     # Stdio transport
] }

# ============================================
# Anthropic API
# ============================================
anthropic-sdk-rust = { version = "0.1", features = [
    "streaming",        # Streaming responses
    "tools",            # Tool use support
    # "vision",         # Image support (when available)
] }

# ============================================
# Async Runtime & Streaming
# ============================================
tokio = { version = "1.0", features = ["rt-multi-thread", "macros", "time", "sync"] }
futures = "0.3"
async-stream = "0.3"       # For streaming implementations
bytes = "1.0"              # For stream processing

# ============================================
# HTTP & Web Server
# ============================================
reqwest = { version = "0.12", features = ["json", "stream"] }
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# ============================================
# Serialization
# ============================================
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# ============================================
# Database
# ============================================
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }

# ============================================
# Error handling
# ============================================
thiserror = "1.0"
anyhow = "1.0"

# ============================================
# Logging
# ============================================
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# ============================================
# Utilities
# ============================================
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

---

## 8. MCP Transport Architecture

### 8.1 Streamable HTTP Transport (March 2025 MCP Spec)

The server supports the Streamable HTTP transport, which replaced SSE for better scalability
and simpler implementation.

```rust
// src/server/transport.rs

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::Response,
};

/// Streamable HTTP transport for MCP
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

impl StreamableHttpTransport {
    /// Handle incoming MCP request
    pub async fn handle_request(
        &self,
        session_id: Option<String>,
        body: serde_json::Value,
    ) -> Result<Response<Body>, TransportError> {
        // Parse JSON-RPC request
        let request = self.parse_jsonrpc(&body)?;

        // Execute and get response (may be streamed)
        let response = self.execute_request(session_id, request).await?;

        // Return response with appropriate headers
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&response)?))?)
    }

    /// Handle streaming responses for long-running operations
    pub async fn handle_streaming_request(
        &self,
        session_id: Option<String>,
        body: serde_json::Value,
    ) -> Result<Response<Body>, TransportError> {
        let request = self.parse_jsonrpc(&body)?;

        // Create streaming response body
        let stream = self.create_response_stream(session_id, request);

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json-seq")
            .body(Body::wrap_stream(stream))?)
    }
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

### 8.2 Transport Configuration

```rust
// Transport options in Config
pub struct ServerConfig {
    pub transport: TransportType,
    pub host: String,              // Default: "127.0.0.1"
    pub port: u16,                 // Default: 8080
    pub enable_streaming: bool,    // Default: true
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
    /// Server manages session lifecycle
    ServerManaged,
    /// Client provides session ID in header
    ClientManaged,
    /// Stateless (no session tracking)
    Stateless,
}
```

### 8.3 Dependencies for Transport

```toml
# Add to Cargo.toml for HTTP transport
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
futures = "0.3"
```

---

## 9. rmcp SDK Integration

The server uses the official MCP Rust SDK (`rmcp`) with its macro system for clean tool definitions.

### 9.1 Tool Definition with Macros

```rust
// src/server/tools.rs

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
        let mode = self.state.modes.linear.clone();
        mode.process(&content, session_id, confidence).await
    }

    /// Branching exploration with multiple reasoning paths
    #[tool(
        name = "reasoning_tree",
        description = "Create and explore branching reasoning paths. Operations: create, focus, list, complete.",
        annotations(
            title = "Tree Reasoning",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn reasoning_tree(
        &self,
        #[arg(description = "Operation: create, focus, list, complete")]
        operation: Option<TreeOperation>,
        #[arg(description = "Content to explore (for create)")] content: Option<String>,
        #[arg(description = "Session ID")] session_id: Option<String>,
        #[arg(description = "Branch ID (for focus/complete)")] branch_id: Option<String>,
        #[arg(description = "Number of branches (2-4)")] num_branches: Option<u32>,
    ) -> Result<TreeResponse, ToolError> {
        let mode = self.state.modes.tree.clone();
        mode.execute(operation.unwrap_or_default(), content, session_id, branch_id, num_branches).await
    }

    /// Creative multi-perspective reasoning
    #[tool(
        name = "reasoning_divergent",
        description = "Generate novel perspectives with assumption challenges and optional force_rebellion mode.",
        annotations(
            title = "Divergent Reasoning",
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn reasoning_divergent(
        &self,
        #[arg(description = "Content to explore creatively")] content: String,
        #[arg(description = "Session ID")] session_id: Option<String>,
        #[arg(description = "Number of perspectives (2-5)")] num_perspectives: Option<u32>,
        #[arg(description = "Challenge assumptions explicitly")] challenge_assumptions: Option<bool>,
        #[arg(description = "Enable maximum creativity mode")] force_rebellion: Option<bool>,
    ) -> Result<DivergentResponse, ToolError> {
        let mode = self.state.modes.divergent.clone();
        mode.process(&content, session_id, num_perspectives, challenge_assumptions, force_rebellion).await
    }

    // ... Additional tools follow the same pattern
}
```

### 9.2 Automatic Schema Generation

The rmcp macros automatically generate JSON schemas from Rust types:

```rust
// Response types with automatic schema generation
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Branch {
    pub id: String,
    pub content: String,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub score: f64,
    pub status: BranchStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BranchStatus {
    Active,
    Completed,
    Abandoned,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TreeOperation {
    #[default]
    Create,
    Focus,
    List,
    Complete,
}
```

### 9.3 Server Registration

```rust
// src/main.rs

use rmcp::transport::{SseTransport, StdioTransport};
use rmcp::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize state
    let state = Arc::new(AppState::new().await?);

    // Create server with tools
    let server = ReasoningServer { state };

    // Choose transport based on environment
    match std::env::var("MCP_TRANSPORT").as_deref() {
        Ok("http") | Ok("sse") => {
            // HTTP/SSE transport for web clients
            let transport = SseTransport::new("127.0.0.1:8080");
            Server::new(server)
                .with_transport(transport)
                .serve()
                .await?;
        }
        _ => {
            // Default: stdio transport for CLI integration
            let transport = StdioTransport::new();
            Server::new(server)
                .with_transport(transport)
                .serve()
                .await?;
        }
    }

    Ok(())
}
```

### 9.4 Benefits of rmcp Macros

| Aspect | Without Macros | With Macros |
|--------|----------------|-------------|
| Tool definition | 50+ lines JSON schema | 5-10 lines with #[tool] |
| Handler routing | Manual match statement | Automatic via #[tool_router] |
| Schema validation | Manual serde parsing | Automatic type-safe |
| Documentation | Separate from code | Inline with #[arg] |
| Output schemas | Manual JSON | Derive JsonSchema |

---

## 10. Error Handling Architecture

All errors must propagate up and fail loudly. No swallowing, no fallbacks.

### 10.1 Error Hierarchy

```rust
// src/error/mod.rs

/// Top-level application error
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Anthropic API error: {0}")]
    Anthropic(#[from] AnthropicError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("MCP protocol error: {0}")]
    Mcp(#[from] McpError),

    #[error("Mode execution error: {0}")]
    Mode(#[from] ModeError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
}

/// Anthropic API errors
#[derive(Debug, thiserror::Error)]
pub enum AnthropicError {
    #[error("Authentication failed: invalid API key")]
    AuthenticationFailed,

    #[error("Rate limited: retry after {retry_after_seconds}s")]
    RateLimited { retry_after_seconds: u64 },

    #[error("Model overloaded: {model}")]
    ModelOverloaded { model: String },

    #[error("Request timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Unexpected response: {message}")]
    UnexpectedResponse { message: String },
}

/// Storage errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database connection failed: {message}")]
    ConnectionFailed { message: String },

    #[error("Query failed: {query} - {message}")]
    QueryFailed { query: String, message: String },

    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Thought not found: {thought_id}")]
    ThoughtNotFound { thought_id: String },

    #[error("Migration failed: {version} - {message}")]
    MigrationFailed { version: String, message: String },
}

/// MCP protocol errors
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Invalid JSON-RPC request: {message}")]
    InvalidRequest { message: String },

    #[error("Unknown method: {method}")]
    UnknownMethod { method: String },

    #[error("Unknown tool: {tool}")]
    UnknownTool { tool: String },

    #[error("Invalid parameters for {tool}: {message}")]
    InvalidParameters { tool: String, message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Mode execution errors
#[derive(Debug, thiserror::Error)]
pub enum ModeError {
    #[error("Invalid operation {operation} for mode {mode}")]
    InvalidOperation { mode: String, operation: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("Session required but not provided")]
    SessionRequired,

    #[error("JSON parsing failed: {message}")]
    JsonParseFailed { message: String },
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required: {var}")]
    MissingRequired { var: String },

    #[error("Invalid value for {var}: {reason}")]
    InvalidValue { var: String, reason: String },
}
```

### 10.2 Error Handling Patterns

```rust
// CORRECT: Propagate errors
pub async fn handle_linear(params: LinearParams, state: &AppState) -> Result<Response, AppError> {
    let session = state.storage
        .get_or_create_session(&params.session_id)
        .await?;  // Propagates StorageError -> AppError

    let result = state.anthropic
        .reason(&params.content, ReasoningMode::Linear)
        .await?;  // Propagates AnthropicError -> AppError

    state.storage
        .save_thought(&session.id, &result)
        .await?;  // Propagates StorageError -> AppError

    Ok(Response::success(result))
}

// WRONG: Swallowing errors
pub async fn handle_linear_bad(params: LinearParams, state: &AppState) -> Response {
    let session = state.storage
        .get_or_create_session(&params.session_id)
        .await
        .unwrap_or_default();  // NO! Error hidden

    let result = state.anthropic
        .reason(&params.content, ReasoningMode::Linear)
        .await
        .ok();  // NO! Error converted to None

    if let Some(r) = result {
        Response::success(r)
    } else {
        Response::error("Something went wrong")  // NO! Vague error
    }
}
```

---

## 11. Anthropic Client Architecture

### 11.1 Client Structure

```rust
// src/anthropic/client.rs

pub struct AnthropicClient {
    client: reqwest::Client,
    api_key: String,
    config: ClientConfig,
}

pub struct ClientConfig {
    pub base_url: String,           // Default: "https://api.anthropic.com/v1"
    pub timeout_ms: u64,            // Default: 60000
    pub max_retries: u32,           // Default: 3
    pub retry_delay_ms: u64,        // Default: 1000
}

impl AnthropicClient {
    pub fn new(api_key: String, config: ClientConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key, config }
    }

    /// Main reasoning entry point
    pub async fn reason(
        &self,
        content: &str,
        mode: ReasoningMode,
        mode_config: &ModeConfig,
    ) -> Result<ReasoningResponse, AnthropicError> {
        let prompt = get_prompt_for_mode(mode);
        let request = self.build_request(content, &prompt, mode_config);
        self.execute_with_retry(request).await
    }

    fn build_request(
        &self,
        content: &str,
        system_prompt: &str,
        mode_config: &ModeConfig,
    ) -> AnthropicRequest {
        AnthropicRequest {
            model: mode_config.model.clone(),
            max_tokens: mode_config.max_tokens,
            temperature: mode_config.temperature,
            system: system_prompt.to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: content.to_string(),
                }
            ],
        }
    }

    async fn execute_with_retry(
        &self,
        request: AnthropicRequest,
    ) -> Result<ReasoningResponse, AnthropicError> {
        let mut last_error = None;
        let mut delay = self.config.retry_delay_ms;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                tracing::warn!(attempt, "Retrying Anthropic request");
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

        Err(last_error.unwrap())
    }

    async fn execute_once(
        &self,
        request: &AnthropicRequest,
    ) -> Result<ReasoningResponse, AnthropicError> {
        let response = self.client
            .post(&format!("{}/messages", self.config.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| AnthropicError::Network { message: e.to_string() })?;

        let status = response.status();

        if status == 401 {
            return Err(AnthropicError::AuthenticationFailed);
        }

        if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(AnthropicError::RateLimited { retry_after_seconds: retry_after });
        }

        if status == 529 {
            let model = request.model.clone();
            return Err(AnthropicError::ModelOverloaded { model });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AnthropicError::UnexpectedResponse {
                message: format!("Status {}: {}", status, body),
            });
        }

        let body: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| AnthropicError::UnexpectedResponse { message: e.to_string() })?;

        self.parse_response(body)
    }

    fn parse_response(&self, response: AnthropicResponse) -> Result<ReasoningResponse, AnthropicError> {
        let text = response.content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| AnthropicError::UnexpectedResponse {
                message: "No content in response".to_string(),
            })?;

        // Try to extract JSON from response
        let json = extract_json(&text)
            .map_err(|e| AnthropicError::UnexpectedResponse {
                message: format!("JSON extraction failed: {}", e),
            })?;

        Ok(ReasoningResponse {
            raw_text: text,
            parsed: json,
            usage: response.usage,
        })
    }
}

impl AnthropicError {
    fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } |
            Self::ModelOverloaded { .. } |
            Self::Timeout { .. } |
            Self::Network { .. }
        )
    }
}
```

### 11.2 Request/Response Types

```rust
// src/anthropic/types.rs

#[derive(Debug, Clone, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    pub system: String,
    pub messages: Vec<Message>,

    // Extended Thinking (for deep analysis modes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,

    // Tool Use (for agentic reasoning)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
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

    /// Standard budget for reflection/analysis modes
    pub fn standard() -> Self { Self::enabled(4096) }

    /// Deep budget for complex decision/evidence modes
    pub fn deep() -> Self { Self::enabled(8192) }

    /// Maximum budget for counterfactual/mcts modes
    pub fn maximum() -> Self { Self::enabled(16384) }
}

/// Tool definition for agentic reasoning
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Tool choice configuration
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ToolChoice {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "tool")]
    Specific { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub usage: Usage,
    pub stop_reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ReasoningResponse {
    pub raw_text: String,
    pub parsed: serde_json::Value,
    pub usage: Usage,
    pub thinking: Option<ThinkingBlock>,  // Extended thinking output
    pub tool_use: Option<Vec<ToolUseBlock>>,  // Tool calls made
}

/// Extended thinking output block
#[derive(Debug, Clone, Deserialize)]
pub struct ThinkingBlock {
    pub thinking: String,      // The model's thinking process
    pub thinking_tokens: u32,  // Tokens used for thinking
}

/// Tool use output block
#[derive(Debug, Clone, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

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

/// Streaming event types
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Message started
    MessageStart { message_id: String },
    /// Content block started (text or thinking)
    ContentBlockStart { index: usize, block_type: String },
    /// Text delta received
    TextDelta { index: usize, text: String },
    /// Thinking delta received (extended thinking)
    ThinkingDelta { thinking: String },
    /// Content block finished
    ContentBlockStop { index: usize },
    /// Message finished
    MessageStop { stop_reason: String, usage: Usage },
    /// Error occurred
    Error { error: String },
}
```

### 11.3 Streaming Client

```rust
// src/anthropic/streaming.rs

use futures::Stream;
use tokio::io::AsyncBufReadExt;

impl AnthropicClient {
    /// Stream reasoning response for long-running operations
    pub async fn reason_streaming(
        &self,
        content: &str,
        mode: ReasoningMode,
        mode_config: &ModeConfig,
    ) -> Result<impl Stream<Item = Result<StreamEvent, AnthropicError>>, AnthropicError> {
        let prompt = get_prompt_for_mode(mode);
        let mut request = self.build_request(content, &prompt, mode_config);
        request.stream = Some(true);

        let response = self.client
            .post(&format!("{}/messages", self.config.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AnthropicError::Network { message: e.to_string() })?;

        Ok(self.parse_sse_stream(response.bytes_stream()))
    }

    /// Parse SSE stream into StreamEvents
    fn parse_sse_stream(
        &self,
        bytes: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>,
    ) -> impl Stream<Item = Result<StreamEvent, AnthropicError>> {
        async_stream::try_stream! {
            let mut buffer = String::new();

            tokio::pin!(bytes);

            while let Some(chunk) = bytes.next().await {
                let chunk = chunk.map_err(|e| AnthropicError::Network {
                    message: e.to_string(),
                })?;

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Parse SSE events from buffer
                while let Some(event) = self.extract_sse_event(&mut buffer) {
                    yield event;
                }
            }
        }
    }

    fn extract_sse_event(&self, buffer: &mut String) -> Option<StreamEvent> {
        // Find complete event (ends with \n\n)
        if let Some(end_idx) = buffer.find("\n\n") {
            let event_str = buffer.drain(..=end_idx + 1).collect::<String>();
            self.parse_sse_event(&event_str)
        } else {
            None
        }
    }
}
```

### 11.4 Mode Configuration

```rust
// src/anthropic/config.rs

#[derive(Debug, Clone)]
pub struct ModeConfig {
    pub model: String,
    pub temperature: Option<f64>,
    pub max_tokens: u32,
    pub thinking: Option<ThinkingConfig>,  // Extended thinking for deep analysis
    pub streaming: bool,                    // Enable streaming for long operations
    pub tools: Option<Vec<ToolDefinition>>, // Tools available for agentic modes
}

impl ModeConfig {
    /// Create config with extended thinking enabled
    pub fn with_thinking(mut self, budget: ThinkingConfig) -> Self {
        self.thinking = Some(budget);
        self
    }

    /// Enable streaming for this mode
    pub fn with_streaming(mut self) -> Self {
        self.streaming = true;
        self
    }
}

/// Get configuration for a reasoning mode
pub fn get_mode_config(mode: ReasoningMode, overrides: &ConfigOverrides) -> ModeConfig {
    // Check for runtime overrides first (from self-improvement)
    if let Some(override_config) = overrides.get(&mode) {
        return override_config.clone();
    }

    // Fall back to defaults with extended thinking for deep analysis modes
    match mode {
        ReasoningMode::Linear => ModeConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.7),
            max_tokens: 2000,
            thinking: None,  // Fast mode, no extended thinking
            streaming: false,
            tools: None,
        },
        ReasoningMode::Tree => ModeConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.8),
            max_tokens: 3000,
            thinking: None,
            streaming: true,  // Stream for branch exploration
            tools: None,
        },
        ReasoningMode::Divergent => ModeConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.9),
            max_tokens: 3000,
            thinking: Some(ThinkingConfig::standard()),  // Think for creativity
            streaming: true,
            tools: None,
        },
        ReasoningMode::Reflection => ModeConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.6),
            max_tokens: 2500,
            thinking: Some(ThinkingConfig::deep()),  // Deep thinking for meta-cognition
            streaming: true,
            tools: None,
        },
        ReasoningMode::Auto => ModeConfig {
            model: "claude-3-5-haiku-20241022".to_string(),
            temperature: Some(0.5),
            max_tokens: 1000,
            thinking: None,  // Fast routing, no thinking needed
            streaming: false,
            tools: None,
        },
        ReasoningMode::Graph => ModeConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.7),
            max_tokens: 2500,
            thinking: Some(ThinkingConfig::standard()),
            streaming: true,
            tools: None,
        },
        ReasoningMode::Decision => ModeConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.6),
            max_tokens: 4000,
            thinking: Some(ThinkingConfig::deep()),  // Deep analysis for decisions
            streaming: true,
            tools: None,
        },
        ReasoningMode::Evidence => ModeConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.6),
            max_tokens: 3000,
            thinking: Some(ThinkingConfig::deep()),  // Deep for Bayesian analysis
            streaming: true,
            tools: None,
        },
        ReasoningMode::Counterfactual => ModeConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.6),
            max_tokens: 3000,
            thinking: Some(ThinkingConfig::maximum()),  // Maximum for causal chains
            streaming: true,
            tools: None,
        },
        ReasoningMode::Mcts => ModeConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: Some(0.7),
            max_tokens: 2500,
            thinking: Some(ThinkingConfig::maximum()),  // Maximum for tree search
            streaming: true,
            tools: None,
        },
        // ... other modes with appropriate thinking budgets
    }
}
```

---

## 12. Core Principles

### No Optional Features
- Every feature is ALWAYS enabled
- No `--enable` or `--disable` flags
- No environment variables to turn features off
- No conditional compilation for features

### No Fallbacks
- If something fails, it FAILS LOUDLY
- No graceful degradation
- No silent retries that hide problems
- No "optional" error handling

### Loud Failures
- Every error propagates up
- Every failure is logged at ERROR level
- No swallowing exceptions
- No default values that hide missing data

---

## 13. Implementation Order (TDD with 100% Coverage)

**CRITICAL RULE**: Every phase MUST maintain 100% test coverage. No code merges without passing coverage gate.

### Phase 0: Coverage Infrastructure (FIRST)
Before any feature code, establish:
1. Coverage tooling setup (cargo-llvm-cov)
2. CI/CD pipeline with 100% coverage gate
3. Pre-commit hooks for local coverage checks
4. Coverage exclusion patterns for legitimate cases

**Exit Criteria**: `cargo llvm-cov --fail-under-lines 100` passes on empty project skeleton.

### Phase 1: Foundation (TDD)
| Step | Component | Tests First | Coverage Target |
|------|-----------|-------------|-----------------|
| 1.1 | Project skeleton | Smoke test | 100% |
| 1.2 | Config parsing | Valid/invalid configs, env vars | 100% |
| 1.3 | Error types | All variants, Display, From impls | 100% |
| 1.4 | AnthropicClient | Mock HTTP, retry logic, all error paths | 100% |

**Test Types**: Unit tests with mocked HTTP client.
**Exit Criteria**: `cargo llvm-cov --fail-under-lines 100` passes.

### Phase 2: Storage & Self-Improvement (TDD)
| Step | Component | Tests First | Coverage Target |
|------|-----------|-------------|-----------------|
| 2.1 | Storage trait | Trait method contracts | 100% |
| 2.2 | SQLite impl | CRUD operations, migrations, edge cases | 100% |
| 2.3 | Monitor | Metric collection, baseline calculation | 100% |
| 2.4 | Analyzer | Mock LLM diagnosis, action selection | 100% |
| 2.5 | Executor | Action execution, rollback paths | 100% |
| 2.6 | Learner | Reward calculation, lesson synthesis | 100% |
| 2.7 | Circuit breaker | Trip conditions, reset logic | 100% |
| 2.8 | Allowlist | Valid/invalid action validation | 100% |

**Test Types**: Unit tests with in-memory SQLite, mocked Anthropic client.
**Exit Criteria**: `cargo llvm-cov --fail-under-lines 100` passes.

### Phase 3: Core Modes (TDD)
| Step | Component | Tests First | Coverage Target |
|------|-----------|-------------|-----------------|
| 3.1 | ModeCore | Shared functionality tests | 100% |
| 3.2 | LinearMode | Process flow, confidence scoring | 100% |
| 3.3 | TreeMode | All 4 operations (create/focus/list/complete) | 100% |
| 3.4 | DivergentMode | Perspectives, force_rebellion | 100% |
| 3.5 | ReflectionMode | Both operations (process/evaluate) | 100% |

**Test Types**: Unit tests with mocked ModeCore dependencies.
**Exit Criteria**: `cargo llvm-cov --fail-under-lines 100` passes.

### Phase 4: Advanced Modes (TDD)
| Step | Component | Tests First | Coverage Target |
|------|-----------|-------------|-----------------|
| 4.1 | CheckpointMode | All 3 operations, state restoration | 100% |
| 4.2 | AutoMode | Mode selection logic, all routes | 100% |
| 4.3 | GraphMode | All 8 operations, graph traversal | 100% |
| 4.4 | DetectMode | Bias detection, fallacy detection | 100% |
| 4.5 | DecisionMode | All 4 methods, stakeholder mapping | 100% |
| 4.6 | EvidenceMode | Assessment, Bayesian updates | 100% |

**Test Types**: Unit tests, property-based tests for graph operations.
**Exit Criteria**: `cargo llvm-cov --fail-under-lines 100` passes.

### Phase 5: Time Machine Modes (TDD)
| Step | Component | Tests First | Coverage Target |
|------|-----------|-------------|-----------------|
| 5.1 | TimelineMode | All 4 operations, merge strategies | 100% |
| 5.2 | MctsMode | UCB1 exploration, auto-backtrack | 100% |
| 5.3 | CounterfactualMode | Causal analysis, Pearl's Ladder levels | 100% |

**Test Types**: Unit tests, property-based tests for MCTS.
**Exit Criteria**: `cargo llvm-cov --fail-under-lines 100` passes.

### Phase 6: Server Infrastructure (TDD)
| Step | Component | Tests First | Coverage Target |
|------|-----------|-------------|-----------------|
| 6.1 | PresetMode | List, run, step sequencing | 100% |
| 6.2 | MetricsMode | All 5 query types | 100% |
| 6.3 | Tool handlers | All 15 tools, all operations | 100% |
| 6.4 | MCP protocol | JSON-RPC parsing, errors | 100% |
| 6.5 | Transport | HTTP and stdio handlers | 100% |
| 6.6 | Graceful shutdown | Signal handling, cleanup | 100% |

**Test Types**: Unit tests, integration tests with real server.
**Exit Criteria**: `cargo llvm-cov --fail-under-lines 100` passes.

### Phase 7: Integration & E2E (TDD)
| Step | Component | Tests First | Coverage Target |
|------|-----------|-------------|-----------------|
| 7.1 | Full workflow tests | Multi-mode scenarios | 100% |
| 7.2 | Self-improvement E2E | Complete loop execution | 100% |
| 7.3 | Error recovery | All failure modes | 100% |
| 7.4 | Performance baselines | Latency, throughput benchmarks | N/A |

**Test Types**: Integration tests, E2E with mocked Anthropic.
**Exit Criteria**: `cargo llvm-cov --fail-under-lines 100` passes.

### Phase 8: Production Hardening
| Step | Component | Tests First | Coverage Target |
|------|-----------|-------------|-----------------|
| 8.1 | Fuzz testing | Property-based edge cases | N/A (supplemental) |
| 8.2 | Load testing | Concurrent request handling | N/A (supplemental) |
| 8.3 | Documentation | All doc examples compile | 100% |

**Exit Criteria**: All tests pass, coverage maintained at 100%.

---

## 14. Self-Improvement System Architecture

The self-improvement system is **always enabled** and runs as an integral part of the server.
Based on the proven implementation from mcp-langbase-reasoning.

### 14.1 Four-Phase Loop

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SELF-IMPROVEMENT SYSTEM                           │
│                                                                      │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐       │
│  │ MONITOR  │───▶│ ANALYZER │───▶│ EXECUTOR │───▶│ LEARNER  │       │
│  │ Phase 1  │    │ Phase 2  │    │ Phase 3  │    │ Phase 4  │       │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘       │
│       │                                               │              │
│       └───────────────────◀───────────────────────────┘              │
│                              │                                       │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    Shared Components                         │    │
│  │  CircuitBreaker │ Allowlist │ Storage │ Baselines           │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### 14.2 Core Types (`types.rs`)

```rust
/// Severity levels for detected issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info = 0,      // Minor deviation, no action needed
    Warning = 1,   // Moderate deviation, consider action
    High = 2,      // Significant deviation, action recommended
    Critical = 3,  // Severe deviation, immediate action required
}

impl Severity {
    pub fn from_deviation(deviation_pct: f64) -> Self {
        match deviation_pct {
            d if d >= 100.0 => Severity::Critical,
            d if d >= 50.0 => Severity::High,
            d if d >= 25.0 => Severity::Warning,
            _ => Severity::Info,
        }
    }
}

/// What triggered the diagnosis
#[derive(Debug, Clone)]
pub enum TriggerMetric {
    ErrorRate { observed: f64, baseline: f64, threshold: f64 },
    Latency { observed_p95_ms: i64, baseline_ms: i64, threshold_ms: i64 },
    QualityScore { observed: f64, baseline: f64, minimum: f64 },
}

impl TriggerMetric {
    pub fn deviation_pct(&self) -> f64 {
        match self {
            TriggerMetric::ErrorRate { observed, baseline, .. } => {
                if *baseline == 0.0 { if *observed > 0.0 { 100.0 } else { 0.0 } }
                else { ((observed - baseline) / baseline) * 100.0 }
            }
            // ... similar for other variants
        }
    }
}

/// Actions the system can take (ALL reversible)
#[derive(Debug, Clone)]
pub enum SuggestedAction {
    AdjustParam {
        key: String,
        old_value: ParamValue,
        new_value: ParamValue,
        scope: ConfigScope,
    },
    ScaleResource {
        resource: ResourceType,
        old_value: u32,
        new_value: u32,
    },
    NoOp {
        reason: String,
        revisit_after: Duration,
    },
}

/// Parameter value types
#[derive(Debug, Clone)]
pub enum ParamValue {
    Integer(i64),
    Float(f64),
    String(String),
    DurationMs(u64),
    Boolean(bool),
}

/// Resource types that can be scaled
#[derive(Debug, Clone, Copy)]
pub enum ResourceType {
    MaxConcurrentRequests,
    ConnectionPoolSize,
    CacheSize,
    TimeoutMs,
    MaxRetries,
    RetryDelayMs,
}

/// Complete diagnosis report
#[derive(Debug, Clone)]
pub struct SelfDiagnosis {
    pub id: DiagnosisId,
    pub created_at: DateTime<Utc>,
    pub trigger: TriggerMetric,
    pub severity: Severity,
    pub description: String,
    pub suspected_cause: Option<String>,
    pub suggested_action: SuggestedAction,
    pub action_rationale: Option<String>,
    pub status: DiagnosisStatus,
}

/// Normalized reward for comparing improvements
#[derive(Debug, Clone)]
pub struct NormalizedReward {
    pub value: f64,           // -1.0 to 1.0 (positive = improvement)
    pub breakdown: RewardBreakdown,
    pub confidence: f64,      // Based on sample size
}

impl NormalizedReward {
    pub fn calculate(
        trigger: &TriggerMetric,
        pre_metrics: &MetricsSnapshot,
        post_metrics: &MetricsSnapshot,
        baselines: &Baselines,
    ) -> Self {
        // Weighted calculation based on trigger type
        let weights = RewardWeights::for_trigger(trigger);
        // ... calculation logic
    }

    pub fn is_positive(&self) -> bool { self.value > 0.0 }
    pub fn is_negative(&self) -> bool { self.value < 0.0 }
}
```

### 14.3 Phase 1: Monitor (`monitor.rs`)

Collects metrics, calculates baselines, detects anomalies.

```rust
pub struct Monitor {
    config: MonitorConfig,
    baselines: RwLock<BaselineCollection>,
    raw_metrics: RwLock<RawMetrics>,
}

#[derive(Debug, Clone)]
pub struct InvocationEvent {
    pub tool_name: String,
    pub latency_ms: i64,
    pub success: bool,
    pub quality_score: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct HealthReport {
    pub current_metrics: MetricsSnapshot,
    pub baselines: Baselines,
    pub triggers: Vec<TriggerMetric>,
    pub is_healthy: bool,
    pub generated_at: DateTime<Utc>,
}

impl Monitor {
    /// Record invocation (called on EVERY request)
    pub async fn record_invocation(
        &self,
        is_error: bool,
        latency_ms: i64,
        quality_score: f64,
    );

    /// Check health - returns report if enough samples
    pub async fn check_health(&self) -> Option<HealthReport>;

    /// Force health check regardless of timing
    pub async fn force_check(&self) -> Option<HealthReport>;

    /// Get current baselines
    pub async fn get_baselines(&self) -> Baselines;

    /// Get current aggregated metrics
    pub async fn get_current_metrics(&self) -> MetricsSnapshot;
}
```

### 14.4 Phase 2: Analyzer (`analyzer.rs`)

Uses LLM to diagnose issues and select actions.

```rust
pub struct Analyzer {
    config: AnalyzerConfig,
    anthropic: Arc<AnthropicClient>,  // For Claude-powered diagnosis
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub diagnosis: SelfDiagnosis,
    pub analysis_stats: AnalyzerStats,
}

#[derive(Debug)]
pub enum AnalysisBlocked {
    CircuitOpen { remaining_secs: u64 },
    NoTriggers,
    SeverityTooLow { severity: Severity, minimum: Severity },
    MaxPendingReached { count: u32 },
}

impl Analyzer {
    /// Analyze health report and generate diagnosis
    pub async fn analyze(&self, health: &HealthReport) -> Result<AnalysisResult, AnalysisBlocked>;
}
```

### 14.5 Phase 3: Executor (`executor.rs`)

Executes actions with safety checks and rollback capability.

```rust
pub struct Executor {
    config: ExecutorConfig,
    allowlist: ActionAllowlist,
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
    config_state: RwLock<ConfigState>,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub action_id: ActionId,
    pub diagnosis_id: DiagnosisId,
    pub outcome: ActionOutcome,
    pub pre_metrics: MetricsSnapshot,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionOutcome {
    Pending,
    Success,
    Failed,
    RolledBack,
}

#[derive(Debug)]
pub enum ExecutionBlocked {
    CircuitOpen { remaining_secs: u64 },
    CooldownActive { remaining_secs: u64 },
    RateLimitExceeded { count: u32, max: u32 },
    NotAllowed { reason: String },
    NoOpAction { reason: String },
}

impl Executor {
    /// Execute diagnosis action with safety checks
    pub async fn execute(
        &self,
        diagnosis: &SelfDiagnosis,
        current_metrics: &MetricsSnapshot,
    ) -> Result<ExecutionResult, ExecutionBlocked>;

    /// Rollback a specific action by ID
    pub async fn rollback_by_id(&self, action_id: &str) -> Result<(), ExecutorError>;
}
```

### 14.6 Phase 4: Learner (`learner.rs`)

Calculates rewards and synthesizes lessons.

```rust
pub struct Learner {
    config: LearnerConfig,
    anthropic: Arc<AnthropicClient>,  // For lesson synthesis
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
}

#[derive(Debug, Clone)]
pub struct LearningOutcome {
    pub reward: NormalizedReward,
    pub action_effectiveness: ActionEffectiveness,
    pub learning_synthesis: Option<LearningSynthesis>,
}

#[derive(Debug, Clone)]
pub struct LearningSynthesis {
    pub lessons: Vec<String>,
    pub future_recommendations: Vec<String>,
}

#[derive(Debug)]
pub enum LearningBlocked {
    ExecutionNotCompleted { status: ActionOutcome },
    InsufficientSamples { required: u64, actual: u64 },
}

impl Learner {
    /// Learn from execution result
    pub async fn learn(
        &self,
        execution_result: &ExecutionResult,
        diagnosis: &SelfDiagnosis,
        post_metrics: &MetricsSnapshot,
        baselines: &Baselines,
    ) -> Result<LearningOutcome, LearningBlocked>;
}
```

### 14.7 Action Allowlist (`allowlist.rs`)

Validates actions are within safe bounds.

```rust
pub struct ActionAllowlist {
    allowed_params: HashMap<String, ParamBounds>,
    allowed_resources: HashMap<ResourceType, ResourceBounds>,
}

#[derive(Debug, Clone)]
pub struct ParamBounds {
    pub min: ParamValue,
    pub max: ParamValue,
    pub step: Option<ParamValue>,
}

impl ActionAllowlist {
    pub fn default_allowlist() -> Self {
        // Defines safe ranges for all adjustable parameters
    }

    pub fn validate(&self, action: &SuggestedAction) -> Result<(), AllowlistError>;
}
```

### 14.8 Circuit Breaker (`circuit_breaker.rs`)

```rust
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: AtomicU8,
    consecutive_failures: AtomicU32,
    last_failure_time: Mutex<Option<DateTime<Utc>>>,
    successes_in_half_open: AtomicU32,
}

pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,         // Default: 3
    pub reset_timeout_seconds: u64,     // Default: 300 (5 min)
    pub success_threshold: u32,         // Default: 2
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,    // Normal - all operations allowed
    Open,      // Tripped - all operations blocked
    HalfOpen,  // Testing - limited operations allowed
}

impl CircuitBreaker {
    pub fn can_execute(&mut self) -> bool;
    pub fn record_success(&mut self);
    pub fn record_failure(&mut self);
    pub fn state(&self) -> CircuitState;
    pub fn force_reset(&mut self);
}
```

### 14.9 System Orchestration (`system.rs`)

```rust
pub struct SelfImprovementSystem {
    config: SelfImprovementConfig,
    monitor: Monitor,
    analyzer: Analyzer,
    executor: Executor,
    learner: Learner,
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
    allowlist: ActionAllowlist,
    state: Arc<RwLock<SystemState>>,
}

pub struct SelfImprovementConfig {
    pub monitor: MonitorConfig,
    pub analyzer: AnalyzerConfig,
    pub executor: ExecutorConfig,
    pub learner: LearnerConfig,
    pub circuit_breaker: CircuitBreakerConfig,
}

impl SelfImprovementSystem {
    /// Always returns true - system cannot be disabled
    pub fn is_enabled(&self) -> bool { true }

    /// Record invocation (called after EVERY tool use)
    pub async fn on_invocation(&self, event: InvocationEvent);

    /// Check health
    pub async fn check_health(&self) -> Option<HealthReport>;

    /// Run one improvement cycle
    pub async fn run_cycle(&self) -> Result<CycleResult, SelfImprovementError>;

    /// Get current system status
    pub async fn status(&self) -> SystemStatus;
}

#[derive(Debug)]
pub enum SelfImprovementError {
    CircuitBreakerOpen { consecutive_failures: u32 },
    InCooldown { until: DateTime<Utc> },
    RateLimitExceeded { count: u32, max: u32 },
    MonitorFailed { message: String },
    AnalyzerFailed { message: String },
    ExecutorFailed { message: String },
    LearnerFailed { message: String },
}
```

### 14.10 Database Schema for Self-Improvement

```sql
-- Invocation records (fed by Monitor)
CREATE TABLE invocations (
    id TEXT PRIMARY KEY,
    tool_name TEXT NOT NULL,
    latency_ms INTEGER NOT NULL,
    success INTEGER NOT NULL,
    quality_score REAL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_invocations_created_at ON invocations(created_at);
CREATE INDEX idx_invocations_tool ON invocations(tool_name);

-- Diagnosis records
CREATE TABLE diagnoses (
    id TEXT PRIMARY KEY,
    trigger_type TEXT NOT NULL,
    trigger_json TEXT NOT NULL,
    severity TEXT NOT NULL,
    description TEXT NOT NULL,
    suspected_cause TEXT,
    suggested_action_json TEXT NOT NULL,
    action_rationale TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_diagnoses_status ON diagnoses(status);

-- Action records (executed by Executor)
CREATE TABLE actions (
    id TEXT PRIMARY KEY,
    diagnosis_id TEXT NOT NULL REFERENCES diagnoses(id),
    action_type TEXT NOT NULL,
    action_json TEXT NOT NULL,
    outcome TEXT NOT NULL,
    pre_metrics_json TEXT NOT NULL,
    post_metrics_json TEXT,
    execution_time_ms INTEGER NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_actions_diagnosis ON actions(diagnosis_id);
CREATE INDEX idx_actions_outcome ON actions(outcome);

-- Learning records
CREATE TABLE learnings (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL REFERENCES actions(id),
    reward_value REAL NOT NULL,
    reward_breakdown_json TEXT NOT NULL,
    confidence REAL NOT NULL,
    lessons_json TEXT,
    recommendations_json TEXT,
    created_at TEXT NOT NULL
);

-- Config overrides (applied by Executor, read at startup)
CREATE TABLE config_overrides (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    applied_by_action TEXT REFERENCES actions(id),
    updated_at TEXT NOT NULL
);
```

### 14.11 CLI Commands (`cli.rs`)

```rust
/// Self-improvement CLI subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum SelfImproveCommands {
    Status,                              // Show system status
    History { limit: usize, outcome: Option<String> },
    Diagnostics { verbose: bool },
    Config,                              // Show configuration
    CircuitBreaker,                      // Show circuit breaker state
    Baselines,                           // Show metric baselines
    Pause { duration: String },          // Pause for duration (e.g., "30m", "2h")
    Rollback { action_id: String },
    Approve { diagnosis_id: String },
    Reject { diagnosis_id: String, reason: Option<String> },
}
```

### 14.12 LLM Calls (`anthropic_calls.rs`)

Replaces langbase pipes with direct Anthropic Claude API calls:
- `generate_diagnosis()` - Root cause analysis
- `select_action()` - Multi-criteria action selection
- `validate_decision()` - Bias/fallacy detection
- `synthesize_learning()` - Extract lessons from outcomes

### 14.13 Module Structure

```
src/self_improvement/
├── mod.rs              # Re-exports, system documentation
├── system.rs           # Orchestrator (SelfImprovementSystem)
├── types.rs            # Core types (Severity, TriggerMetric, SuggestedAction)
├── monitor.rs          # Phase 1: Metric collection and baseline tracking
├── analyzer.rs         # Phase 2: LLM-powered diagnosis
├── executor.rs         # Phase 3: Action execution with safety
├── learner.rs          # Phase 4: Reward calculation and lesson synthesis
├── circuit_breaker.rs  # Safety: Halt on consecutive failures
├── allowlist.rs        # Safety: Validate actions are within bounds
├── baseline.rs         # Baseline calculation (EMA + rolling average)
├── config.rs           # Configuration structs
├── storage.rs          # Database operations
├── cli.rs              # CLI commands
└── anthropic_calls.rs  # LLM calls for self-improvement
```

---

## 15. Rust Best Practices Compliance

This section ensures the implementation follows current Rust best practices (2024-2025).

### 15.1 Lint Configuration

Enable comprehensive linting in `Cargo.toml`:

```toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
cargo = "warn"

# Specific allows (with justification)
module_name_repetitions = "allow"  # Common in domain-specific code
too_many_arguments = "allow"       # Some tool handlers need many params
```

And in `lib.rs`:

```rust
//! MCP Reasoning Server - Structured reasoning via Anthropic Claude API.
//!
//! This crate provides an MCP server with 15 reasoning tools for
//! sequential, branching, creative, and analytical reasoning modes.

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

// Allow specific lints with justification
#![allow(clippy::module_name_repetitions)] // Domain-specific naming is clearer
```

### 15.2 Error Type Requirements

All error types must satisfy async requirements:

```rust
use static_assertions::assert_impl_all;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AppError {
    #[error("Anthropic API error: {0}")]
    Anthropic(#[from] AnthropicError),
    // ... other variants
}

// Compile-time verification
assert_impl_all!(AppError: Send, Sync, std::error::Error);
assert_impl_all!(AnthropicError: Send, Sync);
assert_impl_all!(StorageError: Send, Sync);
assert_impl_all!(McpError: Send, Sync);
assert_impl_all!(ModeError: Send, Sync);
```

Error message conventions:
- Lowercase sentences without trailing punctuation
- Describe only the error itself, not the source chain
- Include actionable context (IDs, values, limits)

### 15.3 Must-Use Annotations

All Result-returning public functions must use `#[must_use]`:

```rust
impl AnthropicClient {
    /// Execute reasoning with the specified mode.
    ///
    /// # Errors
    /// Returns `AnthropicError` if the API call fails.
    #[must_use = "reasoning result contains the model's response"]
    pub async fn reason(
        &self,
        content: &str,
        mode: ReasoningMode,
        config: &ModeConfig,
    ) -> Result<ReasoningResponse, AnthropicError> {
        // ...
    }
}

impl SqliteStorage {
    /// Retrieve a session by ID.
    ///
    /// # Errors
    /// Returns `StorageError::SessionNotFound` if the session doesn't exist.
    #[must_use = "session data should be used or explicitly ignored"]
    pub async fn get_session(&self, id: &str) -> Result<Session, StorageError> {
        // ...
    }
}
```

### 15.4 Async Best Practices

#### Spawn Blocking for Sync Operations

SQLite operations that may block must use `spawn_blocking`:

```rust
impl SqliteStorage {
    pub async fn execute_blocking<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&Connection) -> Result<T, StorageError> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.pool.get().await?;
        tokio::task::spawn_blocking(move || f(&conn))
            .await
            .map_err(|e| StorageError::Internal {
                message: format!("spawn_blocking failed: {}", e)
            })?
    }
}
```

#### Graceful Shutdown

The server must handle shutdown signals properly:

```rust
// src/server/mod.rs

use tokio::signal;
use tokio::sync::oneshot;

pub struct Server {
    state: Arc<AppState>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Server {
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
        // Flush metrics
        self.state.metrics.flush().await?;
        // Close database connections
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

#### Timeout on External Calls

All external API calls must have timeouts:

```rust
use tokio::time::{timeout, Duration};

impl AnthropicClient {
    async fn execute_once(&self, request: &AnthropicRequest) -> Result<ReasoningResponse, AnthropicError> {
        let timeout_duration = Duration::from_millis(self.config.timeout_ms);

        timeout(timeout_duration, self.do_request(request))
            .await
            .map_err(|_| AnthropicError::Timeout {
                timeout_ms: self.config.timeout_ms
            })?
    }
}
```

### 15.5 Testing Strategy

#### Test Organization

```
tests/
├── common/
│   └── mod.rs              # Shared test utilities, fixtures
├── integration/
│   ├── linear_mode.rs      # Linear reasoning integration tests
│   ├── tree_mode.rs        # Tree reasoning integration tests
│   ├── graph_mode.rs       # Graph-of-Thoughts integration tests
│   ├── decision_mode.rs    # Decision analysis integration tests
│   └── full_workflow.rs    # End-to-end workflow tests
└── mocks/
    └── mod.rs              # Mock Anthropic client, mock storage
```

#### Test Naming Convention

```rust
// Pattern: test_[function]_[scenario]_[expected_outcome]

#[tokio::test]
async fn test_linear_reason_with_valid_content_returns_continuation() { }

#[tokio::test]
async fn test_linear_reason_with_empty_content_returns_validation_error() { }

#[tokio::test]
async fn test_tree_create_with_high_branch_count_limits_to_maximum() { }
```

#### Multi-threaded Tests for Race Conditions

```rust
// Use multi_thread flavor to expose race conditions
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_session_access_maintains_consistency() {
    let storage = create_test_storage().await;
    let session_id = "test-session";

    // Spawn multiple concurrent operations
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let storage = storage.clone();
            tokio::spawn(async move {
                storage.save_thought(&session_id, &format!("thought-{}", i)).await
            })
        })
        .collect();

    // All should succeed without data corruption
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all thoughts saved
    let thoughts = storage.get_thoughts(&session_id).await.unwrap();
    assert_eq!(thoughts.len(), 10);
}
```

#### Time-Controlled Tests

```rust
#[tokio::test]
async fn test_retry_respects_backoff_timing() {
    tokio::time::pause(); // Freeze time

    let client = create_failing_client(failures: 2);
    let start = tokio::time::Instant::now();

    // First attempt fails, waits 1s, second attempt fails, waits 2s, third succeeds
    let result = client.reason("test", ReasoningMode::Linear).await;

    tokio::time::advance(Duration::from_secs(3)).await;

    assert!(result.is_ok());
    assert!(start.elapsed() >= Duration::from_secs(3));
}
```

### 15.6 Documentation Standards

#### Module Documentation

```rust
//! # Modes Module
//!
//! This module provides reasoning mode implementations.
//!
//! ## Available Modes
//!
//! | Mode | Description |
//! |------|-------------|
//! | [`LinearMode`] | Sequential step-by-step reasoning |
//! | [`TreeMode`] | Branching exploration |
//! | [`DivergentMode`] | Creative multi-perspective |
//!
//! ## Usage
//!
//! ```rust
//! use mcp_reasoning::modes::{LinearMode, ModeCore};
//!
//! let core = ModeCore::new(storage, client);
//! let linear = LinearMode::new(core);
//! let result = linear.process("analyze this").await?;
//! ```
```

#### Function Documentation

```rust
/// Process a thought using linear reasoning.
///
/// Takes input content and produces a logical continuation with
/// confidence scoring. The model evaluates the thought step-by-step
/// and suggests the next reasoning step.
///
/// # Arguments
///
/// * `content` - The thought content to process
/// * `session_id` - Optional session for context continuity
/// * `confidence` - Optional confidence threshold (0.0-1.0)
///
/// # Returns
///
/// A [`LinearResponse`] containing:
/// - The reasoning continuation
/// - Confidence score
/// - Suggested next step
///
/// # Errors
///
/// Returns [`ModeError`] if:
/// - Content is empty
/// - API call fails
/// - JSON parsing fails
///
/// # Examples
///
/// ```rust
/// let response = linear.process(
///     "The sky is blue because",
///     Some("session-123"),
///     Some(0.8),
/// ).await?;
///
/// assert!(response.confidence >= 0.8);
/// println!("Continuation: {}", response.content);
/// ```
pub async fn process(
    &self,
    content: &str,
    session_id: Option<&str>,
    confidence: Option<f64>,
) -> Result<LinearResponse, ModeError> {
    // ...
}
```

### 15.7 Dependencies Best Practices

Add to `Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...

# Compile-time assertions
static_assertions = "1.1"

[dev-dependencies]
# Testing utilities
tokio-test = "0.4"
mockall = "0.12"
proptest = "1.4"           # Property-based testing
criterion = "0.5"          # Benchmarking
test-case = "3.3"          # Parameterized tests

[profile.dev]
# Faster compile times in dev
opt-level = 0
debug = true

[profile.release]
# Maximum optimization for production
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true

[profile.test]
# Some optimization for faster tests
opt-level = 1
```

### 15.8 Compliance Checklist

Before implementation, verify:

- [ ] `#![forbid(unsafe_code)]` in lib.rs
- [ ] `#![warn(missing_docs)]` in lib.rs
- [ ] Clippy pedantic enabled in Cargo.toml
- [ ] All error types implement `Send + Sync`
- [ ] `#[must_use]` on all Result-returning public functions
- [ ] `spawn_blocking` for all sync I/O
- [ ] Graceful shutdown handler implemented
- [ ] Timeouts on all external API calls
- [ ] Tests use multi-threaded runtime where appropriate
- [ ] Module and function documentation complete
- [ ] Integration test directory structure created

---

## 16. Test Coverage Infrastructure

**CRITICAL**: This project mandates 100% test coverage from day one. No exceptions. This section defines the infrastructure required to achieve and maintain this standard.

### 16.1 Coverage Tool Selection

**Primary Tool**: `cargo-llvm-cov` (recommended over tarpaulin for accuracy)

**Rationale**:
- Uses LLVM's instrumentation (same as `cargo test`)
- More accurate branch coverage than source-based tools
- Better support for async code and macros
- Active development with regular updates
- Supports `--fail-under-lines` for CI gates

**Installation**:
```bash
# Install llvm-cov
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov

# Verify installation
cargo llvm-cov --version
```

### 16.2 Coverage Commands

```bash
# Basic coverage check (100% required)
cargo llvm-cov --fail-under-lines 100

# Coverage with HTML report
cargo llvm-cov --html --output-dir coverage/

# Coverage with detailed branch information
cargo llvm-cov --branch --fail-under-lines 100

# Coverage for specific package (workspace)
cargo llvm-cov --package mcp-reasoning --fail-under-lines 100

# Exclude test code from coverage stats
cargo llvm-cov --ignore-filename-regex "tests/"

# Show uncovered lines (for debugging)
cargo llvm-cov --show-missing-lines

# Generate lcov format for external tools
cargo llvm-cov --lcov --output-path coverage.lcov
```

### 16.3 Cargo Configuration

Add to `Cargo.toml`:

```toml
[workspace.metadata.llvm-cov]
# Fail if coverage drops below 100%
fail-under-lines = 100

# Exclude test files from coverage stats
ignore-filename-regex = ["tests/", "_test\\.rs$"]

# Include branch coverage (stricter)
branch = true
```

Add to `.cargo/config.toml`:

```toml
[alias]
cov = "llvm-cov --fail-under-lines 100"
cov-html = "llvm-cov --html --output-dir coverage/"
cov-check = "llvm-cov --show-missing-lines --fail-under-lines 100"
```

### 16.4 CI/CD Pipeline Configuration

**GitHub Actions Workflow** (`.github/workflows/coverage.yml`):

```yaml
name: Coverage

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-C instrument-coverage"

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-action@stable
        with:
          components: llvm-tools-preview

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov

      - name: Run tests with coverage
        run: cargo llvm-cov --all-features --fail-under-lines 100 --lcov --output-path lcov.info

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v4
        with:
          files: lcov.info
          fail_ci_if_error: true
          verbose: true
        env:
          CODECOV_TOKEN: ${{ secrets.CODECOV_TOKEN }}

      - name: Coverage gate check
        run: |
          COVERAGE=$(cargo llvm-cov --json | jq '.data[0].totals.lines.percent')
          if (( $(echo "$COVERAGE < 100" | bc -l) )); then
            echo "::error::Coverage is $COVERAGE%, required 100%"
            exit 1
          fi
          echo "Coverage: $COVERAGE%"
```

**Branch Protection Rules**:
- Require status check: `coverage`
- Require coverage to pass before merge
- No force pushes to main

### 16.5 Pre-commit Hook

Install `pre-commit` and configure (`.pre-commit-config.yaml`):

```yaml
repos:
  - repo: local
    hooks:
      - id: coverage-check
        name: Coverage Check
        entry: bash -c 'cargo llvm-cov --fail-under-lines 100 || (echo "Coverage below 100%! Add tests before committing." && exit 1)'
        language: system
        types: [rust]
        pass_filenames: false
        stages: [commit]
```

Alternative shell script (`scripts/pre-commit-coverage.sh`):

```bash
#!/bin/bash
set -e

echo "Running coverage check..."
COVERAGE=$(cargo llvm-cov --json 2>/dev/null | jq -r '.data[0].totals.lines.percent')

if (( $(echo "$COVERAGE < 100" | bc -l) )); then
    echo "Coverage is ${COVERAGE}%, required 100%"
    echo ""
    echo "Uncovered lines:"
    cargo llvm-cov --show-missing-lines 2>/dev/null | grep -E "^\s+\d+\|" | head -20
    echo ""
    echo "Add tests before committing."
    exit 1
fi

echo "Coverage: ${COVERAGE}%"
```

### 16.6 Coverage Exclusion Patterns

**Legitimate Exclusions** (use sparingly):

```rust
// For truly unreachable code (e.g., exhaustive pattern matching safety)
#[cfg(not(tarpaulin_include))]  // Works with tarpaulin
#[coverage(off)]                 // Nightly Rust feature

// Example: Unreachable error variant
impl From<Infallible> for AppError {
    #[coverage(off)]  // Cannot be reached by definition
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

// Example: Debug-only code
#[cfg(debug_assertions)]
#[coverage(off)]
fn debug_dump_state(state: &State) {
    eprintln!("{:#?}", state);
}
```

**Exclusion Rules** (strictly enforced):

| Allowed | Not Allowed |
|---------|-------------|
| `unreachable!()` after exhaustive matches | Error handling branches |
| Debug-only diagnostic code | Main business logic |
| Platform-specific code not testable in CI | "Hard to test" code |
| FFI bindings (extern blocks) | Async timeout branches |

**Coverage Exclusion Audit**:
All `#[coverage(off)]` uses MUST be documented with:
```rust
/// COVERAGE EXCLUSION: [specific reason]
/// Added: [date]
#[coverage(off)]
fn excluded_function() { ... }
```

### 16.7 Mock Infrastructure for 100% Coverage

**Strategy**: Mock all external dependencies to enable deterministic testing.

**Mock Traits**:
```rust
// src/traits.rs
#[cfg_attr(test, mockall::automock)]
pub trait AnthropicClient: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, ApiError>;
    async fn complete_streaming(&self, request: CompletionRequest) -> Result<StreamHandle, ApiError>;
}

#[cfg_attr(test, mockall::automock)]
pub trait Storage: Send + Sync {
    async fn get_session(&self, id: &str) -> Result<Option<Session>, StorageError>;
    async fn save_session(&self, session: &Session) -> Result<(), StorageError>;
    async fn save_thought(&self, thought: &Thought) -> Result<(), StorageError>;
}

#[cfg_attr(test, mockall::automock)]
pub trait TimeProvider: Send + Sync {
    fn now(&self) -> chrono::DateTime<chrono::Utc>;
}
```

**Test Utilities Module** (`src/test_utils.rs`):
```rust
#![cfg(test)]

use crate::traits::*;
use mockall::predicate::*;

/// Create a mock client that returns a successful response
pub fn mock_success_client(response: &str) -> MockAnthropicClient {
    let mut mock = MockAnthropicClient::new();
    let response = response.to_string();
    mock.expect_complete()
        .returning(move |_| Ok(CompletionResponse {
            content: response.clone(),
            ..Default::default()
        }));
    mock
}

/// Create a mock client that fails with specific error
pub fn mock_error_client(error: ApiError) -> MockAnthropicClient {
    let mut mock = MockAnthropicClient::new();
    mock.expect_complete()
        .returning(move |_| Err(error.clone()));
    mock
}

/// Create a mock storage with pre-loaded sessions
pub fn mock_storage_with_sessions(sessions: Vec<Session>) -> MockStorage {
    let mut mock = MockStorage::new();
    let sessions = std::sync::Arc::new(sessions);
    mock.expect_get_session()
        .returning(move |id| {
            Ok(sessions.iter().find(|s| s.id == id).cloned())
        });
    mock
}

/// Fixed time provider for deterministic tests
pub fn mock_time(timestamp: &str) -> MockTimeProvider {
    let dt = chrono::DateTime::parse_from_rfc3339(timestamp)
        .unwrap()
        .with_timezone(&chrono::Utc);
    let mut mock = MockTimeProvider::new();
    mock.expect_now().returning(move || dt);
    mock
}
```

### 16.8 Testing Error Paths

**Requirement**: Every `Result::Err` and `Option::None` path must have a test.

**Pattern for Error Testing**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Happy path test
    #[tokio::test]
    async fn test_process_thought_success() {
        let mock = mock_success_client("{\"result\": \"ok\"}");
        let mode = LinearMode::new(Arc::new(mock));
        let result = mode.process("test").await;
        assert!(result.is_ok());
    }

    // Error path: API failure
    #[tokio::test]
    async fn test_process_thought_api_error() {
        let mock = mock_error_client(ApiError::RateLimit { retry_after: 60 });
        let mode = LinearMode::new(Arc::new(mock));
        let result = mode.process("test").await;
        assert!(matches!(result, Err(ModeError::ApiUnavailable { .. })));
    }

    // Error path: Invalid input
    #[tokio::test]
    async fn test_process_thought_empty_content() {
        let mock = MockAnthropicClient::new();
        let mode = LinearMode::new(Arc::new(mock));
        let result = mode.process("").await;
        assert!(matches!(result, Err(ModeError::ValidationError { field: "content", .. })));
    }

    // Error path: Timeout
    #[tokio::test]
    async fn test_process_thought_timeout() {
        let mut mock = MockAnthropicClient::new();
        mock.expect_complete()
            .returning(|_| Err(ApiError::Timeout { elapsed_ms: 30000 }));
        let mode = LinearMode::new(Arc::new(mock));
        let result = mode.process("test").await;
        assert!(matches!(result, Err(ModeError::Timeout { .. })));
    }
}
```

### 16.9 Coverage Reporting

**Local Development Dashboard**:
```bash
# Generate HTML report and open in browser
cargo llvm-cov --html --open

# Generate report with per-function details
cargo llvm-cov --html --output-dir coverage/ && open coverage/index.html
```

**Codecov Configuration** (`codecov.yml`):
```yaml
coverage:
  status:
    project:
      default:
        target: 100%
        threshold: 0%  # No tolerance, must be exactly 100%
    patch:
      default:
        target: 100%  # New code must also be 100%

parsers:
  gcov:
    branch_detection:
      conditional: yes
      loop: yes
      method: no
      macro: no

comment:
  layout: "reach,diff,flags,files"
  behavior: default
  require_changes: true
```

### 16.10 Coverage Metrics to Track

| Metric | Target | Measurement |
|--------|--------|-------------|
| Line Coverage | 100% | `cargo llvm-cov` |
| Branch Coverage | 100% | `cargo llvm-cov --branch` |
| Function Coverage | 100% | All pub functions have tests |
| Error Path Coverage | 100% | Every `Err` variant tested |
| Edge Case Coverage | Complete | Boundary conditions tested |

### 16.11 Coverage Debugging Workflow

When coverage drops below 100%:

1. **Identify uncovered lines**:
   ```bash
   cargo llvm-cov --show-missing-lines
   ```

2. **Generate detailed HTML report**:
   ```bash
   cargo llvm-cov --html --output-dir coverage/
   ```

3. **Review uncovered code in browser**:
   - Red = not covered
   - Yellow = partially covered (branch)
   - Green = fully covered

4. **Add missing tests**:
   - For each uncovered line, add test case
   - Focus on error paths and edge cases

5. **Verify fix**:
   ```bash
   cargo llvm-cov --fail-under-lines 100
   ```

### 16.12 Coverage Exclusion Registry

Track all exclusions in `docs/COVERAGE_EXCLUSIONS.md`:

```markdown
# Coverage Exclusions Registry

| Location | Reason | Date |
|----------|--------|------|
| `src/error.rs:45` | Infallible conversion | 2025-01-15 |
| `src/debug.rs:*` | Debug-only diagnostics | 2025-01-15 |

## Rules

1. Document every exclusion with clear justification
2. Review quarterly - remove if no longer needed
3. No exclusions for "hard to test" code
4. No exclusions for business logic
```

---

## 17. Client Integration & Deployment

This server is designed exclusively for **Claude Code** and **Claude Desktop** as MCP clients. This section covers everything needed to build, deploy, and configure the server for use.

### 17.1 Target Clients

| Client | Transport | Platform | Config Location |
|--------|-----------|----------|-----------------|
| Claude Code (CLI) | stdio | Windows, macOS, Linux | `claude mcp add` command |
| Claude Desktop | stdio | macOS, Windows | `claude_desktop_config.json` |

**Note**: This server uses **stdio transport** (stdin/stdout JSON-RPC) which is the standard for local MCP servers invoked by Claude clients.

### 17.2 Build & Installation

**Prerequisites**:
- Rust 1.75+ (for async trait stability)
- Anthropic API key

**Build Commands**:
```bash
# Clone repository
git clone https://github.com/[user]/mcp-reasoning.git
cd mcp-reasoning

# Build release binary
cargo build --release

# Binary location (platform-specific)
# Windows: target/release/mcp-reasoning.exe
# macOS/Linux: target/release/mcp-reasoning
```

**Recommended Installation Path**:
```bash
# Windows
$HOME\.local\bin\mcp-reasoning.exe

# macOS/Linux
~/.local/bin/mcp-reasoning

# Or keep in project directory:
C:\Development\Projects\MCP\mcp-servers\mcp-reasoning\target\release\mcp-reasoning.exe
```

### 17.3 Claude Code Configuration

**Add Server (Recommended Method)**:
```bash
# Add with environment variable for API key
claude mcp add mcp-reasoning \
  --transport stdio \
  --env ANTHROPIC_API_KEY=sk-ant-xxx \
  -- /path/to/mcp-reasoning

# Or if ANTHROPIC_API_KEY is already in shell environment:
claude mcp add mcp-reasoning \
  --transport stdio \
  /path/to/mcp-reasoning
```

**Windows Example**:
```bash
claude mcp add mcp-reasoning ^
  --transport stdio ^
  --env ANTHROPIC_API_KEY=sk-ant-xxx ^
  -- C:\Users\%USERNAME%\.local\bin\mcp-reasoning.exe
```

**Verify Installation**:
```bash
# List all servers and check connection status
claude mcp list

# Expected output:
# mcp-reasoning: /path/to/mcp-reasoning -  Connected

# Get detailed config
claude mcp get mcp-reasoning
```

**Remove Server**:
```bash
claude mcp remove mcp-reasoning
```

### 17.4 Claude Desktop Configuration

**Config File Location**:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

**Configuration Format**:
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

**Windows Example**:
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

### 17.5 Environment Variables (Client-Side)

Environment variables can be set in three ways:

| Method | Claude Code | Claude Desktop |
|--------|-------------|----------------|
| Shell environment | Inherited | Not inherited |
| `--env` flag | `claude mcp add --env` | N/A |
| Config file | N/A | `"env": {}` block |

**Required Variables**:
```bash
ANTHROPIC_API_KEY=sk-ant-xxx  # Required: Anthropic API key
```

**Optional Variables**:
```bash
DATABASE_PATH=./data/reasoning.db  # SQLite path (default: ./data/reasoning.db)
LOG_LEVEL=info                      # Logging level (default: info)
REQUEST_TIMEOUT_MS=30000            # API timeout (default: 30000)
MAX_RETRIES=3                       # Retry attempts (default: 3)
```

### 17.6 Stdio Protocol Behavior

The server communicates via JSON-RPC over stdio:

```
┌─────────────┐     stdin      ┌─────────────┐
│ Claude Code │───────────────▶│ MCP Server  │
│ or Desktop  │◀───────────────│             │
└─────────────┘     stdout     └─────────────┘
                      │
                   stderr (logs only)
```

**Protocol Details**:
- **stdin**: Receives JSON-RPC requests from Claude
- **stdout**: Sends JSON-RPC responses to Claude
- **stderr**: Logging output (not read by Claude)

**Important**: The server MUST NOT write anything to stdout except valid JSON-RPC messages. All logging goes to stderr.

### 17.7 Logging & Debugging

**Log Output**:
- All logs go to **stderr** (never stdout)
- Uses `tracing` with structured fields

**Log Levels**:
```bash
LOG_LEVEL=error  # Only errors
LOG_LEVEL=warn   # Errors and warnings
LOG_LEVEL=info   # Default: normal operation
LOG_LEVEL=debug  # Verbose debugging
LOG_LEVEL=trace  # Maximum verbosity
```

**Debugging Tips**:
```bash
# Run server standalone to see logs
ANTHROPIC_API_KEY=xxx LOG_LEVEL=debug ./mcp-reasoning 2>&1

# Test with sample JSON-RPC input
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | ./mcp-reasoning
```

### 17.8 Verification & Testing

**Step 1: Verify Binary Works**
```bash
# Should output JSON-RPC tools list
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | mcp-reasoning
```

**Step 2: Verify Claude Code Connection**
```bash
claude mcp list
# Should show: mcp-reasoning: ... -  Connected
```

**Step 3: Test Tool Invocation**
```
# In Claude Code conversation:
> Use reasoning_linear to analyze "What causes rain?"
```

**Expected Response**: Tool output with thought_id, session_id, content, and confidence.

### 17.9 Troubleshooting

| Symptom | Cause | Solution |
|---------|-------|----------|
| `✗ Failed to connect` | Binary not found | Check path in `claude mcp get mcp-reasoning` |
| `✗ Failed to connect` | Missing API key | Add `--env ANTHROPIC_API_KEY=xxx` |
| `✗ Failed to connect` | Binary not executable | `chmod +x mcp-reasoning` (Unix) |
| Server connects but tools fail | API key invalid | Verify key at console.anthropic.com |
| Timeout errors | API rate limits | Increase `REQUEST_TIMEOUT_MS` |
| Database errors | Path not writable | Check `DATABASE_PATH` permissions |

**Common Issues on Windows**:
```bash
# Path with spaces - use quotes
claude mcp add mcp-reasoning --transport stdio -- "C:\Program Files\mcp\mcp-reasoning.exe"

# Backslash escaping in JSON
"command": "C:\\Users\\name\\bin\\mcp-reasoning.exe"
```

### 17.10 Server Lifecycle

**Startup**: Claude starts the server process when first tool is invoked or on client startup.

**Shutdown**: Server terminates when:
- Claude Code session ends
- Claude Desktop closes
- Client explicitly disconnects

**Graceful Shutdown**:
```rust
// Server handles SIGTERM/SIGINT
tokio::select! {
    _ = shutdown_signal() => {
        // Flush pending writes to SQLite
        // Close database connection
        // Exit cleanly
    }
}
```

### 17.11 Multiple Sessions

The server handles multiple concurrent reasoning sessions:
- Each session has unique `session_id`
- Sessions persist in SQLite
- Claude can reference previous sessions by ID

**Session Isolation**: Different Claude Code instances share the same database, enabling session continuity.

### 17.12 Distribution Checklist

Before releasing:

- [ ] Binary builds on Windows, macOS, Linux
- [ ] `claude mcp add` command documented
- [ ] `claude_desktop_config.json` example provided
- [ ] Environment variables documented
- [ ] README.md has Quick Start section
- [ ] Troubleshooting guide covers common issues
- [ ] Verification steps work end-to-end

---

## 18. Project Scaffolding

Complete project skeleton ready to copy and build. All files needed to pass `cargo build` and `cargo llvm-cov --fail-under-lines 100` on an empty project.

**Repository**: `https://github.com/quanticsoul4772/mcp-reasoning`

### 18.1 Directory Structure

```
mcp-reasoning/
├── .cargo/
│   └── config.toml
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── coverage.yml
├── data/
│   └── .gitkeep
├── docs/
│   ├── API_REFERENCE.md
│   ├── ARCHITECTURE.md
│   ├── COVERAGE_EXCLUSIONS.md
│   └── DESIGN.md
├── migrations/
│   └── 001_initial_schema.sql
├── src/
│   ├── anthropic/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── config.rs
│   │   ├── streaming.rs
│   │   └── types.rs
│   ├── config/
│   │   ├── mod.rs
│   │   └── validation.rs
│   ├── error/
│   │   └── mod.rs
│   ├── metrics/
│   │   └── mod.rs
│   ├── modes/
│   │   ├── mod.rs
│   │   ├── core.rs
│   │   ├── linear.rs
│   │   ├── tree.rs
│   │   ├── divergent.rs
│   │   ├── reflection.rs
│   │   ├── checkpoint.rs
│   │   ├── auto.rs
│   │   ├── graph.rs
│   │   ├── detect.rs
│   │   ├── decision.rs
│   │   ├── evidence.rs
│   │   ├── timeline.rs
│   │   ├── mcts.rs
│   │   └── counterfactual.rs
│   ├── presets/
│   │   ├── mod.rs
│   │   └── builtin.rs
│   ├── prompts/
│   │   ├── mod.rs
│   │   ├── core.rs
│   │   └── advanced.rs
│   ├── self_improvement/
│   │   ├── mod.rs
│   │   ├── types.rs
│   │   ├── monitor.rs
│   │   ├── analyzer.rs
│   │   ├── executor.rs
│   │   ├── learner.rs
│   │   ├── allowlist.rs
│   │   ├── circuit_breaker.rs
│   │   └── system.rs
│   ├── server/
│   │   ├── mod.rs
│   │   ├── mcp.rs
│   │   ├── tools.rs
│   │   ├── handlers.rs
│   │   └── transport.rs
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── sqlite.rs
│   │   ├── session.rs
│   │   ├── thought.rs
│   │   ├── graph.rs
│   │   └── types.rs
│   ├── traits.rs
│   ├── test_utils.rs
│   ├── lib.rs
│   └── main.rs
├── tests/
│   ├── common/
│   │   └── mod.rs
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── anthropic_tests.rs
│   │   ├── storage_tests.rs
│   │   └── server_tests.rs
│   └── modes/
│       ├── mod.rs
│       ├── linear_tests.rs
│       ├── tree_tests.rs
│       └── graph_tests.rs
├── .env.example
├── .gitignore
├── Cargo.toml
├── README.md
├── rust-toolchain.toml
└── codecov.yml
```

### 18.2 Cargo.toml

```toml
[package]
name = "mcp-reasoning"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
authors = ["quanticsoul4772"]
description = "MCP server providing structured reasoning capabilities via Anthropic Claude API"
repository = "https://github.com/quanticsoul4772/mcp-reasoning"
license = "MIT"
readme = "README.md"
keywords = ["mcp", "reasoning", "claude", "anthropic", "ai"]
categories = ["development-tools", "command-line-utilities"]

[lib]
name = "mcp_reasoning"
path = "src/lib.rs"

[[bin]]
name = "mcp-reasoning"
path = "src/main.rs"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal", "sync", "time"] }

# MCP SDK
rmcp = { version = "0.1", features = ["server", "macros", "transport-io"] }

# HTTP client for Anthropic API
reqwest = { version = "0.12", features = ["json", "stream"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }

# Error handling
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
futures = "0.3"
async-trait = "0.1"

# Configuration
dotenvy = "0.15"

# Schema generation
schemars = "0.8"

[dev-dependencies]
# Testing
mockall = "0.13"
tokio-test = "0.4"
pretty_assertions = "1"
test-case = "3"
serial_test = "3"

# Compile-time assertions
static_assertions = "1.1"

[profile.release]
lto = true
codegen-units = 1
strip = true

[profile.test]
opt-level = 1

[workspace.metadata.llvm-cov]
fail-under-lines = 100
ignore-filename-regex = ["tests/", "_test\\.rs$"]
branch = true

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

### 18.3 rust-toolchain.toml

```toml
[toolchain]
channel = "1.75"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
```

### 18.4 .cargo/config.toml

```toml
[alias]
# Coverage commands
cov = "llvm-cov --fail-under-lines 100"
cov-html = "llvm-cov --html --output-dir coverage/"
cov-check = "llvm-cov --show-missing-lines --fail-under-lines 100"

# Development commands
dev = "run"
lint = "clippy -- -D warnings"
fmt-check = "fmt -- --check"

[build]
rustflags = ["-D", "warnings"]

[target.x86_64-pc-windows-msvc]
rustflags = ["-D", "warnings"]

[target.x86_64-unknown-linux-gnu]
rustflags = ["-D", "warnings"]

[target.x86_64-apple-darwin]
rustflags = ["-D", "warnings"]
```

### 18.5 .gitignore

```gitignore
# Build artifacts
/target/
**/*.rs.bk

# Environment
.env
.env.local
.env.*.local

# Database
/data/*.db
/data/*.db-*

# Coverage
/coverage/
*.lcov
lcov.info
*.profraw
*.profdata

# IDE
.idea/
.vscode/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log
```

### 18.6 .env.example

```bash
# Required: Anthropic API key
ANTHROPIC_API_KEY=sk-ant-api03-xxxxx

# Optional: Database path (default: ./data/reasoning.db)
DATABASE_PATH=./data/reasoning.db

# Optional: Logging level (default: info)
# Options: error, warn, info, debug, trace
LOG_LEVEL=info

# Optional: Request timeout in milliseconds (default: 30000)
REQUEST_TIMEOUT_MS=30000

# Optional: Maximum retry attempts (default: 3)
MAX_RETRIES=3

# Optional: Model to use (default: claude-sonnet-4-20250514)
ANTHROPIC_MODEL=claude-sonnet-4-20250514
```

### 18.7 src/lib.rs

```rust
//! MCP Reasoning Server
//!
//! Provides structured reasoning capabilities via Anthropic Claude API.
//!
//! # Features
//!
//! - 15 consolidated reasoning tools
//! - SQLite persistence for sessions and thoughts
//! - Self-improvement system
//! - Full MCP protocol support

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

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

#[cfg(test)]
pub mod test_utils;

pub use config::Config;
pub use error::AppError;
pub use server::McpServer;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

### 18.8 src/main.rs

```rust
//! MCP Reasoning Server binary entry point.

use mcp_reasoning::{Config, McpServer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging (stderr only - stdout reserved for MCP)
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("mcp_reasoning=info".parse()?))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    // Load configuration
    let config = Config::from_env()?;

    // Create and run server
    let server = McpServer::new(config).await?;
    server.run().await?;

    Ok(())
}
```

### 18.9 src/error/mod.rs

```rust
//! Error types for the MCP Reasoning Server.

use thiserror::Error;

/// Application-level error type.
#[derive(Debug, Error)]
pub enum AppError {
    /// Anthropic API error.
    #[error("Anthropic API error: {message}")]
    Anthropic {
        /// Error message.
        message: String,
        /// HTTP status code if available.
        status: Option<u16>,
        /// Whether the error is retryable.
        retryable: bool,
    },

    /// Storage error.
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// MCP protocol error.
    #[error("MCP error: {0}")]
    Mcp(String),

    /// Mode-specific error.
    #[error("Mode error: {0}")]
    Mode(#[from] ModeError),
}

/// Storage-related errors.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Database connection error.
    #[error("Database connection failed: {0}")]
    Connection(String),

    /// Query execution error.
    #[error("Query failed: {0}")]
    Query(String),

    /// Session not found.
    #[error("Session not found: {session_id}")]
    SessionNotFound {
        /// The session ID that was not found.
        session_id: String,
    },

    /// Migration error.
    #[error("Migration failed: {0}")]
    Migration(String),
}

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Missing required environment variable.
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    /// Invalid configuration value.
    #[error("Invalid configuration: {field} - {reason}")]
    Invalid {
        /// The field that is invalid.
        field: String,
        /// The reason it is invalid.
        reason: String,
    },
}

/// Mode-specific errors.
#[derive(Debug, Error)]
pub enum ModeError {
    /// Validation error.
    #[error("Validation error: {field} - {reason}")]
    Validation {
        /// The field that failed validation.
        field: String,
        /// The reason for the failure.
        reason: String,
    },

    /// API unavailable.
    #[error("API unavailable after {retries} retries: {message}")]
    ApiUnavailable {
        /// Error message.
        message: String,
        /// Number of retries attempted.
        retries: u32,
    },

    /// Timeout.
    #[error("Operation timed out after {elapsed_ms}ms")]
    Timeout {
        /// Elapsed time in milliseconds.
        elapsed_ms: u64,
    },

    /// Session required but not provided.
    #[error("Session ID required for this operation")]
    SessionRequired,

    /// Invalid operation.
    #[error("Invalid operation '{operation}' for mode '{mode}'")]
    InvalidOperation {
        /// The operation that was attempted.
        operation: String,
        /// The mode that rejected it.
        mode: String,
    },
}

// Ensure errors are Send + Sync for async contexts
#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_impl_all;

    assert_impl_all!(AppError: Send, Sync, std::error::Error);
    assert_impl_all!(StorageError: Send, Sync, std::error::Error);
    assert_impl_all!(ConfigError: Send, Sync, std::error::Error);
    assert_impl_all!(ModeError: Send, Sync, std::error::Error);
}
```

### 18.10 src/config/mod.rs

```rust
//! Configuration management.

mod validation;

pub use validation::validate_config;

use crate::error::ConfigError;

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Anthropic API key.
    pub api_key: String,
    /// Database path.
    pub database_path: String,
    /// Log level.
    pub log_level: String,
    /// Request timeout in milliseconds.
    pub request_timeout_ms: u64,
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Model to use.
    pub model: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if required variables are missing or invalid.
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let config = Self {
            api_key: std::env::var("ANTHROPIC_API_KEY")
                .map_err(|_| ConfigError::MissingEnvVar("ANTHROPIC_API_KEY".into()))?,
            database_path: std::env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "./data/reasoning.db".into()),
            log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            request_timeout_ms: std::env::var("REQUEST_TIMEOUT_MS")
                .unwrap_or_else(|_| "30000".into())
                .parse()
                .map_err(|_| ConfigError::Invalid {
                    field: "REQUEST_TIMEOUT_MS".into(),
                    reason: "must be a positive integer".into(),
                })?,
            max_retries: std::env::var("MAX_RETRIES")
                .unwrap_or_else(|_| "3".into())
                .parse()
                .map_err(|_| ConfigError::Invalid {
                    field: "MAX_RETRIES".into(),
                    reason: "must be a positive integer".into(),
                })?,
            model: std::env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-20250514".into()),
        };

        validate_config(&config)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // This test requires ANTHROPIC_API_KEY to be set
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        let config = Config::from_env().unwrap();
        assert_eq!(config.database_path, "./data/reasoning.db");
        assert_eq!(config.log_level, "info");
        assert_eq!(config.request_timeout_ms, 30000);
        assert_eq!(config.max_retries, 3);
    }
}
```

### 18.11 src/config/validation.rs

```rust
//! Configuration validation.

use crate::error::ConfigError;
use super::Config;

/// Validate configuration values.
///
/// # Errors
///
/// Returns `ConfigError::Invalid` if any value is out of range.
pub fn validate_config(config: &Config) -> Result<(), ConfigError> {
    // API key must not be empty
    if config.api_key.is_empty() {
        return Err(ConfigError::Invalid {
            field: "ANTHROPIC_API_KEY".into(),
            reason: "must not be empty".into(),
        });
    }

    // Timeout must be reasonable (1s to 5m)
    if config.request_timeout_ms < 1000 || config.request_timeout_ms > 300_000 {
        return Err(ConfigError::Invalid {
            field: "REQUEST_TIMEOUT_MS".into(),
            reason: "must be between 1000 and 300000 ms".into(),
        });
    }

    // Max retries must be reasonable (0 to 10)
    if config.max_retries > 10 {
        return Err(ConfigError::Invalid {
            field: "MAX_RETRIES".into(),
            reason: "must be between 0 and 10".into(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            api_key: "test-key".into(),
            database_path: "./data/test.db".into(),
            log_level: "info".into(),
            request_timeout_ms: 30000,
            max_retries: 3,
            model: "claude-sonnet-4-20250514".into(),
        }
    }

    #[test]
    fn test_valid_config() {
        let config = test_config();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_empty_api_key() {
        let mut config = test_config();
        config.api_key = String::new();
        let err = validate_config(&config).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { field, .. } if field == "ANTHROPIC_API_KEY"));
    }

    #[test]
    fn test_timeout_too_low() {
        let mut config = test_config();
        config.request_timeout_ms = 500;
        let err = validate_config(&config).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { field, .. } if field == "REQUEST_TIMEOUT_MS"));
    }

    #[test]
    fn test_timeout_too_high() {
        let mut config = test_config();
        config.request_timeout_ms = 400_000;
        let err = validate_config(&config).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { field, .. } if field == "REQUEST_TIMEOUT_MS"));
    }

    #[test]
    fn test_retries_too_high() {
        let mut config = test_config();
        config.max_retries = 15;
        let err = validate_config(&config).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { field, .. } if field == "MAX_RETRIES"));
    }
}
```

### 18.12 src/traits.rs

```rust
//! Trait definitions for mockable dependencies.

use async_trait::async_trait;
use crate::error::{ModeError, StorageError};

/// Anthropic API client trait for mocking.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AnthropicClientTrait: Send + Sync {
    /// Send a completion request.
    async fn complete(&self, messages: Vec<Message>, config: CompletionConfig)
        -> Result<CompletionResponse, ModeError>;
}

/// Storage trait for mocking.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait StorageTrait: Send + Sync {
    /// Get a session by ID.
    async fn get_session(&self, id: &str) -> Result<Option<Session>, StorageError>;

    /// Create or get a session.
    async fn get_or_create_session(&self, id: Option<&str>) -> Result<Session, StorageError>;

    /// Save a thought.
    async fn save_thought(&self, thought: &Thought) -> Result<(), StorageError>;
}

/// Time provider trait for deterministic testing.
#[cfg_attr(test, mockall::automock)]
pub trait TimeProvider: Send + Sync {
    /// Get the current time.
    fn now(&self) -> chrono::DateTime<chrono::Utc>;
}

/// Real time provider.
pub struct RealTimeProvider;

impl TimeProvider for RealTimeProvider {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

// Placeholder types - will be properly defined in their modules
/// Message for API requests.
#[derive(Debug, Clone)]
pub struct Message {
    /// Role (user, assistant, system).
    pub role: String,
    /// Content.
    pub content: String,
}

/// Completion configuration.
#[derive(Debug, Clone, Default)]
pub struct CompletionConfig {
    /// Maximum tokens.
    pub max_tokens: Option<u32>,
    /// Temperature.
    pub temperature: Option<f32>,
}

/// Completion response.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// Response content.
    pub content: String,
    /// Tokens used.
    pub usage: Usage,
}

/// Token usage.
#[derive(Debug, Clone, Default)]
pub struct Usage {
    /// Input tokens.
    pub input_tokens: u32,
    /// Output tokens.
    pub output_tokens: u32,
}

/// Session data.
#[derive(Debug, Clone)]
pub struct Session {
    /// Session ID.
    pub id: String,
    /// Creation time.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Thought data.
#[derive(Debug, Clone)]
pub struct Thought {
    /// Thought ID.
    pub id: String,
    /// Session ID.
    pub session_id: String,
    /// Content.
    pub content: String,
    /// Mode used.
    pub mode: String,
    /// Confidence score.
    pub confidence: f64,
    /// Creation time.
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

### 18.13 src/server/mod.rs

```rust
//! MCP server implementation.

mod handlers;
mod mcp;
mod tools;
mod transport;

pub use mcp::McpServer;

use crate::config::Config;
use crate::storage::Storage;
use crate::anthropic::AnthropicClient;
use std::sync::Arc;

/// Shared application state.
pub struct AppState {
    /// Configuration.
    pub config: Config,
    /// Storage backend.
    pub storage: Arc<Storage>,
    /// Anthropic client.
    pub client: Arc<AnthropicClient>,
}
```

### 18.14 src/server/mcp.rs (Stub)

```rust
//! MCP protocol handler.

use crate::config::Config;
use crate::error::AppError;
use super::AppState;

/// MCP Server.
pub struct McpServer {
    #[allow(dead_code)]
    state: AppState,
}

impl McpServer {
    /// Create a new MCP server.
    ///
    /// # Errors
    ///
    /// Returns error if initialization fails.
    pub async fn new(config: Config) -> Result<Self, AppError> {
        // TODO: Initialize storage and client
        todo!("Implement McpServer::new")
    }

    /// Run the server.
    ///
    /// # Errors
    ///
    /// Returns error if server fails.
    pub async fn run(self) -> Result<(), AppError> {
        // TODO: Start MCP stdio transport
        todo!("Implement McpServer::run")
    }
}
```

### 18.15 migrations/001_initial_schema.sql

```sql
-- MCP Reasoning Server Database Schema
-- Version: 1

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    metadata TEXT -- JSON metadata
);

-- Thoughts table
CREATE TABLE IF NOT EXISTS thoughts (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    parent_id TEXT REFERENCES thoughts(id) ON DELETE SET NULL,
    mode TEXT NOT NULL,
    content TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.0,
    metadata TEXT, -- JSON metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Branches table (for tree mode)
CREATE TABLE IF NOT EXISTS branches (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    parent_branch_id TEXT REFERENCES branches(id) ON DELETE SET NULL,
    content TEXT NOT NULL,
    score REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'active', -- active, completed, abandoned
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Checkpoints table
CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    state TEXT NOT NULL, -- JSON serialized state
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Graph nodes table (for GoT mode)
CREATE TABLE IF NOT EXISTS graph_nodes (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    node_type TEXT NOT NULL DEFAULT 'thought', -- thought, aggregation, refinement
    score REAL,
    is_terminal INTEGER NOT NULL DEFAULT 0,
    metadata TEXT, -- JSON metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Graph edges table
CREATE TABLE IF NOT EXISTS graph_edges (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    from_node_id TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    to_node_id TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    edge_type TEXT NOT NULL DEFAULT 'continues', -- continues, aggregates, refines
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Metrics table
CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mode TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    latency_ms INTEGER NOT NULL,
    success INTEGER NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Self-improvement actions table
CREATE TABLE IF NOT EXISTS self_improvement_actions (
    id TEXT PRIMARY KEY,
    action_type TEXT NOT NULL,
    parameters TEXT NOT NULL, -- JSON
    status TEXT NOT NULL DEFAULT 'pending', -- pending, executing, completed, failed, rolled_back
    result TEXT, -- JSON
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_thoughts_session ON thoughts(session_id);
CREATE INDEX IF NOT EXISTS idx_thoughts_parent ON thoughts(parent_id);
CREATE INDEX IF NOT EXISTS idx_branches_session ON branches(session_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_session ON graph_nodes(session_id);
CREATE INDEX IF NOT EXISTS idx_graph_edges_session ON graph_edges(session_id);
CREATE INDEX IF NOT EXISTS idx_metrics_mode ON metrics(mode);
CREATE INDEX IF NOT EXISTS idx_metrics_created ON metrics(created_at);
CREATE INDEX IF NOT EXISTS idx_self_improvement_status ON self_improvement_actions(status);
```

### 18.16 .github/workflows/ci.yml

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --all-features

  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - run: cargo clippy --all-targets --all-features -- -D warnings

  test:
    name: Test
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features

  docs:
    name: Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo doc --no-deps --all-features
        env:
          RUSTDOCFLAGS: "-D warnings"
```

### 18.17 .github/workflows/coverage.yml

```yaml
name: Coverage

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  coverage:
    name: Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov

      - name: Run tests with coverage
        run: cargo llvm-cov --all-features --fail-under-lines 100 --lcov --output-path lcov.info

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v4
        with:
          files: lcov.info
          fail_ci_if_error: true
          verbose: true
        env:
          CODECOV_TOKEN: ${{ secrets.CODECOV_TOKEN }}
```

### 18.18 codecov.yml

```yaml
coverage:
  status:
    project:
      default:
        target: 100%
        threshold: 0%
    patch:
      default:
        target: 100%

parsers:
  gcov:
    branch_detection:
      conditional: yes
      loop: yes
      method: no
      macro: no

comment:
  layout: "reach,diff,flags,files"
  behavior: default
  require_changes: true
```

### 18.19 README.md Template

```markdown
# mcp-reasoning

MCP server providing structured reasoning capabilities via Anthropic Claude API.

## Features

- **15 Reasoning Tools** - Linear, tree, divergent, reflection, graph-of-thoughts, and more
- **Direct Anthropic API** - No middleware, direct Claude integration
- **Session Persistence** - SQLite storage for sessions and thoughts
- **Self-Improvement** - Autonomous optimization loop
- **100% Test Coverage** - Enforced via CI

## Quick Start

### Prerequisites

- Rust 1.75+
- Anthropic API key

### Installation

```bash
git clone https://github.com/quanticsoul4772/mcp-reasoning.git
cd mcp-reasoning

# Configure
cp .env.example .env
# Edit .env: set ANTHROPIC_API_KEY

# Build
cargo build --release
```

### Configure Claude Code

```bash
claude mcp add mcp-reasoning \
  --transport stdio \
  --env ANTHROPIC_API_KEY=your-key \
  -- ./target/release/mcp-reasoning
```

### Verify

```bash
claude mcp list
# mcp-reasoning: ... -  Connected
```

## Available Tools

| Tool | Description |
|------|-------------|
| `reasoning_linear` | Sequential step-by-step reasoning |
| `reasoning_tree` | Branching exploration with multiple paths |
| `reasoning_divergent` | Creative multi-perspective reasoning |
| `reasoning_reflection` | Meta-cognitive analysis |
| `reasoning_checkpoint` | Save and restore reasoning state |
| `reasoning_auto` | Automatic mode selection |
| `reasoning_graph` | Graph-of-Thoughts reasoning |
| `reasoning_detect` | Bias and fallacy detection |
| `reasoning_decision` | Multi-criteria decision analysis |
| `reasoning_evidence` | Evidence assessment |
| `reasoning_timeline` | Temporal reasoning exploration |
| `reasoning_mcts` | Monte Carlo Tree Search |
| `reasoning_counterfactual` | "What if?" causal analysis |
| `reasoning_preset` | Workflow presets |
| `reasoning_metrics` | Usage metrics |

## Development

```bash
# Run tests
cargo test

# Run with coverage
cargo llvm-cov --fail-under-lines 100

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

## License

MIT
```

### 18.20 Stub Module Files

All remaining module files should be created as minimal stubs that compile:

```rust
//! Module description.
// src/anthropic/mod.rs, src/modes/mod.rs, etc.

// Re-exports
// pub use submodule::Type;

// TODO: Implement
```

**Example stub** (`src/modes/linear.rs`):
```rust
//! Linear reasoning mode.

use crate::error::ModeError;

/// Linear reasoning mode.
pub struct LinearMode;

impl LinearMode {
    /// Create a new linear mode instance.
    pub fn new() -> Self {
        Self
    }

    /// Process a thought linearly.
    ///
    /// # Errors
    ///
    /// Returns error if processing fails.
    pub async fn process(&self, _content: &str) -> Result<LinearResponse, ModeError> {
        todo!("Implement LinearMode::process")
    }
}

/// Response from linear reasoning.
#[derive(Debug)]
pub struct LinearResponse {
    /// Thought ID.
    pub thought_id: String,
    /// Session ID.
    pub session_id: String,
    /// Reasoning content.
    pub content: String,
    /// Confidence score.
    pub confidence: f64,
}
```

### 18.21 Scaffolding Checklist

Execute in order:

- [ ] Create directory structure (`mkdir -p src/{anthropic,config,error,...}`)
- [ ] Create `Cargo.toml`
- [ ] Create `rust-toolchain.toml`
- [ ] Create `.cargo/config.toml`
- [ ] Create `.gitignore`
- [ ] Create `.env.example`
- [ ] Create `src/lib.rs`
- [ ] Create `src/main.rs`
- [ ] Create `src/error/mod.rs`
- [ ] Create `src/config/mod.rs` and `validation.rs`
- [ ] Create `src/traits.rs`
- [ ] Create `src/server/mod.rs` and `mcp.rs`
- [ ] Create all stub module files
- [ ] Create `migrations/001_initial_schema.sql`
- [ ] Create `.github/workflows/ci.yml`
- [ ] Create `.github/workflows/coverage.yml`
- [ ] Create `codecov.yml`
- [ ] Create `README.md`
- [ ] Run `cargo build` - should compile
- [ ] Run `cargo test` - should pass (no tests yet)
- [ ] Run `cargo clippy` - should pass
- [ ] Commit initial scaffold

---

## Appendix A: Tool Consolidation Summary

| Before (40 tools) | After (15 tools) |
|-------------------|------------------|
| reasoning_linear | reasoning_linear |
| reasoning_tree, tree_focus, tree_list, tree_complete | reasoning_tree |
| reasoning_divergent | reasoning_divergent |
| reasoning_reflection, reflection_evaluate | reasoning_reflection |
| reasoning_backtrack, checkpoint_create, checkpoint_list | reasoning_checkpoint |
| reasoning_auto | reasoning_auto |
| reasoning_got_* (8 tools) | reasoning_graph |
| reasoning_detect_biases, detect_fallacies | reasoning_detect |
| reasoning_make_decision, analyze_perspectives | reasoning_decision |
| reasoning_assess_evidence, probabilistic | reasoning_evidence |
| reasoning_timeline_* (4 tools) | reasoning_timeline |
| reasoning_mcts_explore, auto_backtrack | reasoning_mcts |
| reasoning_counterfactual | reasoning_counterfactual |
| reasoning_preset_list, preset_run | reasoning_preset |
| reasoning_metrics_* (5 tools) | reasoning_metrics |

---

## Appendix B: SDK Utilization Summary

This design maximizes utilization of available SDKs and APIs.

### B.1 rmcp (MCP Rust SDK) Features Used

| Feature | Status | Usage |
|---------|--------|-------|
| `#[tool]` macro | Used | Tool definitions (Section 9.1) |
| `#[tool_router]` macro | Used | Handler routing (Section 9.1) |
| `#[arg]` attribute | Used | Parameter descriptions (Section 9.1) |
| JsonSchema derive | Used | Output schema generation (Section 9.2) |
| Server registration | Used | Main entry point (Section 9.3) |
| Stdio transport | Used | CLI integration (Section 8.2) |
| SSE transport | Used | Web clients (Section 9.3) |
| Axum integration | Used | HTTP transport (Section 8.1) |
| Tool annotations | Used | Behavior hints (Section 3) |
| Output schemas | Used | Type-safe responses (Section 3) |

### B.2 Anthropic Rust SDK Features Used

| Feature | Status | Usage |
|---------|--------|-------|
| Messages API | Used | Core reasoning (Section 11) |
| Streaming | Used | Long-running operations (Section 11.3) |
| Extended Thinking | Used | Deep analysis modes (Section 11.2, 11.4) |
| Tool Use | Used | Agentic reasoning (Section 11.2) |
| Vision (Images) | Used | Image-based reasoning (Section 11.2) |
| Retry logic | Used | Resilience (Section 11.1) |
| Error types | Used | Error handling (Section 10.1) |

### B.3 Extended Thinking Budget by Mode

| Mode | Thinking Budget | Rationale |
|------|-----------------|-----------|
| linear | None | Fast, single-pass reasoning |
| tree | None | Multiple branches, thinking per branch |
| divergent | 4096 (standard) | Creative exploration needs depth |
| reflection | 8192 (deep) | Meta-cognitive analysis requires deep thinking |
| auto | None | Fast routing, no thinking needed |
| graph | 4096 (standard) | Node generation benefits from thinking |
| decision | 8192 (deep) | Multi-criteria analysis needs depth |
| evidence | 8192 (deep) | Bayesian reasoning benefits from thinking |
| counterfactual | 16384 (maximum) | Causal chain analysis is complex |
| mcts | 16384 (maximum) | Tree search exploration is complex |

### B.4 Streaming-Enabled Modes

The following modes support streaming for real-time progress updates:

- `reasoning_tree` - Stream branch exploration
- `reasoning_divergent` - Stream perspective generation
- `reasoning_reflection` - Stream quality analysis
- `reasoning_graph` - Stream node operations
- `reasoning_decision` - Stream criteria evaluation
- `reasoning_evidence` - Stream evidence assessment
- `reasoning_counterfactual` - Stream causal analysis
- `reasoning_mcts` - Stream search iterations

### B.5 SDK Feature Comparison

| Aspect | Before (mcp-langbase-reasoning) | After (mcp-reasoning) |
|--------|----------------------------------|----------------------|
| MCP SDK | Custom implementation | rmcp v0.9 with macros |
| Tool definitions | Manual JSON schemas | `#[tool]` macro |
| Handler routing | 1600-line match statement | `#[tool_router]` |
| API calls | Via Langbase pipes | Direct Anthropic SDK |
| Streaming | Not supported | Full streaming support |
| Extended thinking | Not available | Mode-specific budgets |
| Tool use | Not available | Agentic reasoning ready |
| Vision | Not available | Image input support |
| Schema generation | Manual | Derive JsonSchema |
| Estimated LoC | ~8000 for server | ~3000 for server |
