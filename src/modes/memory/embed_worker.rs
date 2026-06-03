//! Background embedding worker.
//!
//! Drains the [`embedding_queue`](crate::storage) and warms each session's
//! cached embedding via [`embed_session_cached`], so the embedding cost is paid
//! ahead of time instead of on the first `reasoning_search` / `reasoning_relate`.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use crate::error::ModeError;
use crate::modes::memory::similarity::embed_session_cached;
use crate::storage::SqliteStorage;
use crate::traits::EmbeddingProvider;

/// Jobs drained per worker tick.
const EMBED_BATCH_SIZE: u32 = 16;

/// Result of draining one batch of the embedding queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmbedBatchOutcome {
    /// Jobs whose session embedding was warmed (or that had no content).
    pub processed: usize,
    /// Jobs whose embedding attempt errored.
    pub failed: usize,
}

/// Drain up to `batch_size` pending jobs, warming each session's embedding and
/// marking the job processed or failed. Returns the per-batch counts.
pub async fn process_pending_batch<E: EmbeddingProvider>(
    storage: &SqliteStorage,
    embedder: &E,
    model: &str,
    batch_size: u32,
) -> Result<EmbedBatchOutcome, ModeError> {
    let pending = storage
        .dequeue_pending_embeddings(batch_size)
        .await
        .map_err(|e| ModeError::StorageError {
            message: e.to_string(),
        })?;

    let mut outcome = EmbedBatchOutcome::default();
    for (id, session_id) in pending {
        match embed_session_cached(storage, embedder, model, &session_id).await {
            Ok(_) => {
                if let Err(e) = storage.mark_embedding_processed(id).await {
                    tracing::warn!(id, error = %e, "Failed to mark embedding job processed");
                }
                outcome.processed += 1;
            }
            Err(e) => {
                if let Err(e2) = storage.mark_embedding_failed(id, &e.to_string()).await {
                    tracing::warn!(id, error = %e2, "Failed to mark embedding job failed");
                }
                tracing::warn!(session_id, error = %e, "Background embedding failed");
                outcome.failed += 1;
            }
        }
    }
    Ok(outcome)
}

/// Run the embedding worker until `shutdown_rx` flips to `true`, draining the
/// queue on a fixed interval.
#[cfg_attr(coverage_nightly, coverage(off))]
pub async fn run_embed_worker<E: EmbeddingProvider + Send + Sync + 'static>(
    storage: Arc<SqliteStorage>,
    embedder: Arc<E>,
    model: String,
    interval: Duration,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                match process_pending_batch(storage.as_ref(), embedder.as_ref(), &model, EMBED_BATCH_SIZE).await {
                    Ok(o) if o.processed + o.failed > 0 => {
                        tracing::info!(processed = o.processed, failed = o.failed, "Embedding worker drained batch");
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "Embedding worker batch errored"),
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::StoredThought;
    use crate::traits::MockEmbeddingProvider;

    async fn storage_with_thought(session_id: &str, text: &str) -> SqliteStorage {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let session = storage.create_session().await.expect("session");
        // Re-key the seeded session id so the test can reference it directly.
        sqlx::query("UPDATE sessions SET id = ? WHERE id = ?")
            .bind(session_id)
            .bind(&session.id)
            .execute(&storage.get_pool())
            .await
            .expect("rekey session");
        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                session_id,
                "linear",
                text,
                0.8,
            ))
            .await
            .expect("save thought");
        storage
    }

    #[tokio::test]
    async fn test_process_batch_warms_cache_and_marks_processed() {
        let storage = storage_with_thought("s1", "Rust ownership and borrowing").await;

        let mut embedder = MockEmbeddingProvider::new();
        embedder
            .expect_embed_documents()
            .returning(|texts| Ok(texts.iter().map(|_| vec![0.5_f32, 0.5]).collect()));

        let outcome = process_pending_batch(&storage, &embedder, "voyage-4", 16)
            .await
            .expect("batch");
        assert_eq!(outcome.processed, 1);
        assert_eq!(outcome.failed, 0);

        // The cache is now warm and the job is no longer pending.
        assert!(storage
            .get_session_embedding("s1")
            .await
            .expect("get")
            .is_some());
        assert!(storage
            .dequeue_pending_embeddings(16)
            .await
            .expect("dequeue")
            .is_empty());
    }

    #[tokio::test]
    async fn test_process_batch_marks_failed_on_embed_error() {
        let storage = storage_with_thought("s1", "some content").await;

        let mut embedder = MockEmbeddingProvider::new();
        embedder.expect_embed_documents().returning(|_| {
            Err(ModeError::ApiUnavailable {
                message: "voyage down".to_string(),
            })
        });

        let outcome = process_pending_batch(&storage, &embedder, "voyage-4", 16)
            .await
            .expect("batch");
        assert_eq!(outcome.processed, 0);
        assert_eq!(outcome.failed, 1);
        assert!(storage
            .dequeue_pending_embeddings(16)
            .await
            .expect("dequeue")
            .is_empty());
    }

    #[tokio::test]
    async fn test_process_batch_empty_queue_is_noop() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        let embedder = MockEmbeddingProvider::new();
        let outcome = process_pending_batch(&storage, &embedder, "voyage-4", 16)
            .await
            .expect("batch");
        assert_eq!(outcome, EmbedBatchOutcome::default());
    }
}
