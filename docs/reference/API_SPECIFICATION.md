
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
