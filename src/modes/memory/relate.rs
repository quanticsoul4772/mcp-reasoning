//! Relationship mapping between reasoning sessions.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::ModeError;
use crate::storage::SqliteStorage;
use crate::traits::AnthropicClientTrait;

use super::embeddings::{cosine_similarity, get_session_embedding};
use super::types::{RelationshipEdge, RelationshipGraph, RelationshipType, SessionNode};

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

        // Find related sessions
        let related = find_related_sessions(storage, client, &current_id, min_strength).await?;

        for (related_id, relationship_type, strength) in related {
            // Add edge
            edges.push(RelationshipEdge {
                from_session: current_id.clone(),
                to_session: related_id.clone(),
                relationship_type,
                strength: strength as f64,
            });

            // Queue for traversal
            if current_depth < depth {
                queue.push_back((related_id, current_depth + 1));
            }
        }
    }

    Ok(RelationshipGraph {
        nodes: nodes.into_values().collect(),
        edges,
        clusters: Vec::new(), // Clustering not implemented yet
    })
}

/// Analyze all relationships in the database.
async fn analyze_all_relationships<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
    min_strength: f32,
) -> Result<RelationshipGraph, ModeError> {
    let session_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM sessions")
        .fetch_all(storage.pool())
        .await
        .map_err(|e| ModeError::StorageError(format!("Failed to get sessions: {e}")))?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for session_id in &session_ids {
        if let Some(node) = load_session_node(storage, session_id).await? {
            nodes.push(node);
        }

        let related = find_related_sessions(storage, client, session_id, min_strength).await?;
        for (related_id, relationship_type, strength) in related {
            edges.push(RelationshipEdge {
                from_session: session_id.clone(),
                to_session: related_id,
                relationship_type,
                strength: strength as f64,
            });
        }
    }

    Ok(RelationshipGraph {
        nodes,
        edges,
        clusters: Vec::new(),
    })
}

/// Load session node data.
async fn load_session_node(
    storage: &SqliteStorage,
    session_id: &str,
) -> Result<Option<SessionNode>, ModeError> {
    let row = sqlx::query(
        r#"
        SELECT 
            s.id,
            s.created_at,
            (SELECT content FROM thoughts WHERE session_id = s.id ORDER BY created_at LIMIT 1) as preview
        FROM sessions s
        WHERE s.id = ?
        "#,
    )
    .bind(session_id)
    .fetch_optional(storage.pool())
    .await
    .map_err(|e| ModeError::StorageError(format!("Failed to load node: {e}")))?;

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

/// Find sessions with similar embeddings.
async fn find_similar_sessions<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
    session_id: &str,
    min_similarity: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let embedding = get_session_embedding(storage, client, session_id).await?;

    let other_sessions: Vec<String> =
        sqlx::query_scalar("SELECT id FROM sessions WHERE id != ?")
            .bind(session_id)
            .fetch_all(storage.pool())
            .await
            .map_err(|e| ModeError::StorageError(format!("Failed to get sessions: {e}")))?;

    let mut results = Vec::new();
    for other_id in other_sessions {
        let other_embedding = get_session_embedding(storage, client, &other_id).await?;
        let similarity = cosine_similarity(&embedding, &other_embedding);

        if similarity >= min_similarity {
            results.push((other_id, RelationshipType::SimilarTopic, similarity));
        }
    }

    Ok(results)
}

/// Find sessions that use similar modes.
async fn find_mode_related(
    storage: &SqliteStorage,
    session_id: &str,
    min_strength: f32,
) -> Result<Vec<(String, RelationshipType, f32)>, ModeError> {
    let modes: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT mode FROM thoughts WHERE session_id = ?",
    )
    .bind(session_id)
    .fetch_all(storage.pool())
    .await
    .map_err(|e| ModeError::StorageError(format!("Failed to get modes: {e}")))?;

    if modes.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    for mode in modes {
        let related: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT session_id 
            FROM thoughts 
            WHERE mode = ? AND session_id != ?
            LIMIT 10
            "#,
        )
        .bind(&mode)
        .bind(session_id)
        .fetch_all(storage.pool())
        .await
        .map_err(|e| ModeError::StorageError(format!("Failed to find mode related: {e}")))?;

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
        r#"
        SELECT id FROM sessions
        WHERE id != ?
        AND ABS(julianday(created_at) - julianday((SELECT created_at FROM sessions WHERE id = ?))) < 1
        LIMIT 5
        "#,
    )
    .bind(session_id)
    .bind(session_id)
    .fetch_all(storage.pool())
    .await
    .map_err(|e| ModeError::StorageError(format!("Failed to find temporal: {e}")))?;

    Ok(neighbors
        .into_iter()
        .map(|id| (id, RelationshipType::TemporallyAdjacent, min_strength))
        .collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::SqliteStorage;
    use crate::test_utils::create_mock_client;

    #[tokio::test]
    async fn test_relate_empty() {
        let storage = SqliteStorage::new_in_memory().await.expect("create storage");
        let client = create_mock_client();

        let graph = relate_sessions(&storage, &client, None, 2, 0.5)
            .await
            .expect("relate sessions");

        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[tokio::test]
    async fn test_relate_single_session() {
        let storage = SqliteStorage::new_in_memory().await.expect("create storage");
        let client = create_mock_client();

        let session = storage.create_session().await.expect("create session");
        storage
            .create_thought(&session.id, None, "linear", "Test", 0.8, None)
            .await
            .expect("create thought");

        let graph = relate_sessions(&storage, &client, Some(session.id), 1, 0.5)
            .await
            .expect("relate sessions");

        assert_eq!(graph.nodes.len(), 1);
    }
}
