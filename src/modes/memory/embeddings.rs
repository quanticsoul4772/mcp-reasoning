//! Session content extraction for use in FTS5-based search and similarity.

use crate::error::ModeError;
use crate::storage::SqliteStorage;

/// Get session content (first 10 thoughts, up to 2000 chars).
///
/// Used by both search and relate operations to extract text content
/// from a session for FTS5 keyword queries.
pub async fn get_session_content(
    storage: &SqliteStorage,
    session_id: &str,
) -> Result<String, ModeError> {
    let thoughts: Vec<String> = sqlx::query_scalar(
        "SELECT content FROM thoughts WHERE session_id = ? ORDER BY created_at LIMIT 10",
    )
    .bind(session_id)
    .fetch_all(&storage.get_pool())
    .await
    .map_err(|e| ModeError::StorageError {
        message: format!("Failed to get thoughts: {e}"),
    })?;

    if thoughts.is_empty() {
        return Ok(String::new());
    }

    // Combine first 10 thoughts, truncate to reasonable length
    let combined = thoughts.join(" ");
    Ok(combined.chars().take(2000).collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::{SqliteStorage, StoredThought};

    #[tokio::test]
    async fn test_get_session_content_empty() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let session = storage.create_session().await.expect("create session");

        let content = get_session_content(&storage, &session.id)
            .await
            .expect("get content");

        assert!(content.is_empty());
    }

    #[tokio::test]
    async fn test_get_session_content_with_thoughts() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let session = storage.create_session().await.expect("create session");

        storage
            .save_stored_thought(&StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &session.id,
                "linear",
                "Rust programming and async patterns",
                0.8,
            ))
            .await
            .expect("save thought");

        let content = get_session_content(&storage, &session.id)
            .await
            .expect("get content");

        assert!(content.contains("Rust"));
        assert!(content.contains("async"));
    }
}
