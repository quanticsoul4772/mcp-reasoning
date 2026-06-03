//! Relationship mapping between reasoning sessions.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::ModeError;
use crate::modes::memory::cluster::{compute_clusters, dedup_edges};
use crate::modes::memory::embeddings::get_session_content;
use crate::modes::memory::similarity::{cosine, embed_session_cached};
use crate::modes::memory::types::{
    RelationshipEdge, RelationshipGraph, RelationshipType, SessionNode,
};
use crate::storage::SqliteStorage;
use crate::traits::EmbeddingProvider;
use sqlx::Row;

/// Cap on similar-session edges emitted per source session.
const MAX_SIMILAR_SESSIONS: usize = 50;

/// Global cap on edges in a single relationship graph. A densely connected
/// database can otherwise emit tens of thousands of edges, producing a response
/// that exceeds MCP size limits and fails entirely. Keeping the strongest edges
/// bounds the output while preserving the most meaningful relationships.
const MAX_GRAPH_EDGES: usize = 200;

/// Analyze relationships between reasoning sessions.
///
/// # Arguments
///
/// * `storage` - Storage implementation
/// * `embedder` - Embedding/rerank provider for semantic similarity
/// * `model` - Embedding model name (recorded with cached vectors)
/// * `session_id` - Optional specific session to analyze (None = all sessions)
/// * `depth` - How many levels of relationships to traverse
/// * `min_strength` - Minimum relationship strength (0.0-1.0)
///
/// # Returns
///
/// Relationship graph with nodes and edges
pub async fn relate_sessions<E: EmbeddingProvider>(
    storage: &SqliteStorage,
    embedder: &E,
    model: &str,
    session_id: Option<String>,
    depth: u32,
    min_strength: f32,
) -> Result<RelationshipGraph, ModeError> {
    if let Some(id) = session_id {
        analyze_session_relationships(storage, embedder, model, &id, depth, min_strength).await
    } else {
        analyze_all_relationships(storage, embedder, model, min_strength).await
    }
}

/// Analyze relationships for a specific session (BFS traversal).
async fn analyze_session_relationships<E: EmbeddingProvider>(
    storage: &SqliteStorage,
    embedder: &E,
    model: &str,
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

        // Expand only non-leaf nodes. A node at the maximum depth is a leaf: it
        // is kept as a node (loaded above) but its own edges are not emitted, so
        // the graph never references a session it did not also load as a node
        // (no dangling edges), and the fan-out cannot blow past the depth bound.
        if current_depth < depth {
            let related =
                find_related_sessions(storage, embedder, model, &current_id, min_strength).await?;

            for (related_id, relationship_type, strength) in related {
                edges.push(RelationshipEdge {
                    from_session: current_id.clone(),
                    to_session: related_id.clone(),
                    relationship_type,
                    strength: f64::from(strength),
                });
                // The edge target is queued (and thus loaded as a node when
                // popped), keeping nodes and edges consistent.
                queue.push_back((related_id, current_depth + 1));
            }
        }
    }

    // Dedup symmetric edges (as the all-sessions path does) and bound the graph
    // so the response stays within MCP limits on a densely connected database.
    let edges = dedup_edges(edges);
    let anchors: HashSet<String> = std::iter::once(session_id.to_string()).collect();
    let (nodes, edges) = cap_and_prune_graph(nodes.into_values().collect(), edges, &anchors);
    let clusters = compute_clusters(&edges, &session_contents);

    Ok(RelationshipGraph {
        nodes,
        edges,
        clusters,
    })
}

/// Analyze all relationships in the database.
async fn analyze_all_relationships<E: EmbeddingProvider>(
    storage: &SqliteStorage,
    embedder: &E,
    model: &str,
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

        let related =
            find_related_sessions(storage, embedder, model, session_id, min_strength).await?;
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
    // Bound the graph so the whole-database view also stays within MCP limits.
    let (nodes, edges) = cap_and_prune_graph(nodes, edges, &HashSet::new());
    let clusters = compute_clusters(&edges, &session_contents);

    Ok(RelationshipGraph {
        nodes,
        edges,
        clusters,
    })
}

/// Bound a relationship graph so the response stays within MCP size limits.
///
/// Keeps the strongest [`MAX_GRAPH_EDGES`] edges, then drops any node no longer
/// referenced by a surviving edge — except `anchors` (e.g. the queried session),
/// which are always retained so they appear even with no surviving relationships.
fn cap_and_prune_graph(
    nodes: Vec<SessionNode>,
    mut edges: Vec<RelationshipEdge>,
    anchors: &HashSet<String>,
) -> (Vec<SessionNode>, Vec<RelationshipEdge>) {
    if edges.len() > MAX_GRAPH_EDGES {
        edges.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(Ordering::Equal)
        });
        edges.truncate(MAX_GRAPH_EDGES);
    }

    let mut kept: HashSet<String> = anchors.clone();
    for edge in &edges {
        kept.insert(edge.from_session.clone());
        kept.insert(edge.to_session.clone());
    }
    let nodes = nodes
        .into_iter()
        .filter(|n| kept.contains(&n.session_id))
        .collect();

    (nodes, edges)
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
async fn find_related_sessions<E: EmbeddingProvider>(
    storage: &SqliteStorage,
    embedder: &E,
    model: &str,
    session_id: &str,
    min_strength: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let mut relationships = Vec::new();

    // Find similar sessions by embedding
    let similar = find_similar_sessions(storage, embedder, model, session_id, min_strength).await?;
    relationships.extend(similar);

    // Find sessions with shared modes
    let mode_related = find_mode_related(storage, session_id, min_strength).await?;
    relationships.extend(mode_related);

    // Find temporally adjacent sessions
    let temporal = find_temporal_neighbors(storage, session_id, min_strength).await?;
    relationships.extend(temporal);

    Ok(relationships)
}

/// Find sessions with semantically similar content using embeddings.
///
/// Embeds the source session (cached by content hash) and ranks every other
/// session by cosine similarity of their embeddings, keeping those at or above
/// `min_similarity`. Cosine is the natural metric for session↔session
/// similarity; query→session reranking lives in the search path.
async fn find_similar_sessions<E: EmbeddingProvider>(
    storage: &SqliteStorage,
    embedder: &E,
    model: &str,
    session_id: &str,
    min_similarity: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let Some(source_vec) = embed_session_cached(storage, embedder, model, session_id).await? else {
        return Ok(vec![]);
    };

    let other_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM sessions WHERE id != ?")
        .bind(session_id)
        .fetch_all(&storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError {
            message: format!("Failed to list sessions: {e}"),
        })?;

    let mut scored: Vec<(String, f32)> = Vec::new();
    for other in other_ids {
        if let Some(other_vec) = embed_session_cached(storage, embedder, model, &other).await? {
            let similarity = cosine(&source_vec, &other_vec).clamp(0.0, 1.0);
            if similarity >= min_similarity {
                scored.push((other, similarity));
            }
        }
    }

    // Most-similar first; cap the number of edges per source session.
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(MAX_SIMILAR_SESSIONS);

    Ok(scored
        .into_iter()
        .map(|(sid, sim)| (sid, RelationshipType::SimilarTopic, sim))
        .collect())
}

/// Find sessions that use similar modes, scored by mode-set overlap.
///
/// Strength is the Jaccard overlap of the two sessions' distinct mode sets —
/// `|A ∩ B| / |A ∪ B|` — so `min_strength` filters these edges the same way it
/// filters semantic edges (identical mode sets score 1.0; one shared mode out
/// of many scores low), rather than every shared-mode pair passing at the
/// threshold value.
async fn find_mode_related(
    storage: &SqliteStorage,
    session_id: &str,
    min_strength: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let source_modes: HashSet<String> =
        sqlx::query_scalar("SELECT DISTINCT mode FROM thoughts WHERE session_id = ?")
            .bind(session_id)
            .fetch_all(&storage.get_pool())
            .await
            .map_err(|e| ModeError::StorageError {
                message: format!("Failed to get modes: {e}"),
            })?
            .into_iter()
            .collect();

    if source_modes.is_empty() {
        return Ok(Vec::new());
    }

    // Every other session's full distinct mode set, so edges can be scored.
    let rows = sqlx::query("SELECT session_id, mode FROM thoughts WHERE session_id != ?")
        .bind(session_id)
        .fetch_all(&storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError {
            message: format!("Failed to find mode related: {e}"),
        })?;

    let mut candidate_modes: HashMap<String, HashSet<String>> = HashMap::new();
    for row in &rows {
        candidate_modes
            .entry(row.get::<String, _>("session_id"))
            .or_default()
            .insert(row.get::<String, _>("mode"));
    }

    let mut scored: Vec<(String, f32)> = candidate_modes
        .into_iter()
        .filter_map(|(sid, modes)| {
            let inter = source_modes.intersection(&modes).count();
            if inter == 0 {
                return None;
            }
            let union = source_modes.union(&modes).count();
            #[allow(clippy::cast_precision_loss)]
            let strength = inter as f32 / union as f32;
            (strength >= min_strength).then_some((sid, strength))
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(MAX_SIMILAR_SESSIONS);

    Ok(scored
        .into_iter()
        .map(|(sid, s)| (sid, RelationshipType::SharedMode, s))
        .collect())
}

/// Find sessions created close in time, scored by temporal proximity.
///
/// Strength decays linearly with the gap inside the one-day window
/// (`1 - days_apart`), so sessions created moments apart score ~1.0 and ones a
/// day apart ~0.0 — and `min_strength` genuinely filters these edges instead of
/// every in-window pair passing at the threshold value.
async fn find_temporal_neighbors(
    storage: &SqliteStorage,
    session_id: &str,
    min_strength: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let rows = sqlx::query(
        r"
        SELECT id,
               ABS(julianday(created_at) - julianday((SELECT created_at FROM sessions WHERE id = ?))) AS days_apart
        FROM sessions
        WHERE id != ?
        AND ABS(julianday(created_at) - julianday((SELECT created_at FROM sessions WHERE id = ?))) < 1
        ORDER BY days_apart
        LIMIT 5
        ",
    )
    .bind(session_id)
    .bind(session_id)
    .bind(session_id)
    .fetch_all(&storage.get_pool())
    .await
    .map_err(|e| ModeError::StorageError {
        message: format!("Failed to find temporal: {e}"),
    })?;

    Ok(rows
        .iter()
        .filter_map(|row| {
            let days_apart: f64 = row.get("days_apart");
            #[allow(clippy::cast_possible_truncation)]
            let strength = (1.0 - days_apart as f32).clamp(0.0, 1.0);
            (strength >= min_strength).then(|| {
                (
                    row.get::<String, _>("id"),
                    RelationshipType::TemporallyAdjacent,
                    strength,
                )
            })
        })
        .collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::{SqliteStorage, StoredThought};
    use crate::traits::MockEmbeddingProvider;

    /// An embedder that returns a fixed unit vector for every input, so any two
    /// sessions with content are maximally similar (cosine 1.0).
    fn constant_embedder() -> MockEmbeddingProvider {
        let mut m = MockEmbeddingProvider::new();
        m.expect_embed_documents()
            .returning(|texts| Ok(texts.iter().map(|_| vec![1.0_f32, 0.0, 0.0]).collect()));
        m
    }

    async fn add_session_with_thought(storage: &SqliteStorage, text: &str) -> String {
        let session = storage.create_session().await.expect("create session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &session.id,
                "linear",
                text,
                0.8,
            ))
            .await
            .expect("save thought");
        session.id
    }

    #[tokio::test]
    async fn test_relate_empty() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let graph = relate_sessions(&storage, &constant_embedder(), "voyage-4", None, 2, 0.5)
            .await
            .expect("relate sessions");
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[tokio::test]
    async fn test_relate_single_session_has_no_similar_edges() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let id = add_session_with_thought(&storage, "Rust async patterns").await;

        let graph = relate_sessions(&storage, &constant_embedder(), "voyage-4", Some(id), 1, 0.5)
            .await
            .expect("relate sessions");

        assert_eq!(graph.nodes.len(), 1);
        // Nothing to be similar to.
        assert!(!graph
            .edges
            .iter()
            .any(|e| e.relationship_type == RelationshipType::SimilarTopic));
    }

    #[test]
    fn test_cap_and_prune_keeps_strongest_edges_and_drops_orphans() {
        let nodes = vec![
            SessionNode {
                session_id: "a".to_string(),
                preview: String::new(),
                created_at: String::new(),
            },
            SessionNode {
                session_id: "orphan".to_string(),
                preview: String::new(),
                created_at: String::new(),
            },
        ];
        let edges = vec![
            RelationshipEdge {
                from_session: "a".to_string(),
                to_session: "b".to_string(),
                relationship_type: RelationshipType::SimilarTopic,
                strength: 0.9,
            },
            RelationshipEdge {
                from_session: "a".to_string(),
                to_session: "c".to_string(),
                relationship_type: RelationshipType::SimilarTopic,
                strength: 0.6,
            },
        ];
        let anchors: HashSet<String> = std::iter::once("a".to_string()).collect();
        let (nodes, edges) = cap_and_prune_graph(nodes, edges, &anchors);

        // "orphan" is referenced by no surviving edge and is not an anchor → dropped.
        assert!(nodes.iter().all(|n| n.session_id != "orphan"));
        // The anchor is retained even though no node entry exists for b/c.
        assert!(nodes.iter().any(|n| n.session_id == "a"));
        assert_eq!(edges.len(), 2);
    }

    #[tokio::test]
    async fn test_relate_emits_no_dangling_edges() {
        // Every edge endpoint must resolve to a node in the graph: a max-depth
        // node is a leaf, so the BFS never emits an edge to an unloaded session.
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let a = add_session_with_thought(&storage, "Rust ownership and borrowing").await;
        let _b = add_session_with_thought(&storage, "Memory safety in systems languages").await;
        let _c = add_session_with_thought(&storage, "Borrow checker lifetimes").await;

        let graph = relate_sessions(&storage, &constant_embedder(), "voyage-4", Some(a), 2, 0.5)
            .await
            .expect("relate sessions");

        let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.session_id.as_str()).collect();
        for edge in &graph.edges {
            assert!(
                node_ids.contains(edge.from_session.as_str()),
                "edge from_session has no node: {}",
                edge.from_session
            );
            assert!(
                node_ids.contains(edge.to_session.as_str()),
                "edge to_session has no node: {}",
                edge.to_session
            );
        }
    }

    #[tokio::test]
    async fn test_relate_links_semantically_similar_sessions() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let a = add_session_with_thought(&storage, "Rust ownership and borrowing").await;
        let b = add_session_with_thought(&storage, "Memory safety in systems languages").await;

        let graph = relate_sessions(&storage, &constant_embedder(), "voyage-4", None, 1, 0.5)
            .await
            .expect("relate sessions");

        // Both embeddings are identical → a SimilarTopic edge connects a and b.
        assert!(graph.edges.iter().any(|e| {
            e.relationship_type == RelationshipType::SimilarTopic
                && ((e.from_session == a && e.to_session == b)
                    || (e.from_session == b && e.to_session == a))
        }));
    }

    async fn session_with_modes(storage: &SqliteStorage, modes: &[&str]) -> String {
        let session = storage.create_session().await.expect("session");
        for m in modes {
            storage
                .save_stored_thought(&StoredThought::new(
                    uuid::Uuid::new_v4().to_string(),
                    &session.id,
                    *m,
                    "content",
                    0.8,
                ))
                .await
                .expect("thought");
        }
        session.id
    }

    #[tokio::test]
    async fn test_find_mode_related_scores_by_jaccard() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let a = session_with_modes(&storage, &["linear", "tree"]).await;
        let b = session_with_modes(&storage, &["linear", "tree"]).await; // identical → 1.0
        let c = session_with_modes(&storage, &["linear"]).await; // 1/2 = 0.5

        // min_strength 0.6 keeps the identical-mode session, drops the half-overlap one.
        let rel = find_mode_related(&storage, &a, 0.6).await.expect("mode");
        let by_id: HashMap<String, f32> = rel.into_iter().map(|(id, _, s)| (id, s)).collect();
        assert!((by_id.get(&b).copied().expect("b present") - 1.0).abs() < 1e-6);
        assert!(
            !by_id.contains_key(&c),
            "0.5 Jaccard must be filtered at 0.6"
        );
    }

    #[tokio::test]
    async fn test_find_temporal_neighbors_decays_with_gap() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let seed = |id: &'static str, at: &'static str| {
            let pool = storage.get_pool();
            async move {
                sqlx::query("INSERT INTO sessions (id, created_at, updated_at) VALUES (?, ?, ?)")
                    .bind(id)
                    .bind(at)
                    .bind(at)
                    .execute(&pool)
                    .await
                    .expect("seed");
            }
        };
        seed("a", "2026-01-01 00:00:00").await;
        seed("b", "2026-01-01 00:00:00").await; // 0 gap → strength 1.0
        seed("c", "2026-01-01 12:00:00").await; // 0.5 day → strength 0.5
        seed("d", "2026-01-03 00:00:00").await; // 2 days → outside the 1-day window

        let rel = find_temporal_neighbors(&storage, "a", 0.6)
            .await
            .expect("temporal");
        let by_id: HashMap<String, f32> = rel.into_iter().map(|(id, _, s)| (id, s)).collect();
        assert!((by_id.get("b").copied().expect("b present") - 1.0).abs() < 1e-6);
        assert!(
            !by_id.contains_key("c"),
            "0.5 proximity must be filtered at 0.6"
        );
        assert!(
            !by_id.contains_key("d"),
            "out-of-window session must not appear"
        );
    }
}
