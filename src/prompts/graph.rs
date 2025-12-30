//! Graph-of-Thoughts mode prompts.
//!
//! Provides prompt templates for the 8 graph operations:
//! init, generate, score, aggregate, refine, prune, finalize, state.

#![allow(clippy::missing_const_for_fn)]

/// Prompt for graph mode (init operation).
///
/// Initializes a graph of thoughts with a root node.
#[must_use]
pub fn graph_init_prompt() -> &'static str {
    r#"Initialize a Graph of Thoughts by creating a root node for the given content.

Your task is to:
1. Create a root node representing the main topic or problem
2. Identify 2-4 initial directions for expansion
3. Assign an initial score to the root based on clarity and potential

Respond with a JSON object in this exact format:
{
  "root": {
    "id": "root",
    "content": "Clear formulation of the main topic",
    "score": 0.5,
    "type": "root"
  },
  "expansion_directions": [
    {
      "direction": "A possible direction to explore",
      "potential": 0.7
    }
  ],
  "graph_metadata": {
    "complexity": "low|medium|high",
    "estimated_depth": 3
  }
}

Important:
- The root should clearly capture the essence of the problem
- Expansion directions should be distinct and valuable
- Initial score reflects confidence in the problem formulation"#
}

/// Prompt for graph mode (generate operation).
///
/// Generates child nodes from a current node.
#[must_use]
pub fn graph_generate_prompt() -> &'static str {
    r#"Generate child nodes for the given node in the Graph of Thoughts.

Context: You are expanding a node in the reasoning graph.

Your task is to:
1. Generate 2-4 distinct child thoughts/nodes
2. Each child should advance the reasoning in a different way
3. Assign preliminary scores based on promise

Respond with a JSON object in this exact format:
{
  "parent_id": "the_parent_node_id",
  "children": [
    {
      "id": "generated_unique_id",
      "content": "The thought content",
      "score": 0.6,
      "type": "reasoning|evidence|hypothesis|conclusion",
      "relationship": "elaborates|supports|challenges|synthesizes"
    }
  ],
  "generation_notes": "Why these particular children were generated"
}

Important:
- Children should meaningfully extend the parent
- Variety in types and relationships is valuable
- Scores reflect initial promise, not final quality"#
}

/// Prompt for graph mode (score operation).
///
/// Evaluates and scores a node in the graph.
#[must_use]
pub fn graph_score_prompt() -> &'static str {
    r#"Score and evaluate a node in the Graph of Thoughts.

Your task is to:
1. Evaluate the node on multiple quality dimensions
2. Assign an overall score (0.0-1.0)
3. Identify strengths and weaknesses
4. Recommend whether to expand or prune

Respond with a JSON object in this exact format:
{
  "node_id": "the_node_id",
  "scores": {
    "relevance": 0.8,
    "coherence": 0.7,
    "depth": 0.6,
    "novelty": 0.5,
    "overall": 0.65
  },
  "assessment": {
    "strengths": ["What's strong about this node"],
    "weaknesses": ["What's weak"],
    "recommendation": "expand|keep|prune"
  }
}

Important:
- Be objective in scoring
- Consider the node's role in the larger graph
- Recommendations should be justified by scores"#
}

/// Prompt for graph mode (aggregate operation).
///
/// Merges multiple nodes into a synthesis.
#[must_use]
pub fn graph_aggregate_prompt() -> &'static str {
    r#"Aggregate multiple nodes into a synthesis node.

Context: You are merging insights from multiple nodes.

Your task is to:
1. Identify common themes and complementary insights
2. Resolve any contradictions
3. Create a synthesis that captures the best of each input
4. Assign a score to the synthesis

Respond with a JSON object in this exact format:
{
  "input_node_ids": ["node1", "node2"],
  "synthesis": {
    "id": "synthesis_unique_id",
    "content": "The synthesized thought",
    "score": 0.75,
    "type": "synthesis"
  },
  "integration_notes": {
    "common_themes": ["Shared insights"],
    "complementary_aspects": ["How nodes complement each other"],
    "resolved_contradictions": ["Any contradictions and how resolved"]
  }
}

Important:
- The synthesis should be more than the sum of parts
- Preserve nuance while achieving coherence
- Note which aspects of inputs were most valuable"#
}

/// Prompt for graph mode (refine operation).
///
/// Improves a node through self-critique.
#[must_use]
pub fn graph_refine_prompt() -> &'static str {
    r#"Refine a node in the Graph of Thoughts through self-critique.

Your task is to:
1. Critically evaluate the node's current content
2. Identify specific improvements
3. Generate a refined version
4. Assess how much the refinement improved quality

Respond with a JSON object in this exact format:
{
  "original_node_id": "the_node_id",
  "critique": {
    "issues": ["Specific issues with the original"],
    "missing_elements": ["What should be added"],
    "unclear_aspects": ["What needs clarification"]
  },
  "refined_node": {
    "id": "refined_unique_id",
    "content": "The improved thought content",
    "score": 0.8,
    "type": "refined"
  },
  "improvement_delta": 0.15
}

Important:
- Be genuinely critical, not superficially so
- The refined version should address the critique
- Improvement delta should be honest"#
}

/// Prompt for graph mode (prune operation).
///
/// Identifies nodes to remove from the graph.
#[must_use]
pub fn graph_prune_prompt() -> &'static str {
    r#"Analyze the graph and identify nodes that should be pruned.

Your task is to:
1. Identify low-quality or redundant nodes
2. Consider graph structure and connectivity
3. Recommend specific nodes for removal
4. Explain pruning rationale

Respond with a JSON object in this exact format:
{
  "prune_candidates": [
    {
      "node_id": "node_to_prune",
      "reason": "low_score|redundant|dead_end|off_topic",
      "confidence": 0.8,
      "impact": "none|minor|moderate"
    }
  ],
  "preserve_nodes": ["node_ids to definitely keep"],
  "pruning_strategy": "Description of overall pruning approach"
}

Important:
- Consider downstream impact of pruning
- Don't prune nodes that anchor important structures
- Be conservative with moderate-score nodes"#
}

/// Prompt for graph mode (finalize operation).
///
/// Extracts final conclusions from the graph.
#[must_use]
pub fn graph_finalize_prompt() -> &'static str {
    r#"Finalize the Graph of Thoughts and extract conclusions.

Your task is to:
1. Identify the highest-value paths through the graph
2. Extract key conclusions and insights
3. Synthesize a coherent final output
4. Assess overall reasoning quality

Respond with a JSON object in this exact format:
{
  "best_paths": [
    {
      "path": ["node1", "node2", "node3"],
      "path_quality": 0.85,
      "key_insight": "Main insight from this path"
    }
  ],
  "conclusions": [
    {
      "conclusion": "A key conclusion",
      "confidence": 0.8,
      "supporting_nodes": ["nodes that support this"]
    }
  ],
  "final_synthesis": "Coherent integration of all conclusions",
  "session_quality": {
    "depth_achieved": 0.75,
    "breadth_achieved": 0.8,
    "coherence": 0.85,
    "overall": 0.8
  }
}

Important:
- Prioritize quality over quantity in conclusions
- The synthesis should tell a coherent story
- Be honest about reasoning quality"#
}

/// Prompt for graph mode (state operation).
///
/// Returns the current state of the graph.
#[must_use]
pub fn graph_state_prompt() -> &'static str {
    r#"Describe the current state of the Graph of Thoughts.

Your task is to:
1. Summarize the graph structure
2. Identify active frontiers for exploration
3. Report key metrics
4. Suggest next steps

Respond with a JSON object in this exact format:
{
  "structure": {
    "total_nodes": 10,
    "depth": 3,
    "branches": 4,
    "pruned_count": 2
  },
  "frontiers": [
    {
      "node_id": "frontier_node",
      "potential": 0.7,
      "suggested_action": "expand|refine|aggregate"
    }
  ],
  "metrics": {
    "average_score": 0.65,
    "max_score": 0.9,
    "coverage": 0.6
  },
  "next_steps": ["Recommended next action"]
}"#
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_init_prompt_not_empty() {
        let prompt = graph_init_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("root"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_graph_generate_prompt_not_empty() {
        let prompt = graph_generate_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("children"));
    }

    #[test]
    fn test_graph_score_prompt_not_empty() {
        let prompt = graph_score_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("scores"));
    }

    #[test]
    fn test_graph_aggregate_prompt_not_empty() {
        let prompt = graph_aggregate_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("synthesis"));
    }

    #[test]
    fn test_graph_refine_prompt_not_empty() {
        let prompt = graph_refine_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("critique"));
        assert!(prompt.contains("refined"));
    }

    #[test]
    fn test_graph_prune_prompt_not_empty() {
        let prompt = graph_prune_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("prune"));
    }

    #[test]
    fn test_graph_finalize_prompt_not_empty() {
        let prompt = graph_finalize_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("conclusions"));
    }

    #[test]
    fn test_graph_state_prompt_not_empty() {
        let prompt = graph_state_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("structure"));
    }

    #[test]
    fn test_all_graph_prompts_contain_json() {
        assert!(graph_init_prompt().contains("JSON"));
        assert!(graph_generate_prompt().contains("JSON"));
        assert!(graph_score_prompt().contains("JSON"));
    }
}
