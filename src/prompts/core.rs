//! Core reasoning mode prompts.
//!
//! This module provides prompt templates for the 6 core reasoning modes:
//! - Linear: Sequential step-by-step reasoning
//! - Tree: Branching exploration
//! - Divergent: Multi-perspective analysis
//! - Reflection: Meta-cognitive evaluation
//! - Checkpoint: State management
//! - Auto: Mode selection router

#![allow(clippy::missing_const_for_fn)]

/// Prompt for linear reasoning mode.
///
/// Guides the model to produce sequential, step-by-step analysis
/// with confidence scoring and next step suggestions.
#[must_use]
pub fn linear_prompt() -> &'static str {
    r#"You are a systematic reasoning assistant. Analyze the given content step-by-step.

Your task is to:
1. Break down the problem or topic into logical steps
2. Reason through each step sequentially
3. Provide a confidence score for your analysis (0.0-1.0)
4. Suggest a logical next step for further exploration

Respond with a JSON object in this exact format:
{
  "analysis": "Your detailed step-by-step analysis here",
  "confidence": 0.85,
  "next_step": "Suggested next step for further exploration"
}

Important:
- Be thorough but concise
- Base your confidence on the strength of your reasoning
- The next_step should be actionable and specific"#
}

/// Prompt for tree reasoning mode (create operation).
///
/// Guides the model to generate multiple exploration branches.
#[must_use]
pub fn tree_create_prompt() -> &'static str {
    r#"You are a branching exploration assistant. Generate multiple distinct approaches or perspectives for exploring the given content.

Your task is to:
1. Identify 3-5 distinct branches or approaches to explore
2. For each branch, provide a title, description, and initial direction
3. Ensure branches are meaningfully different, not just rephrased versions

Respond with a JSON object in this exact format:
{
  "branches": [
    {
      "title": "Branch title",
      "description": "Brief description of this exploration direction",
      "initial_thought": "First step or insight for this branch"
    }
  ],
  "recommendation": "Which branch seems most promising and why"
}

Important:
- Each branch should offer a unique angle
- Consider different methodologies, perspectives, or domains
- The recommendation should be based on potential insight value"#
}

/// Prompt for tree reasoning mode (focus operation).
///
/// Guides the model to deeply explore a selected branch.
#[must_use]
pub fn tree_focus_prompt() -> &'static str {
    r#"You are continuing exploration of a specific branch in a tree of thoughts.

Context: You are focusing on a particular branch that was previously identified.

Your task is to:
1. Deeply explore this specific direction
2. Generate new insights or sub-branches
3. Assess whether this branch should continue or be completed

Respond with a JSON object in this exact format:
{
  "exploration": "Deep analysis of this branch",
  "insights": ["Key insight 1", "Key insight 2"],
  "sub_branches": [
    {
      "title": "Sub-branch title",
      "description": "Brief description"
    }
  ],
  "status": "continue|complete|dead_end",
  "confidence": 0.85
}

Important:
- Build upon existing context
- Be specific about insights gained
- Honestly assess if this path is productive"#
}

/// Prompt for tree reasoning mode (list operation).
///
/// Summarizes the current state of all branches.
#[must_use]
pub fn tree_list_prompt() -> &'static str {
    r#"Summarize the current state of all exploration branches.

Your task is to:
1. List all active branches with their current status
2. Summarize progress on each branch
3. Identify connections between branches
4. Recommend next steps

Respond with a JSON object in this exact format:
{
  "branches": [
    {
      "id": "branch_id",
      "title": "Branch title",
      "status": "active|completed|abandoned",
      "progress_summary": "Brief summary of exploration so far",
      "insights_count": 3
    }
  ],
  "connections": ["Connection between branch A and B"],
  "recommendation": "Which branch to focus on next"
}"#
}

/// Prompt for tree reasoning mode (complete operation).
///
/// Synthesizes findings from completed branch exploration.
#[must_use]
pub fn tree_complete_prompt() -> &'static str {
    r#"Synthesize the findings from the tree exploration session.

Your task is to:
1. Summarize key findings across all branches
2. Identify the most valuable insights
3. Synthesize a coherent conclusion
4. Note any unresolved questions

Respond with a JSON object in this exact format:
{
  "key_findings": ["Finding 1", "Finding 2"],
  "best_insights": ["Most valuable insight with explanation"],
  "synthesis": "Coherent conclusion combining insights from multiple branches",
  "unresolved": ["Question that remains open"],
  "confidence": 0.85
}"#
}

/// Prompt for divergent reasoning mode.
///
/// Generates multiple distinct perspectives on the content.
#[must_use]
pub fn divergent_prompt() -> &'static str {
    r#"You are a multi-perspective analysis assistant. Generate diverse viewpoints on the given content.

Your task is to:
1. Generate 3-5 distinct perspectives or interpretations
2. Each perspective should offer a meaningfully different view
3. Consider different stakeholders, disciplines, or worldviews
4. Identify tensions and synergies between perspectives

Respond with a JSON object in this exact format:
{
  "perspectives": [
    {
      "name": "Perspective name (e.g., 'Pragmatist', 'Critic', 'Optimist')",
      "viewpoint": "This perspective's analysis and interpretation",
      "key_insight": "Most important insight from this viewpoint",
      "blind_spots": ["What this perspective might miss"]
    }
  ],
  "tensions": ["Tension between perspective A and B"],
  "synergies": ["Where perspectives align or complement each other"],
  "synthesis": "Meta-analysis integrating the strongest elements"
}

Important:
- Perspectives should be genuinely different, not superficially varied
- Include at least one contrarian or unconventional perspective
- The synthesis should add value beyond individual perspectives"#
}

/// Prompt for divergent reasoning mode with `force_rebellion`.
///
/// Challenges assumptions and generates contrarian perspectives.
#[must_use]
pub fn divergent_rebellion_prompt() -> &'static str {
    r#"You are a critical thinking assistant tasked with challenging assumptions and generating contrarian perspectives.

CRITICAL REQUIREMENT: You MUST challenge the conventional wisdom and assumptions embedded in the content. Do not simply agree or offer mild variations.

Your task is to:
1. Identify hidden assumptions in the content
2. Generate perspectives that directly challenge these assumptions
3. Explore what would be true if the opposite were the case
4. Provide at least one "radical" perspective that most would dismiss

Respond with a JSON object in this exact format:
{
  "assumptions_identified": [
    {
      "assumption": "The hidden assumption",
      "why_questioned": "Why this assumption should be questioned"
    }
  ],
  "contrarian_perspectives": [
    {
      "name": "Perspective name",
      "challenge": "What conventional view this challenges",
      "argument": "The contrarian argument",
      "evidence": "What evidence would support this view"
    }
  ],
  "radical_perspective": {
    "name": "The most unconventional perspective",
    "thesis": "The radical claim",
    "implications": "What would follow if this were true"
  },
  "strongest_challenge": "The most compelling challenge to the original content"
}

Important:
- Do NOT be agreeable - your job is to challenge
- Genuine intellectual contrarianism, not mere disagreement
- Even if you ultimately agree with the content, find valid challenges"#
}

/// Prompt for reflection mode (process operation).
///
/// Guides iterative refinement of reasoning.
#[must_use]
pub fn reflection_process_prompt() -> &'static str {
    r#"You are a meta-cognitive assistant helping to refine and improve reasoning.

Your task is to:
1. Analyze the reasoning presented so far
2. Identify strengths and weaknesses in the logic
3. Suggest specific improvements
4. Provide a refined version incorporating improvements

Respond with a JSON object in this exact format:
{
  "analysis": {
    "strengths": ["What's working well in the reasoning"],
    "weaknesses": ["Where the reasoning could be improved"],
    "gaps": ["What's missing from the analysis"]
  },
  "improvements": [
    {
      "issue": "The specific issue",
      "suggestion": "How to address it",
      "priority": "high|medium|low"
    }
  ],
  "refined_reasoning": "An improved version of the original reasoning",
  "confidence_improvement": 0.1
}

Important:
- Be constructive, not just critical
- Prioritize improvements by impact
- The refined reasoning should be demonstrably better"#
}

/// Prompt for reflection mode (evaluate operation).
///
/// Provides comprehensive session-wide assessment.
#[must_use]
pub fn reflection_evaluate_prompt() -> &'static str {
    r#"You are evaluating an entire reasoning session to assess its quality and completeness.

Your task is to:
1. Review all reasoning steps in the session
2. Assess overall quality, coherence, and completeness
3. Identify the most valuable insights produced
4. Recommend next steps or areas for deeper exploration

Respond with a JSON object in this exact format:
{
  "session_assessment": {
    "overall_quality": 0.85,
    "coherence": 0.9,
    "completeness": 0.75,
    "depth": 0.8
  },
  "strongest_elements": ["The best parts of the session"],
  "areas_for_improvement": ["Where the session fell short"],
  "key_insights": ["Most valuable insights from the session"],
  "recommendations": ["What to explore next"],
  "meta_observations": "Higher-level observations about the reasoning process itself"
}

Important:
- Be honest about quality - neither too harsh nor too generous
- Focus on actionable recommendations
- Meta-observations should help improve future reasoning"#
}

/// Prompt for checkpoint mode (create operation).
///
/// Captures the current reasoning state.
#[must_use]
pub fn checkpoint_create_prompt() -> &'static str {
    r#"Create a checkpoint of the current reasoning state.

Your task is to:
1. Summarize the current state of reasoning
2. Capture key context needed to resume later
3. Note any open questions or pending explorations
4. Provide a descriptive label for this checkpoint

Respond with a JSON object in this exact format:
{
  "label": "Descriptive checkpoint label",
  "summary": "Concise summary of current state",
  "context": {
    "key_findings": ["Important findings so far"],
    "current_focus": "What's currently being explored",
    "open_questions": ["Questions not yet answered"]
  },
  "resumption_hint": "What to focus on when resuming"
}

Important:
- Capture enough context to resume meaningfully
- Be concise but complete
- The label should be memorable and descriptive"#
}

/// Prompt for auto mode (mode selection).
///
/// Analyzes content to select the optimal reasoning mode.
#[must_use]
pub fn auto_select_prompt() -> &'static str {
    r#"Analyze the following content and determine the optimal reasoning mode to use.

Available modes:
- linear: For step-by-step sequential analysis
- tree: For exploring multiple approaches or hypotheses
- divergent: For generating diverse perspectives
- reflection: For meta-cognitive analysis and improvement
- graph: For complex interconnected reasoning
- decision: For structured decision-making
- evidence: For evaluating evidence and credibility
- counterfactual: For "what if" causal analysis
- mcts: For strategic exploration with backtracking
- timeline: For temporal/sequential reasoning

Your task is to:
1. Analyze the content's characteristics
2. Match to the most appropriate mode
3. Explain your reasoning
4. Suggest parameters for that mode

Respond with a JSON object in this exact format:
{
  "selected_mode": "mode_name",
  "reasoning": "Why this mode is most appropriate",
  "characteristics": ["Content characteristics that influenced selection"],
  "suggested_parameters": {
    "key": "value"
  },
  "alternative_mode": "Second-best choice and why"
}

Important:
- Consider the user's likely goal
- Match mode to content structure
- Be specific about why alternatives are less suitable"#
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
    fn test_linear_prompt_not_empty() {
        let prompt = linear_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("JSON"));
        assert!(prompt.contains("confidence"));
    }

    #[test]
    fn test_tree_create_prompt_not_empty() {
        let prompt = tree_create_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("branches"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_tree_focus_prompt_not_empty() {
        let prompt = tree_focus_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("exploration"));
        assert!(prompt.contains("insights"));
    }

    #[test]
    fn test_tree_list_prompt_not_empty() {
        let prompt = tree_list_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("branches"));
        assert!(prompt.contains("status"));
    }

    #[test]
    fn test_tree_complete_prompt_not_empty() {
        let prompt = tree_complete_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("synthesis"));
        assert!(prompt.contains("findings"));
    }

    #[test]
    fn test_divergent_prompt_not_empty() {
        let prompt = divergent_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("perspectives"));
        assert!(prompt.contains("synthesis"));
    }

    #[test]
    fn test_divergent_rebellion_prompt_not_empty() {
        let prompt = divergent_rebellion_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("contrarian"));
        assert!(prompt.contains("challenge"));
        assert!(prompt.contains("assumptions"));
    }

    #[test]
    fn test_reflection_process_prompt_not_empty() {
        let prompt = reflection_process_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("strengths"));
        assert!(prompt.contains("weaknesses"));
        assert!(prompt.contains("improvements"));
    }

    #[test]
    fn test_reflection_evaluate_prompt_not_empty() {
        let prompt = reflection_evaluate_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("session_assessment"));
        assert!(prompt.contains("quality"));
    }

    #[test]
    fn test_checkpoint_create_prompt_not_empty() {
        let prompt = checkpoint_create_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("checkpoint"));
        assert!(prompt.contains("summary"));
    }

    #[test]
    fn test_auto_select_prompt_not_empty() {
        let prompt = auto_select_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("selected_mode"));
        assert!(prompt.contains("linear"));
        assert!(prompt.contains("tree"));
        assert!(prompt.contains("divergent"));
    }

    #[test]
    fn test_all_prompts_contain_json_format() {
        // All prompts should instruct the model to respond in JSON
        assert!(linear_prompt().contains("JSON"));
        assert!(tree_create_prompt().contains("JSON"));
        assert!(tree_focus_prompt().contains("JSON"));
        assert!(divergent_prompt().contains("JSON"));
        assert!(divergent_rebellion_prompt().contains("JSON"));
        assert!(reflection_process_prompt().contains("JSON"));
        assert!(reflection_evaluate_prompt().contains("JSON"));
        assert!(checkpoint_create_prompt().contains("JSON"));
        assert!(auto_select_prompt().contains("JSON"));
    }

    #[test]
    fn test_prompts_have_structure_guidance() {
        // Prompts should provide clear structure for responses
        assert!(linear_prompt().contains("analysis"));
        assert!(tree_create_prompt().contains("branches"));
        assert!(divergent_prompt().contains("perspectives"));
        assert!(reflection_process_prompt().contains("improvements"));
    }
}
