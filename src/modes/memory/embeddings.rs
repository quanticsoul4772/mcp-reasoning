//! Embedding generation and caching for semantic search.

use crate::error::ModeError;
use crate::storage::SqliteStorage;
use crate::traits::AnthropicClientTrait;

const SQL_GET_EMBEDDING: &str =
    "SELECT embedding_json FROM session_embeddings WHERE session_id = ?";

const SQL_STORE_EMBEDDING: &str = r#"
INSERT INTO session_embeddings (session_id, embedding_json, content_hash)
VALUES (?, ?, ?)
ON CONFLICT(session_id) DO UPDATE SET
    embedding_json = excluded.embedding_json,
    content_hash = excluded.content_hash,
    created_at = datetime('now')
"#;

const SQL_GET_SESSIONS_WITHOUT_EMBEDDINGS: &str = r#"
SELECT s.id
FROM sessions s
LEFT JOIN session_embeddings se ON s.id = se.session_id
WHERE se.session_id IS NULL
LIMIT 10
"#;

/// Get embedding for a session, generating if not cached.
pub(crate) async fn get_session_embedding<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
    session_id: &str,
) -> Result<Vec<f32>, ModeError> {
    // Check cache first
    if let Some(embedding) = get_cached_embedding(storage, session_id).await? {
        return Ok(embedding);
    }

    // Generate embedding
    let content = get_session_content(storage, session_id).await?;
    let embedding = generate_embedding(client, &content).await?;

    // Cache it
    store_embedding(storage, session_id, &embedding, &content).await?;

    Ok(embedding)
}

/// Get all session embeddings (generating missing ones).
pub(crate) async fn get_all_embeddings<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
) -> Result<Vec<(String, Vec<f32>)>, ModeError> {
    // Generate missing embeddings first
    ensure_embeddings(storage, client).await?;

    // Get all sessions
    let sessions: Vec<String> = sqlx::query_scalar("SELECT id FROM sessions")
        .fetch_all(storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError { message: format!("Failed to get sessions: {e}")))?;

    let mut embeddings = Vec::new();
    for session_id in sessions {
        if let Some(embedding) = get_cached_embedding(storage, &session_id).await? {
            embeddings.push((session_id, embedding));
        }
    }

    Ok(embeddings)
}

/// Ensure embeddings exist for all sessions (generate missing ones).
async fn ensure_embeddings<C: AnthropicClientTrait>(
    storage: &SqliteStorage,
    client: &C,
) -> Result<(), ModeError> {
    let missing: Vec<String> = sqlx::query_scalar(SQL_GET_SESSIONS_WITHOUT_EMBEDDINGS)
        .fetch_all(storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError { message: format!("Failed to get missing: {e}")))?;

    for session_id in missing {
        let content = get_session_content(storage, &session_id).await?;
        let embedding = generate_embedding(client, &content).await?;
        store_embedding(storage, &session_id, &embedding, &content).await?;
    }

    Ok(())
}

/// Get cached embedding for a session.
async fn get_cached_embedding(
    storage: &SqliteStorage,
    session_id: &str,
) -> Result<Option<Vec<f32>>, ModeError> {
    let row = sqlx::query(SQL_GET_EMBEDDING)
        .bind(session_id)
        .fetch_optional(storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError { message: format!("Failed to get embedding: {e}")))?;

    if let Some(row) = row {
        let json: String = row.get("embedding_json");
        let embedding: Vec<f32> = serde_json::from_str(&json)
            .map_err(|e| ModeError::ParseError {
                message: format!("Invalid embedding JSON: {e}"),
            })?;
        Ok(Some(embedding))
    } else {
        Ok(None)
    }
}

/// Store embedding in cache.
async fn store_embedding(
    storage: &SqliteStorage,
    session_id: &str,
    embedding: &[f32],
    content: &str,
) -> Result<(), ModeError> {
    let embedding_json = serde_json::to_string(embedding)
        .map_err(|e| ModeError::SerializationError {
            message: format!("Failed to serialize: {e}"),
        })?;

    let content_hash = format!("{:x}", md5::compute(content));

    sqlx::query(SQL_STORE_EMBEDDING)
        .bind(session_id)
        .bind(&embedding_json)
        .bind(&content_hash)
        .execute(storage.get_pool())
        .await
        .map_err(|e| ModeError::StorageError { message: format!("Failed to store embedding: {e}")))?;

    Ok(())
}

/// Get session content for embedding.
async fn get_session_content(
    storage: &SqliteStorage,
    session_id: &str,
) -> Result<String, ModeError> {
    let thoughts: Vec<String> = sqlx::query_scalar(
        "SELECT content FROM thoughts WHERE session_id = ? ORDER BY created_at LIMIT 10",
    )
    .bind(session_id)
    .fetch_all(storage.get_pool())
    .await
    .map_err(|e| ModeError::StorageError { message: format!("Failed to get thoughts: {e}")))?;

    if thoughts.is_empty() {
        return Ok(String::new());
    }

    // Combine first 10 thoughts, truncate to reasonable length
    let combined = thoughts.join(" ");
    Ok(combined.chars().take(2000).collect())
}

/// Generate embedding using Claude API.
pub(crate) async fn generate_embedding<C: AnthropicClientTrait>(
    client: &C,
    content: &str,
) -> Result<Vec<f32>, ModeError> {
    if content.is_empty() {
        // Return zero vector for empty content
        return Ok(vec![0.0; 768]);
    }

    // Use Claude to generate a representative summary, then create a simple embedding
    // In a real implementation, this would use a proper embedding model
    // For now, we'll create a simple hash-based embedding
    create_simple_embedding(content)
}

/// Create a simple embedding (placeholder for proper embedding model).
fn create_simple_embedding(content: &str) -> Result<Vec<f32>, ModeError> {
    // Simple hash-based embedding for MVP
    // In production, use proper embedding model
    let hash = md5::compute(content);
    let mut embedding = vec![0.0f32; 768];

    for (i, byte) in hash.iter().enumerate() {
        let idx = (i * 48) % 768;
        embedding[idx] = f32::from(*byte) / 255.0;
    }

    // Normalize
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in &mut embedding {
            *val /= norm;
        }
    }

    Ok(embedding)
}

/// Compute cosine similarity between two embeddings.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&c, &d)).abs() < 0.001);
    }

    #[test]
    fn test_simple_embedding() {
        let content = "This is a test";
        let embedding = create_simple_embedding(content).expect("create embedding");
        assert_eq!(embedding.len(), 768);

        // Verify normalization
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }
}
