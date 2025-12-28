//! `StorageTrait` implementation for `SqliteStorage`.

#![allow(clippy::missing_errors_doc)]

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::StorageError;
use crate::traits::{Session, StorageTrait, Thought};

use super::core::SqliteStorage;
use super::types::{StoredCheckpoint, StoredThought};

#[async_trait]
impl StorageTrait for SqliteStorage {
    async fn get_session(&self, id: &str) -> Result<Option<Session>, StorageError> {
        let stored = self.get_stored_session(id).await?;
        Ok(stored.map(|s| Session::with_timestamp(s.id, s.created_at)))
    }

    async fn get_or_create_session(&self, id: Option<String>) -> Result<Session, StorageError> {
        let session_id = id.unwrap_or_else(Self::generate_id);

        // Try to get existing session
        if let Some(stored) = self.get_stored_session(&session_id).await? {
            return Ok(Session::with_timestamp(stored.id, stored.created_at));
        }

        // Create new session
        let stored = self.create_session_with_id(&session_id).await?;
        Ok(Session::with_timestamp(stored.id, stored.created_at))
    }

    async fn save_thought(&self, thought: &Thought) -> Result<(), StorageError> {
        let stored = StoredThought::new(
            &thought.id,
            &thought.session_id,
            &thought.mode,
            &thought.content,
            thought.confidence,
        )
        .with_timestamp(thought.created_at);

        self.save_stored_thought(&stored).await
    }

    async fn get_thoughts(&self, session_id: &str) -> Result<Vec<Thought>, StorageError> {
        let stored_thoughts = self.get_stored_thoughts(session_id).await?;

        Ok(stored_thoughts
            .into_iter()
            .map(|s| {
                Thought::with_timestamp(
                    &s.id,
                    &s.session_id,
                    &s.content,
                    &s.mode,
                    s.confidence,
                    s.created_at,
                )
            })
            .collect())
    }

    async fn save_checkpoint(&self, checkpoint: &StoredCheckpoint) -> Result<(), StorageError> {
        Self::save_checkpoint(self, checkpoint).await
    }

    async fn get_checkpoint(&self, id: &str) -> Result<Option<StoredCheckpoint>, StorageError> {
        Self::get_checkpoint(self, id).await
    }

    async fn get_checkpoints(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredCheckpoint>, StorageError> {
        Self::get_checkpoints(self, session_id).await
    }
}

/// Blanket implementation for `Arc<SqliteStorage>` to allow sharing storage across threads.
#[async_trait]
impl StorageTrait for Arc<SqliteStorage> {
    async fn get_session(&self, id: &str) -> Result<Option<Session>, StorageError> {
        self.as_ref().get_session(id).await
    }

    async fn get_or_create_session(&self, id: Option<String>) -> Result<Session, StorageError> {
        self.as_ref().get_or_create_session(id).await
    }

    async fn save_thought(&self, thought: &Thought) -> Result<(), StorageError> {
        self.as_ref().save_thought(thought).await
    }

    async fn get_thoughts(&self, session_id: &str) -> Result<Vec<Thought>, StorageError> {
        self.as_ref().get_thoughts(session_id).await
    }

    async fn save_checkpoint(&self, checkpoint: &StoredCheckpoint) -> Result<(), StorageError> {
        self.as_ref().save_checkpoint(checkpoint).await
    }

    async fn get_checkpoint(&self, id: &str) -> Result<Option<StoredCheckpoint>, StorageError> {
        self.as_ref().get_checkpoint(id).await
    }

    async fn get_checkpoints(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredCheckpoint>, StorageError> {
        self.as_ref().get_checkpoints(session_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::core::tests::test_storage;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_storage_trait_get_session() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create");

        let result = StorageTrait::get_session(&storage, "sess-123").await;
        assert!(result.is_ok());
        let session = result.expect("result").expect("session");
        assert_eq!(session.id, "sess-123");
    }

    #[tokio::test]
    #[serial]
    async fn test_storage_trait_get_or_create_session_new() {
        let storage = test_storage().await;

        let result =
            StorageTrait::get_or_create_session(&storage, Some("new-sess".to_string())).await;
        assert!(result.is_ok());
        let session = result.expect("result");
        assert_eq!(session.id, "new-sess");
    }

    #[tokio::test]
    #[serial]
    async fn test_storage_trait_get_or_create_session_existing() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("existing")
            .await
            .expect("create");

        let result =
            StorageTrait::get_or_create_session(&storage, Some("existing".to_string())).await;
        assert!(result.is_ok());
        let session = result.expect("result");
        assert_eq!(session.id, "existing");
    }

    #[tokio::test]
    #[serial]
    async fn test_storage_trait_get_or_create_session_generate_id() {
        let storage = test_storage().await;

        let result = StorageTrait::get_or_create_session(&storage, None).await;
        assert!(result.is_ok());
        let session = result.expect("result");
        assert!(!session.id.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_storage_trait_save_and_get_thought() {
        let storage = test_storage().await;
        storage
            .create_session_with_id("sess-123")
            .await
            .expect("create session");

        let thought = Thought::new("t-1", "sess-123", "Content", "linear", 0.8);
        let result = StorageTrait::save_thought(&storage, &thought).await;
        assert!(result.is_ok());

        let thoughts = StorageTrait::get_thoughts(&storage, "sess-123").await;
        assert!(thoughts.is_ok());
        let thoughts = thoughts.expect("thoughts");
        assert_eq!(thoughts.len(), 1);
        assert_eq!(thoughts[0].id, "t-1");
    }
}
