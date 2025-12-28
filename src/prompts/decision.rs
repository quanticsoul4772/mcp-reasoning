//! Decision analysis mode prompts.
//!
//! Provides prompt templates for the decision mode operations:
//! weighted, pairwise, topsis, perspectives.

#![allow(clippy::missing_const_for_fn)]

/// Prompt for decision mode (weighted operation).
///
/// Performs weighted multi-criteria decision analysis.
#[must_use]
pub fn decision_weighted_prompt() -> &'static str {
    r#"Perform a weighted multi-criteria decision analysis.

Your task is to:
1. Identify the options being compared
2. Define evaluation criteria with weights
3. Score each option on each criterion
4. Calculate weighted totals and rank options

Respond with a JSON object in this exact format:
{
  "options": ["Option A", "Option B", "Option C"],
  "criteria": [
    {
      "name": "Criterion name",
      "weight": 0.3,
      "description": "What this criterion measures"
    }
  ],
  "scores": {
    "Option A": {"criterion1": 0.8, "criterion2": 0.6},
    "Option B": {"criterion1": 0.6, "criterion2": 0.9}
  },
  "weighted_totals": {
    "Option A": 0.72,
    "Option B": 0.78
  },
  "ranking": [
    {"option": "Option B", "score": 0.78, "rank": 1},
    {"option": "Option A", "score": 0.72, "rank": 2}
  ],
  "sensitivity_notes": "How sensitive is the ranking to weight changes"
}

Important:
- Weights must sum to 1.0
- Scores should be 0.0-1.0
- Note any close calls or sensitivity concerns"#
}

/// Prompt for decision mode (pairwise operation).
///
/// Compares options in pairs.
#[must_use]
pub fn decision_pairwise_prompt() -> &'static str {
    r#"Perform pairwise comparison of decision options.

Your task is to:
1. Compare each pair of options directly
2. Determine which is preferred and why
3. Assess preference strength
4. Derive overall ranking from pairwise results

Respond with a JSON object in this exact format:
{
  "comparisons": [
    {
      "option_a": "Option A",
      "option_b": "Option B",
      "preferred": "option_a|option_b|tie",
      "strength": "strong|moderate|slight",
      "reasoning": "Why this option is preferred"
    }
  ],
  "pairwise_matrix": {
    "Option A vs Option B": 1,
    "Option A vs Option C": 0
  },
  "ranking": [
    {"option": "Option A", "wins": 2, "rank": 1}
  ],
  "consistency_check": "Are pairwise preferences transitive?"
}

Important:
- Be consistent in preferences (if A>B and B>C, then A>C)
- Note any intransitivities
- Strength of preference matters"#
}

/// Prompt for decision mode (topsis operation).
///
/// Applies TOPSIS method for decision analysis.
#[must_use]
pub fn decision_topsis_prompt() -> &'static str {
    r#"Apply TOPSIS (Technique for Order of Preference by Similarity to Ideal Solution).

Your task is to:
1. Define criteria and classify as benefit or cost
2. Score options on each criterion
3. Calculate distance to ideal and anti-ideal solutions
4. Rank by relative closeness to ideal

Respond with a JSON object in this exact format:
{
  "criteria": [
    {
      "name": "Criterion name",
      "type": "benefit|cost",
      "weight": 0.25
    }
  ],
  "decision_matrix": {
    "Option A": [0.8, 0.6, 0.7],
    "Option B": [0.6, 0.9, 0.5]
  },
  "ideal_solution": [0.8, 0.9, 0.7],
  "anti_ideal_solution": [0.6, 0.6, 0.5],
  "distances": {
    "Option A": {"to_ideal": 0.15, "to_anti_ideal": 0.12},
    "Option B": {"to_ideal": 0.12, "to_anti_ideal": 0.15}
  },
  "relative_closeness": {
    "Option A": 0.44,
    "Option B": 0.56
  },
  "ranking": [
    {"option": "Option B", "closeness": 0.56, "rank": 1}
  ]
}

Important:
- Benefit criteria: higher is better
- Cost criteria: lower is better
- Relative closeness: 0-1, higher is better"#
}

/// Prompt for decision mode (perspectives operation).
///
/// Analyzes decision from multiple stakeholder perspectives.
#[must_use]
pub fn decision_perspectives_prompt() -> &'static str {
    r#"Analyze the decision from multiple stakeholder perspectives.

Your task is to:
1. Identify relevant stakeholders
2. Analyze how each stakeholder views the options
3. Identify conflicts and alignments
4. Suggest how to balance competing interests

Respond with a JSON object in this exact format:
{
  "stakeholders": [
    {
      "name": "Stakeholder name",
      "interests": ["What they care about"],
      "preferred_option": "Their preferred choice",
      "concerns": ["Their main concerns"],
      "influence_level": "high|medium|low"
    }
  ],
  "conflicts": [
    {
      "between": ["Stakeholder A", "Stakeholder B"],
      "issue": "What they disagree about",
      "severity": "high|medium|low"
    }
  ],
  "alignments": [
    {
      "stakeholders": ["Stakeholder A", "Stakeholder C"],
      "common_ground": "What they agree on"
    }
  ],
  "balanced_recommendation": {
    "option": "Recommended choice",
    "rationale": "How this balances interests",
    "mitigation": "How to address concerns of those who disagree"
  }
}

Important:
- Consider both obvious and hidden stakeholders
- Balance doesn't mean everyone gets what they want
- Some conflicts may be irreconcilable"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_weighted_prompt_not_empty() {
        let prompt = decision_weighted_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("weighted"));
        assert!(prompt.contains("criteria"));
    }

    #[test]
    fn test_decision_pairwise_prompt_not_empty() {
        let prompt = decision_pairwise_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("pairwise"));
    }

    #[test]
    fn test_decision_topsis_prompt_not_empty() {
        let prompt = decision_topsis_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("TOPSIS"));
        assert!(prompt.contains("ideal"));
    }

    #[test]
    fn test_decision_perspectives_prompt_not_empty() {
        let prompt = decision_perspectives_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("stakeholder"));
    }

    #[test]
    fn test_decision_prompts_contain_json() {
        assert!(decision_weighted_prompt().contains("JSON"));
    }
}
