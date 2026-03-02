//! List reasoning sessions.

use crate::error::ModeError;
use crate::modes::memory::types::SessionSummary;
use crate::storage::SqliteStorage;
use sqlx::Row;

const SQL_LIST_SESSIONS: &str = r#"
SELECT 
    s.id,
    s.created_at,
    s.updated_at,
    COUNT(t.id) as thought_count,
    (SELECT content FROM thoughts WHERE session_id = s.id ORDER BY created_at LIMIT 1) as first_thought,
    (SELECT mode FROM thoughts WHERE session_id = s.id GROUP BY mode ORDER BY COUNT(*) DESC LIMIT 1) as primary_mode
FROM sessions s
LEFT JOIN thoughts t ON s.id = t.session_id
GROUP BY s.id
ORDER BY s.updated_at DESC
LIMIT ? OFFSET ?
"#;

const SQL_COUNT_SESSIONS: &str = "SELECT COUNT(*) FROM sessions";

/// List all reasoning sessions with summaries.
///
/// # Arguments
///
/// * `storage` - Storage implementation
/// * `limit` - Maximum number of sessions to return
/// * `offset` - Number of sessions to skip
/// * `mode_filter` - Optional filter by reasoning mode
///
/// # Returns
///
/// Vector of session summaries with metadata
pub async fn list_sessions(
    storage: &SqliteStorage,
    limit: Option<u32>,
    offset: Option<u32>,
    mode_filter: Option<String>,
) -> Result<(Vec<SessionSummary>, u32, bool), ModeError> {
    let limit = limit.unwrap_or(20).min(100);
    let offset = offset.unwrap_or(0);

    // Get total count
    let total: u32 = sqlx::query_scalar(SQL_COUNT_SESSIONS)
        .fetch_one(&storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError {
            message: format!("Failed to count sessions: {e}"),
        })?;

    // Get sessions
    let rows = sqlx::query(SQL_LIST_SESSIONS)
        .bind(limit)
        .bind(offset)
        .fetch_all(&storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError {
            message: format!("Failed to list sessions: {e}"),
        })?;

    let mut sessions = Vec::new();
    for row in rows {
        let session_id: String = row.get("id");
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");
        let thought_count: i64 = row.get("thought_count");
        let first_thought: Option<String> = row.get("first_thought");
        let primary_mode: Option<String> = row.get("primary_mode");

        // Apply mode filter if specified
        if let Some(ref filter) = mode_filter {
            if let Some(ref mode) = primary_mode {
                if mode != filter {
                    continue;
                }
            } else {
                continue;
            }
        }

        let preview = first_thought
            .unwrap_or_else(|| "(empty session)".to_string())
            .chars()
            .take(200)
            .collect::<String>();

        sessions.push(SessionSummary {
            session_id,
            created_at,
            updated_at,
            thought_count: thought_count as u32,
            preview,
            primary_mode,
        });
    }

    let has_more = (offset + limit) < total;

    Ok((sessions, total, has_more))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::SqliteStorage;

    #[tokio::test]
    async fn test_list_empty_sessions() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let (sessions, total, has_more) = list_sessions(&storage, None, None, None)
            .await
            .expect("list sessions");

        assert_eq!(sessions.len(), 0);
        assert_eq!(total, 0);
        assert!(!has_more);
    }

    #[tokio::test]
    async fn test_list_with_sessions() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");

        // Create test sessions
        let session1 = storage.create_session().await.expect("create session");
        storage
            .create_thought(&session1.id, None, "linear", "Test thought 1", 0.8, None)
            .await
            .expect("create thought");

        let session2 = storage.create_session().await.expect("create session");
        storage
            .create_thought(&session2.id, None, "tree", "Test thought 2", 0.9, None)
            .await
            .expect("create thought");

        let (sessions, total, has_more) = list_sessions(&storage, None, None, None)
            .await
            .expect("list sessions");

        assert_eq!(sessions.len(), 2);
        assert_eq!(total, 2);
        assert!(!has_more);
        assert_eq!(sessions[0].thought_count, 1);
    }

    #[tokio::test]
    async fn test_list_with_pagination() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");

        // Create 5 test sessions
        for i in 0..5 {
            let session = storage.create_session().await.expect("create session");
            storage
                .create_thought(
                    &session.id,
                    None,
                    "linear",
                    &format!("Test thought {i}"),
                    0.8,
                    None,
                )
                .await
                .expect("create thought");
        }

        let (sessions, total, has_more) = list_sessions(&storage, Some(2), Some(0), None)
            .await
            .expect("list sessions");

        assert_eq!(sessions.len(), 2);
        assert_eq!(total, 5);
        assert!(has_more);

        let (sessions, _, has_more) = list_sessions(&storage, Some(2), Some(4), None)
            .await
            .expect("list sessions");

        assert_eq!(sessions.len(), 1);
        assert!(!has_more);
    }

    #[tokio::test]
    async fn test_list_with_mode_filter() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");

        let session1 = storage.create_session().await.expect("create session");
        storage
            .create_thought(&session1.id, None, "linear", "Linear thought", 0.8, None)
            .await
            .expect("create thought");

        let session2 = storage.create_session().await.expect("create session");
        storage
            .create_thought(&session2.id, None, "tree", "Tree thought", 0.9, None)
            .await
            .expect("create thought");

        let (sessions, _, _) = list_sessions(&storage, None, None, Some("linear".to_string()))
            .await
            .expect("list sessions");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].primary_mode, Some("linear".to_string()));
    }
}
