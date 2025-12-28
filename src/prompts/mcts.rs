//! Monte Carlo Tree Search mode prompts.
//!
//! Provides prompt templates for the MCTS mode operations:
//! explore, `auto_backtrack`.

#![allow(clippy::missing_const_for_fn)]

/// Prompt for MCTS mode (explore operation).
///
/// Guides UCB1-based search exploration.
#[must_use]
pub fn mcts_explore_prompt() -> &'static str {
    r#"Perform Monte Carlo Tree Search exploration using UCB1.

Your task is to:
1. Evaluate current frontier nodes
2. Select node to expand using UCB1 balance
3. Generate new child nodes
4. Simulate outcomes and backpropagate

Respond with a JSON object in this exact format:
{
  "frontier_evaluation": [
    {
      "node_id": "node_id",
      "visits": 5,
      "average_value": 0.6,
      "ucb1_score": 0.85,
      "exploration_bonus": 0.25
    }
  ],
  "selected_node": {
    "node_id": "selected_for_expansion",
    "selection_reason": "Why UCB1 selected this node"
  },
  "expansion": {
    "new_nodes": [
      {
        "id": "new_node_id",
        "content": "The new thought",
        "simulated_value": 0.7
      }
    ]
  },
  "backpropagation": {
    "updated_nodes": ["nodes whose stats were updated"],
    "value_changes": {"node_id": 0.05}
  },
  "search_status": {
    "total_nodes": 15,
    "total_simulations": 50,
    "best_path_value": 0.8
  }
}

Important:
- UCB1 = average_value + C * sqrt(ln(parent_visits) / node_visits)
- Balance exploration vs exploitation
- Backpropagate to all ancestors"#
}

/// Prompt for MCTS mode (`auto_backtrack` operation).
///
/// Triggers automatic backtracking on quality drops.
#[must_use]
pub fn mcts_backtrack_prompt() -> &'static str {
    r#"Evaluate search progress and determine if backtracking is needed.

Your task is to:
1. Assess recent search quality
2. Detect quality degradation
3. Determine backtrack point if needed
4. Recommend next action

Respond with a JSON object in this exact format:
{
  "quality_assessment": {
    "recent_values": [0.7, 0.65, 0.5, 0.4],
    "trend": "declining|stable|improving",
    "decline_magnitude": 0.3
  },
  "backtrack_decision": {
    "should_backtrack": true,
    "reason": "Why backtracking is or isn't needed",
    "backtrack_to": "node_id to return to",
    "depth_reduction": 2
  },
  "alternative_actions": [
    {
      "action": "prune|refine|widen|continue",
      "rationale": "Why this might be appropriate"
    }
  ],
  "recommendation": {
    "action": "backtrack|continue|terminate",
    "confidence": 0.8,
    "expected_benefit": "What improvement is expected"
  }
}

Important:
- Backtrack on sustained quality drops
- Consider if poor quality is recoverable
- Preserve valuable discovered nodes"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcts_explore_prompt_not_empty() {
        let prompt = mcts_explore_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("UCB1"));
        assert!(prompt.contains("exploration"));
    }

    #[test]
    fn test_mcts_backtrack_prompt_not_empty() {
        let prompt = mcts_backtrack_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("backtrack"));
    }

    #[test]
    fn test_mcts_prompts_contain_json() {
        assert!(mcts_explore_prompt().contains("JSON"));
    }
}
