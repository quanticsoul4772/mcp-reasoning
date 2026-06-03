//! Vector-similarity helpers for semantic session relate/search.
//!
//! Provides cosine similarity and a content-hash-cached session embedding so the
//! memory mode can rank sessions by meaning (via [`EmbeddingProvider`]) instead
//! of keyword overlap.

use crate::error::ModeError;
use crate::modes::memory::embeddings::get_session_content;
use crate::storage::{content_hash, SqliteStorage, StoredEmbedding};
use crate::traits::EmbeddingProvider;

/// Cosine similarity of two vectors, in [-1.0, 1.0]. Returns 0.0 when the
/// vectors differ in length, are empty, or either has zero magnitude.
#[must_use]
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

/// Return a session's embedding, reusing the cached vector when the session
/// content is unchanged and (re)computing + caching it otherwise.
///
/// Returns `None` when the session has no content to embed.
pub async fn embed_session_cached<E: EmbeddingProvider>(
    storage: &SqliteStorage,
    embedder: &E,
    model: &str,
    session_id: &str,
) -> Result<Option<Vec<f32>>, ModeError> {
    let content = get_session_content(storage, session_id).await?;
    if content.trim().is_empty() {
        return Ok(None);
    }
    let hash = content_hash(&content);

    if let Some(cached) = storage
        .get_session_embedding(session_id)
        .await
        .map_err(|e| ModeError::StorageError {
            message: e.to_string(),
        })?
    {
        // Keyed on BOTH content and model: vectors from a different model live
        // in a different space, so a model change is a miss (recompute). This
        // matters here because the cache may hold voyage-context-3 vectors from
        // a previous build that must not be reused under voyage-4.
        if cached.content_hash == hash && cached.model == model {
            return Ok(Some(cached.vector));
        }
    }

    let mut vectors = embedder.embed_documents(&[content]).await?;
    let vector = vectors.pop().ok_or_else(|| ModeError::ParseError {
        message: "embedding provider returned no vector".to_string(),
    })?;
    let stored = StoredEmbedding::new(session_id, model, vector.clone(), hash);
    if let Err(e) = storage.upsert_session_embedding(&stored).await {
        tracing::warn!(error = %e, session_id, "Failed to cache session embedding");
    }
    Ok(Some(vector))
}

/// Objective novelty for a set of texts, grounded in their embeddings.
///
/// Each text's novelty is its distance from its nearest neighbour among the
/// others — `1 - max_cosine` — clamped to `[0,1]`. A text with a near-duplicate
/// scores ~0; one unlike everything else scores ~1. With 0 or 1 texts every
/// score is `1.0` (nothing to be similar to). Returns one score per input,
/// in input order.
pub async fn novelty_scores<E: EmbeddingProvider>(
    embedder: &E,
    texts: &[String],
) -> Result<Vec<f64>, ModeError> {
    if texts.len() <= 1 {
        return Ok(vec![1.0; texts.len()]);
    }
    let vectors = embedder.embed_documents(texts).await?;
    if vectors.len() != texts.len() {
        return Err(ModeError::ParseError {
            message: format!(
                "embedding provider returned {} vectors for {} texts",
                vectors.len(),
                texts.len()
            ),
        });
    }
    let scores = (0..vectors.len())
        .map(|i| {
            let max_sim = (0..vectors.len())
                .filter(|&j| j != i)
                .map(|j| cosine(&vectors[i], &vectors[j]))
                .fold(f32::MIN, f32::max);
            f64::from((1.0 - max_sim).clamp(0.0, 1.0))
        })
        .collect();
    Ok(scores)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical_is_one() {
        assert!((cosine(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal_is_zero() {
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_opposite_is_negative_one() {
        assert!((cosine(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_degenerate_inputs_are_zero() {
        assert_eq!(cosine(&[], &[]), 0.0);
        assert_eq!(cosine(&[1.0, 2.0], &[1.0]), 0.0);
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    }

    #[tokio::test]
    async fn test_novelty_scores_flags_near_duplicates() {
        use crate::traits::MockEmbeddingProvider;
        let mut m = MockEmbeddingProvider::new();
        // First two texts embed identically; the third is orthogonal to both.
        m.expect_embed_documents().returning(|texts| {
            Ok(texts
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    if i < 2 {
                        vec![1.0_f32, 0.0]
                    } else {
                        vec![0.0_f32, 1.0]
                    }
                })
                .collect())
        });
        let texts = vec![
            "a".to_string(),
            "a-dup".to_string(),
            "different".to_string(),
        ];
        let n = novelty_scores(&m, &texts).await.expect("novelty");
        // The duplicated pair each have a near-identical neighbour → novelty ~0.
        assert!(n[0] < 1e-6 && n[1] < 1e-6);
        // The orthogonal one is unlike both → max cosine 0 → novelty 1.0.
        assert!((n[2] - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_novelty_scores_single_text_is_one() {
        use crate::traits::MockEmbeddingProvider;
        // 0 or 1 texts: nothing to compare against, embedder is never called.
        let m = MockEmbeddingProvider::new();
        assert_eq!(
            novelty_scores(&m, &["only".to_string()])
                .await
                .expect("novelty"),
            vec![1.0]
        );
        assert!(novelty_scores(&m, &[]).await.expect("novelty").is_empty());
    }

    #[tokio::test]
    async fn test_embed_session_cached_reuses_cache_on_second_call() {
        use crate::storage::{SqliteStorage, StoredThought};
        use crate::traits::MockEmbeddingProvider;

        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let session = storage.create_session().await.expect("session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &session.id,
                "linear",
                "some session content to embed",
                0.8,
            ))
            .await
            .expect("thought");

        let mut embedder = MockEmbeddingProvider::new();
        // times(1): the second call must hit the content-hash cache, not the API.
        embedder
            .expect_embed_documents()
            .times(1)
            .returning(|texts| Ok(texts.iter().map(|_| vec![0.5_f32, 0.5]).collect()));

        let v1 = embed_session_cached(&storage, &embedder, "voyage-4", &session.id)
            .await
            .expect("embed")
            .expect("vector");
        let v2 = embed_session_cached(&storage, &embedder, "voyage-4", &session.id)
            .await
            .expect("embed")
            .expect("vector");
        assert_eq!(v1, v2);
    }

    #[tokio::test]
    async fn test_embed_session_cached_empty_session_is_none() {
        use crate::storage::SqliteStorage;
        use crate::traits::MockEmbeddingProvider;

        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let session = storage.create_session().await.expect("session");
        // No thoughts → no content → no embedding, embedder never called.
        let embedder = MockEmbeddingProvider::new();
        let out = embed_session_cached(&storage, &embedder, "voyage-4", &session.id)
            .await
            .expect("embed");
        assert!(out.is_none());
    }
}
