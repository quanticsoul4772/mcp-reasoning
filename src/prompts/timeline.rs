//! Temporal reasoning mode prompts.
//!
//! Provides prompt templates for the timeline mode operations:
//! create, branch, compare, merge.

#![allow(clippy::missing_const_for_fn)]

/// Prompt for timeline mode (create operation).
///
/// Creates a new timeline for temporal reasoning.
#[must_use]
pub fn timeline_create_prompt() -> &'static str {
    r#"Create a timeline for temporal reasoning about the scenario.

Your task is to:
1. Identify key events or states
2. Establish temporal ordering
3. Note causal relationships
4. Identify decision points

Respond with a JSON object in this exact format:
{
  "timeline_id": "unique_id",
  "events": [
    {
      "id": "event_id",
      "description": "What happens",
      "time": "relative or absolute time marker",
      "type": "event|state|decision_point",
      "causes": ["event_ids that cause this"],
      "effects": ["event_ids caused by this"]
    }
  ],
  "decision_points": [
    {
      "id": "decision_id",
      "description": "The decision to be made",
      "options": ["possible choices"],
      "deadline": "when decision must be made"
    }
  ],
  "temporal_structure": {
    "start": "beginning event_id",
    "current": "present event_id",
    "horizon": "how far into future we're considering"
  }
}

Important:
- Be precise about temporal ordering
- Distinguish events, states, and decision points
- Capture causal chains accurately"#
}

/// Prompt for timeline mode (branch operation).
///
/// Creates alternative timeline branches.
#[must_use]
pub fn timeline_branch_prompt() -> &'static str {
    r#"Create alternative timeline branches from a decision point.

Context: You are branching the timeline at a specific decision point.

Your task is to:
1. Identify the decision point
2. Create branches for each major option
3. Project consequences along each branch
4. Assess branch plausibility

Respond with a JSON object in this exact format:
{
  "branch_point": {
    "event_id": "the_decision_point",
    "description": "The decision being made"
  },
  "branches": [
    {
      "id": "branch_id",
      "choice": "The option chosen",
      "events": [
        {
          "id": "event_id",
          "description": "Consequent event",
          "probability": 0.8,
          "time_offset": "how long after branch point"
        }
      ],
      "plausibility": 0.7,
      "outcome_quality": 0.6
    }
  ],
  "comparison": {
    "most_likely_good_outcome": "branch_id",
    "highest_risk": "branch_id",
    "key_differences": ["What distinguishes the branches"]
  }
}

Important:
- Be realistic about consequences
- Consider second and third-order effects
- Note uncertainties in projections"#
}

/// Prompt for timeline mode (compare operation).
///
/// Compares different timeline branches.
#[must_use]
pub fn timeline_compare_prompt() -> &'static str {
    r#"Compare timeline branches to analyze differences and tradeoffs.

Your task is to:
1. Identify key differences between branches
2. Compare outcomes on relevant dimensions
3. Assess risks and opportunities
4. Make a recommendation if appropriate

Respond with a JSON object in this exact format:
{
  "branches_compared": ["branch_1", "branch_2"],
  "divergence_point": "Where the branches split",
  "key_differences": [
    {
      "dimension": "What's being compared",
      "branch_1_value": "Outcome in branch 1",
      "branch_2_value": "Outcome in branch 2",
      "significance": "Why this difference matters"
    }
  ],
  "risk_assessment": {
    "branch_1_risks": ["Risks in this branch"],
    "branch_2_risks": ["Risks in this branch"]
  },
  "opportunity_assessment": {
    "branch_1_opportunities": ["Opportunities"],
    "branch_2_opportunities": ["Opportunities"]
  },
  "recommendation": {
    "preferred_branch": "branch_id or 'depends'",
    "conditions": "Under what conditions this is preferred",
    "key_factors": "What factors are most decisive"
  }
}

Important:
- Compare on multiple dimensions
- Be explicit about uncertainties
- Recommendations should be contingent where appropriate"#
}

/// Prompt for timeline mode (merge operation).
///
/// Synthesizes insights from timeline exploration.
#[must_use]
pub fn timeline_merge_prompt() -> &'static str {
    r#"Merge timeline branches to synthesize insights.

Your task is to:
1. Identify common patterns across branches
2. Synthesize key learnings
3. Identify robust strategies
4. Provide actionable recommendations

Respond with a JSON object in this exact format:
{
  "branches_merged": ["branch_ids"],
  "common_patterns": [
    {
      "pattern": "Something that happens in multiple branches",
      "frequency": 0.8,
      "implications": "What this suggests"
    }
  ],
  "robust_strategies": [
    {
      "strategy": "Action that works well across branches",
      "effectiveness": 0.75,
      "conditions": "When this strategy is applicable"
    }
  ],
  "fragile_strategies": [
    {
      "strategy": "Action that only works in some branches",
      "failure_modes": "When it fails"
    }
  ],
  "synthesis": "Overall conclusions from timeline exploration",
  "recommendations": ["Actionable next steps"]
}

Important:
- Focus on robust insights
- Distinguish luck from skill
- Recommendations should be actionable"#
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_create_prompt_not_empty() {
        let prompt = timeline_create_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("timeline"));
        assert!(prompt.contains("events"));
    }

    #[test]
    fn test_timeline_branch_prompt_not_empty() {
        let prompt = timeline_branch_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("branch"));
    }

    #[test]
    fn test_timeline_compare_prompt_not_empty() {
        let prompt = timeline_compare_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("compare"));
    }

    #[test]
    fn test_timeline_merge_prompt_not_empty() {
        let prompt = timeline_merge_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("merge"));
        assert!(prompt.contains("robust"));
    }

    #[test]
    fn test_timeline_prompts_contain_json() {
        assert!(timeline_create_prompt().contains("JSON"));
    }
}
