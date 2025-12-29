//! Error recovery and edge case tests.
//!
//! Tests how the system handles error conditions and recovers gracefully.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mcp_reasoning::storage::SqliteStorage;
use mcp_reasoning::traits::{StorageTrait, Thought};
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
async fn test_get_thoughts_for_nonexistent_session() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Get thoughts for a session that doesn't exist
    let thoughts = storage
        .get_thoughts("nonexistent-session")
        .await
        .expect("Get should work");

    // Should return empty list, not error
    assert!(thoughts.is_empty());
}

#[tokio::test]
#[serial]
async fn test_duplicate_thought_id_handling() {
    // Use in-memory storage for consistent behavior
    let storage = SqliteStorage::new_in_memory()
        .await
        .expect("Create storage");

    let session = storage
        .get_or_create_session(Some("duplicate-test".to_string()))
        .await
        .expect("Failed to create session");

    // Save first thought (Thought::new takes 5 args: id, session_id, content, mode, confidence)
    let thought1 = Thought::new("duplicate-id", &session.id, "First content", "linear", 0.85);
    storage
        .save_thought(&thought1)
        .await
        .expect("First save should work");

    // Try to save another with same ID - SQLite INSERT will fail with UNIQUE constraint
    let thought2 = Thought::new(
        "duplicate-id",
        &session.id,
        "Second content",
        "linear",
        0.90,
    );

    // SQLite with plain INSERT rejects duplicates (UNIQUE constraint violation)
    // This is expected behavior - applications should use unique IDs
    let result = storage.save_thought(&thought2).await;
    assert!(result.is_err(), "Duplicate ID should be rejected by SQLite");
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
    let thought = Thought::new("empty-thought", &session.id, "", "linear", 0.75);

    let result = storage.save_thought(&thought).await;
    assert!(result.is_ok());

    // Verify it was saved by getting all thoughts for the session
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Get should work");

    assert_eq!(thoughts.len(), 1);
    assert!(thoughts[0].content.is_empty());
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

    let thought = Thought::new("special", &session.id, content, "linear", 0.80);

    storage
        .save_thought(&thought)
        .await
        .expect("Save should work");

    // Verify by getting all thoughts
    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Get should work");

    assert_eq!(thoughts.len(), 1);
    assert_eq!(thoughts[0].content, content);
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

    let thought = Thought::new("long", &session.id, &content, "linear", 0.85);

    storage
        .save_thought(&thought)
        .await
        .expect("Save should work");

    let thoughts = storage
        .get_thoughts(&session.id)
        .await
        .expect("Get should work");

    assert_eq!(thoughts.len(), 1);
    assert_eq!(thoughts[0].content.len(), 50_000);
}

// Config tests are already covered in src/config/mod.rs unit tests.
// Environment variable manipulation during parallel test runs causes flaky behavior.
// See: tests/integration_tests.rs for properly isolated config tests using serial_test.

#[tokio::test]
#[serial]
async fn test_storage_after_session_delete() {
    let (storage, _temp_dir) = create_test_storage().await;

    // Create and populate session
    let session = storage
        .get_or_create_session(Some("to-delete".to_string()))
        .await
        .expect("Failed to create session");

    let thought = Thought::new("thought-1", &session.id, "Content", "linear", 0.85);
    storage
        .save_thought(&thought)
        .await
        .expect("Save should work");

    // Delete session
    storage
        .delete_session(&session.id)
        .await
        .expect("Delete should work");

    // Session should not exist
    let result = storage
        .get_session(&session.id)
        .await
        .expect("Get should work");
    assert!(result.is_none());
}

#[tokio::test]
#[serial]
async fn test_concurrent_session_access() {
    // Use in-memory storage for consistent behavior
    let storage = SqliteStorage::new_in_memory()
        .await
        .expect("Create storage");

    // First create the session so all tasks use the same one
    let session = storage
        .get_or_create_session(Some("concurrent-test".to_string()))
        .await
        .expect("Failed to create session");
    let session_id = session.id.clone();

    // Spawn multiple tasks saving thoughts to the same session
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let storage = storage.clone();
            let sid = session_id.clone();
            tokio::spawn(async move {
                let thought = Thought::new(
                    format!("concurrent-{i}"),
                    &sid,
                    format!("Concurrent thought {i}"),
                    "linear",
                    0.80,
                );

                storage
                    .save_thought(&thought)
                    .await
                    .expect("Save should work");
            })
        })
        .collect();

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task should complete");
    }

    // Verify all thoughts were saved
    let thoughts = storage
        .get_thoughts(&session_id)
        .await
        .expect("Get should work");

    assert_eq!(thoughts.len(), 10);
}
