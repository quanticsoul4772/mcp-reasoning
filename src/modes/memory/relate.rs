//! Relationship mapping between reasoning sessions.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::ModeError;
use crate::modes::memory::embeddings::get_session_content;
use crate::modes::memory::types::{
    RelationshipEdge, RelationshipGraph, RelationshipType, SessionCluster, SessionNode,
};
use crate::storage::SqliteStorage;
use crate::traits::AnthropicClientTrait;
use sqlx::Row;

/// Analyze relationships between reasoning sessions.
///
/// # Arguments
///
/// * `storage` - Storage implementation
/// * `client` - Anthropic client for embeddings
/// * `session_id` - Optional specific session to analyze (None = all sessions)
/// * `depth` - How many levels of relationships to traverse
/// * `min_strength` - Minimum relationship strength (0.0-1.0)
///
/// # Returns
///
/// Relationship graph with nodes and edges
pub async fn relate_sessions<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
    session_id: Option<String>,
    depth: u32,
    min_strength: f32,
) -> Result<RelationshipGraph, ModeError> {
    if let Some(id) = session_id {
        analyze_session_relationships(storage, client, &id, depth, min_strength).await
    } else {
        analyze_all_relationships(storage, client, min_strength).await
    }
}

/// Analyze relationships for a specific session (BFS traversal).
async fn analyze_session_relationships<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
    session_id: &str,
    depth: u32,
    min_strength: f32,
) -> Result<RelationshipGraph, ModeError> {
    let mut nodes = HashMap::new();
    let mut edges = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back((session_id.to_string(), 0));

    let mut session_contents: HashMap<String, String> = HashMap::new();

    while let Some((current_id, current_depth)) = queue.pop_front() {
        if current_depth > depth || visited.contains(&current_id) {
            continue;
        }
        visited.insert(current_id.clone());

        // Load node data
        if !nodes.contains_key(&current_id) {
            if let Some(node) = load_session_node(storage, &current_id).await? {
                nodes.insert(current_id.clone(), node);
            }
        }

        // Cache session content for clustering
        if !session_contents.contains_key(&current_id) {
            let content = get_session_content(storage, &current_id).await?;
            session_contents.insert(current_id.clone(), content);
        }

        // Find related sessions
        let related = find_related_sessions(storage, client, &current_id, min_strength).await?;

        for (related_id, relationship_type, strength) in related {
            // Add edge
            edges.push(RelationshipEdge {
                from_session: current_id.clone(),
                to_session: related_id.clone(),
                relationship_type,
                strength: f64::from(strength),
            });

            // Queue for traversal
            if current_depth < depth {
                queue.push_back((related_id, current_depth + 1));
            }
        }
    }

    let clusters = compute_clusters(&edges, &session_contents);

    Ok(RelationshipGraph {
        nodes: nodes.into_values().collect(),
        edges,
        clusters,
    })
}

/// Analyze all relationships in the database.
async fn analyze_all_relationships<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
    min_strength: f32,
) -> Result<RelationshipGraph, ModeError> {
    let session_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM sessions")
        .fetch_all(&storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError {
            message: format!("Failed to get sessions: {e}"),
        })?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut session_contents: HashMap<String, String> = HashMap::new();

    for session_id in &session_ids {
        if let Some(node) = load_session_node(storage, session_id).await? {
            nodes.push(node);
        }

        let content = get_session_content(storage, session_id).await?;
        session_contents.insert(session_id.clone(), content);

        let related = find_related_sessions(storage, client, session_id, min_strength).await?;
        for (related_id, relationship_type, strength) in related {
            edges.push(RelationshipEdge {
                from_session: session_id.clone(),
                to_session: related_id,
                relationship_type,
                strength: f64::from(strength),
            });
        }
    }

    // Deduplicate symmetric edges (SimilarTopic, SharedMode, TemporallyAdjacent are
    // undirected relationships; both A→B and B→A are generated — keep only canonical form).
    let edges = dedup_edges(edges);
    let clusters = compute_clusters(&edges, &session_contents);

    Ok(RelationshipGraph {
        nodes,
        edges,
        clusters,
    })
}

/// Deduplicate undirected relationship edges.
///
/// For symmetric relationship types, A→B and B→A represent the same edge.
/// Keep only the one where `from_session <= to_session` (lexicographic order).
/// `ContinuesFrom` and `CommonConclusion` are directional — kept as-is.
fn dedup_edges(edges: Vec<RelationshipEdge>) -> Vec<RelationshipEdge> {
    use std::collections::HashSet;

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

/// Load session node data.
#[allow(clippy::option_if_let_else)]
async fn load_session_node(
    storage: &SqliteStorage,
    session_id: &str,
) -> Result<Option<SessionNode>, ModeError> {
    let row = sqlx::query(
        r"
        SELECT
            s.id,
            s.created_at,
            (SELECT content FROM thoughts WHERE session_id = s.id ORDER BY created_at LIMIT 1) as preview
        FROM sessions s
        WHERE s.id = ?
        ",
    )
    .bind(session_id)
    .fetch_optional(&storage.get_pool())
    .await
    .map_err(|e| ModeError::StorageError {
        message: format!("Failed to load node: {e}"),
    })?;

    if let Some(row) = row {
        let preview: Option<String> = row.get("preview");
        Ok(Some(SessionNode {
            session_id: session_id.to_string(),
            preview: preview.unwrap_or_default().chars().take(100).collect(),
            created_at: row.get("created_at"),
        }))
    } else {
        Ok(None)
    }
}

/// Find all sessions related to the given session.
async fn find_related_sessions<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
    session_id: &str,
    min_strength: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let mut relationships = Vec::new();

    // Find similar sessions by embedding
    let similar = find_similar_sessions(storage, client, session_id, min_strength).await?;
    relationships.extend(similar);

    // Find sessions with shared modes
    let mode_related = find_mode_related(storage, session_id, min_strength).await?;
    relationships.extend(mode_related);

    // Find temporally adjacent sessions
    let temporal = find_temporal_neighbors(storage, session_id, min_strength).await?;
    relationships.extend(temporal);

    Ok(relationships)
}

/// Find sessions with similar content using FTS5 full-text search.
///
/// Extracts significant keywords from the source session and searches
/// for other sessions containing those keywords. Similarity is derived
/// from BM25 rank normalized to [0.5, 1.0].
async fn find_similar_sessions<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    _client: &C,
    session_id: &str,
    min_similarity: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let content = get_session_content(storage, session_id).await?;
    if content.is_empty() {
        return Ok(vec![]);
    }

    // Extract query terms: lowercase words with 4+ chars (skip common short words)
    let query_terms: Vec<String> = content
        .split_whitespace()
        .map(str::to_lowercase)
        .map(|w| {
            // Strip common punctuation from word boundaries
            w.trim_matches(|c: char| !c.is_alphanumeric()).to_string()
        })
        .filter(|w| w.len() >= 4)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .take(20) // Use top 20 unique terms for the query
        .collect();

    if query_terms.is_empty() {
        return Ok(vec![]);
    }

    // Build FTS5 OR query from extracted terms (each term double-quoted for exact match)
    let fts_query = query_terms
        .iter()
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ");

    // Find other sessions matching these terms, ranked by BM25
    let rows = sqlx::query(
        r"
        SELECT
            session_id,
            bm25(thoughts_fts) AS score
        FROM thoughts_fts
        WHERE thoughts_fts MATCH ?
          AND session_id != ?
        ORDER BY score ASC
        LIMIT 50
        ",
    )
    .bind(&fts_query)
    .bind(session_id)
    .fetch_all(&storage.get_pool())
    .await
    .map_err(|e| ModeError::StorageError {
        message: format!("FTS5 similarity search failed: {e}"),
    })?;

    if rows.is_empty() {
        return Ok(vec![]);
    }

    // Deduplicate by session_id (keep best BM25 score per session)
    let mut best_per_session: HashMap<String, f64> = HashMap::new();
    let mut session_order: Vec<String> = Vec::new();
    for row in &rows {
        let sid: String = row.get("session_id");
        let score: f64 = row.get("score");
        let entry = best_per_session.entry(sid.clone()).or_insert(f64::MAX);
        if score < *entry {
            *entry = score;
        }
        if !session_order.contains(&sid) {
            session_order.push(sid);
        }
    }

    // Sort by best score (most negative = most relevant)
    session_order.sort_by(|a, b| {
        best_per_session[a]
            .partial_cmp(&best_per_session[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Normalize to [0.5, 1.0] similarity range
    let count = session_order.len();
    let results = session_order
        .into_iter()
        .enumerate()
        .map(|(i, sid)| {
            let similarity = if count == 1 {
                1.0_f32
            } else {
                1.0 - (i as f32 / (count - 1) as f32) * 0.5
            };
            (sid, RelationshipType::SimilarTopic, similarity)
        })
        .filter(|(_, _, sim)| *sim >= min_similarity)
        .collect();

    Ok(results)
}

/// Find sessions that use similar modes.
async fn find_mode_related(
    storage: &SqliteStorage,
    session_id: &str,
    min_strength: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let modes: Vec<String> =
        sqlx::query_scalar("SELECT DISTINCT mode FROM thoughts WHERE session_id = ?")
            .bind(session_id)
            .fetch_all(&storage.get_pool())
            .await
            .map_err(|e| ModeError::StorageError {
                message: format!("Failed to get modes: {e}"),
            })?;

    if modes.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    for mode in modes {
        let related: Vec<String> = sqlx::query_scalar(
            r"
            SELECT DISTINCT session_id
            FROM thoughts
            WHERE mode = ? AND session_id != ?
            LIMIT 10
            ",
        )
        .bind(&mode)
        .bind(session_id)
        .fetch_all(&storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError {
            message: format!("Failed to find mode related: {e}"),
        })?;

        for related_id in related {
            results.push((related_id, RelationshipType::SharedMode, min_strength));
        }
    }

    Ok(results)
}

/// Find sessions created close in time.
async fn find_temporal_neighbors(
    storage: &SqliteStorage,
    session_id: &str,
    min_strength: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let neighbors: Vec<String> = sqlx::query_scalar(
        r"
        SELECT id FROM sessions
        WHERE id != ?
        AND ABS(julianday(created_at) - julianday((SELECT created_at FROM sessions WHERE id = ?))) < 1
        LIMIT 5
        ",
    )
    .bind(session_id)
    .bind(session_id)
    .fetch_all(&storage.get_pool())
    .await
    .map_err(|e| ModeError::StorageError {
        message: format!("Failed to find temporal: {e}"),
    })?;

    Ok(neighbors
        .into_iter()
        .map(|id| (id, RelationshipType::TemporallyAdjacent, min_strength))
        .collect())
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
    use crate::storage::{SqliteStorage, StoredThought};
    use crate::test_utils::mock_anthropic_success;

    #[tokio::test]
    async fn test_relate_empty() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let graph = relate_sessions(&storage, &client, None, 2, 0.5)
            .await
            .expect("relate sessions");

        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[tokio::test]
    async fn test_relate_single_session() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let session = storage.create_session().await.expect("create session");
        let thought = StoredThought::new(
            uuid::Uuid::new_v4().to_string(),
            &session.id,
            "linear",
            "Test",
            0.8,
        );
        storage
            .save_stored_thought(&thought)
            .await
            .expect("save thought");

        let graph = relate_sessions(&storage, &client, Some(session.id), 1, 0.5)
            .await
            .expect("relate sessions");

        assert_eq!(graph.nodes.len(), 1);
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
        // Theme should mention common keywords (memory, reasoning, systems all appear in 3 sessions)
        assert!(!cluster.common_theme.is_empty());
        assert_ne!(cluster.common_theme, "mixed topics");
    }

    #[test]
    fn test_compute_clusters_ignores_weak_edges() {
        // Edge below 0.8 threshold should not form a cluster
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
        // SharedMode edges should not form clusters
        let edges = vec![RelationshipEdge {
            from_session: "s1".to_string(),
            to_session: "s2".to_string(),
            relationship_type: RelationshipType::SharedMode,
            strength: 0.95,
        }];
        let clusters = compute_clusters(&edges, &HashMap::new());
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_dedup_edges_removes_reverse_symmetric() {
        let make = |from: &str, to: &str, rt: RelationshipType| RelationshipEdge {
            from_session: from.to_string(),
            to_session: to.to_string(),
            relationship_type: rt,
            strength: 0.8,
        };

        let edges = vec![
            make("s1", "s2", RelationshipType::SimilarTopic),
            make("s2", "s1", RelationshipType::SimilarTopic), // duplicate
            make("s1", "s2", RelationshipType::SharedMode),
            make("s2", "s1", RelationshipType::SharedMode), // duplicate
            make("s1", "s2", RelationshipType::TemporallyAdjacent),
            make("s2", "s1", RelationshipType::TemporallyAdjacent), // duplicate
        ];

        let deduped = dedup_edges(edges);
        // Each symmetric type should appear only once
        assert_eq!(deduped.len(), 3);
    }

    #[test]
    fn test_dedup_edges_preserves_directional() {
        let make = |from: &str, to: &str, rt: RelationshipType| RelationshipEdge {
            from_session: from.to_string(),
            to_session: to.to_string(),
            relationship_type: rt,
            strength: 0.9,
        };

        // ContinuesFrom is directional — both directions should be kept if both exist
        let edges = vec![
            make("s1", "s2", RelationshipType::ContinuesFrom),
            make("s2", "s1", RelationshipType::ContinuesFrom),
        ];

        let deduped = dedup_edges(edges);
        assert_eq!(deduped.len(), 2);
    }
}
