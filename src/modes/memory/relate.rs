//! Relationship mapping between reasoning sessions.

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

        // Find related sessions
        let related =
            find_related_sessions(storage, embedder, model, &current_id, min_strength).await?;

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
    let clusters = compute_clusters(&edges, &session_contents);

    Ok(RelationshipGraph {
        nodes,
        edges,
        clusters,
    })
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
}
