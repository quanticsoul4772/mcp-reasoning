//! Error recovery and edge case tests.
//!
//! Tests how the system handles error conditions and recovers gracefully.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mcp_reasoning::config::Config;
use mcp_reasoning::error::{AppError, ConfigError};
use mcp_reasoning::storage::SqliteStorage;
use mcp_reasoning::traits::StorageTrait;
use serial_test::serial;
use tempfile::TempDir;

/// Create a test database in a temporary directory.
async fn create_test_storage() -> (SqliteStorage, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let storage = SqliteStorage::new(db_path.to_str().expect("Invalid path"))
        .await
        .expect("Failed to create storage");
    (storage, temp_dir)
}

#[tokio::test]
#[serial]
async fn test_get_nonexistent_session() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Try to get a session that doesn't exist
    let result = storage.get_session("nonexistent-session-id").await;

    // Should return None, not an error
    assert!(result.is_ok());
    assert!(result.expect("Expected Ok").is_none());
}

#[tokio::test]
#[serial]
async fn test_get_nonexistent_thought() {
    let (storage, _temp_dir) = create_test_storage().await;

    let result = storage.get_thought("nonexistent-thought-id").await;

    assert!(result.is_ok());
    assert!(result.expect("Expected Ok").is_none());
}

#[tokio::test]
#[serial]
async fn test_get_thoughts_empty_session() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create session but don't add thoughts
    let session = storage
        .get_or_create_session(Some("empty-session".to_string()))
        .await
        .expect("Failed to create session");

    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Failed to get thoughts");

    assert!(thoughts.is_empty());
}

#[tokio::test]
#[serial]
async fn test_duplicate_thought_id_handling() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("duplicate-test".to_string()))
        .await
        .expect("Failed to create session");

    // Save first thought
    let thought1 = mcp_reasoning::traits::Thought::new(
        "duplicate-id",
        &session.id,
        "linear",
        "First content",
    );
    storage.save_thought(&thought1).await.expect("First save should work");

    // Try to save another with same ID - behavior depends on implementation
    // (could be upsert or error)
    let thought2 = mcp_reasoning::traits::Thought::new(
        "duplicate-id",
        &session.id,
        "linear",
        "Second content",
    );

    // Most implementations should handle this gracefully (upsert)
    let result = storage.save_thought(&thought2).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_empty_content_thought() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("empty-content".to_string()))
        .await
        .expect("Failed to create session");

    // Save thought with empty content
    let thought =
        mcp_reasoning::traits::Thought::new("empty-thought", &session.id, "linear", "");

    let result = storage.save_thought(&thought).await;
    assert!(result.is_ok());

    // Verify it was saved
    let retrieved = storage.get_thought("empty-thought").await.expect("Get should work");
    assert!(retrieved.is_some());
    assert!(retrieved.expect("Should exist").content.is_empty());
}

#[tokio::test]
#[serial]
async fn test_special_characters_in_content() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("special-chars".to_string()))
        .await
        .expect("Failed to create session");

    // Content with special characters
    let content = r#"This has "quotes", 'apostrophes', \backslashes\,
newlines,	tabs, and unicode: 日本語 🎉 émoji"#;

    let thought = mcp_reasoning::traits::Thought::new("special", &session.id, "linear", content);

    storage.save_thought(&thought).await.expect("Save should work");

    let retrieved = storage
        .get_thought("special")
        .await
        .expect("Get should work")
        .expect("Should exist");

    assert_eq!(retrieved.content, content);
}

#[tokio::test]
#[serial]
async fn test_very_long_content() {
    let (storage, _temp_dir) = create_test_storage().await;

    let session = storage
        .get_or_create_session(Some("long-content".to_string()))
        .await
        .expect("Failed to create session");

    // Very long content (50KB)
    let content = "A".repeat(50_000);

    let thought = mcp_reasoning::traits::Thought::new("long", &session.id, "linear", &content);

    storage.save_thought(&thought).await.expect("Save should work");

    let retrieved = storage
        .get_thought("long")
        .await
        .expect("Get should work")
        .expect("Should exist");

    assert_eq!(retrieved.content.len(), 50_000);
}

#[test]
fn test_config_missing_api_key() {
    // Clear environment
    std::env::remove_var("ANTHROPIC_API_KEY");

    let result = Config::from_env();

    // Should fail with missing API key error
    assert!(result.is_err());
    if let Err(AppError::Config(ConfigError::MissingApiKey)) = result {
        // Expected
    } else {
        panic!("Expected MissingApiKey error, got: {:?}", result);
    }
}

#[test]
fn test_config_with_api_key() {
    // Set environment
    std::env::set_var("ANTHROPIC_API_KEY", "test-key-123");

    let result = Config::from_env();

    // Should succeed
    assert!(result.is_ok());

    // Clean up
    std::env::remove_var("ANTHROPIC_API_KEY");
}

#[tokio::test]
#[serial]
async fn test_storage_after_session_delete() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create and populate session
    let session = storage
        .get_or_create_session(Some("to-delete".to_string()))
        .await
        .expect("Failed to create session");

    let thought =
        mcp_reasoning::traits::Thought::new("thought-1", &session.id, "linear", "Content");
    storage.save_thought(&thought).await.expect("Save should work");

    // Delete session
    storage.delete_session(&session.id).await.expect("Delete should work");

    // Session should not exist
    let result = storage.get_session(&session.id).await.expect("Get should work");
    assert!(result.is_none());
}

#[tokio::test]
#[serial]
async fn test_concurrent_session_access() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Spawn multiple tasks accessing the same session
    let storage_clone = storage.clone();

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let storage = storage_clone.clone();
            tokio::spawn(async move {
                let session = storage
                    .get_or_create_session(Some("concurrent-test".to_string()))
                    .await
                    .expect("Failed to get session");

                let thought = mcp_reasoning::traits::Thought::new(
                    &format!("concurrent-{i}"),
                    &session.id,
                    "linear",
                    &format!("Concurrent thought {i}"),
                );

                storage.save_thought(&thought).await.expect("Save should work");
            })
        })
        .collect();

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task should complete");
    }

    // Verify all thoughts were saved
    let thoughts = storage
        .get_thoughts("concurrent-test")
        .await
        .expect("Get should work");

    assert_eq!(thoughts.len(), 10);
}
