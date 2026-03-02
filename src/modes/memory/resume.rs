//! Resume reasoning sessions.

use crate::error::ModeError;
use crate::modes::memory::types::{CheckpointInfo, SessionContext, ThoughtSummary};
use crate::storage::SqliteStorage;
use crate::traits::AnthropicClientTrait;
use sqlx::Row;

const SQL_GET_SESSION: &str = "SELECT id, created_at, updated_at FROM sessions WHERE id = ?";

const SQL_GET_THOUGHTS: &str = r"
SELECT id, mode, content, confidence, created_at
FROM thoughts
WHERE session_id = ?
ORDER BY created_at
";

const SQL_GET_LATEST_CHECKPOINT: &str = r"
SELECT id, name, description
FROM checkpoints
WHERE session_id = ?
ORDER BY created_at DESC
LIMIT 1
";

/// Resume a reasoning session with full context.
///
/// # Arguments
///
/// * `storage` - Storage implementation
/// * `client` - Anthropic client for compression
/// * `session_id` - Session to resume
/// * `include_checkpoints` - Whether to include checkpoint info
/// * `compress` - Whether to compress the context using Claude
///
/// # Returns
///
/// Full session context ready for continuation
pub async fn resume_session<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
    session_id: &str,
    include_checkpoints: bool,
    compress: bool,
) -> Result<SessionContext, ModeError> {
    // Get session metadata
    let session_row = sqlx::query(SQL_GET_SESSION)
        .bind(session_id)
        .fetch_optional(&storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError {
            message: format!("Failed to get session: {e}"),
        })?
        .ok_or_else(|| ModeError::NotFound {
            message: format!("Session not found: {session_id}"),
        })?;

    let created_at: String = session_row.get("created_at");

    // Get all thoughts
    let thought_rows = sqlx::query(SQL_GET_THOUGHTS)
        .bind(session_id)
        .fetch_all(&storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError {
            message: format!("Failed to get thoughts: {e}"),
        })?;

    let mut thought_chain = Vec::new();
    let mut last_mode = None;
    let mut full_content = Vec::new();

    for row in thought_rows {
        let id: String = row.get("id");
        let mode: String = row.get("mode");
        let content: String = row.get("content");
        let confidence: f64 = row.get("confidence");

        last_mode = Some(mode.clone());
        full_content.push(content.clone());

        thought_chain.push(ThoughtSummary {
            id,
            mode,
            content: content.chars().take(500).collect(),
            confidence,
        });
    }

    // Get latest checkpoint if requested
    let checkpoint = if include_checkpoints {
        let checkpoint_row = sqlx::query(SQL_GET_LATEST_CHECKPOINT)
            .bind(session_id)
            .fetch_optional(&storage.get_pool())
            .await
            .map_err(|e| ModeError::StorageError {
                message: format!("Failed to get checkpoint: {e}"),
            })?;

        checkpoint_row.map(|row| CheckpointInfo {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
        })
    } else {
        None
    };

    // Generate summary
    let summary = if compress && !full_content.is_empty() {
        compress_session(client, &full_content).await?
    } else {
        format!(
            "Session with {} thoughts using modes: {}",
            thought_chain.len(),
            last_mode.as_deref().unwrap_or("unknown")
        )
    };

    // Extract key conclusions (last few thoughts)
    let key_conclusions = thought_chain
        .iter()
        .rev()
        .take(3)
        .map(|t| t.content.clone())
        .collect();

    // Generate continuation suggestions
    let continuation_suggestions = generate_suggestions(&thought_chain, &last_mode);

    Ok(SessionContext {
        session_id: session_id.to_string(),
        created_at,
        summary,
        thought_chain,
        key_conclusions,
        last_mode,
        checkpoint,
        continuation_suggestions,
    })
}

/// Compress session content (placeholder for future Claude API compression).
#[allow(clippy::unused_async)]
async fn compress_session<C: AnthropicClientTrait>(
    _client: &C,
    thoughts: &[String],
) -> Result<String, ModeError> {
    let combined = thoughts.join("\n\n");

    // MVP: Simple truncation. Future: Use Claude API for intelligent summarization.
    if combined.len() > 1000 {
        Ok(combined.chars().take(1000).collect::<String>() + "...")
    } else {
        Ok(combined)
    }
}

/// Generate continuation suggestions based on the reasoning chain.
#[allow(clippy::ref_option)]
fn generate_suggestions(thoughts: &[ThoughtSummary], last_mode: &Option<String>) -> Vec<String> {
    let mut suggestions = Vec::new();

    if thoughts.is_empty() {
        suggestions.push("Start reasoning about a new problem".to_string());
        return suggestions;
    }

    // Suggest continuing with same mode
    if let Some(mode) = last_mode {
        suggestions.push(format!("Continue with {mode} reasoning"));
    }

    // Suggest reflection if many thoughts
    if thoughts.len() > 5 {
        suggestions.push("Use reflection mode to evaluate the reasoning quality".to_string());
    }

    // Suggest checkpoint if none exists
    suggestions.push("Create a checkpoint to save current state".to_string());

    // Suggest exploring alternatives
    if last_mode.as_deref() == Some("linear") {
        suggestions.push("Use tree mode to explore alternative paths".to_string());
    }

    suggestions
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::{SqliteStorage, StoredCheckpoint, StoredThought};
    use crate::test_utils::mock_anthropic_success;

    #[tokio::test]
    async fn test_resume_nonexistent_session() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let result = resume_session(&storage, &client, "nonexistent", false, false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resume_empty_session() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let session = storage.create_session().await.expect("create session");

        let context = resume_session(&storage, &client, &session.id, false, false)
            .await
            .expect("resume session");

        assert_eq!(context.session_id, session.id);
        assert_eq!(context.thought_chain.len(), 0);
    }

    #[tokio::test]
    async fn test_resume_session_with_thoughts() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let session = storage.create_session().await.expect("create session");
        let t1 = StoredThought::new(
            uuid::Uuid::new_v4().to_string(),
            &session.id,
            "linear",
            "Thought 1",
            0.8,
        );
        storage
            .save_stored_thought(&t1)
            .await
            .expect("save thought");
        let t2 = StoredThought::new(
            uuid::Uuid::new_v4().to_string(),
            &session.id,
            "linear",
            "Thought 2",
            0.9,
        );
        storage
            .save_stored_thought(&t2)
            .await
            .expect("save thought");

        let context = resume_session(&storage, &client, &session.id, false, false)
            .await
            .expect("resume session");

        assert_eq!(context.thought_chain.len(), 2);
        assert_eq!(context.last_mode, Some("linear".to_string()));
        assert!(!context.continuation_suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_resume_with_checkpoint() {
        let storage = SqliteStorage::new_in_memory()
            .await
            .expect("create storage");
        let client = mock_anthropic_success("", 0, 0);

        let session = storage.create_session().await.expect("create session");
        let checkpoint =
            StoredCheckpoint::new(uuid::Uuid::new_v4().to_string(), &session.id, "test", "{}")
                .with_description("Test checkpoint");
        storage
            .save_checkpoint(&checkpoint)
            .await
            .expect("save checkpoint");

        let context = resume_session(&storage, &client, &session.id, true, false)
            .await
            .expect("resume session");

        assert!(context.checkpoint.is_some());
        assert_eq!(context.checkpoint.unwrap().name, "test");
    }
}
