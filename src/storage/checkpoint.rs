//! Checkpoint storage operations.

#![allow(clippy::missing_errors_doc)]

use crate::error::StorageError;
use sqlx::Row;

use super::core::SqliteStorage;
use super::types::StoredCheckpoint;

impl SqliteStorage {
    /// Save a checkpoint to the database.
    pub async fn save_checkpoint(&self, checkpoint: &StoredCheckpoint) -> Result<(), StorageError> {
        let created_at_str = checkpoint.created_at.to_rfc3339();

        sqlx::query(
            "INSERT INTO checkpoints (id, session_id, name, description, state, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&checkpoint.id)
        .bind(&checkpoint.session_id)
        .bind(&checkpoint.name)
        .bind(&checkpoint.description)
        .bind(&checkpoint.state)
        .bind(&created_at_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::query_error("INSERT checkpoints", format!("{e}")))?;

        Ok(())
    }

    /// Get a checkpoint by ID.
    pub async fn get_checkpoint(&self, id: &str) -> Result<Option<StoredCheckpoint>, StorageError> {
        let row = sqlx::query(
            "SELECT id, session_id, name, description, state, created_at
             FROM checkpoints WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT checkpoints", format!("{e}")))?;

        match row {
            Some(row) => {
                let checkpoint = Self::row_to_checkpoint(&row)?;
                Ok(Some(checkpoint))
            }
            None => Ok(None),
        }
    }

    /// Get all checkpoints for a session.
    pub async fn get_checkpoints(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredCheckpoint>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, session_id, name, description, state, created_at
             FROM checkpoints WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT checkpoints", format!("{e}")))?;

        let mut checkpoints = Vec::with_capacity(rows.len());
        for row in &rows {
            checkpoints.push(Self::row_to_checkpoint(row)?);
        }

        Ok(checkpoints)
    }

    /// Convert a database row to a `StoredCheckpoint`.
    fn row_to_checkpoint(row: &sqlx::sqlite::SqliteRow) -> Result<StoredCheckpoint, StorageError> {
        let id: String = row.get("id");
        let session_id: String = row.get("session_id");
        let name: String = row.get("name");
        let description: Option<String> = row.get("description");
        let state: String = row.get("state");
        let created_at_str: String = row.get("created_at");

        let created_at = Self::parse_datetime(&created_at_str)?;

        let mut checkpoint = StoredCheckpoint::new(&id, &session_id, &name, &state);
        checkpoint.created_at = created_at;

        if let Some(d) = description {
            checkpoint = checkpoint.with_description(d);
        }

        Ok(checkpoint)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::core::tests::test_storage;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_save_checkpoint() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let checkpoint = StoredCheckpoint::new("cp-1", "sess-123", "Checkpoint 1", "{}");
        let result = storage.save_checkpoint(&checkpoint).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_checkpoint() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let checkpoint = StoredCheckpoint::new("cp-1", "sess-123", "Checkpoint 1", "{}")
            .with_description("Test checkpoint");
        storage.save_checkpoint(&checkpoint).await.expect("save");

        let fetched = storage.get_checkpoint("cp-1").await;
        assert!(fetched.is_ok());
        let fetched = fetched.expect("fetch").expect("checkpoint exists");
        assert_eq!(fetched.id, "cp-1");
        assert_eq!(fetched.name, "Checkpoint 1");
        assert_eq!(fetched.description, Some("Test checkpoint".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn test_get_checkpoints() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let cp1 = StoredCheckpoint::new("cp-1", "sess-123", "First", "{}");
        let cp2 = StoredCheckpoint::new("cp-2", "sess-123", "Second", "{}");

        storage.save_checkpoint(&cp1).await.expect("save 1");
        storage.save_checkpoint(&cp2).await.expect("save 2");

        let checkpoints = storage.get_checkpoints("sess-123").await;
        assert!(checkpoints.is_ok());
        let checkpoints = checkpoints.expect("checkpoints");
        assert_eq!(checkpoints.len(), 2);
    }
}
