use std::sync::Arc;
use std::time::Duration;

use crate::metrics::{MetricEvent, Timer};
use crate::modes::{DecisionMode, DecisionValidation, EvidenceMode};
use crate::server::requests::{DecisionRequest, EvidenceRequest};
use crate::server::responses::{
    ComparisonInfo, CriterionInfo, DecisionBreakdown, DecisionResponse, DecisionValidationInfo,
    DistanceInfo, EvidenceAssessment, EvidenceResponse, PairwiseBreakdown, RankedOption,
    StakeholderMap, TopsisBreakdown, TopsisCriterionInfo, WeightedBreakdown,
};

use super::DEEP_THINKING;

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
                    Ok(resp) => (
                        EvidenceResponse {
                            overall_credibility: resp.confidence_in_conclusion,
                            evidence_assessments: Some(
                                resp.evidence_pieces
                                    .into_iter()
                                    .map(|p| EvidenceAssessment {
                                        content: p.summary,
                                        credibility_score: p.credibility.overall,
                                        source_tier: p.source_type.as_str().to_string(),
                                        corroborated_by: None,
                                    })
                                    .collect(),
                            ),
                            posterior: None,
                            prior: None,
                            likelihood_ratio: None,
                            entropy: None,
                            confidence_interval: None,
                            synthesis: Some(format!(
                                "Strengths: {}. Weaknesses: {}. Gaps: {}",
                                resp.overall_assessment.key_strengths.join(", "),
                                resp.overall_assessment.key_weaknesses.join(", "),
                                resp.overall_assessment.gaps.join(", ")
                            )),
                            metadata: None,
                        },
                        true,
                    ),
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
                            metadata: None,
                        },
                        false,
                    ),
                },
                "probabilistic" => match mode.probabilistic(content, req.session_id).await {
                    Ok(resp) => {
                        let likelihood_ratio =
                            resp.evidence_analysis.first().map(|a| a.bayes_factor);
                        (
                            EvidenceResponse {
                                overall_credibility: resp.posterior.probability,
                                evidence_assessments: Some(
                                    resp.evidence_analysis
                                        .into_iter()
                                        .map(|a| EvidenceAssessment {
                                            content: a.evidence,
                                            credibility_score: a.bayes_factor.min(1.0),
                                            source_tier: "computed".to_string(),
                                            corroborated_by: None,
                                        })
                                        .collect(),
                                ),
                                posterior: Some(resp.posterior.probability),
                                prior: Some(resp.prior.probability),
                                likelihood_ratio,
                                entropy: None,
                                confidence_interval: None,
                                synthesis: Some(format!(
                                    "{} ({:?} {:?}). Sensitivity: {}",
                                    resp.belief_update.interpretation,
                                    resp.belief_update.direction,
                                    resp.belief_update.magnitude,
                                    resp.sensitivity
                                )),
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
