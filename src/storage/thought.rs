//! Thought storage operations.

#![allow(clippy::missing_errors_doc)]

use crate::error::StorageError;
use sqlx::Row;

use super::core::SqliteStorage;
use super::types::StoredThought;

impl SqliteStorage {
    /// Save a stored thought to the database.
    pub async fn save_stored_thought(&self, thought: &StoredThought) -> Result<(), StorageError> {
        let created_at_str = thought.created_at.to_rfc3339();

        sqlx::query(
            "INSERT INTO thoughts (id, session_id, parent_id, mode, content, confidence, metadata, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&thought.id)
        .bind(&thought.session_id)
        .bind(&thought.parent_id)
        .bind(&thought.mode)
        .bind(&thought.content)
        .bind(thought.confidence)
        .bind(&thought.metadata)
        .bind(&created_at_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::query_error("INSERT thoughts", format!("{e}")))?;

        Ok(())
    }

    /// Get a stored thought by ID.
    pub async fn get_stored_thought(
        &self,
        id: &str,
    ) -> Result<Option<StoredThought>, StorageError> {
        let row = sqlx::query(
            "SELECT id, session_id, parent_id, mode, content, confidence, metadata, created_at
             FROM thoughts WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT thoughts", format!("{e}")))?;

        match row {
            Some(row) => {
                let thought = Self::row_to_stored_thought(&row)?;
                Ok(Some(thought))
            }
            None => Ok(None),
        }
    }

    /// Get all stored thoughts for a session.
    pub async fn get_stored_thoughts(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredThought>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, session_id, parent_id, mode, content, confidence, metadata, created_at
             FROM thoughts WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Self::query_error("SELECT thoughts", format!("{e}")))?;

        let mut thoughts = Vec::with_capacity(rows.len());
        for row in &rows {
            thoughts.push(Self::row_to_stored_thought(row)?);
        }

        Ok(thoughts)
    }

    /// Convert a database row to a `StoredThought`.
    fn row_to_stored_thought(row: &sqlx::sqlite::SqliteRow) -> Result<StoredThought, StorageError> {
        let id: String = row.get("id");
        let session_id: String = row.get("session_id");
        let parent_id: Option<String> = row.get("parent_id");
        let mode: String = row.get("mode");
        let content: String = row.get("content");
        let confidence: f64 = row.get("confidence");
        let metadata: Option<String> = row.get("metadata");
        let created_at_str: String = row.get("created_at");

        let created_at = Self::parse_datetime(&created_at_str)?;

        let mut thought = StoredThought::new(&id, &session_id, &mode, &content, confidence)
            .with_timestamp(created_at);

        if let Some(p) = parent_id {
            thought = thought.with_parent(p);
        }
        if let Some(m) = metadata {
            thought = thought.with_metadata(m);
        }

        Ok(thought)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::core::tests::test_storage;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_save_stored_thought() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let thought = StoredThought::new("t-1", "sess-123", "linear", "Test content", 0.85);
        let result = storage.save_stored_thought(&thought).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_stored_thought() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let thought = StoredThought::new("t-1", "sess-123", "linear", "Test content", 0.85);
        storage.save_stored_thought(&thought).await.expect("save");

        let fetched = storage.get_stored_thought("t-1").await;
        assert!(fetched.is_ok());
        let fetched = fetched.expect("fetch").expect("thought exists");
        assert_eq!(fetched.id, "t-1");
        assert_eq!(fetched.content, "Test content");
        assert!((fetched.confidence - 0.85).abs() < f64::EPSILON);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_stored_thought_not_found() {
        let storage = test_storage().await;
        let result = storage.get_stored_thought("nonexistent").await;

        assert!(result.is_ok());
        assert!(result.expect("result").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_stored_thoughts() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let thought1 = StoredThought::new("t-1", "sess-123", "linear", "First", 0.8);
        let thought2 = StoredThought::new("t-2", "sess-123", "linear", "Second", 0.9);

        storage
            .save_stored_thought(&thought1)
            .await
            .expect("save 1");
        storage
            .save_stored_thought(&thought2)
            .await
            .expect("save 2");

        let thoughts = storage.get_stored_thoughts("sess-123").await;
        assert!(thoughts.is_ok());
        let thoughts = thoughts.expect("thoughts");
        assert_eq!(thoughts.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_thought_with_parent() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let parent = StoredThought::new("t-1", "sess-123", "linear", "Parent", 0.8);
        let child =
            StoredThought::new("t-2", "sess-123", "linear", "Child", 0.9).with_parent("t-1");

        storage
            .save_stored_thought(&parent)
            .await
            .expect("save parent");
        storage
            .save_stored_thought(&child)
            .await
            .expect("save child");

        let fetched = storage
            .get_stored_thought("t-2")
            .await
            .expect("fetch")
            .expect("exists");
        assert_eq!(fetched.parent_id, Some("t-1".to_string()));
    }
}
