//! Semantic search over reasoning sessions.

use crate::error::ModeError;
use crate::modes::memory::embeddings::{cosine_similarity, generate_embedding, get_all_embeddings};
use crate::modes::memory::types::SearchResult;
use crate::storage::SqliteStorage;
use crate::traits::AnthropicClientTrait;
use sqlx::Row;

/// Search reasoning sessions by semantic similarity.
///
/// # Arguments
///
/// * `storage` - Storage implementation
/// * `client` - Anthropic client for embeddings
/// * `query` - Search query
/// * `limit` - Maximum number of results
/// * `min_similarity` - Minimum similarity threshold (0.0-1.0)
/// * `mode_filter` - Optional filter by reasoning mode
///
/// # Returns
///
/// Search results sorted by similarity
pub async fn search_sessions<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
    query: &str,
    limit: u32,
    min_similarity: f32,
    mode_filter: Option<String>,
) -> Result<Vec<SearchResult>, ModeError> {
    // Generate embedding for query
    let query_embedding = generate_embedding(client, query).await?;

    // Get all session embeddings
    let session_embeddings = get_all_embeddings(storage, client).await?;

    // Compute similarities
    let mut similarities: Vec<(String, f32)> = session_embeddings
        .iter()
        .map(|(session_id, embedding)| {
            let sim = cosine_similarity(&query_embedding, embedding);
            (session_id.clone(), sim)
        })
        .filter(|(_, sim)| *sim >= min_similarity)
        .collect();

    // Sort by similarity descending
    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top N
    similarities.truncate(limit as usize);

    // Load full session data
    let mut results = Vec::new();
    for (session_id, similarity_score) in similarities {
        if let Some(result) = load_search_result(storage, &session_id, similarity_score).await? {
            // Apply mode filter
            if let Some(ref filter) = mode_filter {
                if let Some(ref mode) = result.primary_mode {
                    if mode != filter {
                        continue;
                    }
                }
            }
            results.push(result);
        }
    }

    Ok(results)
}

/// Load search result data for a session.
#[allow(clippy::option_if_let_else)]
async fn load_search_result(
    storage: &SqliteStorage,
    session_id: &str,
    similarity_score: f32,
) -> Result<Option<SearchResult>, ModeError> {
    let row = sqlx::query(
        r"
        SELECT 
            s.id,
            s.created_at,
            (SELECT content FROM thoughts WHERE session_id = s.id ORDER BY created_at LIMIT 1) as preview,
            (SELECT mode FROM thoughts WHERE session_id = s.id GROUP BY mode ORDER BY COUNT(*) DESC LIMIT 1) as primary_mode
        FROM sessions s
        WHERE s.id = ?
        ",
    )
    .bind(session_id)
    .fetch_optional(&storage.get_pool())
    .await
    .map_err(|e| ModeError::StorageError {
        message: format!("Failed to load search result: {e}"),
    })?;

    if let Some(row) = row {
        let preview: Option<String> = row.get("preview");
        let primary_mode: Option<String> = row.get("primary_mode");
        let created_at: String = row.get("created_at");

        Ok(Some(SearchResult {
            session_id: session_id.to_string(),
            similarity_score: f64::from(similarity_score),
            preview: preview.unwrap_or_default().chars().take(200).collect(),
            created_at,
            primary_mode,
        }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::SqliteStorage;
    use crate::test_utils::create_mock_client;

    #[tokio::test]
    async fn test_search_empty() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = create_mock_client();

        let results = search_sessions(&storage, &client, "test query", 5, 0.7, None)
            .await
            .expect("search sessions");

        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_with_sessions() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = create_mock_client();

        // Create test session
        let session = storage.create_session().await.expect("create session");
        storage
            .create_thought(
                &session.id,
                None,
                "linear",
                "Test thought about reasoning",
                0.8,
                None,
            )
            .await
            .expect("create thought");

        let results = search_sessions(&storage, &client, "reasoning", 5, 0.0, None)
            .await
            .expect("search sessions");

        assert!(results.len() > 0);
    }
}
