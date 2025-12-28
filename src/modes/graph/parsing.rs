//! Graph mode parsing utilities.
//!
//! Parsing functions for graph operation responses.

use crate::error::ModeError;

use super::types::{
    ChildNode, ComplexityLevel, ExpansionDirection, FrontierNodeInfo, GraphConclusion,
    GraphMetadata, GraphMetrics, GraphPath, GraphStructure, IntegrationNotes, NodeAssessment,
    NodeCritique, NodeRecommendation, NodeRelationship, NodeScores, NodeType, PruneCandidate,
    PruneImpact, PruneReason, RefinedNode, RootNode, SessionQuality, SuggestedAction, SynthesisNode,
};

// ============================================================================
// Basic Helpers
// ============================================================================

pub fn get_str(json: &serde_json::Value, field: &str) -> Result<String, ModeError> {
    json.get(field)
        .and_then(serde_json::Value::as_str)
        .map(String::from)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })
}

pub fn get_f64(json: &serde_json::Value, field: &str) -> Result<f64, ModeError> {
    json.get(field)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })
}

// JSON numbers are u64; truncation is acceptable for counts/indices that fit in u32
#[allow(clippy::cast_possible_truncation)]
pub fn get_u32(json: &serde_json::Value, field: &str) -> Result<u32, ModeError> {
    json.get(field)
        .and_then(serde_json::Value::as_u64)
        .map(|v| v as u32)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })
}

pub fn get_string_array(json: &serde_json::Value, field: &str) -> Result<Vec<String>, ModeError> {
    Ok(json
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: field.to_string(),
        })?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect())
}

// ============================================================================
// Enum Parsers
// ============================================================================

pub fn parse_node_type(s: &str) -> Result<NodeType, ModeError> {
    match s.to_lowercase().as_str() {
        "root" => Ok(NodeType::Root),
        "reasoning" => Ok(NodeType::Reasoning),
        "evidence" => Ok(NodeType::Evidence),
        "hypothesis" => Ok(NodeType::Hypothesis),
        "conclusion" => Ok(NodeType::Conclusion),
        "synthesis" => Ok(NodeType::Synthesis),
        "refined" => Ok(NodeType::Refined),
        _ => Err(ModeError::InvalidValue {
            field: "type".to_string(),
            reason: format!("unknown node type: {s}"),
        }),
    }
}

pub fn parse_node_relationship(s: &str) -> Result<NodeRelationship, ModeError> {
    match s.to_lowercase().as_str() {
        "elaborates" => Ok(NodeRelationship::Elaborates),
        "supports" => Ok(NodeRelationship::Supports),
        "challenges" => Ok(NodeRelationship::Challenges),
        "synthesizes" => Ok(NodeRelationship::Synthesizes),
        _ => Err(ModeError::InvalidValue {
            field: "relationship".to_string(),
            reason: format!("unknown relationship: {s}"),
        }),
    }
}

pub fn parse_complexity(s: &str) -> Result<ComplexityLevel, ModeError> {
    match s.to_lowercase().as_str() {
        "low" => Ok(ComplexityLevel::Low),
        "medium" => Ok(ComplexityLevel::Medium),
        "high" => Ok(ComplexityLevel::High),
        _ => Err(ModeError::InvalidValue {
            field: "complexity".to_string(),
            reason: format!("must be low, medium, or high, got {s}"),
        }),
    }
}

pub fn parse_recommendation(s: &str) -> Result<NodeRecommendation, ModeError> {
    match s.to_lowercase().as_str() {
        "expand" => Ok(NodeRecommendation::Expand),
        "keep" => Ok(NodeRecommendation::Keep),
        "prune" => Ok(NodeRecommendation::Prune),
        _ => Err(ModeError::InvalidValue {
            field: "recommendation".to_string(),
            reason: format!("must be expand, keep, or prune, got {s}"),
        }),
    }
}

pub fn parse_prune_reason(s: &str) -> Result<PruneReason, ModeError> {
    match s.to_lowercase().as_str() {
        "low_score" => Ok(PruneReason::LowScore),
        "redundant" => Ok(PruneReason::Redundant),
        "dead_end" => Ok(PruneReason::DeadEnd),
        "off_topic" => Ok(PruneReason::OffTopic),
        _ => Err(ModeError::InvalidValue {
            field: "reason".to_string(),
            reason: format!("unknown prune reason: {s}"),
        }),
    }
}

pub fn parse_prune_impact(s: &str) -> Result<PruneImpact, ModeError> {
    match s.to_lowercase().as_str() {
        "none" => Ok(PruneImpact::None),
        "minor" => Ok(PruneImpact::Minor),
        "moderate" => Ok(PruneImpact::Moderate),
        _ => Err(ModeError::InvalidValue {
            field: "impact".to_string(),
            reason: format!("must be none, minor, or moderate, got {s}"),
        }),
    }
}

pub fn parse_suggested_action(s: &str) -> Result<SuggestedAction, ModeError> {
    match s.to_lowercase().as_str() {
        "expand" => Ok(SuggestedAction::Expand),
        "refine" => Ok(SuggestedAction::Refine),
        "aggregate" => Ok(SuggestedAction::Aggregate),
        _ => Err(ModeError::InvalidValue {
            field: "suggested_action".to_string(),
            reason: format!("unknown action: {s}"),
        }),
    }
}

// ============================================================================
// Init Parsers
// ============================================================================

pub fn parse_root(json: &serde_json::Value) -> Result<RootNode, ModeError> {
    let r = json
        .get("root")
        .ok_or_else(|| ModeError::MissingField {
            field: "root".to_string(),
        })?;

    let type_str = get_str(r, "type")?;
    let node_type = parse_node_type(&type_str)?;

    Ok(RootNode {
        id: get_str(r, "id")?,
        content: get_str(r, "content")?,
        score: get_f64(r, "score")?,
        node_type,
    })
}

pub fn parse_expansion_directions(
    json: &serde_json::Value,
) -> Result<Vec<ExpansionDirection>, ModeError> {
    let arr = json
        .get("expansion_directions")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "expansion_directions".to_string(),
        })?;

    arr.iter()
        .map(|d| {
            Ok(ExpansionDirection {
                direction: get_str(d, "direction")?,
                potential: get_f64(d, "potential")?,
            })
        })
        .collect()
}

pub fn parse_graph_metadata(json: &serde_json::Value) -> Result<GraphMetadata, ModeError> {
    let m = json
        .get("graph_metadata")
        .ok_or_else(|| ModeError::MissingField {
            field: "graph_metadata".to_string(),
        })?;

    let complexity_str = get_str(m, "complexity")?;
    let complexity = parse_complexity(&complexity_str)?;

    Ok(GraphMetadata {
        complexity,
        estimated_depth: get_u32(m, "estimated_depth")?,
    })
}

// ============================================================================
// Generate Parsers
// ============================================================================

pub fn parse_children(json: &serde_json::Value) -> Result<Vec<ChildNode>, ModeError> {
    let arr = json
        .get("children")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "children".to_string(),
        })?;

    arr.iter()
        .map(|c| {
            let type_str = get_str(c, "type")?;
            let node_type = parse_node_type(&type_str)?;

            let rel_str = get_str(c, "relationship")?;
            let relationship = parse_node_relationship(&rel_str)?;

            Ok(ChildNode {
                id: get_str(c, "id")?,
                content: get_str(c, "content")?,
                score: get_f64(c, "score")?,
                node_type,
                relationship,
            })
        })
        .collect()
}

// ============================================================================
// Score Parsers
// ============================================================================

pub fn parse_node_scores(json: &serde_json::Value) -> Result<NodeScores, ModeError> {
    let s = json
        .get("scores")
        .ok_or_else(|| ModeError::MissingField {
            field: "scores".to_string(),
        })?;

    Ok(NodeScores {
        relevance: get_f64(s, "relevance")?,
        coherence: get_f64(s, "coherence")?,
        depth: get_f64(s, "depth")?,
        novelty: get_f64(s, "novelty")?,
        overall: get_f64(s, "overall")?,
    })
}

pub fn parse_node_assessment(json: &serde_json::Value) -> Result<NodeAssessment, ModeError> {
    let a = json
        .get("assessment")
        .ok_or_else(|| ModeError::MissingField {
            field: "assessment".to_string(),
        })?;

    let rec_str = get_str(a, "recommendation")?;
    let recommendation = parse_recommendation(&rec_str)?;

    Ok(NodeAssessment {
        strengths: get_string_array(a, "strengths")?,
        weaknesses: get_string_array(a, "weaknesses")?,
        recommendation,
    })
}

// ============================================================================
// Aggregate Parsers
// ============================================================================

pub fn parse_synthesis(json: &serde_json::Value) -> Result<SynthesisNode, ModeError> {
    let s = json
        .get("synthesis")
        .ok_or_else(|| ModeError::MissingField {
            field: "synthesis".to_string(),
        })?;

    let type_str = get_str(s, "type")?;
    let node_type = parse_node_type(&type_str)?;

    Ok(SynthesisNode {
        id: get_str(s, "id")?,
        content: get_str(s, "content")?,
        score: get_f64(s, "score")?,
        node_type,
    })
}

pub fn parse_integration_notes(json: &serde_json::Value) -> Result<IntegrationNotes, ModeError> {
    let n = json
        .get("integration_notes")
        .ok_or_else(|| ModeError::MissingField {
            field: "integration_notes".to_string(),
        })?;

    Ok(IntegrationNotes {
        common_themes: get_string_array(n, "common_themes")?,
        complementary_aspects: get_string_array(n, "complementary_aspects")?,
        resolved_contradictions: get_string_array(n, "resolved_contradictions")?,
    })
}

// ============================================================================
// Refine Parsers
// ============================================================================

pub fn parse_critique(json: &serde_json::Value) -> Result<NodeCritique, ModeError> {
    let c = json
        .get("critique")
        .ok_or_else(|| ModeError::MissingField {
            field: "critique".to_string(),
        })?;

    Ok(NodeCritique {
        issues: get_string_array(c, "issues")?,
        missing_elements: get_string_array(c, "missing_elements")?,
        unclear_aspects: get_string_array(c, "unclear_aspects")?,
    })
}

pub fn parse_refined_node(json: &serde_json::Value) -> Result<RefinedNode, ModeError> {
    let r = json
        .get("refined_node")
        .ok_or_else(|| ModeError::MissingField {
            field: "refined_node".to_string(),
        })?;

    let type_str = get_str(r, "type")?;
    let node_type = parse_node_type(&type_str)?;

    Ok(RefinedNode {
        id: get_str(r, "id")?,
        content: get_str(r, "content")?,
        score: get_f64(r, "score")?,
        node_type,
    })
}

// ============================================================================
// Prune Parsers
// ============================================================================

pub fn parse_prune_candidates(json: &serde_json::Value) -> Result<Vec<PruneCandidate>, ModeError> {
    let arr = json
        .get("prune_candidates")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "prune_candidates".to_string(),
        })?;

    arr.iter()
        .map(|c| {
            let reason_str = get_str(c, "reason")?;
            let reason = parse_prune_reason(&reason_str)?;

            let impact_str = get_str(c, "impact")?;
            let impact = parse_prune_impact(&impact_str)?;

            Ok(PruneCandidate {
                node_id: get_str(c, "node_id")?,
                reason,
                confidence: get_f64(c, "confidence")?,
                impact,
            })
        })
        .collect()
}

// ============================================================================
// Finalize Parsers
// ============================================================================

pub fn parse_best_paths(json: &serde_json::Value) -> Result<Vec<GraphPath>, ModeError> {
    let arr = json
        .get("best_paths")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "best_paths".to_string(),
        })?;

    arr.iter()
        .map(|p| {
            Ok(GraphPath {
                path: get_string_array(p, "path")?,
                path_quality: get_f64(p, "path_quality")?,
                key_insight: get_str(p, "key_insight")?,
            })
        })
        .collect()
}

pub fn parse_conclusions(json: &serde_json::Value) -> Result<Vec<GraphConclusion>, ModeError> {
    let arr = json
        .get("conclusions")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "conclusions".to_string(),
        })?;

    arr.iter()
        .map(|c| {
            Ok(GraphConclusion {
                conclusion: get_str(c, "conclusion")?,
                confidence: get_f64(c, "confidence")?,
                supporting_nodes: get_string_array(c, "supporting_nodes")?,
            })
        })
        .collect()
}

pub fn parse_session_quality(json: &serde_json::Value) -> Result<SessionQuality, ModeError> {
    let q = json
        .get("session_quality")
        .ok_or_else(|| ModeError::MissingField {
            field: "session_quality".to_string(),
        })?;

    Ok(SessionQuality {
        depth_achieved: get_f64(q, "depth_achieved")?,
        breadth_achieved: get_f64(q, "breadth_achieved")?,
        coherence: get_f64(q, "coherence")?,
        overall: get_f64(q, "overall")?,
    })
}

// ============================================================================
// State Parsers
// ============================================================================

pub fn parse_structure(json: &serde_json::Value) -> Result<GraphStructure, ModeError> {
    let s = json
        .get("structure")
        .ok_or_else(|| ModeError::MissingField {
            field: "structure".to_string(),
        })?;

    Ok(GraphStructure {
        total_nodes: get_u32(s, "total_nodes")?,
        depth: get_u32(s, "depth")?,
        branches: get_u32(s, "branches")?,
        pruned_count: get_u32(s, "pruned_count")?,
    })
}

pub fn parse_frontiers(json: &serde_json::Value) -> Result<Vec<FrontierNodeInfo>, ModeError> {
    let arr = json
        .get("frontiers")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ModeError::MissingField {
            field: "frontiers".to_string(),
        })?;

    arr.iter()
        .map(|f| {
            let action_str = get_str(f, "suggested_action")?;
            let suggested_action = parse_suggested_action(&action_str)?;

            Ok(FrontierNodeInfo {
                node_id: get_str(f, "node_id")?,
                potential: get_f64(f, "potential")?,
                suggested_action,
            })
        })
        .collect()
}

pub fn parse_metrics(json: &serde_json::Value) -> Result<GraphMetrics, ModeError> {
    let m = json
        .get("metrics")
        .ok_or_else(|| ModeError::MissingField {
            field: "metrics".to_string(),
        })?;

    Ok(GraphMetrics {
        average_score: get_f64(m, "average_score")?,
        max_score: get_f64(m, "max_score")?,
        coverage: get_f64(m, "coverage")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_node_type() {
        assert!(matches!(parse_node_type("root"), Ok(NodeType::Root)));
        assert!(matches!(
            parse_node_type("reasoning"),
            Ok(NodeType::Reasoning)
        ));
        assert!(parse_node_type("unknown").is_err());
    }

    #[test]
    fn test_parse_complexity() {
        assert!(matches!(parse_complexity("low"), Ok(ComplexityLevel::Low)));
        assert!(parse_complexity("invalid").is_err());
    }

    #[test]
    fn test_parse_prune_reason() {
        assert!(matches!(
            parse_prune_reason("low_score"),
            Ok(PruneReason::LowScore)
        ));
        assert!(matches!(
            parse_prune_reason("dead_end"),
            Ok(PruneReason::DeadEnd)
        ));
    }

    #[test]
    fn test_get_str() {
        let json: serde_json::Value = serde_json::json!({"name": "test"});
        assert_eq!(get_str(&json, "name").unwrap(), "test");
        assert!(get_str(&json, "missing").is_err());
    }

    #[test]
    fn test_get_f64() {
        let json: serde_json::Value = serde_json::json!({"score": 0.75});
        let result = get_f64(&json, "score").unwrap();
        assert!((result - 0.75).abs() < f64::EPSILON);
    }
}
