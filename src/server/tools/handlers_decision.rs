use std::sync::Arc;
use std::time::Duration;

use crate::metrics::{MetricEvent, Timer};
use crate::modes::{DecisionMode, DecisionValidation, EvidenceAnalysis, EvidenceMode};
use crate::server::requests::{DecisionRequest, EvidenceRequest};
use crate::server::responses::{
    BayesianBreakdown, BayesianEvidence, ComparisonInfo, CredibilityBreakdown, CriterionInfo,
    DecisionBreakdown, DecisionResponse, DecisionValidationInfo, DistanceInfo, EvidenceAssessment,
    EvidenceResponse, EvidenceValidationInfo, PairwiseBreakdown, QualityBreakdown, RankedOption,
    StakeholderMap, TopsisBreakdown, TopsisCriterionInfo, WeightedBreakdown,
};

use super::DEEP_THINKING;

/// Binary Shannon entropy (bits) of a probability `p`. Zero at the extremes.
fn binary_entropy(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        0.0
    } else {
        -(p * p.log2() + (1.0 - p) * (1.0 - p).log2())
    }
}

/// Apply a set of independent Bayes factors to a prior via the odds form of
/// Bayes' rule. Returns `(combined_bayes_factor, posterior)`.
fn bayes_update<'a>(prior: f64, bayes_factors: impl Iterator<Item = &'a f64>) -> (f64, f64) {
    let combined: f64 = bayes_factors.product();
    let p = prior.clamp(0.0, 1.0);
    if p <= 0.0 {
        return (combined, 0.0);
    }
    if p >= 1.0 {
        return (combined, 1.0);
    }
    let prior_odds = p / (1.0 - p);
    let posterior_odds = prior_odds * combined;
    (combined, posterior_odds / (1.0 + posterior_odds))
}

/// Verify the Bayesian arithmetic the model produced. Returns the validation,
/// the combined Bayes factor, and the recomputed posterior.
fn verify_probabilistic(
    prior: f64,
    evidence: &[EvidenceAnalysis],
    stated_posterior: f64,
    direction: &str,
) -> (EvidenceValidationInfo, f64, f64) {
    const PROB_TOL: f64 = 0.05;
    let mut warnings = Vec::new();

    if !(0.0..=1.0).contains(&prior) {
        warnings.push(format!("Prior {prior:.3} is outside [0, 1]"));
    }
    if !(0.0..=1.0).contains(&stated_posterior) {
        warnings.push(format!("Posterior {stated_posterior:.3} is outside [0, 1]"));
    }

    // Each Bayes factor should equal P(E|H) / P(E|¬H).
    for (i, e) in evidence.iter().enumerate() {
        if e.likelihood_if_false.abs() < f64::EPSILON {
            continue; // division undefined; skip the ratio check
        }
        let ratio = e.likelihood_if_true / e.likelihood_if_false;
        if (e.bayes_factor - ratio).abs() > 0.1 * ratio.abs().max(1.0) {
            warnings.push(format!(
                "Evidence {} Bayes factor stated as {:.3} but P(E|H)/P(E|¬H) = {:.3}",
                i + 1,
                e.bayes_factor,
                ratio
            ));
        }
    }

    let (combined_bf, computed_posterior) =
        bayes_update(prior, evidence.iter().map(|e| &e.bayes_factor));

    if (computed_posterior - stated_posterior).abs() > PROB_TOL {
        warnings.push(format!(
            "Stated posterior {stated_posterior:.3} differs from Bayes' rule result {computed_posterior:.3} (prior {prior:.3} × combined Bayes factor {combined_bf:.3})"
        ));
    }

    // Direction should match the sign of (posterior − prior).
    let expected_direction = if stated_posterior - prior > 0.01 {
        "increase"
    } else if prior - stated_posterior > 0.01 {
        "decrease"
    } else {
        "unchanged"
    };
    if direction != expected_direction {
        warnings.push(format!(
            "Belief direction stated as '{direction}' but posterior {stated_posterior:.3} vs prior {prior:.3} implies '{expected_direction}'"
        ));
    }

    (
        EvidenceValidationInfo {
            consistent: warnings.is_empty(),
            warnings,
        },
        combined_bf,
        computed_posterior,
    )
}

/// Map a mode-internal validation result to its JsonSchema response form.
fn to_validation_info(v: DecisionValidation) -> DecisionValidationInfo {
    DecisionValidationInfo {
        consistent: v.consistent,
        warnings: v.warnings,
        ranking_corrected: v.ranking_corrected,
    }
}

/// Serialize a `#[serde(rename_all)]` enum to its string form (e.g. `option_a`).
fn enum_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

impl super::ReasoningServer {
    pub(super) async fn handle_decision(&self, req: DecisionRequest) -> DecisionResponse {
        let timer = Timer::start();
        let mode = DecisionMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let base_content = req
            .question
            .as_deref()
            .or(req.topic.as_deref())
            .or(req.context.as_deref())
            .unwrap_or("");

        // Include user-provided options in the content sent to Claude
        let content = match &req.options {
            Some(opts) if !opts.is_empty() => {
                format!(
                    "Options to evaluate:\n- {}\n\nContext: {}",
                    opts.join("\n- "),
                    base_content
                )
            }
            _ => base_content.to_string(),
        };
        let content = content.as_str();

        let decision_type = req.decision_type.as_deref().unwrap_or("weighted");

        // Apply tool-level timeout to prevent indefinite hangs
        let timeout_ms = self.state.config.timeout_for_thinking_budget(DEEP_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);
        let decision_type_for_timeout = decision_type.to_string();

        let (response, success) = match tokio::time::timeout(timeout_duration, async {
            match decision_type_for_timeout.as_str() {
                "weighted" => match mode.weighted(content, req.session_id).await {
                    Ok(resp) => {
                        let recommendation = resp
                            .ranking
                            .first()
                            .map(|r| r.option.clone())
                            .unwrap_or_default();
                        let rankings = resp
                            .ranking
                            .iter()
                            .map(|r| RankedOption {
                                option: r.option.clone(),
                                score: r.score,
                                rank: r.rank,
                            })
                            .collect();
                        let breakdown = DecisionBreakdown {
                            weighted: Some(WeightedBreakdown {
                                criteria: resp
                                    .criteria
                                    .iter()
                                    .map(|c| CriterionInfo {
                                        name: c.name.clone(),
                                        weight: c.weight,
                                        description: c.description.clone(),
                                    })
                                    .collect(),
                                scores: resp.scores.clone(),
                                weighted_totals: resp.weighted_totals.clone(),
                            }),
                            topsis: None,
                            pairwise: None,
                        };
                        (
                            DecisionResponse {
                                recommendation,
                                rankings: Some(rankings),
                                stakeholder_map: None,
                                conflicts: None,
                                alignments: None,
                                rationale: Some(resp.sensitivity_notes),
                                breakdown: Some(breakdown),
                                validation: Some(to_validation_info(resp.validation)),
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!(
                                "weighted decision failed: {e}. \
                                 Provide at least 2 options and a question/topic. \
                                 Try decision_type='pairwise' for head-to-head comparison."
                            ),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            breakdown: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "pairwise" => match mode.pairwise(content, req.session_id).await {
                    Ok(resp) => {
                        let recommendation = resp
                            .ranking
                            .first()
                            .map(|r| r.option.clone())
                            .unwrap_or_default();
                        let rankings = resp
                            .ranking
                            .iter()
                            .map(|r| RankedOption {
                                option: r.option.clone(),
                                score: f64::from(r.wins),
                                rank: r.rank,
                            })
                            .collect();
                        let breakdown = DecisionBreakdown {
                            weighted: None,
                            topsis: None,
                            pairwise: Some(PairwiseBreakdown {
                                comparisons: resp
                                    .comparisons
                                    .iter()
                                    .map(|c| ComparisonInfo {
                                        option_a: c.option_a.clone(),
                                        option_b: c.option_b.clone(),
                                        preferred: enum_to_string(&c.preferred),
                                        strength: enum_to_string(&c.strength),
                                        reasoning: c.reasoning.clone(),
                                    })
                                    .collect(),
                                consistency_check: resp.consistency_check.clone(),
                            }),
                        };
                        (
                            DecisionResponse {
                                recommendation,
                                rankings: Some(rankings),
                                stakeholder_map: None,
                                conflicts: None,
                                alignments: None,
                                rationale: Some(resp.consistency_check),
                                breakdown: Some(breakdown),
                                validation: Some(to_validation_info(resp.validation)),
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!(
                                "pairwise decision failed: {e}. \
                                 Provide at least 2 options for head-to-head comparison. \
                                 Try decision_type='weighted' for multi-criteria scoring."
                            ),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            breakdown: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "topsis" => match mode.topsis(content, req.session_id).await {
                    Ok(resp) => {
                        let recommendation = resp
                            .ranking
                            .first()
                            .map(|r| r.option.clone())
                            .unwrap_or_default();
                        let rankings = resp
                            .ranking
                            .iter()
                            .map(|r| RankedOption {
                                option: r.option.clone(),
                                score: r.closeness,
                                rank: r.rank,
                            })
                            .collect();
                        let breakdown = DecisionBreakdown {
                            weighted: None,
                            topsis: Some(TopsisBreakdown {
                                criteria: resp
                                    .criteria
                                    .iter()
                                    .map(|c| TopsisCriterionInfo {
                                        name: c.name.clone(),
                                        criterion_type: enum_to_string(&c.criterion_type),
                                        weight: c.weight,
                                    })
                                    .collect(),
                                closeness: resp.relative_closeness.clone(),
                                distances: resp
                                    .distances
                                    .iter()
                                    .map(|(opt, d)| {
                                        (
                                            opt.clone(),
                                            DistanceInfo {
                                                to_ideal: d.to_ideal,
                                                to_anti_ideal: d.to_anti_ideal,
                                            },
                                        )
                                    })
                                    .collect(),
                            }),
                            pairwise: None,
                        };
                        (
                            DecisionResponse {
                                recommendation,
                                rankings: Some(rankings),
                                stakeholder_map: None,
                                conflicts: None,
                                alignments: None,
                                rationale: Some(resp.rationale),
                                breakdown: Some(breakdown),
                                validation: Some(to_validation_info(resp.validation)),
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!(
                                "topsis decision failed: {e}. \
                                 TOPSIS requires numeric criteria weights alongside options. \
                                 Try decision_type='weighted' if criteria weights are unavailable."
                            ),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            breakdown: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "perspectives" => match mode.perspectives(content, req.session_id).await {
                    Ok(resp) => (
                        DecisionResponse {
                            recommendation: resp.balanced_recommendation.option.clone(),
                            rankings: None,
                            stakeholder_map: Some(StakeholderMap {
                                key_players: resp
                                    .stakeholders
                                    .iter()
                                    .filter(|s| {
                                        s.influence_level == crate::modes::InfluenceLevel::High
                                    })
                                    .map(|s| s.name.clone())
                                    .collect(),
                                keep_satisfied: vec![],
                                keep_informed: resp
                                    .stakeholders
                                    .iter()
                                    .filter(|s| {
                                        s.influence_level == crate::modes::InfluenceLevel::Medium
                                    })
                                    .map(|s| s.name.clone())
                                    .collect(),
                                minimal_effort: resp
                                    .stakeholders
                                    .iter()
                                    .filter(|s| {
                                        s.influence_level == crate::modes::InfluenceLevel::Low
                                    })
                                    .map(|s| s.name.clone())
                                    .collect(),
                            }),
                            conflicts: Some(resp.conflicts.into_iter().map(|c| c.issue).collect()),
                            alignments: Some(
                                resp.alignments
                                    .into_iter()
                                    .map(|a| a.common_ground)
                                    .collect(),
                            ),
                            rationale: Some(resp.balanced_recommendation.rationale),
                            breakdown: None,
                            validation: None,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!(
                                "perspectives decision failed: {e}. \
                                 Provide a topic with stakeholders to map. \
                                 Try decision_type='weighted' for options without stakeholder data."
                            ),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            breakdown: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                _ => (
                    DecisionResponse {
                        recommendation: format!(
                            "unknown decision_type '{}'. Valid types: weighted, pairwise, topsis, perspectives.",
                            decision_type_for_timeout
                        ),
                        rankings: None,
                        stakeholder_map: None,
                        conflicts: None,
                        alignments: None,
                        rationale: None,
                        breakdown: None,
                        validation: None,
                        metadata: None,
                    },
                    false,
                ),
            }
        })
        .await
        {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                tracing::error!(
                    tool = "reasoning_decision",
                    timeout_ms = timeout_ms,
                    decision_type = %decision_type,
                    "Tool execution timed out"
                );
                (
                    DecisionResponse {
                        recommendation: format!(
                            "decision timed out after {timeout_ms}ms. \
                             Retry with fewer options or a simpler question."
                        ),
                        rankings: None,
                        stakeholder_map: None,
                        conflicts: None,
                        alignments: None,
                        rationale: None,
                        breakdown: None,
                        validation: None,
                        metadata: None,
                    },
                    false,
                )
            }
        };

        self.state.metrics.record(
            MetricEvent::new("decision", timer.elapsed_ms(), success).with_operation(decision_type),
        );

        response
    }

    pub(super) async fn handle_evidence(&self, req: EvidenceRequest) -> EvidenceResponse {
        let timer = Timer::start();
        let mode = EvidenceMode::new(
            Arc::clone(&self.state.storage),
            Arc::clone(&self.state.client),
        );

        let evidence_type = req.evidence_type.as_deref().unwrap_or("assess");
        let content = req
            .claim
            .as_deref()
            .or(req.hypothesis.as_deref())
            .or(req.context.as_deref())
            .unwrap_or("");

        // Apply tool-level timeout to prevent indefinite hangs
        let timeout_ms = self.state.config.timeout_for_thinking_budget(DEEP_THINKING);
        let timeout_duration = Duration::from_millis(timeout_ms);
        let evidence_type_for_timeout = evidence_type.to_string();

        let (response, success) = match tokio::time::timeout(timeout_duration, async {
            match evidence_type_for_timeout.as_str() {
                "assess" => match mode.assess(content, req.session_id).await {
                    Ok(resp) => {
                        let assessments: Vec<EvidenceAssessment> = resp
                            .evidence_pieces
                            .into_iter()
                            .map(|p| EvidenceAssessment {
                                content: p.summary,
                                credibility_score: p.credibility.overall,
                                source_tier: p.source_type.as_str().to_string(),
                                corroborated_by: None,
                                credibility: Some(CredibilityBreakdown {
                                    expertise: p.credibility.expertise,
                                    objectivity: p.credibility.objectivity,
                                    corroboration: p.credibility.corroboration,
                                    recency: p.credibility.recency,
                                    overall: p.credibility.overall,
                                }),
                                quality: Some(QualityBreakdown {
                                    relevance: p.quality.relevance,
                                    strength: p.quality.strength,
                                    representativeness: p.quality.representativeness,
                                    overall: p.quality.overall,
                                }),
                            })
                            .collect();
                        let a = resp.overall_assessment;
                        let pivot = a.pivot_evidence.clone();
                        (
                            EvidenceResponse {
                                overall_credibility: resp.confidence_in_conclusion,
                                evidence_assessments: Some(assessments),
                                posterior: None,
                                prior: None,
                                likelihood_ratio: None,
                                entropy: None,
                                confidence_interval: None,
                                synthesis: Some(format!(
                                    "Strengths: {}. Weaknesses: {}. Gaps: {}",
                                    a.key_strengths.join(", "),
                                    a.key_weaknesses.join(", "),
                                    a.gaps.join(", ")
                                )),
                                evidential_support: Some(a.evidential_support),
                                pivot_evidence: (!pivot.is_empty()).then_some(pivot),
                                bayesian: None,
                                validation: None,
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(e) => (
                        EvidenceResponse {
                            overall_credibility: 0.0,
                            evidence_assessments: None,
                            posterior: None,
                            prior: None,
                            likelihood_ratio: None,
                            entropy: None,
                            confidence_interval: None,
                            synthesis: Some(format!(
                                "evidence assess failed: {e}. \
                                 Provide a claim or hypothesis to evaluate. \
                                 Try evidence_type='probabilistic' for Bayesian belief updates."
                            )),
                            evidential_support: None,
                            pivot_evidence: None,
                            bayesian: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "probabilistic" => match mode.probabilistic(content, req.session_id).await {
                    Ok(resp) => {
                        let direction = enum_to_string(&resp.belief_update.direction);
                        let magnitude = enum_to_string(&resp.belief_update.magnitude);
                        let (validation, combined_bf, computed_posterior) = verify_probabilistic(
                            resp.prior.probability,
                            &resp.evidence_analysis,
                            resp.posterior.probability,
                            &direction,
                        );
                        let bayes_evidence: Vec<BayesianEvidence> = resp
                            .evidence_analysis
                            .iter()
                            .map(|a| BayesianEvidence {
                                evidence: a.evidence.clone(),
                                likelihood_if_true: a.likelihood_if_true,
                                likelihood_if_false: a.likelihood_if_false,
                                bayes_factor: a.bayes_factor,
                            })
                            .collect();
                        let bayesian = BayesianBreakdown {
                            prior: resp.prior.probability,
                            prior_basis: resp.prior.basis,
                            evidence: bayes_evidence,
                            combined_bayes_factor: combined_bf,
                            stated_posterior: resp.posterior.probability,
                            computed_posterior,
                            posterior_calculation: resp.posterior.calculation,
                            belief_direction: direction,
                            belief_magnitude: magnitude,
                            interpretation: resp.belief_update.interpretation.clone(),
                            sensitivity: resp.sensitivity.clone(),
                        };
                        (
                            EvidenceResponse {
                                overall_credibility: resp.posterior.probability,
                                evidence_assessments: None,
                                posterior: Some(resp.posterior.probability),
                                prior: Some(resp.prior.probability),
                                // The combined Bayes factor across all evidence,
                                // not just the first piece.
                                likelihood_ratio: Some(combined_bf),
                                entropy: Some(binary_entropy(resp.posterior.probability)),
                                confidence_interval: None,
                                synthesis: Some(format!(
                                    "{}. Sensitivity: {}",
                                    resp.belief_update.interpretation, resp.sensitivity
                                )),
                                evidential_support: None,
                                pivot_evidence: None,
                                bayesian: Some(bayesian),
                                validation: Some(validation),
                                metadata: None,
                            },
                            true,
                        )
                    }
                    Err(e) => (
                        EvidenceResponse {
                            overall_credibility: 0.0,
                            evidence_assessments: None,
                            posterior: None,
                            prior: None,
                            likelihood_ratio: None,
                            entropy: None,
                            confidence_interval: None,
                            synthesis: Some(format!(
                                "probabilistic evidence failed: {e}. \
                                 Provide a hypothesis with a prior probability and evidence. \
                                 Try evidence_type='assess' for qualitative credibility scoring."
                            )),
                            evidential_support: None,
                            pivot_evidence: None,
                            bayesian: None,
                            validation: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                _ => (
                    EvidenceResponse {
                        overall_credibility: 0.0,
                        evidence_assessments: None,
                        posterior: None,
                        prior: None,
                        likelihood_ratio: None,
                        entropy: None,
                        confidence_interval: None,
                        synthesis: Some(format!(
                            "Unknown evidence type: {}",
                            evidence_type_for_timeout
                        )),
                        evidential_support: None,
                        pivot_evidence: None,
                        bayesian: None,
                        validation: None,
                        metadata: None,
                    },
                    false,
                ),
            }
        })
        .await
        {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                tracing::error!(
                    tool = "reasoning_evidence",
                    timeout_ms = timeout_ms,
                    evidence_type = %evidence_type,
                    "Tool execution timed out"
                );
                (
                    EvidenceResponse {
                        overall_credibility: 0.0,
                        evidence_assessments: None,
                        posterior: None,
                        prior: None,
                        likelihood_ratio: None,
                        entropy: None,
                        confidence_interval: None,
                        synthesis: Some(format!("Tool execution timed out after {}ms", timeout_ms)),
                        evidential_support: None,
                        pivot_evidence: None,
                        bayesian: None,
                        validation: None,
                        metadata: None,
                    },
                    false,
                )
            }
        };

        self.state.metrics.record(
            MetricEvent::new("evidence", timer.elapsed_ms(), success).with_operation(evidence_type),
        );

        response
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]
mod bayes_tests {
    use super::{bayes_update, binary_entropy, verify_probabilistic};
    use crate::modes::EvidenceAnalysis;

    fn ev(lt: f64, lf: f64, bf: f64) -> EvidenceAnalysis {
        EvidenceAnalysis {
            evidence: "e".to_string(),
            likelihood_if_true: lt,
            likelihood_if_false: lf,
            bayes_factor: bf,
        }
    }

    #[test]
    fn test_binary_entropy_bounds() {
        assert_eq!(binary_entropy(0.0), 0.0);
        assert_eq!(binary_entropy(1.0), 0.0);
        assert!((binary_entropy(0.5) - 1.0).abs() < 1e-9);
        // Symmetric.
        assert!((binary_entropy(0.1) - binary_entropy(0.9)).abs() < 1e-9);
    }

    #[test]
    fn test_bayes_update_disease_example() {
        // prior 0.01, BF 10 → posterior ≈ 0.0918
        let (combined, posterior) = bayes_update(0.01, [10.0].iter());
        assert!((combined - 10.0).abs() < 1e-9);
        assert!((posterior - 0.0918).abs() < 0.001, "got {posterior}");
    }

    #[test]
    fn test_bayes_update_combines_factors() {
        let (combined, _) = bayes_update(0.5, [2.0, 3.0].iter());
        assert!((combined - 6.0).abs() < 1e-9);
    }

    #[test]
    fn test_verify_consistent_when_math_checks_out() {
        let (v, _, computed) =
            verify_probabilistic(0.01, &[ev(0.9, 0.09, 10.0)], 0.092, "increase");
        assert!(v.consistent, "warnings: {:?}", v.warnings);
        assert!((computed - 0.0918).abs() < 0.001);
    }

    #[test]
    fn test_verify_flags_posterior_inconsistent_with_bayes_rule() {
        // Base-rate neglect: model claims 0.5 when prior 0.01 × BF 10 → 0.092.
        let (v, _, _) = verify_probabilistic(0.01, &[ev(0.9, 0.09, 10.0)], 0.5, "increase");
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("Bayes' rule result")));
    }

    #[test]
    fn test_verify_flags_bad_bayes_factor() {
        // BF stated 4.0 but P(E|H)/P(E|¬H) = 0.8/0.2 = 4.0 is correct; use a wrong one.
        let (v, _, _) = verify_probabilistic(0.5, &[ev(0.8, 0.2, 9.0)], 0.9, "increase");
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("Bayes factor stated")));
    }

    #[test]
    fn test_verify_flags_direction_mismatch() {
        // Posterior (0.092) < prior would be decrease, but here posterior > prior
        // yet direction says "decrease".
        let (v, _, _) = verify_probabilistic(0.01, &[ev(0.9, 0.09, 10.0)], 0.092, "decrease");
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("direction")));
    }
}
