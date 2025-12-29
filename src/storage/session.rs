//! Session storage operations.

#![allow(clippy::missing_errors_doc)]

use crate::error::StorageError;
use chrono::Utc;
use sqlx::Row;

use super::core::SqliteStorage;
use super::types::StoredSession;

impl SqliteStorage {
    /// Create a new session.
    pub async fn create_session(&self) -> Result<StoredSession, StorageError> {
        let id = Self::generate_id();
        self.create_session_with_id(&id).await
    }

    /// Create a new session with a specific ID.
    pub async fn create_session_with_id(&self, id: &str) -> Result<StoredSession, StorageError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        sqlx::query("INSERT INTO sessions (id, created_at, updated_at) VALUES (?, ?, ?)")
            .bind(id)
            .bind(&now_str)
            .bind(&now_str)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("INSERT sessions", format!("{e}")))?;

        Ok(StoredSession::with_timestamps(id, now, now))
    }

    /// Get a stored session by ID.
    pub async fn get_stored_session(
        &self,
        id: &str,
    ) -> Result<Option<StoredSession>, StorageError> {
        let row =
            sqlx::query("SELECT id, created_at, updated_at, metadata FROM sessions WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Self::query_error("SELECT sessions", format!("{e}")))?;

        match row {
            Some(row) => {
                let id: String = row.get("id");
                let created_at_str: String = row.get("created_at");
                let updated_at_str: String = row.get("updated_at");
                let metadata: Option<String> = row.get("metadata");

                let created_at = Self::parse_datetime(&created_at_str)?;
                let updated_at = Self::parse_datetime(&updated_at_str)?;

                let mut session = StoredSession::with_timestamps(&id, created_at, updated_at);
                if let Some(m) = metadata {
                    session = session.with_metadata(m);
                }

                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    /// Update session's `updated_at` timestamp.
    pub async fn touch_session(&self, id: &str) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query("UPDATE sessions SET updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("UPDATE sessions", format!("{e}")))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::SessionNotFound {
                session_id: id.to_string(),
            });
        }

        Ok(())
    }

    /// Delete a session and all related data.
    pub async fn delete_session(&self, id: &str) -> Result<(), StorageError> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Self::query_error("DELETE sessions", format!("{e}")))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::SessionNotFound {
                session_id: id.to_string(),
            });
        }

        Ok(())
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
    async fn test_create_session() {
        let storage = test_storage().await;
        let session = storage.create_session().await;

        assert!(session.is_ok());
        let session = session.expect("session");
        assert!(!session.id.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_create_session_with_id() {
        let storage = test_storage().await;
        let session = storage.create_session_with_id("custom-id").await;

        assert!(session.is_ok());
        let session = session.expect("session");
        assert_eq!(session.id, "custom-id");
    }

    #[tokio::test]
    #[serial]
    async fn test_get_stored_session_exists() {
        let storage = test_storage().await;
        let created = storage
            .create_session_with_id("sess-123")
            .await
            .expect("create");

        let fetched = storage.get_stored_session("sess-123").await;
        assert!(fetched.is_ok());
        let fetched = fetched.expect("fetch").expect("session exists");
        assert_eq!(fetched.id, created.id);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_stored_session_not_found() {
        let storage = test_storage().await;
        let result = storage.get_stored_session("nonexistent").await;

        assert!(result.is_ok());
        assert!(result.expect("result").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_touch_session() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let result = storage.touch_session("sess-123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_touch_session_not_found() {
        let storage = test_storage().await;
        let result = storage.touch_session("nonexistent").await;

        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::SessionNotFound { .. })));
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_session() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create");

        let result = storage.delete_session("sess-123").await;
        assert!(result.is_ok());

        let fetched = storage.get_stored_session("sess-123").await.expect("fetch");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_session_not_found() {
        let storage = test_storage().await;
        let result = storage.delete_session("nonexistent").await;

        assert!(result.is_err());
        assert!(matches!(result, Err(StorageError::SessionNotFound { .. })));
    }
}
