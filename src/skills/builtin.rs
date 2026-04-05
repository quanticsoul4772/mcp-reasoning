//! Built-in skill definitions.
//!
//! These skills compose reasoning tools into higher-level workflows
//! with context passing between steps.

use super::registry::SkillRegistry;
use super::types::{ErrorStrategy, Skill, SkillCategory, SkillStep, StepCondition};

/// Register all built-in skills.
pub fn register_builtin_skills(registry: &mut SkillRegistry) {
    registry.register(deep_code_review());
    registry.register(hypothesis_testing());
    registry.register(systems_analysis());
    registry.register(risk_assessment());
    registry.register(creative_solution());
    registry.register(claim_verification());
}

/// Deep code review: analyst + explorer adversarial review.
fn deep_code_review() -> Skill {
    Skill::new(
        "deep-code-review",
        "Deep Code Review",
        "Multi-perspective code review with bias detection and alternative exploration",
        SkillCategory::CodeQuality,
        vec![
            SkillStep::new("linear")
                .with_description("Understand code structure and purpose")
                .with_output_key("analysis"),
            SkillStep::new("detect")
                .with_operation("biases")
                .with_description("Check for cognitive biases in implementation choices")
                .with_output_key("biases")
                .with_error_strategy(ErrorStrategy::Skip),
            SkillStep::new("detect")
                .with_operation("fallacies")
                .with_description("Check for logical fallacies in design rationale")
                .with_output_key("fallacies")
                .with_error_strategy(ErrorStrategy::Skip),
            SkillStep::new("divergent")
                .with_config(
                    serde_json::json!({"num_perspectives": 4, "challenge_assumptions": true}),
                )
                .with_description("Generate alternative approaches from multiple perspectives")
                .with_output_key("alternatives"),
            SkillStep::new("reflection")
                .with_operation("evaluate")
                .with_description("Synthesize findings into actionable recommendations")
                .with_output_key("recommendations"),
        ],
    )
}

/// Hypothesis testing: branching evidence gathering.
fn hypothesis_testing() -> Skill {
    Skill::new(
        "hypothesis-testing",
        "Hypothesis Testing",
        "Structured hypothesis generation and evidence-based evaluation",
        SkillCategory::Analysis,
        vec![
            SkillStep::new("linear")
                .with_description("Understand the problem and formulate initial understanding")
                .with_output_key("understanding"),
            SkillStep::new("tree")
                .with_operation("create")
                .with_config(serde_json::json!({"num_branches": 3}))
                .with_description("Generate competing hypotheses")
                .with_output_key("hypotheses"),
            SkillStep::new("evidence")
                .with_operation("assess")
                .with_description("Evaluate evidence for each hypothesis")
                .with_output_key("evidence"),
            SkillStep::new("evidence")
                .with_operation("probabilistic")
                .with_description("Update probabilities based on evidence")
                .with_output_key("probabilities")
                .with_condition(StepCondition::IfKeyExists("evidence".to_string())),
            SkillStep::new("reflection")
                .with_operation("evaluate")
                .with_description("Final assessment of most likely hypothesis"),
        ],
    )
}

/// Systems analysis: graph mapping + leverage point analysis.
fn systems_analysis() -> Skill {
    Skill::new(
        "systems-analysis",
        "Systems Analysis",
        "Map system structure, identify feedback loops, and find leverage points",
        SkillCategory::Analysis,
        vec![
            SkillStep::new("graph")
                .with_operation("init")
                .with_description("Initialize system graph with key components")
                .with_output_key("graph"),
            SkillStep::new("graph")
                .with_operation("generate")
                .with_description("Generate additional system connections")
                .with_output_key("connections"),
            SkillStep::new("graph")
                .with_operation("score")
                .with_description("Score nodes by influence and centrality")
                .with_output_key("scores"),
            SkillStep::new("counterfactual")
                .with_description("Analyze what-if scenarios for key leverage points")
                .with_output_key("scenarios"),
            SkillStep::new("graph")
                .with_operation("finalize")
                .with_description("Synthesize system analysis"),
        ],
    )
}

/// Risk assessment: collaborative strategist + analyst review.
fn risk_assessment() -> Skill {
    Skill::new(
        "risk-assessment",
        "Risk Assessment",
        "Comprehensive risk analysis with probability estimation and mitigation planning",
        SkillCategory::Decision,
        vec![
            SkillStep::new("linear")
                .with_description("Identify potential risks and their categories")
                .with_output_key("risks"),
            SkillStep::new("evidence")
                .with_operation("probabilistic")
                .with_description("Estimate risk probabilities")
                .with_output_key("probabilities"),
            SkillStep::new("decision")
                .with_operation("weighted")
                .with_description("Score risks by impact and probability")
                .with_output_key("risk_scores"),
            SkillStep::new("counterfactual")
                .with_description("Analyze scenarios if risks materialize")
                .with_output_key("scenarios"),
            SkillStep::new("timeline")
                .with_operation("create")
                .with_description("Plan risk mitigation timeline")
                .with_error_strategy(ErrorStrategy::Skip),
        ],
    )
}

/// Creative solution: divergent thinking + feasibility check.
fn creative_solution() -> Skill {
    Skill::new(
        "creative-solution",
        "Creative Solution",
        "Generate innovative solutions through divergent thinking with feasibility validation",
        SkillCategory::Research,
        vec![
            SkillStep::new("divergent")
                .with_config(serde_json::json!({"force_rebellion": true, "num_perspectives": 5}))
                .with_description("Generate radical alternative approaches")
                .with_output_key("ideas"),
            SkillStep::new("mcts")
                .with_operation("explore")
                .with_description("Explore promising solution paths")
                .with_output_key("paths"),
            SkillStep::new("decision")
                .with_operation("topsis")
                .with_description("Rank solutions by feasibility and impact")
                .with_output_key("ranking"),
            SkillStep::new("reflection")
                .with_operation("evaluate")
                .with_description("Evaluate top solutions for practical viability"),
        ],
    )
}

/// Claim verification via factored Chain-of-Verification (CoVe).
///
/// Step 3 uses `with_input_map` to pass only the verification questions —
/// NOT the baseline response. This is the critical "factored" property: the
/// model answering verification questions cannot copy errors from its own draft.
/// Source: Dhuliawala et al. ACL 2024 — factored CoVe improves FACTSCORE by ~28%.
fn claim_verification() -> Skill {
    Skill::new(
        "claim-verification",
        "Claim Verification (CoVe)",
        "Factored Chain-of-Verification: generate baseline, plan verification questions, \
         answer each question WITHOUT the baseline in context (factored step prevents copying \
         errors), then produce final verified response. Reduces research confabulation ~28%.",
        SkillCategory::Research,
        vec![
            // Step 1: Generate baseline response
            SkillStep::new("linear")
                .with_description(
                    "Generate initial response — answer the question or draft the claim",
                )
                .with_output_key("baseline"),
            // Step 2: Plan shortform verification questions
            SkillStep::new("linear")
                .with_description(
                    "Plan shortform verification questions for each factual claim in the baseline. \
                     One question per claim (e.g. 'Does source X say Y or Z?'). Do not answer yet.",
                )
                .with_output_key("verification_questions"),
            // Step 3: Factored verification — only receives questions, NOT the baseline
            SkillStep::new("evidence")
                .with_operation("assess")
                .with_description(
                    "Answer each verification question independently. \
                     Do NOT reference the baseline response — this is the factored CoVe step. \
                     Shortform answers only. Tag each answer CONFIRMED or CONTRADICTED.",
                )
                .with_input_map("verification_questions", "questions")
                .with_output_key("verified_answers"),
            // Step 4: Detect inconsistencies between baseline and verified answers
            SkillStep::new("detect")
                .with_operation("fallacies")
                .with_description(
                    "Identify claims in the baseline that conflict with the verified answers",
                )
                .with_output_key("inconsistencies")
                .with_error_strategy(ErrorStrategy::Skip),
            // Step 5: Produce final verified response
            SkillStep::new("reflection")
                .with_operation("evaluate")
                .with_description(
                    "Produce the final answer using verified_answers, correcting any \
                     inconsistencies found. Tag each claim [VERIFIED] or [INFERRED].",
                ),
        ],
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_code_review() {
        let skill = deep_code_review();
        assert_eq!(skill.id, "deep-code-review");
        assert_eq!(skill.category, SkillCategory::CodeQuality);
        assert_eq!(skill.steps.len(), 5);
        assert_eq!(skill.steps[0].mode, "linear");
        assert_eq!(skill.steps[1].mode, "detect");
    }

    #[test]
    fn test_hypothesis_testing() {
        let skill = hypothesis_testing();
        assert_eq!(skill.id, "hypothesis-testing");
        assert_eq!(skill.steps.len(), 5);
        assert!(matches!(
            skill.steps[3].condition,
            StepCondition::IfKeyExists(_)
        ));
    }

    #[test]
    fn test_systems_analysis() {
        let skill = systems_analysis();
        assert_eq!(skill.id, "systems-analysis");
        assert_eq!(skill.steps.len(), 5);
        assert_eq!(skill.steps[0].mode, "graph");
    }

    #[test]
    fn test_risk_assessment() {
        let skill = risk_assessment();
        assert_eq!(skill.id, "risk-assessment");
        assert_eq!(skill.category, SkillCategory::Decision);
        assert_eq!(skill.steps.len(), 5);
        assert_eq!(skill.steps[4].on_error, ErrorStrategy::Skip);
    }

    #[test]
    fn test_creative_solution() {
        let skill = creative_solution();
        assert_eq!(skill.id, "creative-solution");
        assert_eq!(skill.steps.len(), 4);
        assert_eq!(skill.steps[0].mode, "divergent");
        assert!(skill.steps[0].config.is_some());
    }

    #[test]
    fn test_register_builtin_skills() {
        let mut registry = SkillRegistry::default();
        register_builtin_skills(&mut registry);
        assert_eq!(registry.list().len(), 6);
    }

    #[test]
    fn test_claim_verification() {
        let skill = claim_verification();
        assert_eq!(skill.id, "claim-verification");
        assert_eq!(skill.category, SkillCategory::Research);
        assert_eq!(skill.steps.len(), 5);
        // Step 1: baseline generation
        assert_eq!(skill.steps[0].mode, "linear");
        assert_eq!(skill.steps[0].output_key.as_deref(), Some("baseline"));
        // Step 2: plan verification questions
        assert_eq!(skill.steps[1].mode, "linear");
        assert_eq!(
            skill.steps[1].output_key.as_deref(),
            Some("verification_questions")
        );
        // Step 3: factored — only receives verification_questions, NOT baseline
        assert_eq!(skill.steps[2].mode, "evidence");
        assert_eq!(
            skill.steps[2].input_mapping.get("verification_questions"),
            Some(&"questions".to_string())
        );
        assert!(
            !skill.steps[2].input_mapping.contains_key("baseline"),
            "factored CoVe: baseline must NOT be in step 3 input"
        );
        // Step 4: detect inconsistencies (skippable)
        assert_eq!(skill.steps[3].on_error, ErrorStrategy::Skip);
        // Step 5: final verified response
        assert_eq!(skill.steps[4].mode, "reflection");
    }

    #[test]
    fn test_all_skills_have_output_keys() {
        let mut registry = SkillRegistry::default();
        register_builtin_skills(&mut registry);

        for skill in registry.list() {
            // At least some steps should have output keys
            let has_output_keys = skill.steps.iter().any(|s| s.output_key.is_some());
            assert!(
                has_output_keys,
                "Skill '{}' should have at least one step with an output key",
                skill.id
            );
        }
    }

    #[test]
    fn test_all_skills_have_descriptions() {
        let mut registry = SkillRegistry::default();
        register_builtin_skills(&mut registry);

        for skill in registry.list() {
            for (i, step) in skill.steps.iter().enumerate() {
                assert!(
                    step.description.is_some(),
                    "Skill '{}' step {} should have a description",
                    skill.id,
                    i
                );
            }
        }
    }
}
