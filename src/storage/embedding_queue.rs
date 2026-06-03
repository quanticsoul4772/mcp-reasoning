//! Background embedding queue (`embedding_queue` table).
//!
//! A thought write enqueues its session for (re)embedding; a background worker
//! drains pending rows, warms the [`session_embeddings`](super::embeddings)
//! cache, and marks each row processed or failed. This moves the embedding cost
//! off the first `reasoning_search` / `reasoning_relate` call.
//!
//! Enqueue is idempotent per session: at most one `pending` row exists for a
//! session at a time, so repeated writes between drains collapse to one job.

#![allow(clippy::missing_errors_doc)]

use sqlx::Row;

use super::core::SqliteStorage;
use crate::error::StorageError;

/// Insert a pending job only when the session has no pending job already.
const ENQUEUE_EMBEDDING: &str = "INSERT INTO embedding_queue (session_id, status) \
     SELECT ?, 'pending' \
     WHERE NOT EXISTS ( \
         SELECT 1 FROM embedding_queue WHERE session_id = ? AND status = 'pending' \
     )";
const DEQUEUE_PENDING: &str =
    "SELECT id, session_id FROM embedding_queue WHERE status = 'pending' ORDER BY id LIMIT ?";
const MARK_PROCESSED: &str =
    "UPDATE embedding_queue SET status = 'processed', processed_at = datetime('now') WHERE id = ?";
const MARK_FAILED: &str = "UPDATE embedding_queue SET status = 'failed', \
     attempts = attempts + 1, error_message = ?, processed_at = datetime('now') WHERE id = ?";

impl SqliteStorage {
    /// Enqueue a session for background (re)embedding. Idempotent: a no-op when a
    /// `pending` job already exists for the session.
    pub async fn enqueue_embedding(&self, session_id: &str) -> Result<(), StorageError> {
        sqlx::query(ENQUEUE_EMBEDDING)
            .bind(session_id)
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("INSERT embedding_queue", format!("{e}")))?;
        Ok(())
    }

    /// Fetch up to `limit` pending jobs as `(queue_id, session_id)`, oldest first.
    pub async fn dequeue_pending_embeddings(
        &self,
        limit: u32,
    ) -> Result<Vec<(i64, String)>, StorageError> {
        let rows = sqlx::query(DEQUEUE_PENDING)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Self::query_error("SELECT embedding_queue", format!("{e}")))?;
        Ok(rows
            .iter()
            .map(|row| (row.get::<i64, _>("id"), row.get::<String, _>("session_id")))
            .collect())
    }

    /// Mark a queue job as successfully processed.
    pub async fn mark_embedding_processed(&self, id: i64) -> Result<(), StorageError> {
        sqlx::query(MARK_PROCESSED)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("UPDATE embedding_queue processed", format!("{e}")))?;
        Ok(())
    }

    /// Mark a queue job as failed, recording the error and bumping `attempts`.
    pub async fn mark_embedding_failed(&self, id: i64, error: &str) -> Result<(), StorageError> {
        sqlx::query(MARK_FAILED)
            .bind(error)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("UPDATE embedding_queue failed", format!("{e}")))?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    async fn storage_with_session(id: &str) -> SqliteStorage {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        sqlx::query("INSERT INTO sessions (id, created_at, updated_at) VALUES (?, datetime('now'), datetime('now'))")
            .bind(id)
            .execute(&storage.get_pool())
            .await
            .expect("seed session");
        storage
    }

    #[tokio::test]
    async fn test_enqueue_is_idempotent_while_pending() {
        let storage = storage_with_session("s1").await;
        storage.enqueue_embedding("s1").await.expect("enqueue 1");
        storage.enqueue_embedding("s1").await.expect("enqueue 2");

        let pending = storage
            .dequeue_pending_embeddings(10)
            .await
            .expect("dequeue");
        assert_eq!(
            pending.len(),
            1,
            "second enqueue must not add a second pending row"
        );
        assert_eq!(pending[0].1, "s1");
    }

    #[tokio::test]
    async fn test_processed_job_frees_the_session_to_requeue() {
        let storage = storage_with_session("s1").await;
        storage.enqueue_embedding("s1").await.expect("enqueue");
        let pending = storage
            .dequeue_pending_embeddings(10)
            .await
            .expect("dequeue");
        let id = pending[0].0;

        storage
            .mark_embedding_processed(id)
            .await
            .expect("processed");
        assert!(storage
            .dequeue_pending_embeddings(10)
            .await
            .expect("dequeue")
            .is_empty());

        // A later write re-enqueues because no pending row remains.
        storage.enqueue_embedding("s1").await.expect("re-enqueue");
        assert_eq!(
            storage
                .dequeue_pending_embeddings(10)
                .await
                .expect("dequeue")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn test_mark_failed_records_error_and_clears_pending() {
        let storage = storage_with_session("s1").await;
        storage.enqueue_embedding("s1").await.expect("enqueue");
        let id = storage
            .dequeue_pending_embeddings(10)
            .await
            .expect("dequeue")[0]
            .0;

        storage
            .mark_embedding_failed(id, "boom")
            .await
            .expect("failed");

        // No longer pending.
        assert!(storage
            .dequeue_pending_embeddings(10)
            .await
            .expect("dequeue")
            .is_empty());
        let (status, error): (String, String) =
            sqlx::query_as("SELECT status, error_message FROM embedding_queue WHERE id = ?")
                .bind(id)
                .fetch_one(&storage.get_pool())
                .await
                .expect("row");
        assert_eq!(status, "failed");
        assert_eq!(error, "boom");
    }

    #[tokio::test]
    async fn test_dequeue_respects_limit_and_order() {
        let storage = storage_with_session("s1").await;
        sqlx::query("INSERT INTO sessions (id, created_at, updated_at) VALUES ('s2', datetime('now'), datetime('now'))")
            .execute(&storage.get_pool())
            .await
            .expect("seed s2");
        storage.enqueue_embedding("s1").await.expect("enqueue s1");
        storage.enqueue_embedding("s2").await.expect("enqueue s2");

        let pending = storage
            .dequeue_pending_embeddings(1)
            .await
            .expect("dequeue");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].1, "s1", "oldest job first");
    }
}
