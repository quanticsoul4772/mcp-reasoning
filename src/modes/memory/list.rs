//! List reasoning sessions.

use crate::error::ModeError;
use crate::modes::memory::types::SessionSummary;
use crate::storage::SqliteStorage;
use sqlx::Row;

/// List all reasoning sessions with summaries.
///
/// # Arguments
///
/// * `storage` - Storage implementation
/// * `limit` - Maximum number of sessions to return
/// * `offset` - Number of sessions to skip
/// * `mode_filter` - Optional filter by reasoning mode (applied in SQL for accuracy)
///
/// # Returns
///
/// `(sessions, total_matching, has_more)` — total and has_more reflect the filter.
pub async fn list_sessions(
    storage: &SqliteStorage,
    limit: Option<u32>,
    offset: Option<u32>,
    mode_filter: Option<String>,
) -> Result<(Vec<SessionSummary>, u32, bool), ModeError> {
    let limit = limit.unwrap_or(20).min(100);
    let offset = offset.unwrap_or(0);

    // Build SQL dynamically so mode_filter is applied in the database,
    // giving accurate total/has_more counts and correct pagination.
    let mode_join = if mode_filter.is_some() {
        r"
        JOIN (
            SELECT session_id
            FROM thoughts
            GROUP BY session_id
            HAVING (SELECT mode FROM thoughts t2 WHERE t2.session_id = thoughts.session_id
                    GROUP BY mode ORDER BY COUNT(*) DESC LIMIT 1) = ?
        ) mf ON s.id = mf.session_id
        "
    } else {
        ""
    };

    let count_sql = format!("SELECT COUNT(DISTINCT s.id) FROM sessions s {mode_join}");
    let list_sql = format!(
        r"
        SELECT
            s.id,
            s.created_at,
            s.updated_at,
            COUNT(t.id) as thought_count,
            (SELECT content FROM thoughts WHERE session_id = s.id ORDER BY created_at LIMIT 1) as first_thought,
            (SELECT mode FROM thoughts WHERE session_id = s.id GROUP BY mode ORDER BY COUNT(*) DESC LIMIT 1) as primary_mode
        FROM sessions s
        LEFT JOIN thoughts t ON s.id = t.session_id
        {mode_join}
        GROUP BY s.id
        ORDER BY s.updated_at DESC
        LIMIT ? OFFSET ?
        "
    );

    // Get total count matching the filter
    let total: u32 = if let Some(ref filter) = mode_filter {
        sqlx::query_scalar(&count_sql)
            .bind(filter)
            .fetch_one(&storage.get_pool())
            .await
            .map_err(|e| ModeError::StorageError {
                message: format!("Failed to count sessions: {e}"),
            })?
    } else {
        sqlx::query_scalar(&count_sql)
            .fetch_one(&storage.get_pool())
            .await
            .map_err(|e| ModeError::StorageError {
                message: format!("Failed to count sessions: {e}"),
            })?
    };

    // Get sessions
    let rows = if let Some(ref filter) = mode_filter {
        sqlx::query(&list_sql)
            .bind(filter)
            .bind(limit)
            .bind(offset)
            .fetch_all(&storage.get_pool())
            .await
    } else {
        sqlx::query(&list_sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&storage.get_pool())
            .await
    }
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

        let preview = first_thought
            .unwrap_or_else(|| "(empty session)".to_string())
            .chars()
            .take(200)
            .collect::<String>();

        sessions.push(SessionSummary {
            session_id,
            created_at,
            updated_at,
            #[allow(clippy::cast_sign_loss)]
            thought_count: thought_count.max(0) as u32,
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
    use crate::storage::{SqliteStorage, StoredThought};

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
        let t1 = StoredThought::new(
            uuid::Uuid::new_v4().to_string(),
            &session1.id,
            "linear",
            "Test thought 1",
            0.8,
        );
        storage
            .save_stored_thought(&t1)
            .await
            .expect("save thought");

        let session2 = storage.create_session().await.expect("create session");
        let t2 = StoredThought::new(
            uuid::Uuid::new_v4().to_string(),
            &session2.id,
            "tree",
            "Test thought 2",
            0.9,
        );
        storage
            .save_stored_thought(&t2)
            .await
            .expect("save thought");

        let (listed, total, has_more) = list_sessions(&storage, None, None, None)
            .await
            .expect("list sessions");

        assert_eq!(listed.len(), 2);
        assert_eq!(total, 2);
        assert!(!has_more);
        assert_eq!(listed[0].thought_count, 1);
    }

    #[tokio::test]
    async fn test_list_with_pagination() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");

        // Create 5 test sessions
        for i in 0..5 {
            let session = storage.create_session().await.expect("create session");
            let thought = StoredThought::new(
                uuid::Uuid::new_v4().to_string(),
                &session.id,
                "linear",
                format!("Test thought {i}"),
                0.8,
            );
            storage
                .save_stored_thought(&thought)
                .await
                .expect("save thought");
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
        let t1 = StoredThought::new(
            uuid::Uuid::new_v4().to_string(),
            &session1.id,
            "linear",
            "Linear thought",
            0.8,
        );
        storage
            .save_stored_thought(&t1)
            .await
            .expect("save thought");

        let session2 = storage.create_session().await.expect("create session");
        let t2 = StoredThought::new(
            uuid::Uuid::new_v4().to_string(),
            &session2.id,
            "tree",
            "Tree thought",
            0.9,
        );
        storage
            .save_stored_thought(&t2)
            .await
            .expect("save thought");

        let (listed, _, _) = list_sessions(&storage, None, None, Some("linear".to_string()))
            .await
            .expect("list sessions");

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].primary_mode, Some("linear".to_string()));
    }

    #[tokio::test]
    async fn test_mode_filter_total_reflects_filter() {
        // Regression: total and has_more must reflect the mode filter, not all sessions.
        // Previously the filter was applied in Rust after fetching from DB, making
        // total/has_more inaccurate and pagination unreliable.
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");

        // Create 3 tree sessions and 2 linear sessions
        for _ in 0..3 {
            let s = storage.create_session().await.expect("create session");
            storage
                .save_stored_thought(&StoredThought::new(
                    uuid::Uuid::new_v4().to_string(),
                    &s.id,
                    "tree",
                    "tree thought content",
                    0.8,
                ))
                .await
                .expect("save thought");
        }
        for _ in 0..2 {
            let s = storage.create_session().await.expect("create session");
            storage
                .save_stored_thought(&StoredThought::new(
                    uuid::Uuid::new_v4().to_string(),
                    &s.id,
                    "linear",
                    "linear thought content",
                    0.8,
                ))
                .await
                .expect("save thought");
        }

        // Filter by linear: should see 2 results, total=2, has_more=false
        let (sessions, total, has_more) =
            list_sessions(&storage, None, None, Some("linear".to_string()))
                .await
                .expect("list sessions");

        assert_eq!(sessions.len(), 2);
        assert_eq!(total, 2, "total should reflect filter, not all 5 sessions");
        assert!(!has_more);
        assert!(sessions
            .iter()
            .all(|s| s.primary_mode.as_deref() == Some("linear")));
    }
}
