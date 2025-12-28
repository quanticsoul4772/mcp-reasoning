//! Counterfactual causal analysis mode prompts.
//!
//! Provides prompt templates for the counterfactual mode using Pearl's Ladder.

#![allow(clippy::missing_const_for_fn)]

/// Prompt for counterfactual mode.
///
/// Applies Pearl's Ladder for causal analysis.
#[must_use]
pub fn counterfactual_prompt() -> &'static str {
    r#"Perform counterfactual causal analysis using Pearl's Ladder.

Pearl's Ladder:
1. Association (Seeing): P(Y|X) - What if I observe X?
2. Intervention (Doing): P(Y|do(X)) - What if I make X happen?
3. Counterfactual (Imagining): P(Y_X|X', Y') - What if X had been different?

Your task is to:
1. Identify the causal question being asked
2. Determine which rung of the ladder is relevant
3. Construct the causal model
4. Answer the counterfactual question

Respond with a JSON object in this exact format:
{
  "causal_question": {
    "statement": "Clear statement of the question",
    "ladder_rung": "association|intervention|counterfactual",
    "variables": {
      "cause": "The hypothesized cause",
      "effect": "The outcome of interest",
      "intervention": "What change we're considering"
    }
  },
  "causal_model": {
    "nodes": ["variable names"],
    "edges": [{"from": "X", "to": "Y", "type": "direct|mediated|confounded"}],
    "confounders": ["variables that affect both cause and effect"]
  },
  "analysis": {
    "association_level": {
      "observed_correlation": 0.7,
      "interpretation": "What we can infer from observation"
    },
    "intervention_level": {
      "causal_effect": 0.5,
      "mechanism": "How the intervention would work"
    },
    "counterfactual_level": {
      "scenario": "If X had been different...",
      "outcome": "Y would have been...",
      "confidence": 0.6
    }
  },
  "conclusions": {
    "causal_claim": "Clear statement of causal relationship",
    "strength": "strong|moderate|weak",
    "caveats": ["Important qualifications"],
    "actionable_insight": "What this means for decisions"
  }
}

Important:
- Distinguish correlation from causation
- Identify confounders
- Be explicit about assumptions
- Counterfactuals require specifying the alternative world"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counterfactual_prompt_not_empty() {
        let prompt = counterfactual_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Pearl"));
        assert!(prompt.contains("counterfactual"));
        assert!(prompt.contains("causal"));
    }

    #[test]
    fn test_counterfactual_prompt_contains_json() {
        assert!(counterfactual_prompt().contains("JSON"));
    }
}
