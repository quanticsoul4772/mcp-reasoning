//! Session embedding storage (the `session_embeddings` cache).
//!
//! Vectors are persisted as a JSON object `{model, dtype, dim, vector}` in the
//! `embedding_json` column, alongside a `content_hash` so callers can skip
//! re-embedding unchanged sessions. The JSON-object format is forward
//! compatible with quantized dtypes (a later phase) without a schema migration.

#![allow(clippy::missing_errors_doc)]

use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::core::SqliteStorage;
use super::types::StoredEmbedding;
use crate::error::StorageError;

const UPSERT_EMBEDDING: &str =
    "INSERT INTO session_embeddings (session_id, embedding_json, content_hash) \
     VALUES (?, ?, ?) \
     ON CONFLICT(session_id) DO UPDATE SET \
         embedding_json = excluded.embedding_json, \
         content_hash = excluded.content_hash, \
         created_at = datetime('now')";
const SELECT_EMBEDDING: &str =
    "SELECT session_id, embedding_json, content_hash FROM session_embeddings WHERE session_id = ?";
const SELECT_ALL_EMBEDDINGS: &str =
    "SELECT session_id, embedding_json, content_hash FROM session_embeddings";

/// JSON shape stored in `embedding_json`.
#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingPayload {
    model: String,
    dtype: String,
    dim: u32,
    vector: Vec<f32>,
}

impl SqliteStorage {
    /// Insert or replace the cached embedding for a session.
    pub async fn upsert_session_embedding(
        &self,
        emb: &StoredEmbedding,
    ) -> Result<(), StorageError> {
        let payload = EmbeddingPayload {
            model: emb.model.clone(),
            dtype: emb.dtype.clone(),
            dim: emb.dim,
            vector: emb.vector.clone(),
        };
        let embedding_json = serde_json::to_string(&payload)
            .map_err(|e| Self::query_error("serialize embedding", format!("{e}")))?;

        sqlx::query(UPSERT_EMBEDDING)
            .bind(&emb.session_id)
            .bind(&embedding_json)
            .bind(&emb.content_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("UPSERT session_embeddings", format!("{e}")))?;
        Ok(())
    }

    /// Get the cached embedding for a session, if present.
    pub async fn get_session_embedding(
        &self,
        session_id: &str,
    ) -> Result<Option<StoredEmbedding>, StorageError> {
        let row = sqlx::query(SELECT_EMBEDDING)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Self::query_error("SELECT session_embeddings", format!("{e}")))?;
        match row {
            Some(row) => Ok(Some(Self::row_to_embedding(&row)?)),
            None => Ok(None),
        }
    }

    /// Get every cached session embedding (for brute-force similarity ranking).
    pub async fn all_session_embeddings(&self) -> Result<Vec<StoredEmbedding>, StorageError> {
        let rows = sqlx::query(SELECT_ALL_EMBEDDINGS)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Self::query_error("SELECT session_embeddings", format!("{e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            out.push(Self::row_to_embedding(row)?);
        }
        Ok(out)
    }

    fn row_to_embedding(row: &sqlx::sqlite::SqliteRow) -> Result<StoredEmbedding, StorageError> {
        let session_id: String = row.get("session_id");
        let embedding_json: String = row.get("embedding_json");
        let content_hash: String = row.get("content_hash");
        let payload: EmbeddingPayload = serde_json::from_str(&embedding_json)
            .map_err(|e| Self::query_error("parse embedding_json", format!("{e}")))?;
        Ok(StoredEmbedding {
            session_id,
            model: payload.model,
            dtype: payload.dtype,
            dim: payload.dim,
            vector: payload.vector,
            content_hash,
        })
    }
}

/// Stable content hash used to invalidate cached embeddings when a session's
/// content changes. Not cryptographic — it only needs to detect change.
#[must_use]
pub fn content_hash(content: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
mod tests {
    use super::*;

    async fn storage_with_session(id: &str) -> SqliteStorage {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        // session_embeddings has a FK to sessions(id); create the session first.
        sqlx::query("INSERT INTO sessions (id, created_at, updated_at) VALUES (?, datetime('now'), datetime('now'))")
            .bind(id)
            .execute(&storage.get_pool())
            .await
            .expect("seed session");
        storage
    }

    #[tokio::test]
    async fn test_upsert_and_get_roundtrip() {
        let storage = storage_with_session("s1").await;
        let emb = StoredEmbedding::new("s1", "voyage-4", vec![0.1, 0.2, 0.3], "h1");
        storage
            .upsert_session_embedding(&emb)
            .await
            .expect("upsert");

        let got = storage
            .get_session_embedding("s1")
            .await
            .expect("get")
            .expect("present");
        assert_eq!(got, emb);
        assert_eq!(got.dim, 3);
        assert_eq!(got.dtype, "float");
    }

    #[tokio::test]
    async fn test_upsert_overwrites_existing() {
        let storage = storage_with_session("s1").await;
        storage
            .upsert_session_embedding(&StoredEmbedding::new("s1", "voyage-4", vec![1.0], "old"))
            .await
            .expect("first");
        storage
            .upsert_session_embedding(&StoredEmbedding::new(
                "s1",
                "voyage-4",
                vec![2.0, 3.0],
                "new",
            ))
            .await
            .expect("second");

        let got = storage.get_session_embedding("s1").await.unwrap().unwrap();
        assert_eq!(got.vector, vec![2.0, 3.0]);
        assert_eq!(got.content_hash, "new");
        // Still exactly one row for the session.
        assert_eq!(storage.all_session_embeddings().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_get_missing_returns_none() {
        let storage = SqliteStorage::new_in_memory().await.expect("storage");
        assert!(storage
            .get_session_embedding("nope")
            .await
            .expect("get")
            .is_none());
        assert!(storage.all_session_embeddings().await.unwrap().is_empty());
    }

    #[test]
    fn test_content_hash_is_stable_and_distinguishes() {
        assert_eq!(content_hash("hello world"), content_hash("hello world"));
        assert_ne!(content_hash("hello world"), content_hash("hello  world"));
    }
}
