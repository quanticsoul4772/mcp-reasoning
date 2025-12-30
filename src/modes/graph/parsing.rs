//! Graph mode parsing utilities.
//!
//! Parsing functions for graph operation responses.

use crate::error::ModeError;

use super::types::{
    ChildNode, ComplexityLevel, ExpansionDirection, FrontierNodeInfo, GraphConclusion,
    GraphMetadata, GraphMetrics, GraphPath, GraphStructure, IntegrationNotes, NodeAssessment,
    NodeCritique, NodeRecommendation, NodeRelationship, NodeScores, NodeType, PruneCandidate,
    PruneImpact, PruneReason, RefinedNode, RootNode, SessionQuality, SuggestedAction,
    SynthesisNode,
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
    let r = json.get("root").ok_or_else(|| ModeError::MissingField {
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
    let s = json.get("scores").ok_or_else(|| ModeError::MissingField {
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
    let m = json.get("metrics").ok_or_else(|| ModeError::MissingField {
        field: "metrics".to_string(),
    })?;

    Ok(GraphMetrics {
        average_score: get_f64(m, "average_score")?,
        max_score: get_f64(m, "max_score")?,
        coverage: get_f64(m, "coverage")?,
    })
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
    use serde_json::json;

    // ========================================================================
    // Helper function tests
    // ========================================================================

    #[test]
    fn test_get_str_success() {
        let json = json!({"name": "test"});
        assert_eq!(get_str(&json, "name").unwrap(), "test");
    }

    #[test]
    fn test_get_str_missing() {
        let json = json!({});
        assert!(
            matches!(get_str(&json, "name"), Err(ModeError::MissingField { field }) if field == "name")
        );
    }

    #[test]
    fn test_get_f64_success() {
        let json = json!({"score": 0.75});
        let result = get_f64(&json, "score").unwrap();
        assert!((result - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_get_f64_missing() {
        let json = json!({});
        assert!(
            matches!(get_f64(&json, "score"), Err(ModeError::MissingField { field }) if field == "score")
        );
    }

    #[test]
    fn test_get_u32_success() {
        let json = json!({"count": 42});
        assert_eq!(get_u32(&json, "count").unwrap(), 42);
    }

    #[test]
    fn test_get_u32_missing() {
        let json = json!({});
        assert!(
            matches!(get_u32(&json, "count"), Err(ModeError::MissingField { field }) if field == "count")
        );
    }

    #[test]
    fn test_get_string_array_success() {
        let json = json!({"items": ["a", "b", "c"]});
        let result = get_string_array(&json, "items").unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_get_string_array_missing() {
        let json = json!({});
        assert!(
            matches!(get_string_array(&json, "items"), Err(ModeError::MissingField { field }) if field == "items")
        );
    }

    #[test]
    fn test_get_string_array_filters_non_strings() {
        let json = json!({"items": ["a", 123, "b", null, true]});
        let result = get_string_array(&json, "items").unwrap();
        assert_eq!(result, vec!["a", "b"]);
    }

    // ========================================================================
    // Enum parser tests
    // ========================================================================

    #[test]
    fn test_parse_node_type_all_variants() {
        assert!(matches!(parse_node_type("root"), Ok(NodeType::Root)));
        assert!(matches!(
            parse_node_type("reasoning"),
            Ok(NodeType::Reasoning)
        ));
        assert!(matches!(
            parse_node_type("evidence"),
            Ok(NodeType::Evidence)
        ));
        assert!(matches!(
            parse_node_type("hypothesis"),
            Ok(NodeType::Hypothesis)
        ));
        assert!(matches!(
            parse_node_type("conclusion"),
            Ok(NodeType::Conclusion)
        ));
        assert!(matches!(
            parse_node_type("synthesis"),
            Ok(NodeType::Synthesis)
        ));
        assert!(matches!(parse_node_type("refined"), Ok(NodeType::Refined)));
    }

    #[test]
    fn test_parse_node_type_case_insensitive() {
        assert!(matches!(parse_node_type("ROOT"), Ok(NodeType::Root)));
        assert!(matches!(
            parse_node_type("Reasoning"),
            Ok(NodeType::Reasoning)
        ));
    }

    #[test]
    fn test_parse_node_type_invalid() {
        assert!(
            matches!(parse_node_type("unknown"), Err(ModeError::InvalidValue { field, .. }) if field == "type")
        );
    }

    #[test]
    fn test_parse_node_relationship_all_variants() {
        assert!(matches!(
            parse_node_relationship("elaborates"),
            Ok(NodeRelationship::Elaborates)
        ));
        assert!(matches!(
            parse_node_relationship("supports"),
            Ok(NodeRelationship::Supports)
        ));
        assert!(matches!(
            parse_node_relationship("challenges"),
            Ok(NodeRelationship::Challenges)
        ));
        assert!(matches!(
            parse_node_relationship("synthesizes"),
            Ok(NodeRelationship::Synthesizes)
        ));
    }

    #[test]
    fn test_parse_node_relationship_invalid() {
        assert!(
            matches!(parse_node_relationship("unknown"), Err(ModeError::InvalidValue { field, .. }) if field == "relationship")
        );
    }

    #[test]
    fn test_parse_complexity_all_variants() {
        assert!(matches!(parse_complexity("low"), Ok(ComplexityLevel::Low)));
        assert!(matches!(
            parse_complexity("medium"),
            Ok(ComplexityLevel::Medium)
        ));
        assert!(matches!(
            parse_complexity("high"),
            Ok(ComplexityLevel::High)
        ));
    }

    #[test]
    fn test_parse_complexity_invalid() {
        assert!(
            matches!(parse_complexity("extreme"), Err(ModeError::InvalidValue { field, .. }) if field == "complexity")
        );
    }

    #[test]
    fn test_parse_recommendation_all_variants() {
        assert!(matches!(
            parse_recommendation("expand"),
            Ok(NodeRecommendation::Expand)
        ));
        assert!(matches!(
            parse_recommendation("keep"),
            Ok(NodeRecommendation::Keep)
        ));
        assert!(matches!(
            parse_recommendation("prune"),
            Ok(NodeRecommendation::Prune)
        ));
    }

    #[test]
    fn test_parse_recommendation_invalid() {
        assert!(
            matches!(parse_recommendation("unknown"), Err(ModeError::InvalidValue { field, .. }) if field == "recommendation")
        );
    }

    #[test]
    fn test_parse_prune_reason_all_variants() {
        assert!(matches!(
            parse_prune_reason("low_score"),
            Ok(PruneReason::LowScore)
        ));
        assert!(matches!(
            parse_prune_reason("redundant"),
            Ok(PruneReason::Redundant)
        ));
        assert!(matches!(
            parse_prune_reason("dead_end"),
            Ok(PruneReason::DeadEnd)
        ));
        assert!(matches!(
            parse_prune_reason("off_topic"),
            Ok(PruneReason::OffTopic)
        ));
    }

    #[test]
    fn test_parse_prune_reason_invalid() {
        assert!(
            matches!(parse_prune_reason("unknown"), Err(ModeError::InvalidValue { field, .. }) if field == "reason")
        );
    }

    #[test]
    fn test_parse_prune_impact_all_variants() {
        assert!(matches!(parse_prune_impact("none"), Ok(PruneImpact::None)));
        assert!(matches!(
            parse_prune_impact("minor"),
            Ok(PruneImpact::Minor)
        ));
        assert!(matches!(
            parse_prune_impact("moderate"),
            Ok(PruneImpact::Moderate)
        ));
    }

    #[test]
    fn test_parse_prune_impact_invalid() {
        assert!(
            matches!(parse_prune_impact("severe"), Err(ModeError::InvalidValue { field, .. }) if field == "impact")
        );
    }

    #[test]
    fn test_parse_suggested_action_all_variants() {
        assert!(matches!(
            parse_suggested_action("expand"),
            Ok(SuggestedAction::Expand)
        ));
        assert!(matches!(
            parse_suggested_action("refine"),
            Ok(SuggestedAction::Refine)
        ));
        assert!(matches!(
            parse_suggested_action("aggregate"),
            Ok(SuggestedAction::Aggregate)
        ));
    }

    #[test]
    fn test_parse_suggested_action_invalid() {
        assert!(
            matches!(parse_suggested_action("unknown"), Err(ModeError::InvalidValue { field, .. }) if field == "suggested_action")
        );
    }

    // ========================================================================
    // Init parser tests
    // ========================================================================

    #[test]
    fn test_parse_root_success() {
        let json = json!({
            "root": {
                "id": "node-1",
                "content": "Main problem",
                "score": 0.85,
                "type": "root"
            }
        });

        let result = parse_root(&json).unwrap();
        assert_eq!(result.id, "node-1");
        assert_eq!(result.content, "Main problem");
        assert!((result.score - 0.85).abs() < f64::EPSILON);
        assert!(matches!(result.node_type, NodeType::Root));
    }

    #[test]
    fn test_parse_root_missing() {
        let json = json!({});
        assert!(
            matches!(parse_root(&json), Err(ModeError::MissingField { field }) if field == "root")
        );
    }

    #[test]
    fn test_parse_root_missing_id() {
        let json = json!({
            "root": {
                "content": "test",
                "score": 0.5,
                "type": "root"
            }
        });
        assert!(
            matches!(parse_root(&json), Err(ModeError::MissingField { field }) if field == "id")
        );
    }

    #[test]
    fn test_parse_expansion_directions_success() {
        let json = json!({
            "expansion_directions": [
                {"direction": "explore causes", "potential": 0.9},
                {"direction": "examine effects", "potential": 0.8}
            ]
        });

        let result = parse_expansion_directions(&json).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].direction, "explore causes");
        assert!((result[0].potential - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_expansion_directions_missing() {
        let json = json!({});
        assert!(
            matches!(parse_expansion_directions(&json), Err(ModeError::MissingField { field }) if field == "expansion_directions")
        );
    }

    #[test]
    fn test_parse_graph_metadata_success() {
        let json = json!({
            "graph_metadata": {
                "complexity": "high",
                "estimated_depth": 5
            }
        });

        let result = parse_graph_metadata(&json).unwrap();
        assert!(matches!(result.complexity, ComplexityLevel::High));
        assert_eq!(result.estimated_depth, 5);
    }

    #[test]
    fn test_parse_graph_metadata_missing() {
        let json = json!({});
        assert!(
            matches!(parse_graph_metadata(&json), Err(ModeError::MissingField { field }) if field == "graph_metadata")
        );
    }

    // ========================================================================
    // Generate parser tests
    // ========================================================================

    #[test]
    fn test_parse_children_success() {
        let json = json!({
            "children": [
                {
                    "id": "child-1",
                    "content": "First child",
                    "score": 0.75,
                    "type": "reasoning",
                    "relationship": "elaborates"
                },
                {
                    "id": "child-2",
                    "content": "Second child",
                    "score": 0.8,
                    "type": "evidence",
                    "relationship": "supports"
                }
            ]
        });

        let result = parse_children(&json).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "child-1");
        assert!(matches!(result[0].node_type, NodeType::Reasoning));
        assert!(matches!(
            result[0].relationship,
            NodeRelationship::Elaborates
        ));
        assert!(matches!(result[1].relationship, NodeRelationship::Supports));
    }

    #[test]
    fn test_parse_children_missing() {
        let json = json!({});
        assert!(
            matches!(parse_children(&json), Err(ModeError::MissingField { field }) if field == "children")
        );
    }

    #[test]
    fn test_parse_children_missing_relationship() {
        let json = json!({
            "children": [{
                "id": "child-1",
                "content": "test",
                "score": 0.5,
                "type": "reasoning"
            }]
        });
        assert!(
            matches!(parse_children(&json), Err(ModeError::MissingField { field }) if field == "relationship")
        );
    }

    // ========================================================================
    // Score parser tests
    // ========================================================================

    #[test]
    fn test_parse_node_scores_success() {
        let json = json!({
            "scores": {
                "relevance": 0.9,
                "coherence": 0.85,
                "depth": 0.8,
                "novelty": 0.75,
                "overall": 0.82
            }
        });

        let result = parse_node_scores(&json).unwrap();
        assert!((result.relevance - 0.9).abs() < f64::EPSILON);
        assert!((result.overall - 0.82).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_node_scores_missing() {
        let json = json!({});
        assert!(
            matches!(parse_node_scores(&json), Err(ModeError::MissingField { field }) if field == "scores")
        );
    }

    #[test]
    fn test_parse_node_scores_missing_field() {
        let json = json!({
            "scores": {
                "relevance": 0.9,
                "coherence": 0.85,
                "depth": 0.8,
                "overall": 0.82
            }
        });
        assert!(
            matches!(parse_node_scores(&json), Err(ModeError::MissingField { field }) if field == "novelty")
        );
    }

    #[test]
    fn test_parse_node_assessment_success() {
        let json = json!({
            "assessment": {
                "strengths": ["Clear reasoning", "Good evidence"],
                "weaknesses": ["Missing context"],
                "recommendation": "expand"
            }
        });

        let result = parse_node_assessment(&json).unwrap();
        assert_eq!(result.strengths.len(), 2);
        assert_eq!(result.weaknesses.len(), 1);
        assert!(matches!(result.recommendation, NodeRecommendation::Expand));
    }

    #[test]
    fn test_parse_node_assessment_missing() {
        let json = json!({});
        assert!(
            matches!(parse_node_assessment(&json), Err(ModeError::MissingField { field }) if field == "assessment")
        );
    }

    // ========================================================================
    // Aggregate parser tests
    // ========================================================================

    #[test]
    fn test_parse_synthesis_success() {
        let json = json!({
            "synthesis": {
                "id": "synth-1",
                "content": "Combined insight",
                "score": 0.88,
                "type": "synthesis"
            }
        });

        let result = parse_synthesis(&json).unwrap();
        assert_eq!(result.id, "synth-1");
        assert!(matches!(result.node_type, NodeType::Synthesis));
    }

    #[test]
    fn test_parse_synthesis_missing() {
        let json = json!({});
        assert!(
            matches!(parse_synthesis(&json), Err(ModeError::MissingField { field }) if field == "synthesis")
        );
    }

    #[test]
    fn test_parse_integration_notes_success() {
        let json = json!({
            "integration_notes": {
                "common_themes": ["Theme A", "Theme B"],
                "complementary_aspects": ["Aspect 1"],
                "resolved_contradictions": []
            }
        });

        let result = parse_integration_notes(&json).unwrap();
        assert_eq!(result.common_themes.len(), 2);
        assert_eq!(result.complementary_aspects.len(), 1);
        assert!(result.resolved_contradictions.is_empty());
    }

    #[test]
    fn test_parse_integration_notes_missing() {
        let json = json!({});
        assert!(
            matches!(parse_integration_notes(&json), Err(ModeError::MissingField { field }) if field == "integration_notes")
        );
    }

    // ========================================================================
    // Refine parser tests
    // ========================================================================

    #[test]
    fn test_parse_critique_success() {
        let json = json!({
            "critique": {
                "issues": ["Issue 1"],
                "missing_elements": ["Element A", "Element B"],
                "unclear_aspects": []
            }
        });

        let result = parse_critique(&json).unwrap();
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.missing_elements.len(), 2);
        assert!(result.unclear_aspects.is_empty());
    }

    #[test]
    fn test_parse_critique_missing() {
        let json = json!({});
        assert!(
            matches!(parse_critique(&json), Err(ModeError::MissingField { field }) if field == "critique")
        );
    }

    #[test]
    fn test_parse_refined_node_success() {
        let json = json!({
            "refined_node": {
                "id": "refined-1",
                "content": "Improved reasoning",
                "score": 0.92,
                "type": "refined"
            }
        });

        let result = parse_refined_node(&json).unwrap();
        assert_eq!(result.id, "refined-1");
        assert!(matches!(result.node_type, NodeType::Refined));
    }

    #[test]
    fn test_parse_refined_node_missing() {
        let json = json!({});
        assert!(
            matches!(parse_refined_node(&json), Err(ModeError::MissingField { field }) if field == "refined_node")
        );
    }

    // ========================================================================
    // Prune parser tests
    // ========================================================================

    #[test]
    fn test_parse_prune_candidates_success() {
        let json = json!({
            "prune_candidates": [
                {
                    "node_id": "node-5",
                    "reason": "low_score",
                    "confidence": 0.9,
                    "impact": "minor"
                },
                {
                    "node_id": "node-7",
                    "reason": "redundant",
                    "confidence": 0.85,
                    "impact": "none"
                }
            ]
        });

        let result = parse_prune_candidates(&json).unwrap();
        assert_eq!(result.len(), 2);
        assert!(matches!(result[0].reason, PruneReason::LowScore));
        assert!(matches!(result[0].impact, PruneImpact::Minor));
        assert!(matches!(result[1].reason, PruneReason::Redundant));
    }

    #[test]
    fn test_parse_prune_candidates_missing() {
        let json = json!({});
        assert!(
            matches!(parse_prune_candidates(&json), Err(ModeError::MissingField { field }) if field == "prune_candidates")
        );
    }

    // ========================================================================
    // Finalize parser tests
    // ========================================================================

    #[test]
    fn test_parse_best_paths_success() {
        let json = json!({
            "best_paths": [
                {
                    "path": ["node-1", "node-2", "node-3"],
                    "path_quality": 0.88,
                    "key_insight": "Main conclusion"
                }
            ]
        });

        let result = parse_best_paths(&json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path.len(), 3);
        assert!((result[0].path_quality - 0.88).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_best_paths_missing() {
        let json = json!({});
        assert!(
            matches!(parse_best_paths(&json), Err(ModeError::MissingField { field }) if field == "best_paths")
        );
    }

    #[test]
    fn test_parse_conclusions_success() {
        let json = json!({
            "conclusions": [
                {
                    "conclusion": "Final answer",
                    "confidence": 0.92,
                    "supporting_nodes": ["node-1", "node-3"]
                }
            ]
        });

        let result = parse_conclusions(&json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].conclusion, "Final answer");
        assert_eq!(result[0].supporting_nodes.len(), 2);
    }

    #[test]
    fn test_parse_conclusions_missing() {
        let json = json!({});
        assert!(
            matches!(parse_conclusions(&json), Err(ModeError::MissingField { field }) if field == "conclusions")
        );
    }

    #[test]
    fn test_parse_session_quality_success() {
        let json = json!({
            "session_quality": {
                "depth_achieved": 0.85,
                "breadth_achieved": 0.8,
                "coherence": 0.9,
                "overall": 0.85
            }
        });

        let result = parse_session_quality(&json).unwrap();
        assert!((result.depth_achieved - 0.85).abs() < f64::EPSILON);
        assert!((result.overall - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_session_quality_missing() {
        let json = json!({});
        assert!(
            matches!(parse_session_quality(&json), Err(ModeError::MissingField { field }) if field == "session_quality")
        );
    }

    // ========================================================================
    // State parser tests
    // ========================================================================

    #[test]
    fn test_parse_structure_success() {
        let json = json!({
            "structure": {
                "total_nodes": 15,
                "depth": 4,
                "branches": 3,
                "pruned_count": 2
            }
        });

        let result = parse_structure(&json).unwrap();
        assert_eq!(result.total_nodes, 15);
        assert_eq!(result.depth, 4);
        assert_eq!(result.pruned_count, 2);
    }

    #[test]
    fn test_parse_structure_missing() {
        let json = json!({});
        assert!(
            matches!(parse_structure(&json), Err(ModeError::MissingField { field }) if field == "structure")
        );
    }

    #[test]
    fn test_parse_frontiers_success() {
        let json = json!({
            "frontiers": [
                {
                    "node_id": "node-8",
                    "potential": 0.85,
                    "suggested_action": "expand"
                },
                {
                    "node_id": "node-12",
                    "potential": 0.7,
                    "suggested_action": "refine"
                }
            ]
        });

        let result = parse_frontiers(&json).unwrap();
        assert_eq!(result.len(), 2);
        assert!(matches!(
            result[0].suggested_action,
            SuggestedAction::Expand
        ));
        assert!(matches!(
            result[1].suggested_action,
            SuggestedAction::Refine
        ));
    }

    #[test]
    fn test_parse_frontiers_missing() {
        let json = json!({});
        assert!(
            matches!(parse_frontiers(&json), Err(ModeError::MissingField { field }) if field == "frontiers")
        );
    }

    #[test]
    fn test_parse_metrics_success() {
        let json = json!({
            "metrics": {
                "average_score": 0.78,
                "max_score": 0.95,
                "coverage": 0.85
            }
        });

        let result = parse_metrics(&json).unwrap();
        assert!((result.average_score - 0.78).abs() < f64::EPSILON);
        assert!((result.max_score - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_metrics_missing() {
        let json = json!({});
        assert!(
            matches!(parse_metrics(&json), Err(ModeError::MissingField { field }) if field == "metrics")
        );
    }
}
