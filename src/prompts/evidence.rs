//! Evidence evaluation mode prompts.
//!
//! Provides prompt templates for the evidence mode operations:
//! assess, probabilistic.

#![allow(clippy::missing_const_for_fn)]

/// Prompt for evidence mode (assess operation).
///
/// Evaluates source credibility and evidence quality.
#[must_use]
pub fn evidence_assess_prompt() -> &'static str {
    r#"Evaluate the credibility and quality of the provided evidence.

Your task is to:
1. Identify distinct pieces of evidence
2. Assess source credibility
3. Evaluate evidence quality
4. Rate overall evidential support

Respond with a JSON object in this exact format:
{
  "evidence_pieces": [
    {
      "summary": "Brief description of the evidence",
      "source_type": "primary|secondary|tertiary|anecdotal",
      "credibility": {
        "expertise": 0.8,
        "objectivity": 0.7,
        "corroboration": 0.6,
        "recency": 0.9,
        "overall": 0.75
      },
      "quality": {
        "relevance": 0.9,
        "strength": 0.7,
        "representativeness": 0.6,
        "overall": 0.73
      }
    }
  ],
  "overall_assessment": {
    "evidential_support": 0.72,
    "key_strengths": ["Strong primary sources"],
    "key_weaknesses": ["Limited corroboration"],
    "gaps": ["What evidence is missing"]
  },
  "confidence_in_conclusion": 0.7
}

Important:
- Be rigorous about source credibility
- Note conflicts between evidence pieces
- Identify what additional evidence would be decisive"#
}

/// Prompt for evidence mode (probabilistic operation).
///
/// Performs Bayesian belief updating.
#[must_use]
pub fn evidence_probabilistic_prompt() -> &'static str {
    r#"Perform Bayesian analysis to update beliefs based on evidence.

Your task is to:
1. Establish prior probability for the hypothesis
2. Assess likelihood of evidence given hypothesis
3. Calculate posterior probability
4. Explain the update

Respond with a JSON object in this exact format:
{
  "hypothesis": "Clear statement of the hypothesis",
  "prior": {
    "probability": 0.5,
    "basis": "Why this prior was chosen"
  },
  "evidence_analysis": [
    {
      "evidence": "Description of evidence",
      "likelihood_if_true": 0.8,
      "likelihood_if_false": 0.2,
      "bayes_factor": 4.0
    }
  ],
  "posterior": {
    "probability": 0.8,
    "calculation": "Explanation of how posterior was derived"
  },
  "belief_update": {
    "direction": "increase|decrease|unchanged",
    "magnitude": "strong|moderate|slight",
    "interpretation": "What this means in plain language"
  },
  "sensitivity": "How sensitive is the posterior to prior assumptions"
}

Important:
- Be explicit about prior assumptions
- Bayes factor = P(E|H) / P(E|¬H)
- Note where estimates are uncertain"#
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
    fn test_evidence_assess_prompt_not_empty() {
        let prompt = evidence_assess_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("credibility"));
    }

    #[test]
    fn test_evidence_probabilistic_prompt_not_empty() {
        let prompt = evidence_probabilistic_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Bayesian"));
        assert!(prompt.contains("prior"));
        assert!(prompt.contains("posterior"));
    }

    #[test]
    fn test_evidence_prompts_contain_json() {
        assert!(evidence_assess_prompt().contains("JSON"));
    }
}
