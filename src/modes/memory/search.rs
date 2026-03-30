//! Full-text search over reasoning sessions using SQLite FTS5.
//!
//! Replaces hash-based embedding similarity with BM25-ranked keyword search.
//! Sessions are ranked by relevance using SQLite's built-in BM25 algorithm;
//! scores are normalized to [0.0, 1.0] within the result set.

use std::collections::HashMap;

use crate::error::ModeError;
use crate::modes::memory::types::SearchResult;
use crate::storage::SqliteStorage;
use crate::traits::AnthropicClientTrait;
use sqlx::Row;

/// Search reasoning sessions using full-text BM25 search.
///
/// # Arguments
///
/// * `storage` - Storage implementation
/// * `_client` - Unused (kept for API compatibility with future embedding upgrades)
/// * `query` - Search query (tokenized by SQLite FTS5)
/// * `limit` - Maximum number of results
/// * `min_similarity` - Minimum normalized similarity threshold (0.0–1.0).
///   With BM25 normalization, all matched sessions have similarity ≥ 0.5.
///   Use 0.5 (the default) to return all matches; use higher values to require
///   the session to be in the top fraction of matched sessions.
/// * `mode_filter` - Optional filter by reasoning mode
///
/// # Returns
///
/// Search results sorted by relevance (best match first)
pub async fn search_sessions<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    _client: &C,
    query: &str,
    limit: u32,
    min_similarity: f32,
    mode_filter: Option<String>,
) -> Result<Vec<SearchResult>, ModeError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    // Sanitize query for FTS5: escape double-quotes by doubling them
    let safe_query = query.replace('"', "\"\"");

    // FTS5 MATCH query: get all matching thoughts with BM25 rank score.
    // bm25() returns negative values; more negative = more relevant.
    // We fetch more than needed to allow deduplication by session_id.
    let rows = sqlx::query(
        r"
        SELECT
            session_id,
            bm25(thoughts_fts) AS score
        FROM thoughts_fts
        WHERE thoughts_fts MATCH ?
        ORDER BY score ASC
        LIMIT ?
        ",
    )
    .bind(&safe_query)
    .bind(i64::from(limit.saturating_mul(10))) // fetch extra for dedup + mode filtering
    .fetch_all(&storage.get_pool())
    .await
    .map_err(|e| ModeError::StorageError {
        message: format!("FTS5 search failed: {e}"),
    })?;

    if rows.is_empty() {
        return Ok(vec![]);
    }

    // Deduplicate by session_id, keeping the best (most negative) BM25 score per session.
    // Preserves the ORDER BY score ASC ordering from the query.
    let mut best_per_session: HashMap<String, f64> = HashMap::new();
    let mut session_order: Vec<String> = Vec::new();
    for row in &rows {
        let session_id: String = row.get("session_id");
        let score: f64 = row.get("score");
        let entry = best_per_session
            .entry(session_id.clone())
            .or_insert(f64::MAX);
        if score < *entry {
            *entry = score;
        }
        if !session_order.contains(&session_id) {
            session_order.push(session_id);
        }
    }

    // Sort sessions by their best score (most negative = most relevant first)
    session_order.sort_by(|a, b| {
        let score_a = best_per_session[a];
        let score_b = best_per_session[b];
        score_a
            .partial_cmp(&score_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Normalize BM25 scores to [0.5, 1.0]:
    //   rank 1 (best match) → 1.0, rank N (worst match) → 0.5
    let count = session_order.len();
    let scored: Vec<(String, f32)> = session_order
        .into_iter()
        .enumerate()
        .map(|(i, session_id)| {
            let similarity = if count == 1 {
                1.0_f32
            } else {
                1.0 - (i as f32 / (count - 1) as f32) * 0.5
            };
            (session_id, similarity)
        })
        .filter(|(_, sim)| *sim >= min_similarity)
        .collect();

    // Load full session data, applying mode filter
    let mut results = Vec::new();
    for (session_id, similarity_score) in scored {
        if results.len() >= limit as usize {
            break;
        }
        if let Some(result) = load_search_result(storage, &session_id, similarity_score).await? {
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
    use crate::test_utils::mock_anthropic_success;

    #[tokio::test]
    async fn test_search_empty_db() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let results = search_sessions(&storage, &client, "test query", 5, 0.5, None)
            .await
            .expect("search sessions");

        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_empty_query() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let results = search_sessions(&storage, &client, "", 5, 0.0, None)
            .await
            .expect("search sessions");

        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_finds_matching_session() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let session = storage.create_session().await.expect("create session");
        let thought = StoredThought::new(
            uuid::Uuid::new_v4().to_string(),
            &session.id,
            "linear",
            "Deep analysis of Rust async programming patterns",
            0.8,
        );
        storage
            .save_stored_thought(&thought)
            .await
            .expect("save thought");

        let results = search_sessions(&storage, &client, "async programming", 5, 0.0, None)
            .await
            .expect("search sessions");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session.id);
        assert!(results[0].similarity_score > 0.0);
    }

    #[tokio::test]
    async fn test_search_no_match_returns_empty() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let session = storage.create_session().await.expect("create session");
        let thought = StoredThought::new(
            uuid::Uuid::new_v4().to_string(),
            &session.id,
            "linear",
            "Discussion about machine learning neural networks",
            0.8,
        );
        storage
            .save_stored_thought(&thought)
            .await
            .expect("save thought");

        // Search for something completely unrelated
        let results = search_sessions(&storage, &client, "cooking recipes", 5, 0.5, None)
            .await
            .expect("search sessions");

        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_ranking_best_first() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        // Session 1: mentions Rust once
        let session1 = storage.create_session().await.expect("create session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &session1.id,
                "linear",
                "Rust programming language overview",
                0.8,
            ))
            .await
            .expect("save thought");

        // Session 2: mentions Rust multiple times (should score higher)
        let session2 = storage.create_session().await.expect("create session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &session2.id,
                "linear",
                "Rust Rust Rust: comprehensive guide to Rust borrow checker and Rust lifetimes",
                0.8,
            ))
            .await
            .expect("save thought");

        let results = search_sessions(&storage, &client, "Rust", 5, 0.0, None)
            .await
            .expect("search sessions");

        assert_eq!(results.len(), 2);
        // Best match should have highest similarity score
        assert!(results[0].similarity_score >= results[1].similarity_score);
        // Best match is the one with more Rust mentions
        assert_eq!(results[0].session_id, session2.id);
    }

    #[tokio::test]
    async fn test_search_mode_filter() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let session1 = storage.create_session().await.expect("create session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &session1.id,
                "linear",
                "algorithm analysis and complexity",
                0.8,
            ))
            .await
            .expect("save thought");

        let session2 = storage.create_session().await.expect("create session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &session2.id,
                "tree",
                "algorithm branching and complexity",
                0.8,
            ))
            .await
            .expect("save thought");

        // Filter to only linear sessions
        let results = search_sessions(
            &storage,
            &client,
            "algorithm complexity",
            5,
            0.0,
            Some("linear".to_string()),
        )
        .await
        .expect("search sessions");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session1.id);
    }

    #[tokio::test]
    async fn test_search_mode_filter_excludes_no_mode_sessions() {
        // Regression test: sessions with no matching mode should be excluded
        // even if primary_mode is None (previously they slipped through the filter)
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        // Session using "linear" mode
        let s_linear = storage.create_session().await.expect("create session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &s_linear.id,
                "linear",
                "graph traversal algorithm depth first",
                0.8,
            ))
            .await
            .expect("save thought");

        // Session using "tree" mode
        let s_tree = storage.create_session().await.expect("create session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &s_tree.id,
                "tree",
                "graph traversal algorithm breadth first",
                0.8,
            ))
            .await
            .expect("save thought");

        // Search with mode_filter = "linear" — only s_linear should appear
        let results = search_sessions(
            &storage,
            &client,
            "graph traversal algorithm",
            5,
            0.0,
            Some("linear".to_string()),
        )
        .await
        .expect("search sessions");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, s_linear.id);
    }
}
