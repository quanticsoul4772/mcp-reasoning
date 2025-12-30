//! Bias and fallacy detection mode prompts.
//!
//! Provides prompt templates for the detect mode operations:
//! biases, fallacies.

#![allow(clippy::missing_const_for_fn)]

/// Prompt for detect mode (biases operation).
///
/// Detects cognitive biases in reasoning.
#[must_use]
pub fn detect_biases_prompt() -> &'static str {
    r#"Analyze the content for cognitive biases.

Your task is to:
1. Identify potential cognitive biases present
2. Explain how each bias manifests
3. Assess the severity and impact
4. Suggest debiasing strategies

Respond with a JSON object in this exact format:
{
  "biases_detected": [
    {
      "bias": "Name of the bias (e.g., Confirmation Bias)",
      "evidence": "Specific text or reasoning that shows this bias",
      "severity": "low|medium|high",
      "impact": "How this bias affects the reasoning",
      "debiasing": "Strategy to counteract this bias"
    }
  ],
  "overall_assessment": {
    "bias_count": 3,
    "most_severe": "The most impactful bias",
    "reasoning_quality": 0.7
  },
  "debiased_version": "A brief debiased restatement of the main argument"
}

Common biases to check:
- Confirmation bias, Anchoring, Availability heuristic
- Sunk cost fallacy, Bandwagon effect, Dunning-Kruger
- Hindsight bias, Overconfidence, Status quo bias

Important:
- Only flag genuine biases with clear evidence
- Distinguish bias from reasonable heuristics
- Debiasing suggestions should be practical"#
}

/// Prompt for detect mode (fallacies operation).
///
/// Detects logical fallacies in arguments.
#[must_use]
pub fn detect_fallacies_prompt() -> &'static str {
    r#"Analyze the content for logical fallacies.

Your task is to:
1. Identify logical fallacies in the argument
2. Quote the specific passage containing the fallacy
3. Explain why it's a fallacy
4. Suggest how to fix or strengthen the argument

Respond with a JSON object in this exact format:
{
  "fallacies_detected": [
    {
      "fallacy": "Name of the fallacy (e.g., Ad Hominem)",
      "category": "formal|informal|relevance|presumption",
      "passage": "The specific text containing the fallacy",
      "explanation": "Why this is a fallacy",
      "correction": "How to fix or strengthen this argument"
    }
  ],
  "argument_structure": {
    "premises": ["Identified premises"],
    "conclusion": "The main conclusion",
    "validity": "valid|invalid|partially_valid"
  },
  "overall_assessment": {
    "fallacy_count": 2,
    "argument_strength": 0.6,
    "most_critical": "The fallacy most damaging to the argument"
  }
}

Common fallacies to check:
- Ad hominem, Straw man, False dichotomy
- Appeal to authority, Slippery slope, Circular reasoning
- Hasty generalization, Red herring, Tu quoque

Important:
- Only flag genuine fallacies, not just weak arguments
- Provide constructive corrections
- Consider context - some 'fallacies' may be valid in context"#
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
    fn test_detect_biases_prompt_not_empty() {
        let prompt = detect_biases_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("biases"));
        assert!(prompt.contains("Confirmation"));
    }

    #[test]
    fn test_detect_fallacies_prompt_not_empty() {
        let prompt = detect_fallacies_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("fallacies"));
        assert!(prompt.contains("Ad hominem"));
    }

    #[test]
    fn test_detect_prompts_contain_json() {
        assert!(detect_biases_prompt().contains("JSON"));
        assert!(detect_fallacies_prompt().contains("JSON"));
    }
}
