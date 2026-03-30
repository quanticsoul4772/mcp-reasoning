//! Session clustering and edge deduplication for relationship graphs.

use std::collections::{HashMap, HashSet};

use crate::modes::memory::types::{RelationshipEdge, RelationshipType, SessionCluster};

/// Deduplicate undirected relationship edges.
///
/// For symmetric relationship types, A→B and B→A represent the same edge.
/// Keep only the one where `from_session <= to_session` (lexicographic order).
/// `ContinuesFrom` and `CommonConclusion` are directional — kept as-is.
pub fn dedup_edges(edges: Vec<RelationshipEdge>) -> Vec<RelationshipEdge> {
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    let mut result = Vec::with_capacity(edges.len());

    for edge in edges {
        let is_symmetric = matches!(
            edge.relationship_type,
            RelationshipType::SimilarTopic
                | RelationshipType::SharedMode
                | RelationshipType::TemporallyAdjacent
        );

        if is_symmetric {
            // Canonical key: sort the two session IDs so A→B and B→A hash the same
            let (a, b) = if edge.from_session <= edge.to_session {
                (edge.from_session.clone(), edge.to_session.clone())
            } else {
                (edge.to_session.clone(), edge.from_session.clone())
            };
            let rel = format!("{:?}", edge.relationship_type);
            if seen.insert((a, b, rel)) {
                result.push(edge);
            }
        } else {
            result.push(edge);
        }
    }

    result
}

/// Union-find path-compression helper (iterative to avoid recursion limits).
fn uf_find(parent: &mut HashMap<String, String>, x: &str) -> String {
    let mut current = x.to_string();
    loop {
        let p = parent
            .get(&current)
            .cloned()
            .unwrap_or_else(|| current.clone());
        if p == current {
            break;
        }
        // Path compression: point directly to grandparent
        let gp = parent.get(&p).cloned().unwrap_or_else(|| p.clone());
        parent.insert(current.clone(), gp.clone());
        current = gp;
    }
    current
}

/// Cluster sessions using union-find over strong SimilarTopic edges.
///
/// Sessions with SimilarTopic strength ≥ 0.8 are grouped together.
/// The common theme is derived from the most frequent keywords shared
/// across all sessions in the cluster.
pub fn compute_clusters(
    edges: &[RelationshipEdge],
    session_contents: &HashMap<String, String>,
) -> Vec<SessionCluster> {
    const CLUSTER_THRESHOLD: f64 = 0.8;

    // Collect all session IDs that appear in strong SimilarTopic edges
    let mut parent: HashMap<String, String> = HashMap::new();

    let strong_edges: Vec<(&str, &str)> = edges
        .iter()
        .filter(|e| {
            matches!(e.relationship_type, RelationshipType::SimilarTopic)
                && e.strength >= CLUSTER_THRESHOLD
        })
        .map(|e| (e.from_session.as_str(), e.to_session.as_str()))
        .collect();

    // Initialize union-find
    for &(a, b) in &strong_edges {
        parent.entry(a.to_string()).or_insert_with(|| a.to_string());
        parent.entry(b.to_string()).or_insert_with(|| b.to_string());
    }

    // Union-find: union all connected pairs
    for &(a, b) in &strong_edges {
        let root_a = uf_find(&mut parent, a);
        let root_b = uf_find(&mut parent, b);
        if root_a != root_b {
            parent.insert(root_b, root_a);
        }
    }

    // Group by root
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    let keys: Vec<String> = parent.keys().cloned().collect();
    for k in keys {
        let root = uf_find(&mut parent, &k);
        groups.entry(root).or_default().push(k);
    }

    // Build clusters with 2+ members
    groups
        .into_values()
        .filter(|members| members.len() >= 2)
        .map(|members| {
            let theme = extract_common_theme(&members, session_contents);
            SessionCluster {
                sessions: members,
                common_theme: theme,
            }
        })
        .collect()
}

/// Extract the most frequent keywords shared across a set of sessions.
fn extract_common_theme(sessions: &[String], contents: &HashMap<String, String>) -> String {
    // Count keyword frequency across all sessions in the cluster
    let mut freq: HashMap<String, usize> = HashMap::new();
    for session_id in sessions {
        if let Some(content) = contents.get(session_id) {
            let words: HashSet<String> = content
                .split_whitespace()
                .map(|w| {
                    w.trim_matches(|c: char| !c.is_alphanumeric())
                        .to_lowercase()
                })
                .filter(|w| w.len() >= 5)
                .collect();
            for word in words {
                *freq.entry(word).or_insert(0) += 1;
            }
        }
    }

    // Return top-3 keywords that appear in more than one session
    let min_sessions = sessions.len().min(2);
    let mut top: Vec<(String, usize)> = freq
        .into_iter()
        .filter(|(_, count)| *count >= min_sessions)
        .collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));

    if top.is_empty() {
        "mixed topics".to_string()
    } else {
        top.into_iter()
            .take(3)
            .map(|(w, _)| w)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn make_edge(from: &str, to: &str, rt: RelationshipType) -> RelationshipEdge {
        RelationshipEdge {
            from_session: from.to_string(),
            to_session: to.to_string(),
            relationship_type: rt,
            strength: 0.8,
        }
    }

    #[test]
    fn test_dedup_edges_removes_reverse_symmetric() {
        let edges = vec![
            make_edge("s1", "s2", RelationshipType::SimilarTopic),
            make_edge("s2", "s1", RelationshipType::SimilarTopic), // duplicate
            make_edge("s1", "s2", RelationshipType::SharedMode),
            make_edge("s2", "s1", RelationshipType::SharedMode), // duplicate
            make_edge("s1", "s2", RelationshipType::TemporallyAdjacent),
            make_edge("s2", "s1", RelationshipType::TemporallyAdjacent), // duplicate
        ];

        let deduped = dedup_edges(edges);
        assert_eq!(deduped.len(), 3);
    }

    #[test]
    fn test_dedup_edges_preserves_directional() {
        // ContinuesFrom is directional — both directions should be kept if both exist
        let edges = vec![
            make_edge("s1", "s2", RelationshipType::ContinuesFrom),
            make_edge("s2", "s1", RelationshipType::ContinuesFrom),
        ];

        let deduped = dedup_edges(edges);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_compute_clusters_empty() {
        let clusters = compute_clusters(&[], &HashMap::new());
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_compute_clusters_groups_strong_edges() {
        let edges = vec![
            RelationshipEdge {
                from_session: "s1".to_string(),
                to_session: "s2".to_string(),
                relationship_type: RelationshipType::SimilarTopic,
                strength: 0.9,
            },
            RelationshipEdge {
                from_session: "s2".to_string(),
                to_session: "s3".to_string(),
                relationship_type: RelationshipType::SimilarTopic,
                strength: 0.85,
            },
        ];
        let mut contents = HashMap::new();
        contents.insert(
            "s1".to_string(),
            "reasoning about memory systems architecture".to_string(),
        );
        contents.insert(
            "s2".to_string(),
            "memory systems and reasoning patterns".to_string(),
        );
        contents.insert(
            "s3".to_string(),
            "reasoning systems memory design".to_string(),
        );

        let clusters = compute_clusters(&edges, &contents);
        assert_eq!(clusters.len(), 1);
        let cluster = &clusters[0];
        assert_eq!(cluster.sessions.len(), 3);
        assert!(!cluster.common_theme.is_empty());
        assert_ne!(cluster.common_theme, "mixed topics");
    }

    #[test]
    fn test_compute_clusters_ignores_weak_edges() {
        let edges = vec![RelationshipEdge {
            from_session: "s1".to_string(),
            to_session: "s2".to_string(),
            relationship_type: RelationshipType::SimilarTopic,
            strength: 0.7,
        }];
        let clusters = compute_clusters(&edges, &HashMap::new());
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_compute_clusters_ignores_non_similar_topic() {
        let edges = vec![RelationshipEdge {
            from_session: "s1".to_string(),
            to_session: "s2".to_string(),
            relationship_type: RelationshipType::SharedMode,
            strength: 0.95,
        }];
        let clusters = compute_clusters(&edges, &HashMap::new());
        assert!(clusters.is_empty());
    }
}
