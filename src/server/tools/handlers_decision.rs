use std::sync::Arc;
use std::time::Duration;

use crate::metrics::{MetricEvent, Timer};
use crate::modes::{DecisionMode, EvidenceMode};
use crate::server::requests::{DecisionRequest, EvidenceRequest};
use crate::server::responses::{
    DecisionResponse, EvidenceAssessment, EvidenceResponse, RankedOption, StakeholderMap,
};

use super::DEEP_THINKING;

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
                    Ok(resp) => (
                        DecisionResponse {
                            recommendation: resp
                                .ranking
                                .first()
                                .map(|r| r.option.clone())
                                .unwrap_or_default(),
                            rankings: Some(
                                resp.ranking
                                    .into_iter()
                                    .map(|r| RankedOption {
                                        option: r.option,
                                        score: r.score,
                                        rank: r.rank,
                                    })
                                    .collect(),
                            ),
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: Some(resp.sensitivity_notes),
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!("ERROR: {e}"),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "pairwise" => match mode.pairwise(content, req.session_id).await {
                    Ok(resp) => (
                        DecisionResponse {
                            recommendation: resp
                                .ranking
                                .first()
                                .map(|r| r.option.clone())
                                .unwrap_or_default(),
                            rankings: Some(
                                resp.ranking
                                    .into_iter()
                                    .map(|r| RankedOption {
                                        option: r.option,
                                        score: f64::from(r.wins),
                                        rank: r.rank,
                                    })
                                    .collect(),
                            ),
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!("ERROR: {e}"),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                "topsis" => match mode.topsis(content, req.session_id).await {
                    Ok(resp) => (
                        DecisionResponse {
                            recommendation: resp
                                .ranking
                                .first()
                                .map(|r| r.option.clone())
                                .unwrap_or_default(),
                            rankings: Some(
                                resp.ranking
                                    .into_iter()
                                    .map(|r| RankedOption {
                                        option: r.option,
                                        score: r.closeness,
                                        rank: r.rank,
                                    })
                                    .collect(),
                            ),
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!("ERROR: {e}"),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
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
                            metadata: None,
                        },
                        true,
                    ),
                    Err(e) => (
                        DecisionResponse {
                            recommendation: format!("ERROR: {e}"),
                            rankings: None,
                            stakeholder_map: None,
                            conflicts: None,
                            alignments: None,
                            rationale: None,
                            metadata: None,
                        },
                        false,
                    ),
                },
                _ => (
                    DecisionResponse {
                        recommendation: format!(
                            "ERROR: unknown type: {}",
                            decision_type_for_timeout
                        ),
                        rankings: None,
                        stakeholder_map: None,
                        conflicts: None,
                        alignments: None,
                        rationale: None,
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
                            "ERROR: Tool execution timed out after {}ms",
                            timeout_ms
                        ),
                        rankings: None,
                        stakeholder_map: None,
                        conflicts: None,
                        alignments: None,
                        rationale: None,
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
                            synthesis: Some(format!("ERROR: {e}")),
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
                            synthesis: Some(format!("ERROR: {e}")),
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
