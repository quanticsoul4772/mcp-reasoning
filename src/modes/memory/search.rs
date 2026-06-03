//! Semantic search over reasoning sessions using Voyage embeddings + reranking.
//!
//! The query is embedded and ranked against each session's cached embedding by
//! cosine similarity (recall); the top candidates are then reordered by the
//! cross-encoder reranker (precision). Replaces the previous BM25 keyword search.

use std::cmp::Ordering;

use crate::error::ModeError;
use crate::modes::memory::embeddings::get_session_content;
use crate::modes::memory::similarity::{cosine, embed_session_cached};
use crate::modes::memory::types::SearchResult;
use crate::storage::SqliteStorage;
use crate::traits::EmbeddingProvider;
use sqlx::Row;

/// Cosine candidates passed to the reranker before final ordering.
const SEARCH_RERANK_CANDIDATES: usize = 50;

/// Search reasoning sessions semantically.
///
/// # Arguments
///
/// * `storage` - Storage implementation
/// * `embedder` - Embedding/rerank provider
/// * `model` - Embedding model name (recorded with cached vectors)
/// * `query` - Free-text search query
/// * `limit` - Maximum number of results
/// * `min_similarity` - Minimum cosine similarity for a session to be a candidate
/// * `mode_filter` - Optional filter by reasoning mode
///
/// # Returns
///
/// Search results sorted by reranked relevance (best match first)
pub async fn search_sessions<E: EmbeddingProvider>(
    storage: &SqliteStorage,
    embedder: &E,
    model: &str,
    query: &str,
    limit: u32,
    min_similarity: f32,
    mode_filter: Option<String>,
) -> Result<Vec<SearchResult>, ModeError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    // Embed the query with the same contextualized model used for sessions, so
    // the vectors live in one space. A single chunk is fine for a query.
    let query_vec = embedder
        .embed_contextualized(&[query.to_string()], "query")
        .await?;
    if query_vec.is_empty() {
        return Ok(vec![]);
    }

    let session_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM sessions")
        .fetch_all(&storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError {
            message: format!("Failed to list sessions: {e}"),
        })?;

    // Recall: cosine of the query against every session's embedding.
    let mut scored: Vec<(String, f32)> = Vec::new();
    for sid in session_ids {
        if let Some(vec) = embed_session_cached(storage, embedder, model, &sid).await? {
            let similarity = cosine(&query_vec, &vec).clamp(0.0, 1.0);
            if similarity >= min_similarity {
                scored.push((sid, similarity));
            }
        }
    }
    if scored.is_empty() {
        return Ok(vec![]);
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    scored.truncate(SEARCH_RERANK_CANDIDATES);

    // Precision: rerank the candidates with the cross-encoder. The relevance
    // score replaces cosine as the displayed similarity. Fall back to cosine
    // order if the reranker returns nothing.
    let candidate_ids: Vec<String> = scored.iter().map(|(id, _)| id.clone()).collect();
    let mut contents: Vec<String> = Vec::with_capacity(candidate_ids.len());
    for id in &candidate_ids {
        contents.push(get_session_content(storage, id).await?);
    }
    let reranked = embedder.rerank(query, &contents, None).await?;
    let ordered: Vec<(String, f64)> = if reranked.is_empty() {
        scored
            .iter()
            .map(|(id, s)| (id.clone(), f64::from(*s)))
            .collect()
    } else {
        reranked
            .into_iter()
            .filter_map(|(i, score)| candidate_ids.get(i).map(|id| (id.clone(), score)))
            .collect()
    };

    // Materialize results, applying the mode filter and the limit.
    let mut results = Vec::new();
    for (session_id, score) in ordered {
        if results.len() >= limit as usize {
            break;
        }
        if let Some(result) = load_search_result(storage, &session_id, score as f32).await? {
            if let Some(ref filter) = mode_filter {
                match result.primary_mode.as_deref() {
                    Some(mode) if mode == filter => {}
                    _ => continue,
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
    use crate::storage::{SqliteStorage, StoredThought};
    use crate::traits::MockEmbeddingProvider;

    /// An embedder where any text mentioning "rust" maps to [1, 0] and
    /// everything else to [0, 1], so a "rust" query is similar only to rust
    /// sessions. Rerank preserves cosine order.
    fn topic_embedder() -> MockEmbeddingProvider {
        fn vec_for(text: &str) -> Vec<f32> {
            if text.to_lowercase().contains("rust") {
                vec![1.0_f32, 0.0]
            } else {
                vec![0.0_f32, 1.0]
            }
        }
        let mut m = MockEmbeddingProvider::new();
        // Both query and session embeddings go through embed_contextualized now.
        m.expect_embed_contextualized()
            .returning(|chunks, _input_type| Ok(vec_for(&chunks.join(" "))));
        m.expect_rerank().returning(|_q, docs, _k| {
            Ok((0..docs.len())
                .map(|i| (i, 1.0 - i as f64 * 0.01))
                .collect())
        });
        m
    }

    async fn add_session(storage: &SqliteStorage, mode: &str, text: &str) -> String {
        let session = storage.create_session().await.expect("create session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &session.id,
                mode,
                text,
                0.8,
            ))
            .await
            .expect("save thought");
        session.id
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let results = search_sessions(&storage, &topic_embedder(), "voyage-4", "", 5, 0.0, None)
            .await
            .expect("search");
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_empty_db() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let results = search_sessions(
            &storage,
            &topic_embedder(),
            "voyage-4",
            "rust",
            5,
            0.5,
            None,
        )
        .await
        .expect("search");
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_finds_semantically_similar() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let rust_id = add_session(&storage, "linear", "Rust async programming patterns").await;
        let _other = add_session(&storage, "linear", "cooking pasta recipes").await;

        // Query embeds to the rust vector; only the rust session clears 0.5.
        let results = search_sessions(
            &storage,
            &topic_embedder(),
            "voyage-4",
            "rust",
            5,
            0.5,
            None,
        )
        .await
        .expect("search");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, rust_id);
    }

    #[tokio::test]
    async fn test_search_no_match_below_threshold_is_empty() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        add_session(&storage, "linear", "cooking pasta recipes").await;

        // "rust" query is orthogonal (cosine 0) to the cooking session → empty.
        let results = search_sessions(
            &storage,
            &topic_embedder(),
            "voyage-4",
            "rust",
            5,
            0.5,
            None,
        )
        .await
        .expect("search");
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_mode_filter() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let linear_id = add_session(&storage, "linear", "Rust ownership model").await;
        let _tree = add_session(&storage, "tree", "Rust borrow checker").await;

        let results = search_sessions(
            &storage,
            &topic_embedder(),
            "voyage-4",
            "rust",
            5,
            0.5,
            Some("linear".to_string()),
        )
        .await
        .expect("search");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, linear_id);
    }
}
